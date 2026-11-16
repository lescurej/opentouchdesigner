use otd_core::{Family, OpRegistry};
use otd_engine::{comp, register_all, registry, replicator};

#[test]
fn register_all_builds_the_same_registry_as_registry() {
    let mut from_all = OpRegistry::new();
    register_all(&mut from_all);
    let from_fn = registry();
    let mut all_names: Vec<_> = from_all.iter().map(|d| d.type_name).collect();
    let mut fn_names: Vec<_> = from_fn.iter().map(|d| d.type_name).collect();
    all_names.sort_unstable();
    fn_names.sort_unstable();
    assert_eq!(all_names, fn_names);
}

#[test]
fn every_table_spec_is_in_the_engine_registry() {
    let reg = registry();
    for spec in otd_gpu::ops::all() {
        assert!(
            reg.get(spec.def.type_name).is_some(),
            "TOP `{}` is not in the engine registry",
            spec.def.type_name
        );
    }
    assert!(
        reg.get(otd_gpu::ops::WINDOW).is_some(),
        "windowCOMP is not in the engine registry"
    );
    for spec in otd_chop::ops::all() {
        assert!(
            reg.get(spec.def.type_name).is_some(),
            "CHOP `{}` is not in the engine registry",
            spec.def.type_name
        );
    }
    for spec in otd_dat::ops::all() {
        assert!(
            reg.get(spec.def.type_name).is_some(),
            "DAT `{}` is not in the engine registry",
            spec.def.type_name
        );
    }
    for spec in otd_sop::ops::all() {
        assert!(
            reg.get(spec.def.type_name).is_some(),
            "SOP `{}` is not in the engine registry",
            spec.def.type_name
        );
    }
    for def in otd_gpu::scene::defs() {
        assert!(
            reg.get(def.type_name).is_some(),
            "scene `{}` is not in the engine registry",
            def.type_name
        );
    }
    assert!(
        reg.get(comp::BASE).is_some(),
        "baseCOMP is not in the engine registry"
    );
    assert!(
        reg.get(replicator::REPLICATOR).is_some(),
        "replicatorCOMP is not in the engine registry"
    );
}

#[test]
fn every_cookable_registry_entry_has_a_family_spec() {
    let reg = registry();
    for def in reg.iter() {
        let ok = match def.family {
            Family::Top => otd_gpu::ops::spec(def.type_name).is_some(),
            Family::Chop => otd_chop::ops::spec_for(def.type_name).is_some(),
            Family::Dat => otd_dat::ops::spec_for(def.type_name).is_some(),
            Family::Sop => otd_sop::ops::spec_for(def.type_name).is_some(),
            Family::Mat | Family::Comp => true,
        };
        assert!(
            ok,
            "`{}` ({:?}) is registered but has no cook table entry",
            def.type_name, def.family
        );
    }
}

#[test]
fn family_register_all_covers_every_spec_table_entry() {
    let mut reg = OpRegistry::new();
    otd_gpu::ops::register_all(&mut reg);
    for spec in otd_gpu::ops::all() {
        assert!(
            reg.get(spec.def.type_name).is_some(),
            "TOP {}",
            spec.def.type_name
        );
    }
    assert!(reg.get(otd_gpu::ops::WINDOW).is_some());

    let mut reg = OpRegistry::new();
    otd_chop::ops::register_all(&mut reg);
    for spec in otd_chop::ops::all() {
        assert!(
            reg.get(spec.def.type_name).is_some(),
            "CHOP {}",
            spec.def.type_name
        );
    }

    let mut reg = OpRegistry::new();
    otd_dat::ops::register_all(&mut reg);
    for spec in otd_dat::ops::all() {
        assert!(
            reg.get(spec.def.type_name).is_some(),
            "DAT {}",
            spec.def.type_name
        );
    }

    let mut reg = OpRegistry::new();
    otd_sop::ops::register_all(&mut reg);
    for spec in otd_sop::ops::all() {
        assert!(
            reg.get(spec.def.type_name).is_some(),
            "SOP {}",
            spec.def.type_name
        );
    }
}
