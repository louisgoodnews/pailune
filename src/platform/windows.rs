//! Windows: the game and Steam run natively. Not yet tested with the game.

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The game runs natively.
pub const USES_PROTON: bool = false;

/// Steam's default installation folders.
pub fn steam_roots(_home: &Path) -> Vec<PathBuf> {
    ["ProgramFiles(x86)", "ProgramFiles"]
        .iter()
        .filter_map(|var| std::env::var_os(var))
        .map(|dir| PathBuf::from(dir).join("Steam"))
        .collect()
}

/// A file symlink when Windows allows it (Developer Mode or administrator),
/// else a hard link (same volume), else a copy — decision D-11.
pub fn link_file(source: &Path, target: &Path) -> io::Result<super::LinkKind> {
    use super::LinkKind;
    std::os::windows::fs::symlink_file(source, target)
        .map(|()| LinkKind::Symlink)
        .or_else(|_| fs::hard_link(source, target).map(|()| LinkKind::HardLink))
        .or_else(|_| fs::copy(source, target).map(|_| LinkKind::Copy))
}

/// `explorer.exe <target>`: opens folders, files and URLs with their default
/// app. Unlike `cmd /C start`, nothing in the target is parsed by a shell.
pub fn open_command(target: &OsStr) -> Command {
    let mut command = Command::new("explorer.exe");
    command.arg(target);
    command
}

/// `steam.exe` in the first Steam folder that has it.
pub fn steam_command() -> Command {
    let exe = steam_roots(Path::new(""))
        .into_iter()
        .map(|dir| dir.join("steam.exe"))
        .find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("steam.exe"));
    Command::new(exe)
}

/// `tasklist` filtered by image name.
pub fn game_running(exe: &str) -> bool {
    super::background("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {exe}"), "/NH"])
        .output()
        .is_ok_and(|out| {
            String::from_utf8_lossy(&out.stdout)
                .to_lowercase()
                .contains(&exe.to_lowercase())
        })
}

/// Files in the user profile are private by default.
pub fn make_private(_options: &mut OpenOptions) {}

/// A GUI-subsystem program has no console; for command-line use, write to
/// the terminal that started Pylune (if any).
pub fn attach_console() {
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    // SAFETY: plain Win32 call without pointers; failure (no parent console,
    // e.g. started by an MCP client with pipes) is harmless.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

/// `%LOCALAPPDATA%`.
pub fn local_app_data(_prefix: Option<&Path>) -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
}
