//! Project generator and mod packaging pipeline for the Pailune Modding SDK.
//!
//! Scaffolds standalone ASI (Rust/C++) and JSON (Format 3) projects with
//! build configs, bundled libraries, entry points, and documentation.

use crate::error::{IoContext, PailuneError, Result};
use crate::mod_engine::package::{PackageMetadata, PackageResult, package_mod};
use std::fs;
use std::path::{Path, PathBuf};

/// Supported mod project types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    /// Native Windows DLL / ASI plugin (Rust cdylib or C++).
    Asi,
    /// Format-3 field-name patch document.
    Json,
}

impl ProjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Asi => "asi",
            Self::Json => "json",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "asi" | "dll" | "native" | "rust" => Some(Self::Asi),
            "json" | "format3" | "format-3" | "table" => Some(Self::Json),
            _ => None,
        }
    }
}

/// Configuration used to scaffold a new mod project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    pub name: String,
    pub mod_type: ProjectType,
    pub author: String,
    pub version: String,
    pub description: String,
    pub target_dir: PathBuf,
}

/// Report returned after scaffolding a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldReport {
    pub project_dir: PathBuf,
    pub files_created: Vec<String>,
    pub mod_type: ProjectType,
}

/// Scaffolds a new mod project at `cfg.target_dir.join(&cfg.name)`.
pub fn scaffold_project(cfg: &ProjectConfig) -> Result<ScaffoldReport> {
    let clean_name = crate::mod_engine::install::sanitize_name(&cfg.name);
    if clean_name.is_empty() {
        return Err(PailuneError::Invalid("project name cannot be empty".into()));
    }

    let project_dir = cfg.target_dir.join(&clean_name);
    if project_dir.exists() {
        return Err(PailuneError::Invalid(format!(
            "destination directory already exists: {}",
            project_dir.display()
        )));
    }

    fs::create_dir_all(&project_dir).at(&project_dir)?;
    let mut files_created = Vec::new();

    let version_str = match cfg.version.trim().is_empty() {
        true => "1.0",
        false => cfg.version.trim(),
    };
    let author_str = match cfg.author.trim().is_empty() {
        true => "Unknown",
        false => cfg.author.trim(),
    };
    let desc_str = cfg.description.trim();

    match cfg.mod_type {
        ProjectType::Asi => {
            // 1. Cargo.toml
            let cargo_toml = format!(
                r#"[package]
name = "{clean_name}"
version = "{version_str}"
edition = "2021"
authors = ["{author_str}"]
description = "{desc_str}"

[lib]
crate-type = ["cdylib"]
name = "{clean_name}"

[dependencies]
windows-sys = {{ version = "0.59", features = [
    "Win32_Foundation",
    "Win32_System_SystemServices",
    "Win32_System_LibraryLoader",
    "Win32_System_Memory",
    "Win32_UI_WindowsAndMessaging",
    "Win32_System_Threading",
] }}

[profile.release]
opt-level = 3
lto = true
strip = true
panic = "abort"
"#
            );
            let p = project_dir.join("Cargo.toml");
            fs::write(&p, cargo_toml).at(&p)?;
            files_created.push("Cargo.toml".into());

            // 2. .cargo/config.toml (Cross-compilation for Windows from Linux)
            let cargo_config_dir = project_dir.join(".cargo");
            fs::create_dir_all(&cargo_config_dir).at(&cargo_config_dir)?;
            let cargo_config = r#"[build]
target = "x86_64-pc-windows-gnu"

[target.x86_64-pc-windows-gnu]
rustflags = ["-C", "target-feature=+crt-static"]
"#;
            let p = cargo_config_dir.join("config.toml");
            fs::write(&p, cargo_config).at(&p)?;
            files_created.push(".cargo/config.toml".into());

            // 3. src/lib.rs
            let src_dir = project_dir.join("src");
            fs::create_dir_all(&src_dir).at(&src_dir)?;
            let lib_rs = format!(
                r#"//! {clean_name} — Crimson Desert ASI Plugin
//!
//! Created with Pailune Modding SDK.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Once;
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{{BOOL, HINSTANCE, TRUE}};
use windows_sys::Win32::System::SystemServices::{{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH}};

static INIT: Once = Once::new();

/// Logs a message to `{clean_name}.log` in the game directory.
pub fn log(msg: &str) {{
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("{clean_name}.log")
    {{
        let _ = writeln!(f, "[{clean_name}] {{msg}}");
    }}
}}

/// Background worker thread for initialization.
fn plugin_main() {{
    log("Plugin attached to process");
    thread::sleep(Duration::from_millis(500));
    log("Ready.");
}}

/// Windows DLL entry point.
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(
    _hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *mut std::ffi::c_void,
) -> BOOL {{
    match fdw_reason {{
        DLL_PROCESS_ATTACH => {{
            INIT.call_once(|| {{
                thread::spawn(plugin_main);
            }});
            TRUE
        }}
        DLL_PROCESS_DETACH => {{
            log("Plugin detached");
            TRUE
        }}
        _ => TRUE,
    }}
}}
"#
            );
            let p = src_dir.join("lib.rs");
            fs::write(&p, lib_rs).at(&p)?;
            files_created.push("src/lib.rs".into());

            // 4. config.ini
            let config_ini = format!(
                r#"# {clean_name} Configuration
# Managed by Pailune Mod Manager

[General]
Enabled = 1
LogLevel = info

[Settings]
# Add custom plugin settings here
Multiplier = 1.0
"#
            );
            let p = project_dir.join("config.ini");
            fs::write(&p, config_ini).at(&p)?;
            files_created.push("config.ini".into());

            // 5. modinfo.json
            let modinfo = serde_json::json!({
                "name": clean_name,
                "version": version_str,
                "author": author_str,
                "description": desc_str,
                "type": "asi"
            });
            let p = project_dir.join("modinfo.json");
            fs::write(&p, serde_json::to_string_pretty(&modinfo)?).at(&p)?;
            files_created.push("modinfo.json".into());

            // 6. README.md
            let readme = format!(
                r#"# {clean_name}

**Version:** {version_str}  
**Author:** {author_str}  
**Type:** ASI Native Plugin  

{desc_str}

## Building

Build the 64-bit Windows DLL using the MinGW cross-compiler target:

```bash
cargo build --release --target x86_64-pc-windows-gnu
```

The compiled library will be at:
`target/x86_64-pc-windows-gnu/release/{clean_name}.dll`

## Deploying with Pailune

1. Rename the DLL to `{clean_name}.asi` (or keep it as `{clean_name}.dll`).
2. Place `{clean_name}.asi`, `config.ini`, and `modinfo.json` in your Pailune staging folder or use Pailune's export command:
   ```bash
   pailune export-project .
   ```
3. Enable and Apply via Pailune!
"#
            );
            let p = project_dir.join("README.md");
            fs::write(&p, readme).at(&p)?;
            files_created.push("README.md".into());

            // 7. DEVELOPMENT.md
            let dev_doc = format!(
                r#"# {clean_name} — Development Guide

## Architecture

Crimson Desert loads ASI plugins via the proxy loader (`winmm.dll`). When the game launches:
1. `winmm.dll` initializes and scans the game directory and plugins for `.asi` binaries.
2. `DllMain` is called with `DLL_PROCESS_ATTACH`.
3. To prevent deadlocks in `DllMain`, all heavy initialization occurs in a spawned background thread.

## Debugging

Log output is written to `{clean_name}.log` in the game directory. Use Pailune's Log tab to view real-time events.
"#
            );
            let p = project_dir.join("DEVELOPMENT.md");
            fs::write(&p, dev_doc).at(&p)?;
            files_created.push("DEVELOPMENT.md".into());
        }

        ProjectType::Json => {
            // 1. modinfo.json
            let modinfo = serde_json::json!({
                "name": clean_name,
                "version": version_str,
                "author": author_str,
                "description": desc_str,
                "type": "json"
            });
            let p = project_dir.join("modinfo.json");
            fs::write(&p, serde_json::to_string_pretty(&modinfo)?).at(&p)?;
            files_created.push("modinfo.json".into());

            // 2. {name}.field.json (Format-3 template)
            let field_json = serde_json::json!({
                "format": 3,
                "modinfo": {
                    "title": clean_name,
                    "version": version_str,
                    "author": author_str,
                    "description": desc_str
                },
                "targets": [
                    {
                        "file": "gamedata/binarystaticinfo__/bin/iteminfo.staticinfobody",
                        "intents": [
                            {
                                "comment": "Example: Set maximum stack count on an item by name or key",
                                "entry": "Item_Arrow_01",
                                "field": "_maxStackCount",
                                "op": "set",
                                "new": 999
                            }
                        ]
                    }
                ]
            });
            let p = project_dir.join(format!("{clean_name}.field.json"));
            fs::write(&p, serde_json::to_string_pretty(&field_json)?).at(&p)?;
            files_created.push(format!("{clean_name}.field.json"));

            // 3. README.md
            let readme = format!(
                r#"# {clean_name}

**Version:** {version_str}  
**Author:** {author_str}  
**Type:** JSON Patch (Format 3)  

{desc_str}

## Editing with Pailune

1. Open Pailune and navigate to the **Data Browser** tab to inspect table records and verified fields.
2. Select any field, click **✏ Edit**, and target `{clean_name}` to inject new modifications with live original byte verification.
3. Use the in-editor **⚡ Apply** and **▶ Test** controls to immediately verify changes in game!
"#
            );
            let p = project_dir.join("README.md");
            fs::write(&p, readme).at(&p)?;
            files_created.push("README.md".into());

            // 4. DEVELOPMENT.md
            let dev_doc = format!(
                r#"# {clean_name} — Format 3 Patch Guide

## Specification

Format 3 (`.field.json`) defines intent-based patches resolved against Crimson Desert's data tables:
- `entry`: Record name
- `key`: Record key (u64 / numeric ID)
- `field`: Verified field name (e.g. `_maxStackCount`, `_increasePrice`, `_isBlocked`)
- `op`: `"set"` (field replacement) or `"replace"` (raw byte replacement)
- `new`: New value (scalar number, string, boolean, or hex)
- `match`: (Optional) Predicate selector to match multiple records by field values.

## Table Archive Layout

Table headers and bodies are stored in archive group `0008`:
- `gamedata/binarystaticinfo__/bin/<table_name>.staticinfoheader`
- `gamedata/binarystaticinfo__/bin/<table_name>.staticinfobody`

Pailune packs overlay replacements directly into group `0100`+ ensuring compatibility across game updates.
"#
            );
            let p = project_dir.join("DEVELOPMENT.md");
            fs::write(&p, dev_doc).at(&p)?;
            files_created.push("DEVELOPMENT.md".into());
        }
    }

    Ok(ScaffoldReport {
        project_dir,
        files_created,
        mod_type: cfg.mod_type,
    })
}

/// Exports a mod project (ASI or JSON) into a distribution `.zip` archive.
pub fn export_project(
    project_dir: &Path,
    output_dir: &Path,
    stage_to: Option<&Path>,
) -> Result<PackageResult> {
    if !project_dir.is_dir() {
        return Err(PailuneError::Invalid(format!(
            "project path '{}' is not a directory",
            project_dir.display()
        )));
    }

    // 1. Read metadata from modinfo.json or Cargo.toml
    let modinfo_path = project_dir.join("modinfo.json");
    let (mod_name, version, author, description) = if modinfo_path.exists() {
        let text = fs::read_to_string(&modinfo_path).at(&modinfo_path)?;
        let val: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| PailuneError::Invalid(format!("invalid modinfo.json: {e}")))?;
        (
            val.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| project_dir.file_name().unwrap().to_str().unwrap())
                .to_string(),
            val.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0")
                .to_string(),
            val.get("author")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            val.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        )
    } else {
        (
            project_dir
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Mod".into()),
            "1.0".to_string(),
            "Unknown".to_string(),
            "".to_string(),
        )
    };

    // 2. Prepare staging bundle directory
    let temp_bundle = std::env::temp_dir().join(format!(
        "pailune-export-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let bundle_dir = temp_bundle.join(&mod_name);
    fs::create_dir_all(&bundle_dir).at(&bundle_dir)?;

    let is_asi =
        project_dir.join("Cargo.toml").exists() || project_dir.join("CMakeLists.txt").exists();

    if is_asi {
        // Look for compiled binary
        let clean_name = crate::mod_engine::install::sanitize_name(&mod_name);
        let candidate_bins = [
            project_dir
                .join("target/x86_64-pc-windows-gnu/release")
                .join(format!("{clean_name}.dll")),
            project_dir
                .join("target/x86_64-pc-windows-gnu/release")
                .join(format!("{clean_name}.asi")),
            project_dir
                .join("target/release")
                .join(format!("{clean_name}.dll")),
            project_dir
                .join("target/release")
                .join(format!("{clean_name}.asi")),
            project_dir.join(format!("{clean_name}.asi")),
            project_dir.join(format!("{clean_name}.dll")),
        ];

        let mut found_bin = None;
        for c in &candidate_bins {
            if c.is_file() {
                found_bin = Some(c.clone());
                break;
            }
        }

        if let Some(bin_path) = found_bin {
            let target_bin = bundle_dir.join(format!("{clean_name}.asi"));
            fs::copy(&bin_path, &target_bin).at(&target_bin)?;
        } else {
            // Include source or placeholder
            let notice_p = bundle_dir.join(format!("{clean_name}.asi.txt"));
            fs::write(
                &notice_p,
                "Binary not yet compiled. Build with: cargo build --release --target x86_64-pc-windows-gnu\n",
            )
            .at(&notice_p)?;
        }

        // Copy config.ini if exists
        let ini_path = project_dir.join("config.ini");
        if ini_path.is_file() {
            let _ = fs::copy(&ini_path, bundle_dir.join("config.ini"));
        }
    } else {
        // JSON mod: copy all .json files
        if let Ok(entries) = fs::read_dir(project_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().is_some_and(|e| e == "json") {
                    let file_name = p.file_name().unwrap();
                    let _ = fs::copy(&p, bundle_dir.join(file_name));
                }
            }
        }
    }

    // Always copy or generate README.md and modinfo.json
    let meta = PackageMetadata {
        name: mod_name.clone(),
        author,
        version,
        description,
        bump_version: false,
    };

    let readme_src = project_dir.join("README.md");
    if readme_src.is_file() {
        let _ = fs::copy(&readme_src, bundle_dir.join("README.md"));
    }

    // Package into output directory
    let res = package_mod(&temp_bundle, &mod_name, output_dir, &meta)?;

    // Optional direct staging in Pailune
    if let Some(staging_dir) = stage_to {
        let target_staged = staging_dir.join(&mod_name);
        fs::create_dir_all(&target_staged).at(&target_staged)?;
        for entry in walkdir::WalkDir::new(&bundle_dir)
            .min_depth(1)
            .into_iter()
            .flatten()
        {
            let p = entry.path();
            if p.is_file() {
                let rel = p.strip_prefix(&bundle_dir).unwrap();
                let dest = target_staged.join(rel);
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::copy(p, &dest);
            }
        }
    }

    let _ = fs::remove_dir_all(&temp_bundle);
    Ok(res)
}
