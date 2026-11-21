use super::ChopSpec;

pub(super) fn mark_rt_safe_chops(v: &mut [ChopSpec]) {
    const RT: &[&str] = &[
        "audiooscillatorCHOP",
        "mathCHOP",
        "mergeCHOP",
        "selectCHOP",
        "lagCHOP",
        "limitCHOP",
        "expressionCHOP",
        "audiofilterCHOP",
        "nullCHOP",
    ];
    for s in v.iter_mut() {
        if RT.contains(&s.def.type_name) {
            s.rt_safe = true;
        }
    }
}
