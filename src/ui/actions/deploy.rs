//! Game-directory actions and launching.

use crate::error::{PyluneError, Result};
use crate::jobs::JobKind;
use crate::launch::{GameProcess, LaunchMethod, options, proton, run};
use crate::logging::{Event, EventKind, Severity};
use crate::ui::{DeployAction, Level, PyluneApp, StatusLine};

impl PyluneApp {
    /// Handles game-folder actions (mostly by queueing exclusive jobs).
    pub(super) fn deploy_action(&mut self, action: DeployAction) -> Result<Option<StatusLine>> {
        let (kind, text) = match action {
            DeployAction::Launch => {
                let line = self.launch()?;
                self.guide.played = true;
                return Ok(Some(line));
            }
            DeployAction::VerifyGame => {
                run::verify_files(self.config.game.app_id)?;
                return Ok(Some(StatusLine::new(
                    Level::Info,
                    "Steam is verifying the game files — then press \"Refresh backup\"",
                )));
            }
            DeployAction::Apply => (JobKind::Apply, "Applying mods…"),
            DeployAction::UnlinkAll => (JobKind::UnlinkAll, "Unlinking…"),
            DeployAction::CleanSlate => {
                self.confirm = None;
                (JobKind::CleanSlate, "Restoring a clean slate…")
            }
            DeployAction::RestoreOverlays => (JobKind::RestoreOverlays, "Restoring overlays…"),
            DeployAction::RefreshBackup => (JobKind::RefreshBackup, "Refreshing backup…"),
        };
        self.submit(kind);
        Ok(Some(StatusLine::new(Level::Info, text)))
    }

    /// Launch options of the active profile, else the configured default.
    fn launch_options(&self) -> String {
        self.profiles
            .book
            .active
            .as_ref()
            .and_then(|name| self.profiles.book.profiles.iter().find(|p| &p.name == name))
            .map(|p| p.launch_options.trim().to_string())
            .filter(|o| !o.is_empty())
            .unwrap_or_else(|| self.config.launch.default_options.clone())
    }

    /// Applies first when configured and needed, otherwise launches now.
    fn launch(&mut self) -> Result<StatusLine> {
        if self.process == GameProcess::Running {
            return Err(PyluneError::Invalid(
                "Crimson Desert is already running".into(),
            ));
        }
        match self.config.launch.apply_before_launch && self.dirty {
            true => {
                let id = self.submit(JobKind::Apply);
                self.launch_after = Some(id);
                Ok(StatusLine::new(
                    Level::Info,
                    "Applying mods, then launching…",
                ))
            }
            false => self.launch_now(),
        }
    }

    /// Starts the game with the effective launch options.
    pub(super) fn launch_now(&mut self) -> Result<StatusLine> {
        let text = self.launch_options();
        let spec = options::parse(&text);
        let setup = proton::setup(&self.info);
        let warning = self.backup_saves_before_play();
        let method = run::launch(&self.info, setup.as_ref(), &spec)?;
        let via = match method {
            LaunchMethod::SteamCli => "Steam",
            LaunchMethod::XdgOpen => "the steam:// URL handler",
            LaunchMethod::ProtonDirect => "Proton (direct, Steam must be running)",
            LaunchMethod::Direct => "the game executable (Steam must be running)",
        };
        let profile = self.profiles.book.active.as_deref().unwrap_or("no profile");
        self.journal(Event::new(
            Severity::Info,
            EventKind::Launch,
            None,
            format!("launched via {via} ({profile}); options: {text:?}"),
        ));
        Ok(match warning {
            Some(w) => StatusLine::new(Level::Warning, format!("Launching via {via}; {w}")),
            None => StatusLine::new(
                Level::Success,
                format!("Launching Crimson Desert via {via}…"),
            ),
        })
    }
}
