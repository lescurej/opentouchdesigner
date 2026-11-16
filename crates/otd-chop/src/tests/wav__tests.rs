use super::*;

use super::*;

    #[test]
    fn a_wav_is_read_without_reaching_for_a_subprocess() {
        let bytes = wav_bytes(1, 1, 44100, 16, &[0, 0, 0, 0x40]);
        let dir = std::env::temp_dir().join("otd-wav-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("tone.wav");
        std::fs::write(&path, &bytes).unwrap();

        let wav = load(&path).unwrap();
        assert_eq!(wav.sample_rate, 44100.0);
        assert_eq!(wav.channels, 1);
        assert_ne!(wav.sample_rate, DECODED_RATE);
        assert_eq!(wav.samples.len(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_written_wav_reads_back() {
        let dir = std::env::temp_dir().join("otd-wav-write");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("out.wav");
        write_file(&path, 48_000.0, 2, &[0.25, -0.5, 0.0, 1.0]).unwrap();
        let wav = parse(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(wav.sample_rate, 48_000.0);
        assert_eq!(wav.channels, 2);
        assert_eq!(wav.samples, vec![0.25, -0.5, 0.0, 1.0]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn appended_frames_stay_in_the_header() {
        let dir = std::env::temp_dir().join("otd-wav-write");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("stream.wav");
        let mut out = WavOut::create(&path, 1, 60.0).unwrap();
        out.write_frames(&[0.5]).unwrap();
        out.write_frames(&[0.25, -0.25]).unwrap();
        drop(out);
        let wav = parse(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(wav.sample_rate, 60.0);
        assert_eq!(wav.samples, vec![0.5, 0.25, -0.25]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_missing_file_says_so_rather_than_blaming_the_format() {
        let err = load(std::path::Path::new("/no/such/audio.wav")).unwrap_err();
        assert!(
            err.contains("No such file") || err.contains("cannot find"),
            "{err}"
        );
    }

    #[test]
    fn ffmpegs_complaint_becomes_one_line_worth_reading() {
        let no_stream = b"[out#0/s16le @ 0x14f605e40] Output file does not contain any stream\n\
                          Error opening output file -.\n\
                          Error opening output files: Invalid argument\n";
        assert_eq!(why_it_failed(no_stream), "no audio track in that file");

        let bad_data = b"[in#0 @ 0xad2c10000] Error opening input: Invalid data found\n\
                         Error opening input files: Invalid data found\n";
        assert_eq!(
            why_it_failed(bad_data),
            "Error opening input: Invalid data found"
        );

        assert_eq!(why_it_failed(b""), "ffmpeg could not decode it");
    }

    fn wav_bytes(tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        let block = channels as u32 * bits as u32 / 8;
        out.extend_from_slice(&(rate * block).to_le_bytes());
        out.extend_from_slice(&(block as u16).to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn sixteen_bit_stereo_keeps_both_channels() {
        let mut data = Vec::new();
        for v in [i16::MAX, i16::MIN, 16384, 16384] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        let wav = parse(&wav_bytes(1, 2, 48000, 16, &data)).unwrap();
        assert_eq!(wav.sample_rate, 48000.0);
        assert_eq!(wav.channels, 2);
        assert_eq!(wav.samples.len(), 4);
        assert!(wav.samples[0] > 0.999);
        assert!(wav.samples[1] < -0.999);
        assert!((wav.samples[2] - 0.5).abs() < 1e-3);
        assert!((wav.samples[3] - 0.5).abs() < 1e-3);
    }

    #[test]
    fn float_mono_survives_exactly() {
        let mut data = Vec::new();
        for v in [0.25f32, -0.75, 1.0] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        let wav = parse(&wav_bytes(3, 1, 44100, 32, &data)).unwrap();
        assert_eq!(wav.samples, vec![0.25, -0.75, 1.0]);
        assert!((wav.duration() - 3.0 / 44100.0).abs() < 1e-9);
    }

    #[test]
    fn twenty_four_bit_reaches_full_scale() {
        let data = [0xff, 0xff, 0x7f, 0x00, 0x00, 0x80];
        let wav = parse(&wav_bytes(1, 1, 44100, 24, &data)).unwrap();
        assert!(wav.samples[0] > 0.999);
        assert!(wav.samples[1] < -0.999);
    }

    #[test]
    fn refusals_name_the_problem() {
        assert!(parse(b"not audio").unwrap_err().contains("not a WAV"));
        let mp3ish = wav_bytes(85, 1, 44100, 0, &[]);
        assert!(parse(&mp3ish).unwrap_err().contains("unsupported"));
        let mut truncated = wav_bytes(1, 1, 44100, 16, &[0, 0, 0, 0]);
        let len = truncated.len();
        truncated.truncate(len - 2);
        assert!(parse(&truncated).unwrap_err().contains("truncated"));
    }
