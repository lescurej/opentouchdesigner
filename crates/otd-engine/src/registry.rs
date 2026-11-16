use otd_core::OpRegistry;

use super::{comp, geotext_comp, replicator, text_comp};
use otd_panel::panel;

pub fn register_all(registry: &mut OpRegistry) {
    otd_gpu::register_all(registry);
    otd_chop::register_all(registry);
    otd_dat::register_all(registry);
    otd_sop::register_all(registry);
    comp::register_all(registry);
    panel::register(registry);
    text_comp::register(registry);
    geotext_comp::register(registry);
    replicator::register_all(registry);
}

pub fn registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    register_all(&mut r);
    r
}
