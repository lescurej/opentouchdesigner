use super::*;

#[test]
fn math_combine_channels_choice_survives_reload() {
    let mut p = Patch::new();
    let m = p.add("mathCHOP", "math1");
    p.set(m, "chanop", Value::Select("add".into()));

    let text = otd_core::Project::from_graph(&p.graph, &p.reg, 60.0)
        .to_ron()
        .unwrap();
    let loaded = otd_core::Project::from_ron(&text)
        .unwrap()
        .to_graph(&p.reg)
        .unwrap();
    let id = loaded.find("/math1").unwrap();
    let chanop = loaded.node(id).param("chanop").unwrap();
    assert_eq!(chanop.value.as_str(), "add");
    assert_eq!(chanop.menu_index(&chanop.value.as_str()), 1);

    let legacy = r#"(
  format: "opentouchdesigner-project",
  version: 1,
  fps: 60.0,
  nodes: [
    (
      path: "/math1",
      op: "mathCHOP",
      pos: (0.0, 0.0),
      params: {
        "chanop": (
          page: "Math",
          value: Select("add"),
        ),
      },
    ),
  ],
)
"#;
    let loaded = otd_core::Project::from_ron(legacy)
        .unwrap()
        .to_graph(&p.reg)
        .unwrap();
    let id = loaded.find("/math1").unwrap();
    assert_eq!(
        loaded.node(id).param("chanop").unwrap().value.as_str(),
        "add"
    );
}

#[test]
fn math_combine_channels_multiplies_into_one() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "c1");
    let m = p.add("mathCHOP", "math1");
    p.set(c, "rows", Value::Int(2));
    p.set(c, "name0", Value::Str("a".into()));
    p.set(c, "name1", Value::Str("b".into()));
    p.set(c, "value0", Value::Float(2.0));
    p.set(c, "value1", Value::Float(3.0));
    p.graph.connect(c, m, 0).unwrap();
    p.set(m, "combine", Value::Str("off".into()));
    p.set(m, "chanop", Value::Str("multiply".into()));
    p.run(m, 1);
    assert_eq!(p.data(m).num_channels(), 1);
    assert_eq!(p.value(m, "a"), 6.0);
}

#[test]
fn math_range_and_mult_add() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "c1");
    let m = p.add("mathCHOP", "math1");
    p.set(c, "value0", Value::Float(0.5));
    p.graph.connect(c, m, 0).unwrap();
    p.set(m, "combine", Value::Str("off".into()));
    p.set(m, "fromrange1", Value::Float(0.0));
    p.set(m, "fromrange2", Value::Float(1.0));
    p.set(m, "torange1", Value::Float(0.0));
    p.set(m, "torange2", Value::Float(100.0));
    p.set(m, "preoff", Value::Float(0.0));
    p.set(m, "gain", Value::Float(1.0));
    p.set(m, "offset", Value::Float(0.0));
    p.run(m, 1);
    assert!((p.value(m, "chan1") - 50.0).abs() < 0.01);
}

#[test]
fn a_math_chop_broadcasts_a_single_channel_across_many() {
    let mut p = Patch::new();
    let many = p.add("constantCHOP", "many");
    let one = p.add("constantCHOP", "one");
    let math = p.add("mathCHOP", "math1");
    p.set(many, "rows", Value::Int(3));
    p.set(many, "name0", Value::Str("chan1".into()));
    p.set(many, "name1", Value::Str("chan2".into()));
    p.set(many, "name2", Value::Str("chan3".into()));
    for (i, v) in [1.0, 2.0, 3.0].iter().enumerate() {
        p.set(many, &format!("value{i}"), Value::Float(*v));
    }
    p.set(one, "value0", Value::Float(10.0));
    p.set(one, "name0", Value::Str("gain".into()));
    p.graph.connect(many, math, 0).unwrap();
    p.graph.connect(one, math, 1).unwrap();
    p.set(math, "combine", Value::Str("multiply".into()));

    p.run(math, 1);
    assert_eq!(p.data(math).num_channels(), 3);
    assert_eq!(p.value(math, "chan1"), 10.0);
    assert_eq!(p.value(math, "chan3"), 30.0);
}

#[test]
fn select_reads_channels_from_chop_parameter_without_a_wire() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let sel = p.add("selectCHOP", "sel");
    p.set(c, "name0", Value::Str("level".into()));
    p.set(c, "value0", Value::Float(7.0));
    p.set(sel, "chop", Value::Str("/src".into()));
    p.set(sel, "channames", Value::Str("level".into()));
    p.run(sel, 1);
    assert_eq!(p.data(sel).names(), vec!["level"]);
    assert_eq!(p.value(sel, "level"), 7.0);
}

#[test]
fn a_select_chop_matches_names_with_wildcards() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    let sel = p.add("selectCHOP", "select1");
    p.set(c, "name0", Value::Str("knob[1-4]".into()));
    p.graph.connect(c, sel, 0).unwrap();
    p.set(sel, "channames", Value::Str("knob1 knob4".into()));
    p.run(sel, 1);
    assert_eq!(p.data(sel).names(), vec!["knob1", "knob4"]);

    p.set(sel, "channames", Value::Str("knob*".into()));
    p.set(sel, "renameto", Value::Str("a b".into()));
    p.run(sel, 1);
    assert_eq!(p.data(sel).names(), vec!["a", "b", "knob3", "knob4"]);
}

#[test]
fn merge_chop_accepts_more_than_two_inputs() {
    let mut p = Patch::new();
    let a = p.add("constantCHOP", "a");
    let b = p.add("constantCHOP", "b");
    let c = p.add("constantCHOP", "c");
    let merge = p.add("mergeCHOP", "merge1");
    p.set(a, "name0", Value::Str("a".into()));
    p.set(b, "name0", Value::Str("b".into()));
    p.set(c, "name0", Value::Str("c".into()));
    p.set(a, "value0", Value::Float(1.0));
    p.set(b, "value0", Value::Float(2.0));
    p.set(c, "value0", Value::Float(3.0));
    p.graph.connect(a, merge, 0).unwrap();
    p.graph.connect(b, merge, 1).unwrap();
    p.graph.connect(c, merge, 2).unwrap();
    p.run(merge, 1);
    assert_eq!(p.data(merge).names(), vec!["a", "b", "c"]);
}

#[test]
fn a_merge_chop_disambiguates_duplicate_names() {
    let mut p = Patch::new();
    let a = p.add("constantCHOP", "a");
    let b = p.add("constantCHOP", "b");
    let merge = p.add("mergeCHOP", "merge1");
    p.graph.connect(a, merge, 0).unwrap();
    p.graph.connect(b, merge, 1).unwrap();
    p.set(a, "name0", Value::Str("chan1".into()));
    p.set(b, "name0", Value::Str("chan1".into()));
    p.set(a, "value0", Value::Float(1.0));
    p.set(b, "value0", Value::Float(2.0));
    p.run(merge, 1);

    // Both inputs call their channel `chan1`; an ambiguous name would make
    // an export unresolvable.
    assert_eq!(p.data(merge).names(), vec!["chan1", "chan11"]);
    assert_eq!(p.value(merge, "chan1"), 1.0);
    assert_eq!(p.value(merge, "chan11"), 2.0);
}

#[test]
fn shuffle_transposes_channels_into_samples() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let s = p.add("shuffleCHOP", "shuffle1");
    p.graph.connect(c, s, 0).unwrap();
    p.set(c, "rows", Value::Int(3));
    p.set(c, "name0", Value::Str("chan1".into()));
    p.set(c, "name1", Value::Str("chan2".into()));
    p.set(c, "name2", Value::Str("chan3".into()));
    p.set(c, "value0", Value::Float(0.1));
    p.set(c, "value1", Value::Float(0.2));
    p.set(c, "value2", Value::Float(0.3));
    p.run(s, 1);

    // Three channels of one sample become one channel of three.
    assert_eq!(p.data(s).num_channels(), 1);
    assert_eq!(p.data(s).nth(0).unwrap().samples.len(), 3);
    let got = &p.data(s).nth(0).unwrap().samples;
    assert!((got[0] - 0.1).abs() < 1e-6 && (got[2] - 0.3).abs() < 1e-6);
}

#[test]
fn reorder_sorts_channels_by_numeric_suffix() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let r = p.add("reorderCHOP", "reorder1");
    p.graph.connect(c, r, 0).unwrap();
    p.set(c, "rows", Value::Int(3));
    p.set(c, "name0", Value::Str("chan10".into()));
    p.set(c, "name1", Value::Str("chan2".into()));
    p.set(c, "name2", Value::Str("chan1".into()));
    p.set(r, "method", Value::Str("numsuffix".into()));
    p.run(r, 1);
    let names: Vec<_> = p
        .data(r)
        .channels
        .iter()
        .map(|ch| ch.name.clone())
        .collect();
    assert_eq!(names, vec!["chan1", "chan2", "chan10"]);
}

#[test]
fn rename_numbers_the_channels_it_matches() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let r = p.add("renameCHOP", "rename1");
    p.graph.connect(c, r, 0).unwrap();
    p.set(c, "name0", Value::Str("chan[1-3]".into()));
    p.set(r, "to", Value::Str("led[1-]".into()));
    p.run(r, 1);

    assert_eq!(p.data(r).names(), vec!["led1", "led2", "led3"]);
}

#[test]
fn select_matches_chan_1_3_pattern() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let sel = p.add("selectCHOP", "sel");
    p.graph.connect(c, sel, 0).unwrap();
    p.set(c, "name0", Value::Str("chan[1-5]".into()));
    p.set(sel, "channames", Value::Str("chan[2-4]".into()));
    p.run(sel, 1);
    assert_eq!(p.data(sel).names(), vec!["chan2", "chan3", "chan4"]);
}

#[test]
fn select_rename_to_spreads_numbered_pattern() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let sel = p.add("selectCHOP", "sel");
    p.graph.connect(c, sel, 0).unwrap();
    p.set(c, "name0", Value::Str("chan[1-3]".into()));
    p.set(sel, "channames", Value::Str("*".into()));
    p.set(sel, "renameto", Value::Str("led[1-]".into()));
    p.run(sel, 1);
    assert_eq!(p.data(sel).names(), vec!["led1", "led2", "led3"]);
}

#[test]
fn rename_from_matches_numbered_pattern() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let r = p.add("renameCHOP", "rename1");
    p.graph.connect(c, r, 0).unwrap();
    p.set(c, "name0", Value::Str("chan[1-4]".into()));
    p.set(r, "from", Value::Str("chan[2-3]".into()));
    p.set(r, "to", Value::Str("x[1-]".into()));
    p.run(r, 1);
    assert_eq!(p.data(r).names(), vec!["chan1", "x1", "x2", "chan4"]);
}

#[test]
fn analyze_reduces_a_waveform_to_one_number() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "wave");
    let a = p.add("analyzeCHOP", "peak");
    p.graph.connect(pat, a, 0).unwrap();
    p.set(pat, "length", Value::Int(100));
    p.set(a, "function", Value::Str("maximum".into()));
    p.run(a, 1);

    assert_eq!(p.data(a).num_samples(), 1);
    let peak = p.value(a, "chan1");
    let raw = p.data(pat).nth(0).unwrap().max();
    assert!((peak - raw).abs() < 1e-6, "{peak} vs {raw}");
}

#[test]
fn an_equal_power_cross_does_not_dip_in_the_middle() {
    // The reason the curve exists: at the halfway point a linear blend of two
    // full-scale signals reads 1.0 where each input read 1.0, but the *power*
    // has dropped. Equal power holds it up at ~0.707 each, summing to ~1.41.
    let mut p = Patch::new();
    let a = p.add("constantCHOP", "a");
    let b = p.add("constantCHOP", "b");
    let x = p.add("crossCHOP", "cross1");
    p.graph.connect(a, x, 0).unwrap();
    p.graph.connect(b, x, 1).unwrap();
    p.set(a, "value0", Value::Float(1.0));
    p.set(b, "value0", Value::Float(1.0));

    p.run(x, 1);
    assert!(
        (p.value(x, "chan1") - 1.0).abs() < 1e-5,
        "linear at the middle"
    );

    p.set(x, "curve", Value::Str("equalpower".into()));
    p.run(x, 1);
    let v = p.value(x, "chan1");
    assert!((v - 1.414).abs() < 0.01, "equal power held at {v}");
}

#[test]
fn a_fan_chop_fans_out_an_index_and_back_in() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    p.set(c, "value0", Value::Float(2.0));
    let out = p.add("fanCHOP", "fan1");
    p.graph.connect(c, out, 0).unwrap();
    let back = p.add("fanCHOP", "fan2");
    p.set(back, "operation", Value::Str("fanin".into()));
    p.graph.connect(out, back, 0).unwrap();
    p.run(back, 1);
    assert_eq!(p.data(out).names(), ["chan0", "chan1", "chan2", "chan3"]);
    assert_eq!(p.value(out, "chan2"), 1.0);
    assert_eq!(p.value(out, "chan1"), 0.0);
    assert_eq!(p.value(back, "chan1"), 2.0);
    p.set(c, "value0", Value::Float(9.0));
    p.run(back, 1);
    assert_eq!(
        p.value(back, "chan1"),
        3.0,
        "out-of-range index clamps to the last fan channel"
    );
}

#[test]
fn fan_quantize_off_splits_fraction_between_channels() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    p.set(c, "value0", Value::Float(2.7));
    let out = p.add("fanCHOP", "fan1");
    p.graph.connect(c, out, 0).unwrap();
    p.set(out, "quantize", Value::Str("off".into()));
    p.run(out, 1);
    assert!((p.value(out, "chan2") - 0.3).abs() < 1e-5);
    assert!((p.value(out, "chan3") - 0.7).abs() < 1e-5);
    p.set(out, "quantize", Value::Str("on".into()));
    p.run(out, 1);
    assert_eq!(p.value(out, "chan2"), 1.0);
    assert_eq!(p.value(out, "chan3"), 0.0);
}
