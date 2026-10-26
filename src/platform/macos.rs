//! macOS. The game has no macOS build; it runs through a Wine-based layer
//! (e.g. CrossOver), so Steam data and saves live in that bottle. Not yet
//! tested with the game.

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The game runs through a Wine-based layer.
pub const USES_PROTON: bool = false;

/// Steam's data folder of the native client.
pub fn steam_roots(home: &Path) -> Vec<PathBuf> {
    vec![home.join("Library/Application Support/Steam")]
}

/// Symlinks `target` → `source`, as on Linux.
pub fn link_file(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

/// `open <target>`.
pub fn open_command(target: &OsStr) -> Command {
    let mut command = Command::new("open");
    command.arg(target);
    command
}

/// The Steam client binary inside its app bundle.
pub fn steam_command() -> Command {
    Command::new("/Applications/Steam.app/Contents/MacOS/steam_osx")
}

/// `pgrep -if <exe>` finds the game inside its Wine process.
pub fn game_running(exe: &str) -> bool {
    Command::new("pgrep")
        .args(["-if", exe])
        .output()
        .is_ok_and(|out| out.status.success() && !out.stdout.is_empty())
}

/// New files readable only by the owner (`0600`).
pub fn make_private(options: &mut OpenOptions) {
    options.mode(0o600);
}

/// `AppData/Local` of the Wine prefix's user, when the prefix is known.
pub fn local_app_data(prefix: Option<&Path>) -> Option<PathBuf> {
    prefix.map(|p| p.join("drive_c/users/crossover/AppData/Local"))
}
