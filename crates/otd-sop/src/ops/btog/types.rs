use crate::Point;

pub(crate) struct Rd<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) i: usize,
}

impl<'a> Rd<'a> {
    pub(crate) fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.i)
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .i
            .checked_add(n)
            .ok_or_else(|| "btog: offset overflow".to_string())?;
        if end > self.data.len() {
            return Err("btog: truncated".into());
        }
        let slice = &self.data[self.i..end];
        self.i = end;
        Ok(slice)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, String> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    pub(crate) fn i16(&mut self) -> Result<i16, String> {
        Ok(self.u16()? as i16)
    }

    pub(crate) fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub(crate) fn i32(&mut self) -> Result<i32, String> {
        Ok(self.u32()? as i32)
    }

    /// `int16`, or an `int32` when that `int16` is `-1`.
    pub(crate) fn var16(&mut self) -> Result<i32, String> {
        let v = self.i16()?;
        if v == -1 {
            self.i32()
        } else {
            Ok(i32::from(v))
        }
    }

    pub(crate) fn gpd_string(&mut self) -> Result<String, String> {
        let n = self.var16()?;
        if n < 0 {
            return Err("btog: negative string length".into());
        }
        let bytes = self.take(n as usize)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

pub(crate) struct Attrib {
    pub(crate) name: String,
    pub(crate) components: usize,
    pub(crate) type_code: u32,
    pub(crate) width: usize,
}

pub(crate) struct Counts {
    pub(crate) npoints: usize,
    pub(crate) nprims: usize,
    pub(crate) npoint_groups: usize,
    pub(crate) nprim_groups: usize,
    pub(crate) npoint_attrib: usize,
    pub(crate) nvertex_attrib: usize,
    pub(crate) nprim_attrib: usize,
    pub(crate) ndetail_attrib: usize,
}

pub(crate) struct Detail {
    pub(crate) npoints: usize,
    pub(crate) nprims: usize,
    pub(crate) point_attrs: Vec<Attrib>,
    pub(crate) point_start: usize,
    pub(crate) stride: usize,
    pub(crate) points: Vec<Point>,
    pub(crate) prims: Vec<Prim>,
    pub(crate) has_n: bool,
}

pub(crate) enum Prim {
    Poly {
        closed: bool,
        verts: Vec<u32>,
    },
    Mesh {
        cols: usize,
        rows: usize,
        conn: i32,
        verts: Vec<u32>,
    },
}
