//! Starting the game through Steam.

use crate::error::{PyluneError, Result};
use crate::launch::{GameProcess, LaunchMethod, run};
use crate::ui::{Level, PyluneApp, StatusLine};

impl PyluneApp {
    /// Deploys pending changes (if configured) and asks Steam to start the game.
    pub(super) fn launch_game(&mut self) -> Result<StatusLine> {
        self.require_game()?;
        if self.process == GameProcess::Running {
            return Err(PyluneError::Invalid(
                "Crimson Desert is already running".into(),
            ));
        }
        let applied = match self.config.apply_before_launch && self.dirty {
            true => Some(self.apply_mods()?),
            false => None,
        };
        let via = match run::launch(self.config.app_id)? {
            LaunchMethod::SteamCli => "Steam",
            LaunchMethod::XdgOpen => "the desktop URL handler",
        };
        let prefix = applied
            .map(|line| format!("{} · ", line.text))
            .unwrap_or_default();
        Ok(StatusLine::new(
            Level::Success,
            format!("{prefix}Launching Crimson Desert via {via}…"),
        ))
    }
}
