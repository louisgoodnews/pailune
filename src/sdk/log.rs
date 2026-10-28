//! Plugin log viewer and debug loop for native ASI mods (B-17).
//!
//! Reads live log outputs written by ASI plugins into `bin64/<name>.log`
//! or `<game_dir>/<name>.log`.

use crate::error::{IoContext, Result};
use std::fs;
use std::path::Path;

/// Reads the last `limit` lines from `<game_dir>/bin64/<plugin_name>.log`
/// or `<game_dir>/<plugin_name>.log`.
pub fn read_plugin_log(game_dir: &Path, plugin_name: &str, limit: usize) -> Result<Vec<String>> {
    let clean_name = plugin_name
        .trim()
        .trim_end_matches(".asi")
        .trim_end_matches(".dll")
        .trim_end_matches(".log");
    if clean_name.is_empty() {
        return Ok(Vec::new());
    }

    let candidates = [
        game_dir.join("bin64").join(format!("{clean_name}.log")),
        game_dir.join(format!("{clean_name}.log")),
        game_dir
            .join("bin64/plugins")
            .join(format!("{clean_name}.log")),
    ];

    let log_file = candidates.into_iter().find(|p| p.exists());
    let Some(path) = log_file else {
        return Ok(Vec::new());
    };

    let content = fs::read_to_string(&path).at(&path)?;
    let lines: Vec<String> = content.lines().map(ToString::to_string).collect();

    let start = lines.len().saturating_sub(limit);
    Ok(lines[start..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_plugin_log_tail() {
        let tmp = tempfile::tempdir().unwrap();
        let bin64 = tmp.path().join("bin64");
        std::fs::create_dir_all(&bin64).unwrap();
        let log_path = bin64.join("my_mod.log");
        std::fs::write(&log_path, "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n").unwrap();

        let lines = read_plugin_log(tmp.path(), "my_mod", 3).unwrap();
        assert_eq!(lines, vec!["Line 3", "Line 4", "Line 5"]);

        let missing = read_plugin_log(tmp.path(), "non_existent", 10).unwrap();
        assert!(missing.is_empty());
    }
}
