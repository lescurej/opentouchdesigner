use std::path::Path;

use super::geom::geometry_from;
use super::parse::read_detail;

pub fn dump_text(data: &[u8]) -> String {
    match read_detail(data) {
        Ok(detail) => {
            let mut out = String::new();
            out.push_str(&format!(
                "BtogV version 5  {} bytes\npoints {}  prims {}\npoint attributes:",
                data.len(),
                detail.npoints,
                detail.nprims
            ));
            if detail.point_attrs.is_empty() {
                out.push_str(" (none)");
            }
            for attr in &detail.point_attrs {
                out.push_str(&format!(
                    " {}/{}:type{}",
                    attr.name, attr.components, attr.type_code
                ));
            }
            out.push('\n');
            out.push_str(&format!(
                "point start {}  stride {}\n",
                detail.point_start, detail.stride
            ));
            for (i, p) in detail.points.iter().take(5).enumerate() {
                out.push_str(&format!(
                    "  [{i}] {:.6} {:.6} {:.6}  N {:.4} {:.4} {:.4}\n",
                    p.position[0],
                    p.position[1],
                    p.position[2],
                    p.normal[0],
                    p.normal[1],
                    p.normal[2]
                ));
            }
            match geometry_from(detail) {
                Ok(geo) => {
                    let (lo, hi) = geo.bounds();
                    out.push_str(&format!(
                        "cooked: {} points, {} triangles\nbounds: [{:.4},{:.4},{:.4}] .. [{:.4},{:.4},{:.4}]\n",
                        geo.num_points(),
                        geo.indices.len() / 3,
                        lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]
                    ));
                }
                Err(e) => out.push_str(&format!("cook error: {e}\n")),
            }
            out
        }
        Err(e) => format!("{e}\n"),
    }
}


pub fn dump_path(path: &Path) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("{e}"))?;
    Ok(dump_text(&data))
}
