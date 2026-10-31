use otd_core::indexmap::IndexMap;
use otd_core::Param;

macro_rules! params {
    ($($key:expr => $param:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut m: IndexMap<String, Param> = IndexMap::new();
        $( m.insert($key.into(), $param); )*
        m
    }};
}

pub(crate) fn no_params() -> IndexMap<String, Param> {
    params! {}
}
