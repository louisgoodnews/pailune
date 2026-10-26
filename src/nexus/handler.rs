//! Registering Pylune as the desktop handler for `nxm://` links, and
//! forwarding links for other games to the handler it replaced.
//!
//! Linux uses freedesktop `.desktop` files and `xdg-mime`; Windows the
//! registry ([`super::winreg`]). macOS needs an app bundle and reports that
//! registration is not supported yet. The "previous handler" is a desktop
//! file name on Linux and a command line on Windows.

use crate::error::{IoContext, PyluneError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Desktop entry file name used for the registration.
pub const DESKTOP_FILE: &str = "pylune-nxm.desktop";
/// MIME type desktops use for `nxm://` links.
const MIME: &str = "x-scheme-handler/nxm";

/// `~/.local/share/applications`.
fn applications_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.data_dir().join("applications"))
}

/// The program the desktop should start for links: the AppImage file when
/// running from one (its mount point changes every start), else this binary.
pub fn own_executable() -> Result<PathBuf> {
    pick_executable(std::env::var_os("APPIMAGE"), std::env::current_exe())
}

/// Prefers an existing AppImage file over the running binary.
fn pick_executable(
    appimage: Option<std::ffi::OsString>,
    current: std::io::Result<PathBuf>,
) -> Result<PathBuf> {
    match appimage.map(PathBuf::from).filter(|p| p.is_file()) {
        Some(image) => Ok(image),
        None => current.map_err(|e| PyluneError::Invalid(e.to_string())),
    }
}

/// Quotes one `Exec` argument per the Desktop Entry specification.
pub fn escape_exec_arg(arg: &str) -> String {
    let escaped: String = arg
        .chars()
        .map(|c| match c {
            '"' | '`' | '$' | '\\' => format!("\\{c}"),
            '%' => "%%".into(),
            c => c.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}

/// Contents of the `.desktop` entry launching `exe` with the link.
fn desktop_entry(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Pylune\n\
         Comment=Crimson Desert mod manager (nxm:// handler)\n\
         Exec={} %u\nTerminal=false\nNoDisplay=true\nMimeType={MIME};\n",
        escape_exec_arg(&exe.display().to_string())
    )
}

/// The desktop file currently handling `nxm://`, if any.
fn current_default() -> Option<String> {
    let out = Command::new("xdg-mime")
        .args(["query", "default", MIME])
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string()).filter(|s| !s.is_empty())
}

/// Whether Pylune is the current default `nxm://` handler.
pub fn is_registered() -> bool {
    if cfg!(windows) {
        return own_executable().is_ok_and(|exe| super::winreg::is_registered(&exe));
    }
    cfg!(target_os = "linux") && current_default().as_deref() == Some(DESKTOP_FILE)
}

/// Makes Pylune the default `nxm://` handler. Returns the handler it
/// replaced, so links for other games can be forwarded to it.
pub fn register(exe: &Path) -> Result<Option<String>> {
    if cfg!(windows) {
        return super::winreg::register(exe);
    }
    if !cfg!(target_os = "linux") {
        return Err(PyluneError::Invalid(
            "registering for nxm:// links is not supported on this platform yet".into(),
        ));
    }
    let previous = current_default().filter(|d| d != DESKTOP_FILE);
    let dir = applications_dir()
        .ok_or_else(|| PyluneError::Invalid("cannot locate ~/.local/share/applications".into()))?;
    fs::create_dir_all(&dir).at(&dir)?;
    let entry = dir.join(DESKTOP_FILE);
    fs::write(&entry, desktop_entry(exe)).at(&entry)?;
    let ok = Command::new("xdg-mime")
        .args(["default", DESKTOP_FILE, MIME])
        .status()
        .is_ok_and(|status| status.success());
    // Refreshing the cache is best effort; not every desktop ships the tool.
    let _ = Command::new("update-desktop-database").arg(&dir).status();
    match ok {
        true => Ok(previous),
        false => Err(PyluneError::Invalid(
            "xdg-mime failed to set the nxm handler".into(),
        )),
    }
}

/// Directories searched for `.desktop` files, most specific first.
fn desktop_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let data_home =
        std::env::var_os("XDG_DATA_HOME").map_or_else(|| home.join(".local/share"), PathBuf::from);
    let data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    std::iter::once(data_home)
        .chain(data_dirs.split(':').map(PathBuf::from))
        .chain([
            home.join(".local/share/flatpak/exports/share"),
            PathBuf::from("/var/lib/flatpak/exports/share"),
        ])
        .map(|d| d.join("applications"))
        .collect()
}

/// Splits an `Exec` value into arguments (quotes and backslash escapes).
fn split_exec(exec: &str) -> Vec<String> {
    let (mut args, current, _, _) = exec.chars().fold(
        (Vec::new(), None::<String>, false, false),
        |(mut args, current, quoted, escaped), c| match (quoted, escaped, c) {
            (_, true, c) => (
                args,
                Some(current.unwrap_or_default() + &c.to_string()),
                quoted,
                false,
            ),
            (true, false, '\\') => (args, current, true, true),
            (_, false, '"') => (args, Some(current.unwrap_or_default()), !quoted, false),
            (false, false, c) if c.is_whitespace() => {
                args.extend(current);
                (args, None, false, false)
            }
            (_, false, c) => (
                args,
                Some(current.unwrap_or_default() + &c.to_string()),
                quoted,
                false,
            ),
        },
    );
    args.extend(current);
    args
}

/// Expands field codes: `%u %U %f %F` become the URL, others are dropped.
fn expand_exec(args: Vec<String>, url: &str) -> Vec<String> {
    args.into_iter()
        .filter_map(|arg| match arg.as_str() {
            "%u" | "%U" | "%f" | "%F" => Some(url.to_string()),
            "%i" | "%c" | "%k" => None,
            _ => Some(arg.replace("%%", "%")),
        })
        .collect()
}

/// Passes `url` to the desktop handler `desktop_file` (e.g. MO2 or Vortex).
pub fn forward(desktop_file: &str, url: &str) -> Result<()> {
    if cfg!(windows) {
        return super::winreg::forward(desktop_file, url);
    }
    let text = desktop_dirs()
        .iter()
        .find_map(|dir| fs::read_to_string(dir.join(desktop_file)).ok())
        .ok_or_else(|| {
            PyluneError::Invalid(format!("previous nxm handler {desktop_file} not found"))
        })?;
    let exec = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("Exec="))
        .ok_or_else(|| PyluneError::Invalid(format!("{desktop_file} has no Exec line")))?;
    let argv = expand_exec(split_exec(exec), url);
    let (program, rest) = argv
        .split_first()
        .ok_or_else(|| PyluneError::Invalid("empty Exec line".into()))?;
    let mut command = Command::new(program);
    command.args(rest);
    crate::launch::run::spawn(command)
        .map_err(|e| PyluneError::Invalid(format!("could not start {program}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appimages_register_their_own_file() {
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("Pylune.AppImage");
        fs::write(&image, "").unwrap();
        let mounted = PathBuf::from("/tmp/.mount_x/usr/bin/pylune");
        let picked = pick_executable(Some(image.clone().into()), Ok(mounted.clone()));
        assert_eq!(picked.unwrap(), image);
        let gone = dir.path().join("moved.AppImage");
        let picked = pick_executable(Some(gone.into()), Ok(mounted.clone()));
        assert_eq!(picked.unwrap(), mounted, "a vanished image falls back");
    }

    #[test]
    fn exec_lines_round_trip() {
        let quoted = escape_exec_arg("/opt/My \"Apps\"/100%/pylune");
        assert_eq!(quoted, r#""/opt/My \"Apps\"/100%%/pylune""#);
        let argv = expand_exec(split_exec(&format!("{quoted} %u")), "nxm://x");
        assert_eq!(argv, ["/opt/My \"Apps\"/100%/pylune", "nxm://x"]);
        let mo2 = expand_exec(
            split_exec("env WINEPREFIX=/p wine nxmhandler.exe %u %i"),
            "nxm://y",
        );
        assert_eq!(
            mo2,
            ["env", "WINEPREFIX=/p", "wine", "nxmhandler.exe", "nxm://y"]
        );
    }
}
