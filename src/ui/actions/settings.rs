//! Settings page actions.

use crate::config::file;
use crate::error::{PyluneError, Result};
use crate::logging::{Event, EventKind, Severity};
use crate::ui::{Level, PyluneApp, StatusLine};
use std::path::Path;

impl PyluneApp {
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

    pub(super) fn save_settings(&mut self) -> Result<StatusLine> {
        let next = self.draft.to_config()?;
        if next.game.dir != self.config.game.dir && self.deployed_links > 0 {
            return Err(PyluneError::Invalid(
                "unlink all mods before changing the game directory".into(),
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
