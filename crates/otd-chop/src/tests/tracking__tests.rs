use super::*;
#[test]
fn finds_largest_region_without_joining_across_row_boundaries() {
    let mut p = vec![0.0; 10 * 10 * 4];
    for (x, y) in (2..6)
        .flat_map(|x| (3..7).map(move |y| (x, y)))
        .chain([(9, 0), (0, 1)])
    {
        p[(y * 10 + x) * 4..(y * 10 + x + 1) * 4].fill(1.0);
    }
    let b = largest(10, 10, &p, 0.5, 0.0);
    assert_eq!(b.count, 3);
    assert!((b.x - 0.4).abs() < 0.001 && (b.y - 0.5).abs() < 0.001);
    assert!((b.area - 0.16).abs() < 0.001);
    assert_eq!(largest(10, 10, &p, 0.5, 0.02).count, 1);
}
#[test]
fn missing_dark_and_transparent_inputs_are_empty() {
    assert_eq!(largest(0, 0, &[], 0.5, 0.0).count, 0);
    assert_eq!(largest(2, 2, &[0.0; 16], 0.5, 0.0).count, 0);
    assert_eq!(largest(1, 1, &[1.0, 1.0, 1.0, 0.0], 0.5, 0.0).count, 0);
    assert_eq!(largest(2, 2, &[1.0; 3], 0.5, 0.0).count, 0);
}
