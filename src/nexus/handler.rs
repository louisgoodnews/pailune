//! Registering Pylune as the desktop handler for `nxm://` links.

use crate::error::{IoContext, PyluneError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Desktop entry file name used for the registration.
pub const DESKTOP_FILE: &str = "pylune-nxm.desktop";
const MIME: &str = "x-scheme-handler/nxm";

/// `~/.local/share/applications`.
fn applications_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.data_dir().join("applications"))
}

/// Contents of the `.desktop` entry launching `exe` with the link.
fn desktop_entry(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Pylune\n\
         Comment=Crimson Desert mod manager (nxm:// handler)\n\
         Exec=\"{}\" %u\nTerminal=false\nNoDisplay=true\nMimeType={MIME};\n",
        exe.display()
    )
}

/// Whether Pylune is the current default `nxm://` handler.
pub fn is_registered() -> bool {
    Command::new("xdg-mime")
        .args(["query", "default", MIME])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim() == DESKTOP_FILE)
        .unwrap_or(false)
}

/// Writes the desktop entry and makes it the default `nxm://` handler.
pub fn register(exe: &Path) -> Result<()> {
    let dir = applications_dir()
        .ok_or_else(|| PyluneError::Invalid("cannot locate ~/.local/share/applications".into()))?;
    fs::create_dir_all(&dir).at(&dir)?;
    let entry = dir.join(DESKTOP_FILE);
    fs::write(&entry, desktop_entry(exe)).at(&entry)?;
    let ok = Command::new("xdg-mime")
        .args(["default", DESKTOP_FILE, MIME])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    // Refreshing the cache is best effort; not every desktop ships the tool.
    let _ = Command::new("update-desktop-database").arg(&dir).status();
    match ok {
        true => Ok(()),
        false => Err(PyluneError::Invalid(
            "xdg-mime failed to set the nxm handler".into(),
        )),
    }
}
