//! Starting *Crimson Desert* and watching its process.
//!
//! Without environment variables the game is started through the Steam
//! client (`steam -applaunch`), so Steam's own launch options apply too.
//! Launch options that set environment variables need Pylune to start the
//! game through Proton directly, with the Steam client still running.

pub mod options;
pub mod proton;
pub mod run;

/// How the game was started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMethod {
    /// `steam -applaunch <id> [args]`.
    SteamCli,
    /// `xdg-open steam://rungameid/<id>` (no extra arguments possible).
    XdgOpen,
    /// `<proton> waitforexitandrun CrimsonDesert.exe` with custom environment.
    ProtonDirect,
}

/// Whether the game process is currently alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameProcess {
    Running,
    Stopped,
}

/// Parsed launch options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchSpec {
    /// `KEY=VALUE` pairs placed before `%command%`.
    pub env: Vec<(String, String)>,
    /// Arguments passed to the game executable.
    pub args: Vec<String>,
}

/// What a direct Proton launch needs, read from `compatdata/<id>/config_info`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtonSetup {
    /// Proton installation directory (contains the `proton` script).
    pub proton_dir: std::path::PathBuf,
    /// Steam client installation directory.
    pub steam_dir: std::path::PathBuf,
    /// `steamapps/compatdata/<id>`.
    pub compat_data: std::path::PathBuf,
}
