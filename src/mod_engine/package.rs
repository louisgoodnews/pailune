//! Packaging staged mods for distribution on Nexus Mods.
//!
//! Creates installable `.zip` archives containing mod files, metadata
//! (`modinfo.json`), and documentation (`README.md`).

use crate::error::{IoContext, PailuneError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

/// Metadata for packaging a mod.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageMetadata {
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub bump_version: bool,
}

/// Result summary of packaging a mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageResult {
    pub archive_path: PathBuf,
    pub file_count: usize,
    pub total_bytes: u64,
}

/// Helper to bump a numeric or semantic version string (e.g. "1.0" -> "1.1", "1.0.2" -> "1.0.3").
pub fn bump_version_string(version: &str) -> String {
    let trimmed = version.trim();
    if trimmed.is_empty() {
        return "1.0".to_string();
    }
    let parts: Vec<&str> = trimmed.split('.').collect();
    if let Some(last) = parts.last() {
        if let Ok(num) = last.parse::<u64>() {
            let mut bumped = parts[..parts.len() - 1].to_vec();
            let next = (num + 1).to_string();
            bumped.push(&next);
            return bumped.join(".");
        }
    }
    format!("{trimmed}.1")
}

/// Packages a staged mod into an installable distribution zip archive.
pub fn package_mod(
    staging_dir: &Path,
    mod_name: &str,
    export_dir: &Path,
    meta: &PackageMetadata,
) -> Result<PackageResult> {
    let mod_dir = staging_dir.join(mod_name);
    if !mod_dir.is_dir() {
        return Err(PailuneError::Invalid(format!(
            "Mod '{mod_name}' not found in staging"
        )));
    }

    fs::create_dir_all(export_dir).at(export_dir)?;

    let clean_ver = meta.version.trim().replace(' ', "_");
    let version_str = match clean_ver.is_empty() {
        true => "1.0".to_string(),
        false => clean_ver,
    };

    // 1. Update or write modinfo.json in the mod directory
    let modinfo_path = mod_dir.join("modinfo.json");
    let modinfo = serde_json::json!({
        "name": meta.name.trim(),
        "version": version_str,
        "author": meta.author.trim(),
        "description": meta.description.trim()
    });
    fs::write(&modinfo_path, serde_json::to_string_pretty(&modinfo)?).at(&modinfo_path)?;

    // 2. Also update version in any .field.json if present
    if let Ok(entries) = fs::read_dir(&mod_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "json") && p != modinfo_path {
                if let Ok(text) = fs::read_to_string(&p) {
                    if let Ok(mut doc) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(info) = doc.get_mut("modinfo").and_then(|i| i.as_object_mut()) {
                            info.insert("version".into(), serde_json::json!(version_str));
                            if let Ok(updated) = serde_json::to_string_pretty(&doc) {
                                let _ = fs::write(&p, updated + "\n");
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Ensure README.md exists
    let readme_path = mod_dir.join("README.md");
    if !readme_path.exists() {
        let content = format!(
            "# {}\n\n**Version:** {}\n**Author:** {}\n\n{}\n\n## Installation\n\nInstall using the **Pailune** mod manager or extract directly into your Crimson Desert installation.\n",
            meta.name.trim(),
            version_str,
            if meta.author.trim().is_empty() {
                "Unknown"
            } else {
                meta.author.trim()
            },
            meta.description.trim()
        );
        fs::write(&readme_path, content).at(&readme_path)?;
    }

    // 4. Create the distribution zip archive
    let sanitized_name = crate::mod_engine::install::sanitize_name(mod_name);
    let archive_name = format!("{sanitized_name}-v{version_str}.zip");
    let archive_path = export_dir.join(&archive_name);

    let file = fs::File::create(&archive_path).at(&archive_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let mut file_count = 0;
    for entry in WalkDir::new(&mod_dir).min_depth(1).into_iter().flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let rel = path
            .strip_prefix(&mod_dir)
            .map_err(|e| PailuneError::Invalid(e.to_string()))?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        // Skip internal/temporary files
        if rel_str.starts_with(".pailune") || rel_str.ends_with(".tmp") {
            continue;
        }

        zip.start_file(&rel_str, SimpleFileOptions::default())?;
        let mut source = fs::File::open(path).at(path)?;
        std::io::copy(&mut source, &mut zip).at(path)?;
        file_count += 1;
    }

    zip.finish()?;

    let total_bytes = fs::metadata(&archive_path).map(|m| m.len()).unwrap_or(0);

    Ok(PackageResult {
        archive_path,
        file_count,
        total_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_version_string() {
        assert_eq!(bump_version_string("1.0"), "1.1");
        assert_eq!(bump_version_string("1.0.4"), "1.0.5");
        assert_eq!(bump_version_string("2"), "3");
        assert_eq!(bump_version_string(""), "1.0");
        assert_eq!(bump_version_string("beta"), "beta.1");
    }

    #[test]
    fn test_package_mod_creates_zip_with_metadata_and_readme() {
        let temp = tempfile::tempdir().unwrap();
        let staging = temp.path().join("staging");
        let exports = temp.path().join("exports");
        let mod_dir = staging.join("MyCarryMod");
        fs::create_dir_all(&mod_dir).unwrap();

        let json_path = mod_dir.join("MyCarryMod.field.json");
        fs::write(
            &json_path,
            r#"{"format":3,"modinfo":{"title":"MyCarryMod","version":"1.0"},"targets":[]}"#,
        )
        .unwrap();

        let meta = PackageMetadata {
            name: "MyCarryMod".into(),
            author: "Tester".into(),
            version: "1.1".into(),
            description: "Increased carry weight".into(),
            bump_version: true,
        };

        let res = package_mod(&staging, "MyCarryMod", &exports, &meta).unwrap();
        assert!(res.archive_path.exists());
        assert_eq!(res.file_count, 3); // field.json, modinfo.json, README.md
        assert!(res.total_bytes > 0);

        // Verify zip contains the expected files
        let file = fs::File::open(&res.archive_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().unwrap().to_string())
            .collect();

        assert!(names.contains(&"modinfo.json".to_string()));
        assert!(names.contains(&"README.md".to_string()));
        assert!(names.contains(&"MyCarryMod.field.json".to_string()));
    }
}
