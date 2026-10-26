//! Linux: Steam (native, Debian, Flatpak, Snap) with Proton.

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The game runs through Proton.
pub const USES_PROTON: bool = true;

/// Where Steam keeps its data for native, Debian-style, Flatpak and Snap installs.
pub fn steam_roots(home: &Path) -> Vec<PathBuf> {
    [
        ".local/share/Steam",
        ".steam/steam",
        ".steam/root",
        ".var/app/com.valvesoftware.Steam/.local/share/Steam",
        "snap/steam/common/.local/share/Steam",
    ]
    .iter()
    .map(|rel| home.join(rel))
    .collect()
}

/// Symlinks `target` → `source` (decision D-01).
pub fn link_file(source: &Path, target: &Path) -> io::Result<super::LinkKind> {
    std::os::unix::fs::symlink(source, target).map(|()| super::LinkKind::Symlink)
}

/// `xdg-open <target>`.
pub fn open_command(target: &OsStr) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(target);
    command
}

/// The `steam` client on the `PATH`.
pub fn steam_command() -> Command {
    Command::new("steam")
}

/// Scans `/proc/*/cmdline` for a process whose program is `exe` (Proton runs
/// the game with its Windows path as argv\[0\]).
pub fn game_running(exe: &str) -> bool {
    fs::read_dir("/proc")
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .bytes()
                        .all(|b| b.is_ascii_digit())
                })
                .filter_map(|e| fs::read(e.path().join("cmdline")).ok())
                .any(|raw| {
                    let argv0 = raw.split(|b| *b == 0).next().unwrap_or_default();
                    super::names_program(argv0, exe)
                })
        })
        .unwrap_or(false)
}

/// Terminal programs already have their console.
pub fn attach_console() {}

/// New files readable only by the owner (`0600`).
pub fn make_private(options: &mut OpenOptions) {
    options.mode(0o600);
}

/// `AppData/Local` of the Proton prefix's Windows user.
pub fn local_app_data(prefix: Option<&Path>) -> Option<PathBuf> {
    prefix.map(|p| p.join("drive_c/users/steamuser/AppData/Local"))
}
