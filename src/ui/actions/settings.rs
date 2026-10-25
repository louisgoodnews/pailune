//! Settings panel actions.

use crate::config::store;
use crate::error::{PyluneError, Result};
use crate::nexus::worker::Request;
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
        if next.game_dir != self.config.game_dir && !self.manifest.links.is_empty() {
            return Err(PyluneError::Invalid(
                "unlink all mods before changing the game directory".into(),
            ));
        }
        store::save(&self.paths.config, &next)?;
        let domain_changed = next.nexus_domain != self.config.nexus_domain;
        self.config = next;
        if domain_changed {
            self.configure_worker();
        }
        Ok(StatusLine::new(Level::Success, "Settings saved"))
    }

    /// Sends the current API key and game domain to the Nexus worker.
    pub(super) fn configure_worker(&self) {
        let key = crate::nexus::key::load(&self.paths.nexus_key).unwrap_or_default();
        let domain = self.config.nexus_domain.clone();
        self.worker.send(Request::Configure { key, domain });
    }

    /// Configures the worker at start-up and validates a stored key.
    pub fn connect_nexus(&mut self) {
        self.configure_worker();
        if crate::nexus::key::load(&self.paths.nexus_key).is_some() {
            self.worker.send(Request::Validate);
            self.nexus.pending += 1;
        }
    }
}
