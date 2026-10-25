//! Following a file's update chain (`files.json` → `file_updates`).

use super::{FileList, FileUpdate};

/// The newest file that supersedes `installed`, if any. Nexus records each
/// upload of a newer version as `old_file_id → new_file_id`; the chain is
/// followed to its end (with a guard against cycles).
pub fn latest_successor(updates: &[FileUpdate], installed: u64) -> Option<u64> {
    let next = |id: &u64| {
        updates
            .iter()
            .find(|u| u.old_file_id == *id)
            .map(|u| u.new_file_id)
    };
    let last = std::iter::successors(Some(installed), next)
        .take(64)
        .last()?;
    (last != installed).then_some(last)
}

/// Human-readable label of the successor: its version, else its name.
pub fn describe(list: &FileList, file_id: u64) -> String {
    let file = list.files.iter().find(|f| f.file_id == file_id);
    let named = list
        .file_updates
        .iter()
        .find(|u| u.new_file_id == file_id)
        .map(|u| u.new_file_name.clone());
    file.map(|f| f.version.clone())
        .filter(|v| !v.is_empty())
        .or(file.map(|f| f.name.clone()))
        .or(named)
        .unwrap_or_else(|| format!("file {file_id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(old: u64, new: u64) -> FileUpdate {
        FileUpdate {
            old_file_id: old,
            new_file_id: new,
            new_file_name: format!("f{new}"),
        }
    }

    #[test]
    fn follows_the_chain_to_its_end() {
        let chain = [step(1, 2), step(2, 5), step(9, 10)];
        assert_eq!(latest_successor(&chain, 1), Some(5));
        assert_eq!(latest_successor(&chain, 5), None);
        assert_eq!(latest_successor(&chain, 7), None);
        // A (broken) cyclic chain terminates instead of looping forever.
        assert!(latest_successor(&[step(1, 2), step(2, 1)], 1).is_some());
        let list = FileList {
            files: vec![],
            file_updates: chain.to_vec(),
        };
        assert_eq!(describe(&list, 5), "f5");
    }
}
