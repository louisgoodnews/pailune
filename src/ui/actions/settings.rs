//! Settings page actions.

use crate::backup::snapshot;
use crate::config::file;
use crate::error::{PyluneError, Result};
use crate::logging::{Event, EventKind, Severity};
use crate::steam::{GameStatus, library};
use crate::ui::{Level, PyluneApp, SetupAction, StatusLine};
use std::path::Path;

impl PyluneApp {
    /// Lets the user pick the game folder for the settings draft.
    pub(super) fn browse_game_dir(&mut self) -> StatusLine {
        let start = Path::new(&self.draft.game_dir);
        let picked = rfd::FileDialog::new()
            .set_directory(start.parent().unwrap_or(start))
            .pick_folder();
        if let Some(dir) = picked {
            self.draft.game_dir = dir.display().to_string();
        }
        StatusLine::new(Level::Info, "Press Save to use the new settings")
    }

    /// Validates and saves the settings draft to `config.toml`.
    pub(super) fn save_settings(&mut self) -> Result<StatusLine> {
        let mut next = self.draft.to_config()?;
        next.nexus.api_url = self.config.nexus.api_url.clone();
        // Queued jobs carry the old configuration; never let them run
        // against a different game folder (A-26).
        if next.game.dir != self.config.game.dir
            && (self.deployed_links > 0 || self.game_job_active())
        {
            return Err(PyluneError::Invalid(
                "unlink all mods and wait for running jobs before changing the game directory"
                    .into(),
            ));
        }
        file::save(&self.paths.config_file, &next)?;
        let restart = next.jobs != self.config.jobs || next.logging != self.config.logging;
        self.config = next;
        self.journal(Event::new(
            Severity::Info,
            EventKind::Settings,
            None,
            "settings saved",
        ));
        Ok(match restart {
            true => StatusLine::new(
                Level::Warning,
                "Settings saved — restart Pylune to apply job/log changes",
            ),
            false => StatusLine::new(Level::Success, "Settings saved"),
        })
    }
}

impl PyluneApp {
    /// Handles the first-run dialog.
    pub(super) fn setup_action(&mut self, action: SetupAction) -> Result<Option<StatusLine>> {
        match action {
            SetupAction::Later => {
                self.setup_open = false;
                Ok(Some(StatusLine::new(
                    Level::Warning,
                    "No game folder set — see the Settings tab",
                )))
            }
            SetupAction::Detect => match library::find_game(self.config.game.app_id) {
                Some(dir) => {
                    self.draft.game_dir = dir.display().to_string();
                    Ok(Some(StatusLine::new(
                        Level::Success,
                        "Found it — press \"Use this folder\"",
                    )))
                }
                None => Err(PyluneError::Invalid(
                    "Crimson Desert is not in any Steam library".into(),
                )),
            },
            SetupAction::Confirm => {
                let line = self.save_settings()?;
                self.refresh_environment();
                match self.game {
                    GameStatus::Ready => {
                        self.setup_open = false;
                        snapshot::ensure_meta_backup(&self.info)?;
                        Ok(Some(line))
                    }
                    _ => Err(PyluneError::Invalid(
                        "that folder does not contain bin64/CrimsonDesert.exe and meta/0.papgt"
                            .into(),
                    )),
                }
            }
        }
    }
}
