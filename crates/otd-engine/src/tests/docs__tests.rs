use super::*;

#[test]
fn every_operator_and_parameter_is_documented() {
    // The enforcement PLAN.md wanted from review. An operator arriving
    // without prose fails the build rather than being noticed, or not.
    let missing = undocumented(&crate::registry());
    assert!(
        missing.is_empty(),
        "{} undocumented:\n{}",
        missing.len(),
        missing.join("\n")
    );
}

#[test]
fn the_reference_covers_every_registered_operator() {
    let reg = crate::registry();
    let text = reference(&reg);
    for def in reg.iter() {
        assert!(
            text.contains(def.type_name),
            "{} is missing from the reference",
            def.type_name
        );
    }
    // And it is a document, not a dump: an index, headings, parameter
    // tables.
    assert!(text.contains("# Operator reference"));
    assert!(text.contains("| Parameter | Name | Default | Range |"));
}

#[test]
fn a_page_reports_what_the_editor_would_show() {
    let reg = crate::registry();
    let def = reg.get("levelTOP").unwrap();
    let page = operator_page(def);
    assert!(page.contains("`levelTOP`"));
    assert!(page.contains("Brightness"), "{page}");
    assert!(page.contains("`brightness`"), "{page}");
    // Defaults and ranges come from the operator itself, so they cannot
    // disagree with the parameter panel.
    assert!(page.contains("0 … 4"), "{page}");
}

#[test]
fn a_menu_parameter_lists_its_choices() {
    let reg = crate::registry();
    let page = operator_page(reg.get("rampTOP").unwrap());
    assert!(page.contains("`horizontal`"), "{page}");
    assert!(page.contains("`radial`"), "{page}");
}

#[test]
fn index_links_point_at_headings_that_exist() {
    let reg = crate::registry();
    let text = reference(&reg);
    // A reference whose links go nowhere is worse than one without links.
    for def in reg.iter() {
        let link = format!("](#{})", anchor(def));
        assert!(text.contains(&link), "no index link for {}", def.type_name);
        let heading = format!("### {} — `{}`", def.label, def.type_name);
        assert!(text.contains(&heading), "no heading for {}", def.type_name);
    }
}

#[test]
fn time_dependent_operators_say_so() {
    let reg = crate::registry();
    // Whether an operator cooks every frame is the single fact that most
    // often explains "why is my patch slow", so the page has to state it.
    assert!(operator_page(reg.get("lfoCHOP").unwrap()).contains("Time dependent"));
    assert!(!operator_page(reg.get("levelTOP").unwrap()).contains("Time dependent"));
}
