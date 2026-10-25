//! Starting *Crimson Desert* through the Steam client and watching its process.
//!
//! Launching goes through Steam (`steam://rungameid/<app id>`) so that Proton,
//! the configured launch options (e.g. `WINEDLLOVERRIDES`) and the overlay are
//! applied exactly as when starting from the library.

pub mod run;

/// How the launch request was handed to Steam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMethod {
    /// `steam steam://rungameid/<id>` via the Steam CLI binary.
    SteamCli,
    /// `xdg-open steam://rungameid/<id>` via the desktop URL handler.
    XdgOpen,
}

/// Whether the game process is currently alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameProcess {
    Running,
    Stopped,
}
