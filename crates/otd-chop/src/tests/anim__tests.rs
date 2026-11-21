use super::*;

fn curve(keys: &[(f64, f32, Interp)]) -> Curve {
    Curve {
        keys: keys
            .iter()
            .map(|(time, value, interp)| Key {
                time: *time,
                value: *value,
                interp: *interp,
            })
            .collect(),
    }
}

#[test]
fn a_linear_segment_is_a_straight_line() {
    let c = curve(&[(0.0, 0.0, Interp::Linear), (2.0, 10.0, Interp::Linear)]);
    assert_eq!(c.sample(0.0), 0.0);
    assert_eq!(c.sample(1.0), 5.0);
    assert_eq!(c.sample(2.0), 10.0);
}

#[test]
fn outside_the_keys_the_curve_holds_rather_than_extrapolating() {
    let c = curve(&[(1.0, 3.0, Interp::Linear), (2.0, 4.0, Interp::Linear)]);
    // A value shooting off to infinity three seconds before the cue is a
    // far worse failure than one that sits still.
    assert_eq!(c.sample(-100.0), 3.0);
    assert_eq!(c.sample(100.0), 4.0);
}

#[test]
fn constant_holds_until_the_next_key_then_jumps() {
    let c = curve(&[(0.0, 0.0, Interp::Constant), (1.0, 1.0, Interp::Linear)]);
    assert_eq!(c.sample(0.0), 0.0);
    assert_eq!(c.sample(0.99), 0.0, "no creeping before the jump");
    assert_eq!(c.sample(1.0), 1.0);
}

#[test]
fn smooth_is_flat_at_both_ends_and_halfway_in_the_middle() {
    let c = curve(&[(0.0, 0.0, Interp::Smooth), (1.0, 1.0, Interp::Linear)]);
    assert_eq!(c.sample(0.5), 0.5);
    // The point of an ease: it leaves and arrives slowly.
    assert!(c.sample(0.1) < 0.1, "{}", c.sample(0.1));
    assert!(c.sample(0.9) > 0.9, "{}", c.sample(0.9));
}

#[test]
fn a_spline_passes_through_its_keys() {
    let c = curve(&[
        (0.0, 0.0, Interp::Spline),
        (1.0, 5.0, Interp::Spline),
        (2.0, 2.0, Interp::Spline),
        (3.0, 8.0, Interp::Spline),
    ]);
    // Interpolation, not approximation: whatever it does between the keys,
    // it has to hit the values that were actually authored.
    for (t, v) in [(0.0, 0.0), (1.0, 5.0), (2.0, 2.0), (3.0, 8.0)] {
        assert!((c.sample(t) - v).abs() < 1e-4, "at {t}: {}", c.sample(t));
    }
    // And it does not simply join the keys with straight lines: the
    // midpoint of the 5 -> 2 segment sits off the chord, pulled by the
    // neighbouring keys. That continuity is the reason to choose a spline.
    let chord_midpoint = 3.5;
    assert!(
        (c.sample(1.5) - chord_midpoint).abs() > 0.05,
        "a spline that lies on the chord is just linear: {}",
        c.sample(1.5)
    );
}

#[test]
fn an_empty_or_single_key_curve_does_not_divide_by_zero() {
    assert_eq!(Curve::default().sample(1.0), 0.0);
    let one = curve(&[(5.0, 2.5, Interp::Linear)]);
    assert_eq!(one.sample(0.0), 2.5);
    assert_eq!(one.sample(99.0), 2.5);
}

#[test]
fn two_keys_at_the_same_time_are_a_hard_cut() {
    let c = curve(&[
        (0.0, 0.0, Interp::Linear),
        (1.0, 1.0, Interp::Linear),
        (1.0, 5.0, Interp::Linear),
        (2.0, 5.0, Interp::Linear),
    ]);
    assert_eq!(c.sample(0.5), 0.5);
    assert!(c.sample(1.5).is_finite(), "no NaN from a zero-length span");
    assert_eq!(c.sample(1.5), 5.0);
}

#[test]
fn the_text_form_round_trips() {
    let text = "\
tx  0   0.0   linear
tx  2   1.0   smooth
ty  0   1.0   constant
";
    let (curves, bad) = Curves::parse(text);
    assert!(bad.is_empty(), "{bad:?}");
    assert_eq!(curves.0.len(), 2);
    assert_eq!(curves.0["tx"].keys.len(), 2);
    assert_eq!(curves.0["tx"].keys[1].interp, Interp::Smooth);
    assert_eq!(curves.0["ty"].keys[0].interp, Interp::Constant);

    // Writing and re-reading has to give back the same curves, or a save
    // and load would quietly change the animation.
    let (again, bad) = Curves::parse(&curves.to_text());
    assert!(bad.is_empty(), "{bad:?}");
    assert_eq!(again, curves);
}

#[test]
fn the_text_form_stays_readable_rather_than_full_of_float_noise() {
    let mut curves = Curves::default();
    curves
        .0
        .entry("tx".into())
        .or_default()
        .set(2.0, 1.0, Interp::Linear);
    let text = curves.to_text();
    assert!(text.contains("2 "), "{text}");
    assert!(
        !text.contains("2.000000"),
        "a diff full of trailing zeroes is noise: {text}"
    );
}

#[test]
fn a_bad_line_is_skipped_and_reported_not_fatal() {
    let (curves, bad) = Curves::parse(
        "\
tx 0 0 linear
this line is nonsense
tx 1 wobble linear
tx 2 1 sideways
tx 3 1
# a comment
tx 4 2 linear   # and a trailing one
",
    );
    // Losing every keyframe in the project to one typo halfway down would
    // be an unreasonable punishment.
    assert_eq!(curves.0["tx"].keys.len(), 4, "the good lines survived");
    assert_eq!(bad.len(), 3, "{bad:?}");
    assert!(bad[2].contains("sideways"), "{bad:?}");
    // An unknown interpolation still keeps the key, at linear.
    assert_eq!(curves.0["tx"].keys[2].interp, Interp::Linear);
}

#[test]
fn keys_are_sorted_however_they_were_written() {
    let (curves, _) = Curves::parse("tx 5 5 linear\ntx 1 1 linear\ntx 3 3 linear\n");
    let times: Vec<f64> = curves.0["tx"].keys.iter().map(|k| k.time).collect();
    assert_eq!(times, vec![1.0, 3.0, 5.0]);
    // And sampling relies on that ordering.
    assert_eq!(curves.0["tx"].sample(2.0), 2.0);
}

#[test]
fn setting_a_key_at_an_existing_time_replaces_it() {
    let mut c = Curve::default();
    c.set(1.0, 1.0, Interp::Linear);
    c.set(1.0, 9.0, Interp::Smooth);
    assert_eq!(c.keys.len(), 1);
    assert_eq!(c.keys[0].value, 9.0);
    assert_eq!(c.keys[0].interp, Interp::Smooth);
    assert!(c.remove_at(1.0));
    assert!(!c.remove_at(1.0));
}
