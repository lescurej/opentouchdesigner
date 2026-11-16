use cidre::cv;

use super::super::Frame;

pub(crate) fn pixel_buf_to_frame(pb: &cv::PixelBuf) -> Option<Frame> {
    let w = pb.width() as u32;
    let h = pb.height() as u32;
    if w == 0 || h == 0 {
        return None;
    }
    let fmt = pb.pixel_format();
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    let lock = cv::pixel_buffer::LockFlags::DEFAULT;
    unsafe {
        let pb_mut = pb as *const cv::PixelBuf as *mut cv::PixelBuf;
        (*pb_mut).lock_base_addr(lock).result().ok()?;
        if fmt == cv::PixelFormat::_32_BGRA {
            copy_bgra_to_rgba(pb, w, h, &mut pixels);
        } else if fmt == cv::PixelFormat::_420F || fmt == cv::PixelFormat::_420V {
            copy_420_to_rgba(pb, w, h, fmt == cv::PixelFormat::_420F, &mut pixels);
        } else {
            let _ = (*pb_mut).unlock_lock_base_addr(lock);
            return None;
        }
        let _ = (*pb_mut).unlock_lock_base_addr(lock);
    }
    Some(Frame {
        width: w,
        height: h,
        pixels,
    })
}

fn copy_bgra_to_rgba(pb: &cv::PixelBuf, w: u32, h: u32, out: &mut [u8]) {
    let stride = pb.plane_bytes_per_row(0).max(w as usize * 4);
    let base = pb.plane_base_address(0);
    if !base.is_null() && pb.plane_count() >= 1 {
        for y in 0..h as usize {
            for x in 0..w as usize {
                let i = y * stride + x * 4;
                let o = (y * w as usize + x) * 4;
                unsafe {
                    out[o] = *base.add(i + 2);
                    out[o + 1] = *base.add(i + 1);
                    out[o + 2] = *base.add(i);
                    out[o + 3] = *base.add(i + 3);
                }
            }
        }
        return;
    }
    let base = pixel_base_address(pb);
    let stride = pixel_bytes_per_row(pb).max(w as usize * 4);
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * stride + x * 4;
            let o = (y * w as usize + x) * 4;
            unsafe {
                out[o] = *base.add(i + 2);
                out[o + 1] = *base.add(i + 1);
                out[o + 2] = *base.add(i);
                out[o + 3] = *base.add(i + 3);
            }
        }
    }
}

fn copy_420_to_rgba(pb: &cv::PixelBuf, w: u32, h: u32, full_range: bool, out: &mut [u8]) {
    let y_plane = pb.plane_base_address(0);
    let uv_plane = pb.plane_base_address(1);
    if y_plane.is_null() || uv_plane.is_null() {
        return;
    }
    let y_stride = pb.plane_bytes_per_row(0);
    let uv_stride = pb.plane_bytes_per_row(1);
    for y in 0..h as usize {
        for x in 0..w as usize {
            let yo = y * y_stride + x;
            let uvo = (y / 2) * uv_stride + (x / 2) * 2;
            let yv = unsafe { *y_plane.add(yo) } as i32;
            let u = unsafe { *uv_plane.add(uvo) } as i32;
            let v = unsafe { *uv_plane.add(uvo + 1) } as i32;
            let (y_adj, u_adj, v_adj) = if full_range {
                (yv, u - 128, v - 128)
            } else {
                (yv - 16, u - 128, v - 128)
            };
            let yf = y_adj as f32 * 1.164;
            let r = (yf + 1.596 * v_adj as f32).clamp(0.0, 255.0) as u8;
            let g = (yf - 0.392 * u_adj as f32 - 0.813 * v_adj as f32).clamp(0.0, 255.0) as u8;
            let b = (yf + 2.017 * u_adj as f32).clamp(0.0, 255.0) as u8;
            let o = (y * w as usize + x) * 4;
            out[o] = r;
            out[o + 1] = g;
            out[o + 2] = b;
            out[o + 3] = 255;
        }
    }
}

fn pixel_base_address(pb: &cv::PixelBuf) -> *const u8 {
    unsafe { CVPixelBufferGetBaseAddress(pb) as *const u8 }
}

fn pixel_bytes_per_row(pb: &cv::PixelBuf) -> usize {
    unsafe { CVPixelBufferGetBytesPerRow(pb) }
}

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    fn CVPixelBufferGetBaseAddress(pixel_buffer: *const cv::PixelBuf) -> *mut std::ffi::c_void;
    fn CVPixelBufferGetBytesPerRow(pixel_buffer: *const cv::PixelBuf) -> usize;
}