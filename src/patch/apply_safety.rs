//! Checks of the byte-patch engine: originals, partial apply, inserts;
//! data tables keep their size, overlapping changes are refused, offsets
//! cannot overflow, later mods override earlier ones.

mod safety {
    use super::super::apply::*;
    use crate::patch::Edit;
    use crate::patch::{Anchor, Change, FilePatch};

    fn patch(changes: Vec<Change>) -> FilePatch {
        FilePatch {
            game_file: "x".into(),
            signature: None,
            changes,
            fields: Vec::new(),
        }
    }

    fn replace(at: u64, original: &[u8], patched: &[u8]) -> Change {
        Change {
            anchor: Anchor::Offset(at),
            edit: Edit::Replace {
                original: Some(original.to_vec()),
                patched: patched.to_vec(),
            },
            label: String::new(),
        }
    }

    #[test]
    fn tables_keep_their_size_and_overlaps_are_refused() {
        let mut data = vec![1u8, 2, 3, 4];
        let grow = patch(vec![replace(0, &[1], &[9, 9])]);
        let report = apply(&mut data, &grow, None, true, true);
        assert_eq!((report.applied, data.len()), (0, 4), "{report:?}");
        let overlap = patch(vec![replace(0, &[1, 2], &[7, 7]), replace(1, &[2], &[8])]);
        let report = apply(&mut data, &overlap, None, false, true);
        assert_eq!(report.applied, 1);
        assert!(report.failed[0].contains("overlaps"), "{report:?}");
        assert_eq!(data, [7, 7, 3, 4]);
        let huge = patch(vec![replace(u64::MAX, &[1], &[2])]);
        assert_eq!(
            apply(&mut data, &huge, None, false, true).failed.len(),
            1,
            "no overflow panic"
        );
    }
}

mod basics {
    use super::super::apply::*;
    use crate::patch::Edit;
    use crate::patch::{Anchor, Change, FilePatch};

    fn replace(at: u64, original: &[u8], patched: &[u8]) -> Change {
        Change {
            anchor: Anchor::Offset(at),
            edit: Edit::Replace {
                original: Some(original.to_vec()),
                patched: patched.to_vec(),
            },
            label: String::new(),
        }
    }

    #[test]
    fn verifies_originals_and_respects_partial_apply() {
        let patch = |changes| FilePatch {
            game_file: "x".into(),
            signature: None,
            changes,
            fields: Vec::new(),
        };
        let mut data = vec![0u8, 1, 2, 3, 4, 5];
        let good = patch(vec![replace(1, &[1, 2], &[9, 9]), replace(4, &[4], &[7])]);
        assert_eq!(apply(&mut data, &good, None, false, false).applied, 2);
        assert_eq!(data, [0, 9, 9, 3, 7, 5]);
        assert_eq!(
            apply(&mut data, &good, None, false, false).already,
            2,
            "idempotent"
        );
        let bad = patch(vec![replace(0, &[0], &[5]), replace(3, &[42], &[1])]);
        let strict = apply(&mut data, &bad, None, false, false);
        assert_eq!(
            (strict.applied, strict.failed.len(), data[0]),
            (0, 1, 0),
            "nothing applied"
        );
        assert_eq!(apply(&mut data, &bad, None, false, true).applied, 1);
        assert_eq!(data[0], 5);
        let insert = patch(vec![Change {
            anchor: Anchor::Offset(1),
            edit: Edit::Insert { bytes: vec![0xAA] },
            label: "ins".into(),
        }]);
        assert_eq!(
            apply(&mut data, &insert, None, true, false).failed.len(),
            1,
            "no inserts into tables"
        );
        apply(&mut data, &insert, None, false, false);
        assert_eq!(data[..3], [5, 0xAA, 9]);
    }
}

mod later_wins {
    use super::super::apply::*;
    use crate::patch::{Anchor, Change, Edit, FilePatch};

    #[test]
    fn originals_may_match_the_unmodded_bytes() {
        let patch = FilePatch {
            game_file: "x".into(),
            signature: None,
            changes: vec![Change {
                anchor: Anchor::Offset(1),
                edit: Edit::Replace {
                    original: Some(vec![1, 0]),
                    patched: vec![9, 0],
                },
                label: String::new(),
            }],
            fields: Vec::new(),
        };
        let vanilla = [0u8, 1, 0, 3];
        // An earlier mod made it 0x03e7; this mod still expects vanilla 1.
        let mut data = vec![0u8, 0xe7, 0x03, 3];
        assert_eq!(
            apply(&mut data.clone(), &patch, None, true, false).applied,
            0
        );
        let report = apply_over(&mut data, &patch, None, true, false, Some(&vanilla));
        assert_eq!((report.applied, data.as_slice()), (1, &[0, 9, 0, 3][..]));
        // A reference of another length (an insert moved things) is ignored.
        let mut data = vec![0u8, 0xe7, 0x03, 3];
        let moved = apply_over(&mut data, &patch, None, true, false, Some(&vanilla[1..]));
        assert_eq!(moved.applied, 0);
    }
}
