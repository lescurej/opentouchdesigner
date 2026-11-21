use super::*;

#[test]
fn wave1_fixture() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wave1.bclip");
    let data = load(&path).unwrap();
    assert_eq!(data.channels.len(), 2);
    assert_eq!(data.channels[0].name, "chan1");
    assert_eq!(data.channels[1].name, "chan2");
    assert_eq!(data.num_samples(), 250);
    assert!((data.sample_rate - 25.0).abs() < 0.01);
    for ch in &data.channels {
        assert_eq!(ch.samples.len(), 250);
        assert!(ch.min() >= -1.01 && ch.max() <= 1.01);
    }
    assert_eq!(data.channels[0].samples, data.channels[1].samples);
    assert!((data.channels[0].samples[1] - 0.248_690).abs() < 1e-4);
}

#[test]
fn load_any_picks_format() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wave1.bclip");
    let data = load_any(&path).unwrap();
    assert_eq!(data.num_samples(), 250);
}
