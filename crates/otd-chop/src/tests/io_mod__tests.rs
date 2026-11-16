
use super::*;
    use crate::io::audio::{device_rate, latency_samples, rate_in_spans, rate_mismatch_note};
    use crate::io::midi::{apply_midi, midi_mapping, midi_messages, MidiMapping, MidiState};
    use crate::io::osc::collect_osc;

    #[test]
    fn output_latency_targets_the_setting_and_caps_a_little_above_it() {
        let (target, cap) = latency_samples(48_000.0, 200);
        assert_eq!(target, 9_600);
        assert_eq!(cap, 14_400, "100 ms of headroom above the target");
        assert_eq!(latency_samples(48_000.0, 0).0, target);
        assert_eq!(clamp_audio_latency_ms(1), AUDIO_LATENCY_MIN_MS);
        assert_eq!(clamp_audio_latency_ms(5_000), AUDIO_LATENCY_MAX_MS);
    }

    #[test]
    fn the_device_opens_at_the_project_rate_when_it_can() {
        assert_eq!(device_rate(44_100, 192_000, 44_100), 44_100);
        assert_eq!(device_rate(44_100, 192_000, 96_000), 96_000);
        assert_eq!(device_rate(44_100, 192_000, 192_000), 192_000);
        assert_eq!(device_rate(48_000, 48_000, 44_100), 48_000);
        assert_eq!(device_rate(96_000, 96_000, 48_000), 96_000);
        assert!(rate_mismatch_note(96_000.0, 48_000.0).is_some());
        assert!(rate_mismatch_note(48_000.0, 48_000.0).is_none());
    }

    #[test]
    fn a_device_that_lists_44100_first_still_opens_at_the_project_rate() {
        // The order CoreAudio actually returns: one discrete rate each, 44.1 first.
        let spans = [(44_100, 44_100), (48_000, 48_000), (96_000, 96_000)];
        assert_eq!(rate_in_spans(&spans, 48_000), 48_000);
        assert_eq!(rate_in_spans(&spans, 96_000), 96_000);
        assert_eq!(rate_in_spans(&spans, 44_100), 44_100);
        // 192 kHz is not listed, so the fallback is 48 kHz rather than the
        // highest rate — and not the 44.1 kHz entry that comes first.
        assert_eq!(rate_in_spans(&spans, 192_000), 48_000);
    }

    #[test]
    fn audio_device_menu_starts_with_default_and_keeps_stale_selection() {
        let items = audio_device_menu_items(AudioDeviceList::Input, "Old Microphone");
        assert_eq!(
            items.first().map(|item| item.option.as_str()),
            Some("(default)")
        );
        assert_eq!(items.first().map(|item| item.value.as_str()), Some(""));
        assert!(items.iter().any(|item| item.value == "Old Microphone"));
    }

    #[test]
    fn midi_notes_and_controls_are_decoded() {
        let mut s = MidiState::default();
        apply_midi(&mut s, &[0x90, 60, 100]);
        assert_eq!(s.last_note, 60.0);
        assert!((s.velocity - 100.0 / 127.0).abs() < 1e-6);
        assert!(s.notes[60] > 0.0);

        // Note on with velocity 0 is a note off, by convention.
        apply_midi(&mut s, &[0x90, 60, 0]);
        assert_eq!(s.notes[60], 0.0);
        assert_eq!(s.velocity, 0.0);

        apply_midi(&mut s, &[0xb0, 74, 127]);
        assert_eq!(s.ccs[74], 1.0);
        assert_eq!(s.seen_ccs, vec![74]);
        // A control already seen is not listed twice.
        apply_midi(&mut s, &[0xb0, 74, 0]);
        assert_eq!(s.seen_ccs, vec![74]);

        // Pitch bend is 14-bit, centred.
        apply_midi(&mut s, &[0xe0, 0, 64]);
        assert!(s.pitchbend.abs() < 1e-6);
        apply_midi(&mut s, &[0xe0, 127, 127]);
        assert!(s.pitchbend > 0.99);
    }

    #[test]
    fn a_short_or_unknown_midi_message_is_ignored() {
        let mut s = MidiState::default();
        apply_midi(&mut s, &[0x90]);
        apply_midi(&mut s, &[]);
        apply_midi(&mut s, &[0xf8, 0, 0]);
        assert_eq!(s.last_note, 0.0);
    }

    #[test]
    fn midi_out_sends_changes_and_only_changes() {
        let mut sent = HashMap::new();

        // A note crossing zero is note-on with the value as velocity.
        let on = midi_messages(1, &[("n36".into(), 1.0)], &mut sent);
        assert_eq!(on, vec![[0x90, 36, 127]]);

        // Held at a different pressure: not a new note, nothing sent.
        assert!(midi_messages(1, &[("n36".into(), 0.5)], &mut sent).is_empty());

        // Released: note-off.
        let off = midi_messages(1, &[("n36".into(), 0.0)], &mut sent);
        assert_eq!(off, vec![[0x80, 36, 0]]);

        // A control sends on change and stays quiet otherwise — MIDI is an
        // event protocol; a value repeated every frame must not be.
        let cc = midi_messages(1, &[("cc74".into(), 0.5)], &mut sent);
        assert_eq!(cc, vec![[0xb0, 74, 64]]);
        assert!(midi_messages(1, &[("cc74".into(), 0.5)], &mut sent).is_empty());

        // Channel 10 lands in the status nibble; unmapped names are ignored.
        let ten = midi_messages(10, &[("cc1".into(), 1.0), ("chan1".into(), 1.0)], &mut sent);
        assert_eq!(ten, vec![[0xb9, 1, 127]]);
    }

    #[test]
    fn midi_out_names_mirror_midi_in() {
        assert!(matches!(midi_mapping("n60"), Some(MidiMapping::Note(60))));
        assert!(matches!(
            midi_mapping("cc127"),
            Some(MidiMapping::Control(127))
        ));
        assert!(midi_mapping("cc128").is_none());
        assert!(midi_mapping("note").is_none());
        assert!(midi_mapping("velocity").is_none());
    }

    #[test]
    fn osc_addresses_become_channel_names() {
        let mut out = Vec::new();
        collect_osc(
            &rosc::OscPacket::Message(rosc::OscMessage {
                addr: "/live/track/2".into(),
                args: vec![rosc::OscType::Float(0.5)],
            }),
            &mut out,
        );
        assert_eq!(out, vec![("live/track/2".to_string(), 0.5)]);

        // Several arguments are numbered; a repeat updates in place rather
        // than growing the channel list forever.
        collect_osc(
            &rosc::OscPacket::Message(rosc::OscMessage {
                addr: "/xy".into(),
                args: vec![rosc::OscType::Float(0.1), rosc::OscType::Int(2)],
            }),
            &mut out,
        );
        assert_eq!(out.len(), 3);
        collect_osc(
            &rosc::OscPacket::Message(rosc::OscMessage {
                addr: "/xy".into(),
                args: vec![rosc::OscType::Float(0.9), rosc::OscType::Int(3)],
            }),
            &mut out,
        );
        assert_eq!(out.len(), 3);
        assert_eq!(out[1], ("xy_1".to_string(), 0.9));
    }
