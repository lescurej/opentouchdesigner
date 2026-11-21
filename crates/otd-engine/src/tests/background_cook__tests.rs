use super::*;

#[test]
fn resync_follows_ui_scrub_not_stale_hint() {
    let mut rt = CookContext::default();
    rt.advance_nominal_frame();
    rt.advance_nominal_frame();
    let snap = CookContext::default();
    assert!(!should_resync_time(&rt, &snap));
    let ahead = CookContext {
        time: 5.0,
        abs_time: 5.0,
        ..Default::default()
    };
    assert!(should_resync_time(&rt, &ahead));
}
