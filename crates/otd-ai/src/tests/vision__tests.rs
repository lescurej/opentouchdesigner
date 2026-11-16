use super::*;

    /// A solid-colour image of a given size, as PNG bytes.
    fn png(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
        let buffer = image::RgbaImage::from_pixel(width, height, image::Rgba(pixel));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(buffer)
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    #[test]
    fn base64_matches_the_examples_everybody_tests_against() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        // Bytes above 127 are where a hand-rolled encoder usually goes wrong.
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
        assert_eq!(base64(&[0x00, 0x00, 0x00]), "AAAA");
    }

    #[test]
    fn a_large_image_is_shrunk_to_the_long_edge() {
        // 4000×2000 is an ordinary screenshot on an ordinary monitor, and the
        // whole point is that it does not go out at that size.
        let image = Image::decode(&png(4000, 2000, [10, 20, 30, 255])).unwrap();
        assert_eq!(image.width, MAX_EDGE);
        assert_eq!(image.height, MAX_EDGE / 2);
        // Aspect ratio survives, which is the bit that breaks silently.
        assert!((image.width as f32 / image.height as f32 - 2.0).abs() < 0.01);
    }

    #[test]
    fn a_small_image_is_left_alone() {
        let image = Image::decode(&png(320, 240, [10, 20, 30, 255])).unwrap();
        assert_eq!((image.width, image.height), (320, 240));
    }

    #[test]
    fn transparency_survives_and_everything_else_becomes_a_jpeg() {
        // Opaque: JPEG, because that is where the size is.
        let opaque = Image::decode(&png(64, 64, [200, 100, 50, 255])).unwrap();
        assert_eq!(opaque.media_type(), "image/jpeg");

        // Actually transparent: PNG, because flattening it changes the
        // question being asked.
        let clear = Image::decode(&png(64, 64, [200, 100, 50, 128])).unwrap();
        assert_eq!(clear.media_type(), "image/png");
        assert!(clear.data_uri().starts_with("data:image/png;base64,"));
    }

    #[test]
    fn a_file_that_is_not_an_image_says_so_rather_than_panicking() {
        let e = Image::decode(b"this is not a png").unwrap_err();
        assert!(e.contains("not an image"), "{e}");
    }

    #[test]
    fn a_thumbnail_is_made_once_and_fits_in_the_bar() {
        let image = Image::decode(&png(800, 400, [1, 2, 3, 255])).unwrap();
        assert!(image.thumb.width <= THUMB_EDGE && image.thumb.height <= THUMB_EDGE);
        // RGBA8, so four bytes a pixel and no surprises for the UI.
        assert_eq!(
            image.thumb.rgba.len(),
            (image.thumb.width * image.thumb.height * 4) as usize
        );
    }

    #[test]
    fn the_temp_file_is_the_processed_image_not_the_original() {
        let image = Image::decode(&png(3000, 3000, [9, 9, 9, 255])).unwrap();
        let path = image.write_temp().unwrap();
        let written = std::fs::read(&path).unwrap();
        assert_eq!(written, image.bytes());
        // Same image, same path — a repeated ask does not litter the temp dir.
        assert_eq!(image.write_temp().unwrap(), path);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn nothing_prints_the_bytes() {
        // A `{:?}` on a request must not produce megabytes of base64.
        let image = Image::decode(&png(64, 64, [1, 2, 3, 255])).unwrap();
        let debugged = format!("{image:?}");
        assert!(debugged.len() < 100, "{debugged}");
        assert!(debugged.contains("64×64"), "{debugged}");
    }

    #[test]
    fn a_grabbed_frame_goes_out_opaque() {
        // A keyed camera is mostly transparent; the model should see the
        // picture, so the frame is flattened and sent as a JPEG.
        let rgba = [255u8, 0, 0, 0].repeat(16);
        let image = Image::from_rgba(4, 4, rgba).unwrap();
        assert_eq!(image.media_type(), "image/jpeg");
        assert!(Image::from_rgba(4, 4, vec![0; 3]).is_err());
    }

    #[test]
    fn the_ruler_names_what_is_wrong_with_an_output() {
        let grey = vec![128u8; 64];
        let camera_a = vec![100u8; 64];
        let camera_b = vec![140u8; 64];
        let black = vec![0u8; 64];
        let white = vec![255u8; 64];
        let faults = |a: &[u8], b: &[u8], s: &[u8], t: &[u8]| Measured::of(a, b, s, t).faults();
        assert_eq!(faults(&black, &black, &camera_a, &camera_b), ["BLACK", "STILL"]);
        assert_eq!(faults(&white, &white, &camera_a, &camera_b), ["BLOWN", "STILL"]);
        assert_eq!(faults(&camera_a, &camera_b, &camera_a, &camera_b), ["PASSTHROUGH"]);
        // Frozen output over a frozen camera is not the patch's fault.
        assert!(faults(&grey, &grey, &camera_a, &camera_a).is_empty());
        // Alive and different from the camera.
        assert!(faults(&black.iter().map(|_| 60).collect::<Vec<u8>>(), &grey, &camera_a, &camera_b).is_empty());
    }
