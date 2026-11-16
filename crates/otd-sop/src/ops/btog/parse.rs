use crate::Point;

use super::crypto::{decrypt_p, apply_attr};
use super::header::{HEADER_LEN, P_CHAIN_SEED};
use super::types::{Attrib, Counts, Detail, Prim, Rd};

pub(crate) fn read_detail(data: &[u8]) -> Result<Detail, String> {
    if data.len() < 16 || data[..4] != *b"Btog" || data[4] != b'V' {
        return Err("btog: invalid signature".into());
    }
    let mut rd = Rd { data, i: 5 };
    let version = rd.u32()?;
    if version != 5 {
        return Err(format!("btog: unsupported version {version}"));
    }
    if rd.remaining() < HEADER_LEN - 9 {
        return Err("btog: truncated header".into());
    }
    let counts = Counts {
        npoints: count(&mut rd, "NPoints")?,
        nprims: count(&mut rd, "NPrims")?,
        npoint_groups: count(&mut rd, "NPointGroups")?,
        nprim_groups: count(&mut rd, "NPrimGroups")?,
        npoint_attrib: count(&mut rd, "NPointAttrib")?,
        nvertex_attrib: count(&mut rd, "NVertexAttrib")?,
        nprim_attrib: count(&mut rd, "NPrimAttrib")?,
        ndetail_attrib: count(&mut rd, "NAttrib")?,
    };

    let point_attrs = read_dict(&mut rd, counts.npoint_attrib)?;
    let stride = point_stride(&point_attrs)?;
    let point_bytes = counts
        .npoints
        .checked_mul(stride)
        .ok_or_else(|| "btog: point section overflow".to_string())?;
    if rd.remaining() < point_bytes {
        return Err("btog: truncated point section".into());
    }
    let point_start = rd.i;
    let mut points = Vec::with_capacity(counts.npoints);
    let mut seed = P_CHAIN_SEED;
    for _ in 0..counts.npoints {
        let block = rd.take(16)?;
        let (xyzw, next) = decrypt_p(block, seed)?;
        seed = next;
        let mut point = Point {
            position: [xyzw[0], xyzw[1], xyzw[2]],
            ..Point::default()
        };
        for attr in &point_attrs {
            let raw = rd.take(attr.width)?;
            apply_attr(&mut point, attr, raw);
        }
        points.push(point);
    }

    let vertex_attrs = read_dict(&mut rd, counts.nvertex_attrib)?;
    let prim_attrs = read_dict(&mut rd, counts.nprim_attrib)?;
    let prim_width: usize = prim_attrs.iter().map(|a| a.width).sum();
    let has_n = named(&point_attrs, "N") || named(&vertex_attrs, "N");
    let mut filled = Fill::new(counts.npoints, &point_attrs);
    let prims = read_prims(
        &mut rd,
        &counts,
        &vertex_attrs,
        prim_width,
        &mut points,
        &mut filled,
    )?;

    let detail_attrs = read_dict(&mut rd, counts.ndetail_attrib)?;
    let detail_bytes: usize = detail_attrs.iter().map(|a| a.width).sum();
    rd.take(detail_bytes)?;
    read_groups(&mut rd, counts.npoint_groups)?;
    read_groups(&mut rd, counts.nprim_groups)?;
    read_extra(&mut rd)?;

    Ok(Detail {
        npoints: counts.npoints,
        nprims: counts.nprims,
        point_attrs,
        point_start,
        stride,
        points,
        prims,
        has_n,
    })
}
fn count(rd: &mut Rd<'_>, what: &str) -> Result<usize, String> {
    let v = rd.u32().map_err(|_| format!("btog: unreadable {what}"))?;
    Ok(v as usize)
}

fn point_stride(attrs: &[Attrib]) -> Result<usize, String> {
    let extra: usize = attrs.iter().map(|a| a.width).sum();
    16usize
        .checked_add(extra)
        .ok_or_else(|| "btog: point stride overflow".to_string())
}

fn named(attrs: &[Attrib], name: &str) -> bool {
    attrs.iter().any(|a| a.name == name)
}

fn read_dict(rd: &mut Rd<'_>, n: usize) -> Result<Vec<Attrib>, String> {
    let mut attrs = Vec::with_capacity(n);
    for _ in 0..n {
        attrs.push(read_attrib(rd)?);
    }
    Ok(attrs)
}

fn read_attrib(rd: &mut Rd<'_>) -> Result<Attrib, String> {
    let name = rd.gpd_string()?;
    let components = rd.var16()?;
    if components < 0 {
        return Err(format!("btog: attribute {name} has a negative size"));
    }
    let components = components as usize;
    let type_word = rd.u32()?;
    let type_code = type_word & 0xffff;
    let width = value_width(type_code, components)?;
    if type_code == 4 {
        let entries = rd.i32()?;
        if entries < 0 {
            return Err(format!("btog: attribute {name} has a negative index table"));
        }
        for _ in 0..entries {
            let _label = rd.gpd_string()?;
        }
    } else {
        rd.take(width)?;
    }
    Ok(Attrib {
        name,
        components,
        type_code,
        width,
    })
}

/// Bytes of one value. Type 5 is GPD's 3-float vector, but these files store
/// `components` big-endian f32s for both float (0) and vector (5): `N` is
/// size 3, type 5, 12 bytes.
fn value_width(type_code: u32, components: usize) -> Result<usize, String> {
    let each = match type_code {
        0 | 1 | 4 | 5 => 4,
        2 => 32,
        other => return Err(format!("btog: unknown attribute type {other}")),
    };
    components
        .checked_mul(each)
        .ok_or_else(|| "btog: attribute width overflow".to_string())
}

struct Fill {
    n: Vec<bool>,
    uv: Vec<bool>,
    cd: Vec<bool>,
    alpha: Vec<bool>,
}

impl Fill {
    fn new(npoints: usize, point_attrs: &[Attrib]) -> Self {
        Self {
            n: vec![named(point_attrs, "N"); npoints],
            uv: vec![named(point_attrs, "uv"); npoints],
            cd: vec![named(point_attrs, "Cd"); npoints],
            alpha: vec![named(point_attrs, "Alpha"); npoints],
        }
    }
}

fn read_prims(
    rd: &mut Rd<'_>,
    counts: &Counts,
    vertex_attrs: &[Attrib],
    prim_width: usize,
    points: &mut [Point],
    filled: &mut Fill,
) -> Result<Vec<Prim>, String> {
    let mut prims = Vec::with_capacity(counts.nprims);
    let mut left = counts.nprims;
    while left > 0 {
        let key = rd.u32()?;
        if key == 0xffff_ffff {
            let run = usize::from(rd.u16()?);
            if run == 0 || run > left {
                return Err("btog: bad primitive run".into());
            }
            let key = rd.u32()?;
            for _ in 0..run {
                prims.push(read_one_prim(
                    rd,
                    key,
                    counts.npoints,
                    vertex_attrs,
                    prim_width,
                    points,
                    filled,
                )?);
            }
            left -= run;
        } else {
            prims.push(read_one_prim(
                rd,
                key,
                counts.npoints,
                vertex_attrs,
                prim_width,
                points,
                filled,
            )?);
            left -= 1;
        }
    }
    Ok(prims)
}

fn read_one_prim(
    rd: &mut Rd<'_>,
    key: u32,
    npoints: usize,
    vertex_attrs: &[Attrib],
    prim_width: usize,
    points: &mut [Point],
    filled: &mut Fill,
) -> Result<Prim, String> {
    let prim = match key {
        1 => {
            let nv = rd.i32()?;
            if nv < 0 {
                return Err("btog: negative polygon vertex count".into());
            }
            let closed = match rd.u8()? {
                b'<' => true,
                b':' => false,
                other => {
                    return Err(format!("btog: polygon flag {other:#x}"));
                }
            };
            let mut verts = Vec::with_capacity(nv as usize);
            for _ in 0..nv {
                verts.push(read_vertex(rd, npoints, vertex_attrs, points, filled)?);
            }
            Prim::Poly { closed, verts }
        }
        0x10 => {
            let cols = nonnegative(rd.i32()?, "mesh columns")?;
            let rows = nonnegative(rd.i32()?, "mesh rows")?;
            let _flags = rd.u32()?;
            let conn = rd.i32()?;
            let nvert = cols
                .checked_mul(rows)
                .ok_or_else(|| "btog: mesh size overflow".to_string())?;
            let mut verts = Vec::with_capacity(nvert);
            for _ in 0..nvert {
                verts.push(read_vertex(rd, npoints, vertex_attrs, points, filled)?);
            }
            Prim::Mesh {
                cols,
                rows,
                conn,
                verts,
            }
        }
        other => return Err(format!("btog: unsupported primitive {other:#x}")),
    };
    rd.take(prim_width)?;
    Ok(prim)
}

fn nonnegative(v: i32, what: &str) -> Result<usize, String> {
    if v < 0 {
        Err(format!("btog: negative {what}"))
    } else {
        Ok(v as usize)
    }
}

fn read_vertex(
    rd: &mut Rd<'_>,
    npoints: usize,
    vertex_attrs: &[Attrib],
    points: &mut [Point],
    filled: &mut Fill,
) -> Result<u32, String> {
    let idx = if npoints <= 65535 {
        u32::from(rd.u16()?)
    } else {
        rd.u32()?
    };
    if idx as usize >= npoints {
        return Err(format!("btog: point index {idx} out of range"));
    }
    for attr in vertex_attrs {
        let raw = rd.take(attr.width)?;
        let slot = idx as usize;
        let take = match attr.name.as_str() {
            "N" => !filled.n[slot],
            "uv" => !filled.uv[slot],
            "Cd" => !filled.cd[slot],
            "Alpha" => !filled.alpha[slot],
            _ => false,
        };
        if take {
            apply_attr(&mut points[slot], attr, raw);
            match attr.name.as_str() {
                "N" => filled.n[slot] = true,
                "uv" => filled.uv[slot] = true,
                "Cd" => filled.cd[slot] = true,
                "Alpha" => filled.alpha[slot] = true,
                _ => {}
            }
        }
    }
    Ok(idx)
}

fn read_groups(rd: &mut Rd<'_>, count: usize) -> Result<(), String> {
    for _ in 0..count {
        let ordered = rd.data.get(rd.i) == Some(&1);
        if ordered {
            rd.u8()?;
        }
        let _name = rd.gpd_string()?;
        let n = rd.i32()?;
        if n < 0 {
            return Err("btog: negative group size".into());
        }
        let words = (n as usize)
            .checked_add(31)
            .ok_or_else(|| "btog: group mask overflow".to_string())?
            / 32;
        let bytes = words
            .checked_mul(4)
            .ok_or_else(|| "btog: group mask overflow".to_string())?;
        rd.take(bytes)?;
        if ordered {
            let nordered = rd.i32()?;
            if nordered < 0 {
                return Err("btog: negative ordered group".into());
            }
            for _ in 0..nordered {
                let primary = rd.i32()?;
                if primary < 0 {
                    let _secondary = rd.i32()?;
                }
            }
        }
    }
    Ok(())
}

fn read_extra(rd: &mut Rd<'_>) -> Result<(), String> {
    if rd.u8()? != 0x00 {
        return Err("btog: truncated file".into());
    }
    loop {
        match rd.u8()? {
            0xff => break,
            0x00 => {
                let _class = rd.i16()?;
                let _signature = rd.i16()?;
                let len = rd.i32()?;
                if len < 0 {
                    return Err("btog: bad extra packet".into());
                }
                rd.take(len as usize)?;
            }
            _ => return Err("btog: bad extra block".into()),
        }
    }
    if rd.i != rd.data.len() {
        return Err("btog: trailing bytes".into());
    }
    Ok(())
}
