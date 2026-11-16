use super::*;

use std::collections::BTreeMap;

    #[test]
    fn known_parms_are_not_gaps_and_unknown_ones_are() {
        let reg = otd_engine::registry();
        let mut files = BTreeMap::new();
        files.insert("base.n".into(), b"COMP:base\n".to_vec());
        files.insert(
            "base.cparm".into(),
            b"pages 1 Custom\n1 Label label 1 1 0 0 1 1 1 2 0 \"\" \"\" Custom 0\n".to_vec(),
        );
        files.insert(
            "base.parm".into(),
            b"?\nLabel 0 3\npageindex 0 1\nenableexternaltox 0 off\n?\n".to_vec(),
        );
        files.insert("base/lfo1.n".into(), b"CHOP:lfo\n".to_vec());
        files.insert(
            "base/lfo1.parm".into(),
            b"?\nfrequency 0 2\namp 0 18\nnotaparm 0 1\npageindex 0 1\n?\n".to_vec(),
        );

        let report = analyze_expansion(&files, &reg);
        assert!(
            report.missing.iter().all(|m| m.td_type != "CHOP:lfo"),
            "lfo is registered"
        );
        assert!(
            report
                .missing_params
                .iter()
                .any(|p| p.otd_op == "lfoCHOP" && p.td_key == "notaparm"),
            "unknown parm is a gap: {:?}",
            report.missing_params
        );
        for key in [
            "frequency",
            "amp",
            "pageindex",
            "Label",
            "enableexternaltox",
        ] {
            assert!(
                report.missing_params.iter().all(|p| p.td_key != key),
                "{key} should not be reported missing: {:?}",
                report.missing_params
            );
        }
    }

    #[test]
    fn thresh_and_info_are_known_types() {
        let reg = otd_engine::registry();
        assert!(reg.get(&td_to_otd_guess("TOP:thresh")).is_some());
        assert!(reg.get(&td_to_otd_guess("DAT:info")).is_some());
    }
