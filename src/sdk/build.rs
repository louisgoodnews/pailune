//! In-app build runner and staging pipeline for native ASI mod projects (B-17).
//!
//! Compiles native Rust `cdylib` or C++ MinGW projects targeting Windows x64
//! (`x86_64-pc-windows-gnu`) from Linux, and stages the output into Pailune.

use super::ProjectType;
use crate::error::{IoContext, PailuneError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Report returned after building an ASI mod project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildReport {
    /// Name of the mod.
    pub name: String,
    /// Detected project type.
    pub mod_type: ProjectType,
    /// Absolute path to the compiled binary (`.dll` or `.asi`).
    pub binary_path: PathBuf,
    /// Compiler stdout and stderr log.
    pub output_log: String,
    /// Optional path in Pailune staging if staging was performed.
    pub staged_path: Option<PathBuf>,
}

/// Detects the project type of `project_dir`.
pub fn detect_project_type(project_dir: &Path) -> Result<(ProjectType, String)> {
    if project_dir.join("Cargo.toml").exists() {
        let name = read_cargo_pkg_name(project_dir)
            .unwrap_or_else(|| project_name_from_dir(project_dir));
        Ok((ProjectType::Asi, name))
    } else if project_dir.join("Makefile").exists() {
        let name = read_modinfo_name(project_dir)
            .unwrap_or_else(|| project_name_from_dir(project_dir));
        Ok((ProjectType::AsiCpp, name))
    } else if project_dir.join("modinfo.json").exists() {
        let name = read_modinfo_name(project_dir)
            .unwrap_or_else(|| project_name_from_dir(project_dir));
        Ok((ProjectType::Json, name))
    } else {
        Err(PailuneError::Invalid(format!(
            "{} does not contain a recognized Cargo.toml, Makefile, or modinfo.json",
            project_dir.display()
        )))
    }
}

fn project_name_from_dir(dir: &Path) -> String {
    dir.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unnamed_mod".into())
}

fn read_cargo_pkg_name(dir: &Path) -> Option<String> {
    let content = fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("name =") {
            let name = rest.trim().trim_matches('"').trim_matches('\'');
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn read_modinfo_name(dir: &Path) -> Option<String> {
    let content = fs::read_to_string(dir.join("modinfo.json")).ok()?;
    let val: serde_json::Value = serde_json::from_str(&content).ok()?;
    val.get("name")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

/// Builds an ASI mod project using `cargo` or `make`.
pub fn build_project(project_dir: &Path, release: bool) -> Result<BuildReport> {
    let (mod_type, name) = detect_project_type(project_dir)?;
    match mod_type {
        ProjectType::Asi => build_rust_asi(project_dir, &name, release),
        ProjectType::AsiCpp => build_cpp_asi(project_dir, &name, release),
        ProjectType::Json => {
            // JSON mods do not need a compiler; verify schema syntax
            let binary_path = project_dir.join(format!("{name}.json"));
            Ok(BuildReport {
                name,
                mod_type: ProjectType::Json,
                binary_path,
                output_log: "JSON patch mod ready (no native compilation step required)".into(),
                staged_path: None,
            })
        }
    }
}

fn build_rust_asi(project_dir: &Path, name: &str, release: bool) -> Result<BuildReport> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--target").arg("x86_64-pc-windows-gnu");
    if release {
        cmd.arg("--release");
    }
    cmd.current_dir(project_dir);

    let output = cmd.output().map_err(|err| {
        PailuneError::Invalid(format!(
            "failed to run 'cargo': {err}. Make sure Rust and the 'x86_64-pc-windows-gnu' target are installed."
        ))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full_log = format!("{stdout}\n{stderr}").trim().to_string();

    if !output.status.success() {
        return Err(PailuneError::Invalid(format!(
            "cargo build failed with exit code {:?}:\n{full_log}",
            output.status.code()
        )));
    }

    let profile_dir = if release { "release" } else { "debug" };
    let target_dirs = [
        project_dir
            .join("target/x86_64-pc-windows-gnu")
            .join(profile_dir),
        project_dir.join("target").join(profile_dir),
    ];

    let candidates = [
        format!("{name}.dll"),
        format!("lib{name}.dll"),
        format!("{name}.asi"),
    ];

    let mut found_binary = None;
    for dir in &target_dirs {
        for candidate in &candidates {
            let p = dir.join(candidate);
            if p.exists() {
                found_binary = Some(p);
                break;
            }
        }
        if found_binary.is_some() {
            break;
        }
    }

    let binary_path = found_binary.ok_or_else(|| {
        PailuneError::Invalid(format!(
            "compilation reported success, but output binary ({name}.dll) was not found under target/"
        ))
    })?;

    Ok(BuildReport {
        name: name.to_string(),
        mod_type: ProjectType::Asi,
        binary_path,
        output_log: full_log,
        staged_path: None,
    })
}

fn build_cpp_asi(project_dir: &Path, name: &str, _release: bool) -> Result<BuildReport> {
    let mut cmd = Command::new("make");
    cmd.current_dir(project_dir);

    let output = cmd.output().map_err(|err| {
        PailuneError::Invalid(format!(
            "failed to run 'make': {err}. Make sure 'make' and MinGW 'x86_64-w64-mingw32-g++' are installed."
        ))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full_log = format!("{stdout}\n{stderr}").trim().to_string();

    if !output.status.success() {
        return Err(PailuneError::Invalid(format!(
            "make failed with exit code {:?}:\n{full_log}",
            output.status.code()
        )));
    }

    let search_paths = [
        project_dir.join("build").join(format!("{name}.asi")),
        project_dir.join("build").join(format!("{name}.dll")),
        project_dir.join(format!("{name}.asi")),
        project_dir.join(format!("{name}.dll")),
    ];

    let binary_path = search_paths
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| {
            PailuneError::Invalid(format!(
                "make reported success, but output binary ({name}.asi) was not found in build/"
            ))
        })?;

    Ok(BuildReport {
        name: name.to_string(),
        mod_type: ProjectType::AsiCpp,
        binary_path,
        output_log: full_log,
        staged_path: None,
    })
}

/// Copies the compiled mod binary and metadata into Pailune's staging folder.
pub fn stage_built_project(
    project_dir: &Path,
    binary_path: &Path,
    staging_dir: &Path,
) -> Result<PathBuf> {
    let name = read_cargo_pkg_name(project_dir)
        .or_else(|| read_modinfo_name(project_dir))
        .unwrap_or_else(|| project_name_from_dir(project_dir));

    let mod_root = staging_dir.join(&name);
    let bin64 = mod_root.join("bin64");
    fs::create_dir_all(&bin64).at(&bin64)?;

    // 1. Copy binary as bin64/<name>.asi
    let target_asi = bin64.join(format!("{name}.asi"));
    fs::copy(binary_path, &target_asi).at(&target_asi)?;

    // 2. Copy config.ini if present
    let config_ini = project_dir.join("config.ini");
    if config_ini.exists() {
        let target_ini = bin64.join(format!("{name}.ini"));
        let _ = fs::copy(&config_ini, &target_ini);
    }

    // 3. Copy modinfo.json if present
    let modinfo = project_dir.join("modinfo.json");
    if modinfo.exists() {
        let _ = fs::copy(&modinfo, mod_root.join("modinfo.json"));
    } else {
        let generated = serde_json::json!({
            "name": name,
            "version": "1.0",
            "type": "asi",
            "description": "Compiled with Pailune Modding SDK"
        });
        let _ = fs::write(
            mod_root.join("modinfo.json"),
            serde_json::to_string_pretty(&generated).unwrap_or_default(),
        );
    }

    // 4. Copy README.md if present
    let readme = project_dir.join("README.md");
    if readme.exists() {
        let _ = fs::copy(&readme, mod_root.join("README.md"));
    }

    Ok(mod_root)
}
