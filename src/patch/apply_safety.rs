//! Safety checks of the byte-patch engine: data tables keep their size,
//! overlapping changes are refused, offsets cannot overflow.

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
