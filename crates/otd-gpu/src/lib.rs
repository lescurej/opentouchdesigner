//! `otd-gpu` — the TOP (texture operator) engine.
//!
//! Everything here is wgpu; nothing here knows about the editor. The public
//! surface is [`TopEngine`], which implements `otd_core::Cooker`.

pub mod context;
pub mod demo;
pub mod engine;
pub mod isf;
pub mod math;
pub mod ops;
pub mod ramp;
pub mod ramp_dat;
pub mod record;
pub mod render3d;
pub mod scene;
pub mod shader;
pub mod text;
pub mod texture;
pub mod video;

pub use context::GpuContext;
pub use engine::TopEngine;
pub use render3d::OrbitCamera;
pub use texture::{TOP_FORMAT, TopTexture};

use otd_core::OpRegistry;

pub fn register_all(registry: &mut OpRegistry) {
    ops::register_all(registry);
    scene::register_all(registry);
}

/// Copy a TOP's texture back to the CPU as 8-bit RGBA.
///
/// Slow by design — it stalls the pipeline. It exists for tests and for
/// still-image export, not for the render path.
pub fn read_pixels_rgba8(
    ctx: &GpuContext,
    tex: &TopTexture,
) -> Result<(u32, u32, Vec<u8>), String> {
    let (w, h, floats) = read_pixels_rgba_f32(ctx, tex)?;
    let out = floats
        .into_iter()
        .map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
        .collect();
    Ok((w, h, out))
}

/// The same copy, keeping the full 16-bit float range.
///
/// TOPs are `Rgba16Float` throughout — that is the headline the README makes —
/// so anything that reads pixels back in order to *compute* with them, rather
/// than to write a PNG, has to see the values that are actually there.
/// A TOP to CHOP that clamped to 0..1 would quietly destroy the HDR range the
/// rest of the pipeline goes to some trouble to keep.
pub struct PendingPixelReadback {
    width: u32,
    height: u32,
    unpadded: u64,
    padded: u64,
    buffer: wgpu::Buffer,
    done: std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
}

pub fn start_read_pixels_rgba_f32(
    ctx: &GpuContext,
    tex: &TopTexture,
) -> Result<PendingPixelReadback, String> {
    let (w, h) = (tex.key.width, tex.key.height);
    let unpadded = w as u64 * 8;
    let padded = unpadded.div_ceil(256) * 256;

    let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("otd readback"),
        size: padded * h as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("otd readback"),
        });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &tex.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    ctx.queue.submit(Some(encoder.finish()));

    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });

    Ok(PendingPixelReadback {
        width: w,
        height: h,
        unpadded,
        padded,
        buffer,
        done: rx,
    })
}

fn decode_readback_rgba_f32(
    pending: &PendingPixelReadback,
) -> Result<Vec<f32>, String> {
    let w = pending.width;
    let h = pending.height;
    let padded = pending.padded;
    let unpadded = pending.unpadded;
    let slice = pending.buffer.slice(..);
    let data = slice.get_mapped_range();
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as usize {
        let row = &data[y * padded as usize..y * padded as usize + unpadded as usize];
        for x in 0..w as usize {
            for c in 0..4 {
                let i = x * 8 + c * 2;
                let bits = u16::from_le_bytes([row[i], row[i + 1]]);
                out.push(f16_to_f32(bits));
            }
        }
    }
    drop(data);
    pending.buffer.unmap();
    Ok(out)
}

pub fn poll_pixel_readback(
    ctx: &GpuContext,
    pending: &mut PendingPixelReadback,
) -> Result<Option<(u32, u32, Vec<f32>)>, String> {
    let _ = ctx.device.poll(wgpu::PollType::Poll);
    match pending.done.try_recv() {
        Ok(Ok(())) => {
            let w = pending.width;
            let h = pending.height;
            let out = decode_readback_rgba_f32(pending)?;
            Ok(Some((w, h, out)))
        }
        Ok(Err(e)) => Err(format!("buffer map failed: {e:?}")),
        Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            Err("readback channel closed".into())
        }
    }
}

pub fn read_pixels_rgba_f32(
    ctx: &GpuContext,
    tex: &TopTexture,
) -> Result<(u32, u32, Vec<f32>), String> {
    let mut pending = start_read_pixels_rgba_f32(ctx, tex)?;
    loop {
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("device poll failed: {e}"))?;
        if let Some(done) = poll_pixel_readback(ctx, &mut pending)? {
            return Ok(done);
        }
    }
}

/// IEEE 754 binary32 -> binary16, round-to-nearest-even.
///
/// The inverse of [`f16_to_f32`], for the CPU side of an upload into a
/// `Rgba16Float` texture. Values beyond half's range saturate to infinity
/// rather than wrapping, which is what a shader reading them would want.
pub(crate) fn f32_to_f16(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp = ((bits >> 23) & 0xff) as i32;
    let mant = bits & 0x7f_ffff;

    if exp == 0xff {
        // Inf or NaN. A NaN must stay a NaN: zeroing the mantissa would turn
        // it into an infinity.
        let m = if mant != 0 { 0x200 } else { 0 };
        return sign | 0x7c00 | m;
    }
    let unbiased = exp - 127 + 15;
    if unbiased >= 0x1f {
        return sign | 0x7c00; // Overflow to infinity.
    }
    if unbiased <= 0 {
        if unbiased < -10 {
            return sign; // Smaller than the smallest subnormal.
        }
        let m = mant | 0x80_0000;
        let shift = (14 - unbiased) as u32;
        let half = (m >> shift) as u16;
        // Round to nearest even.
        let round = (m >> (shift - 1)) & 1;
        return sign | (half + round as u16);
    }
    let half = ((unbiased as u16) << 10) | (mant >> 13) as u16;
    let round = if (mant & 0x1000) != 0 && ((mant & 0x0fff) != 0 || (half & 1) == 1) {
        1
    } else {
        0
    };
    sign | (half + round)
}

/// IEEE 754 binary16 -> binary32.
fn f16_to_f32(bits: u16) -> f32 {
    let sign = ((bits >> 15) & 1) as u32;
    let exp = ((bits >> 10) & 0x1f) as u32;
    let mant = (bits & 0x3ff) as u32;
    let f = match exp {
        0 => {
            if mant == 0 {
                sign << 31
            } else {
                // Subnormal: renormalise. Its value is `mant * 2^-24`, so
                // shifting the mantissa left until the implicit bit appears
                // leaves `(1 + m/1024) * 2^(-14 - shifts)`, and the biased
                // binary32 exponent is `127 - 14 - shifts`.
                let mut shifts = 0i32;
                let mut m = mant;
                while m & 0x400 == 0 {
                    m <<= 1;
                    shifts += 1;
                }
                let m = m & 0x3ff;
                (sign << 31) | (((113 - shifts) as u32) << 23) | (m << 13)
            }
        }
        0x1f => (sign << 31) | (0xff << 23) | (mant << 13),
        _ => (sign << 31) | ((exp + 127 - 15) << 23) | (mant << 13),
    };
    f32::from_bits(f)
}

#[cfg(test)]
#[path = "tests/lib__tests.rs"]
mod tests;
