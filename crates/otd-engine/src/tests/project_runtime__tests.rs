use super::*;

#[test]
fn summarize_reports_the_typical_frame_and_the_worst_one() {
    let mut v = vec![2.0; 50];
    v.push(40.0);
    let (median, _, max) = summarize(v);
    assert_eq!(median, 2.0);
    assert_eq!(max, 40.0);
}

#[test]
fn summarize_survives_an_empty_run() {
    assert_eq!(summarize(vec![]), (0.0, 0.0, 0.0));
}
