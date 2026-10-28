use otd_chop::data::{slice_len, slice_times};
use otd_core::CookContext;

#[test]
fn slice_length_scales_with_dt() {
    let mut ctx = CookContext {
        dt: 1.0 / 60.0,
        ..Default::default()
    };
    assert_eq!(slice_len(60.0, &ctx), 1);
    ctx.dt = 2.0 / 60.0;
    assert_eq!(slice_len(60.0, &ctx), 2);
}

#[test]
fn slice_times_end_at_now() {
    let ctx = CookContext {
        time: 5.0,
        dt: 0.1,
        ..Default::default()
    };
    let last = slice_times(&ctx, 4).last().unwrap();
    assert!((last - ctx.time).abs() < 1e-9);
}
