//! Everything that differs between operating systems (board item B-08).
//!
//! One module per platform, chosen at compile time, each providing the same
//! functions. The rest of Pylune calls only these, so a port means filling in
//! one file. Linux is the reference; the Windows and macOS versions are
//! checked by CI and not yet tested with the game.
//!
//! | Function | Purpose |
//! |----------|---------|
//! | `steam_roots` | Candidate Steam installation folders. |
//! | `link_file` | Put a mod file into the game folder (decision D-11). |
//! | `open_command` | Open a file, folder or URL with the desktop's default app. |
//! | `steam_command` | The Steam client executable. |
//! | `game_running` | Whether the game executable is running. |
//! | `make_private` | Restrict a new file to the current user. |
//! | `local_app_data` | The game's `AppData/Local` (saves), inside the Proton prefix on Linux. |
//! | `attach_console` | Give command-line runs a console (Windows GUI builds). |
//! | `USES_PROTON` | Whether the game runs through Proton/Wine. |

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

use std::path::PathBuf;

/// How a mod file was put into the game folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Linux and macOS only ever create symlinks.
#[cfg_attr(not(windows), allow(dead_code))]
pub enum LinkKind {
    Symlink,
    /// Same file, second name (same volume only).
    HardLink,
    /// An independent copy.
    Copy,
}

/// The user's home folder (empty if unknown).
pub fn home() -> PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .unwrap_or_default()
}

/// Whether argv\[0\] of a command line names `exe`: compared case-insensitively
/// after the last `/` or `\`, so an editor that merely has the file open does
/// not count.
pub fn names_program(argv0: &[u8], exe: &str) -> bool {
    let name = argv0
        .rsplit(|b| *b == b'/' || *b == b'\\')
        .next()
        .unwrap_or_default();
    name.eq_ignore_ascii_case(exe.as_bytes())
}
