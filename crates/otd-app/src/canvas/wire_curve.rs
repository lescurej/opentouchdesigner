use egui::{Color32, Pos2, Rect, Shape, Stroke, Vec2};

pub(crate) fn segment_intersects_canvas(canvas: Rect, a: Pos2, b: Pos2) -> bool {
    canvas.intersects(Rect::from_two_pos(a, b))
}

pub(crate) fn dist_to_bezier(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let pts = bezier_points(a, b);
    let chord = (a - b).length();
    let steps = ((chord / 16.0).ceil() as usize).clamp(12, 24);
    let mut best = f32::INFINITY;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let q = sample_cubic(pts, t);
        let d = (p - q).length_sq();
        if d < best {
            best = d;
        }
    }
    best.sqrt()
}

fn bezier_points(a: Pos2, b: Pos2) -> [Pos2; 4] {
    let dx = ((b.x - a.x).abs() * 0.5).max(30.0);
    [a, a + Vec2::new(dx, 0.0), b - Vec2::new(dx, 0.0), b]
}

fn sample_cubic(p: [Pos2; 4], t: f32) -> Pos2 {
    let u = 1.0 - t;
    let uu = u * u;
    let tt = t * t;
    let uuu = uu * u;
    let ttt = tt * t;
    Pos2::new(
        uuu * p[0].x + 3.0 * uu * t * p[1].x + 3.0 * u * tt * p[2].x + ttt * p[3].x,
        uuu * p[0].y + 3.0 * uu * t * p[1].y + 3.0 * u * tt * p[2].y + ttt * p[3].y,
    )
}

pub(crate) fn bezier(a: Pos2, b: Pos2, stroke: Stroke) -> Shape {
    let pts = bezier_points(a, b);
    Shape::CubicBezier(egui::epaint::CubicBezierShape::from_points_stroke(
        pts,
        false,
        Color32::TRANSPARENT,
        stroke,
    ))
}

pub(crate) fn dashed_bezier(a: Pos2, b: Pos2, stroke: Stroke) -> Shape {
    let pts = bezier_points(a, b);
    let chord = (a - b).length();
    let steps = ((chord / 14.0).ceil() as usize).clamp(8, 20);
    let mut shapes = Vec::with_capacity(steps / 2);
    let mut prev = sample_cubic(pts, 0.0);
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let cur = sample_cubic(pts, t);
        if i % 2 == 1 {
            shapes.push(Shape::line_segment([prev, cur], stroke));
        }
        prev = cur;
    }
    Shape::Vec(shapes)
}