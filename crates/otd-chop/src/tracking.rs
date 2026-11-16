//! Connected bright regions; deliberately no person recognition or identity.
#[derive(Debug, Default)]
pub(crate) struct Blob {
    pub count: usize,
    pub area: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub(crate) fn largest(width: usize, height: usize, rgba: &[f32], threshold: f32, min_area: f32) -> Blob {
    let Some(size) = width.checked_mul(height) else { return Blob::default(); };
    if size == 0 || size > 4_194_304 || rgba.len() / 4 != size {
        return Blob::default();
    }
    let mut bright: Vec<bool> = rgba.chunks_exact(4)
        .map(|p| (0.2126*p[0] + 0.7152*p[1] + 0.0722*p[2]) * p[3] > threshold)
        .collect();
    let mut result = Blob::default();
    let mut largest_size = 0;
    let mut stack = Vec::new();
    for start in 0..size {
        if !bright[start] { continue; }
        bright[start] = false;
        stack.push(start);
        let (mut n, mut sx, mut sy) = (0usize, 0usize, 0usize);
        let (mut minx, mut miny, mut maxx, mut maxy) = (width, height, 0, 0);
        while let Some(i) = stack.pop() {
            let (x, y) = (i % width, i / width);
            n += 1; sx += x; sy += y;
            minx = minx.min(x); miny = miny.min(y);
            maxx = maxx.max(x); maxy = maxy.max(y);
            let neighbours = [
                (x > 0).then(|| i - 1), (x + 1 < width).then(|| i + 1),
                (y > 0).then(|| i - width), (y + 1 < height).then(|| i + width),
            ];
            for next in neighbours.into_iter().flatten() {
                if bright[next] { bright[next] = false; stack.push(next); }
            }
        }
        let area = n as f32 / size as f32;
        if area < min_area { continue; }
        result.count += 1;
        if n > largest_size {
            largest_size = n;
            result.area = area;
            result.x = (sx as f32 / n as f32 + 0.5) / width as f32;
            result.y = (sy as f32 / n as f32 + 0.5) / height as f32;
            result.width = (maxx - minx + 1) as f32 / width as f32;
            result.height = (maxy - miny + 1) as f32 / height as f32;
        }
    }
    result
}

#[cfg(test)]
#[path = "tests/tracking__tests.rs"]
mod tests;
