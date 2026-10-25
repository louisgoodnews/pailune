//! State transitions triggered by [`Action`]s, grouped by feature.

mod launch;
mod mods;
mod nexus;
mod poll;
mod profiles;
mod settings;

use super::{Action, Confirm, Level, PyluneApp, StatusLine};
use crate::backup::{reset, snapshot};
use crate::config::store;
use crate::error::{PyluneError, Result};
use crate::launch::GameProcess;
use crate::mod_engine::model::Registry;
use crate::mod_engine::registry;
use crate::steam::{GameStatus, locate};
use crate::symlink::Report;

impl StatusLine {
    pub fn new(level: Level, text: impl Into<String>) -> Self {
        Self {
            level,
            text: text.into(),
        }
    }
}

/// Turns a deploy/undeploy report into a status line.
fn report_line(verb: &str, report: &Report) -> StatusLine {
    let text = format!(
        "{verb}: {} link(s), {} backup(s) touched",
        report.linked, report.backed_up
    );
    match report.warnings.as_slice() {
        [] => StatusLine::new(Level::Success, text),
        [first, rest @ ..] => StatusLine::new(
            Level::Warning,
            format!("{text}; {} warning(s): {first}", rest.len() + 1),
        ),
    }
}

impl PyluneApp {
    /// Applies one action and records its outcome in the status bar.
    pub fn dispatch(&mut self, action: Action) {
        let outcome = match action {
            Action::Toggle(i) => self.set_registry(registry::toggled(&self.registry, i), true),
            Action::Move { from, to } => {
                self.set_registry(registry::swapped(&self.registry, from, to), true)
            }
            Action::Remove(i) => self.remove(i),
            Action::ImportFolder => self.import(rfd::FileDialog::new().pick_folder()),
            Action::ImportFile => self.import(
                rfd::FileDialog::new()
                    .add_filter("Mods", &["zip", "asi", "dll", "json", "ini"])
                    .pick_file(),
            ),
            Action::Apply => self.apply_mods(),
            Action::UnlinkAll => self.unlink_all(),
            Action::CleanSlate => self.clean_slate(),
            Action::RestoreOverlays => reset::restore_overlays(&self.info).map(|n| {
                StatusLine::new(Level::Success, format!("Restored {n} overlay folder(s)"))
            }),
            Action::RefreshBackup => self.refresh_backup(),
            Action::BrowseGameDir => Ok(self.browse_game_dir()),
            Action::SaveSettings => self.save_settings(),
            Action::Rescan => self.rescan(),
            Action::Launch => self.launch_game(),
            Action::SwitchTab(tab) => {
                self.tab = tab;
                Ok(self.status.clone())
            }
            Action::Profile(action) => self.profile_action(action),
            Action::Nexus(action) => self.nexus_action(action),
            Action::Ask(confirm) => Ok(self.ask(Some(confirm))),
            Action::Dismiss => Ok(self.ask(None)),
        };
        self.status = outcome.unwrap_or_else(|err| StatusLine::new(Level::Error, err.to_string()));
        self.refresh_environment();
    }

    /// Re-probes the game directory, loader and backup state.
    pub fn refresh_environment(&mut self) {
        self.info = locate::resolve(&self.config);
        self.game = locate::game_status(&self.info);
        self.loader = locate::detect_loader(&self.info);
        self.backup = snapshot::status(&self.info, &self.manifest);
    }

    fn ask(&mut self, confirm: Option<Confirm>) -> StatusLine {
        self.confirm = confirm;
        self.status.clone()
    }

    /// Persists and installs a new registry.
    fn set_registry(&mut self, next: Registry, dirty: bool) -> Result<StatusLine> {
        store::save(&self.paths.registry, &next)?;
        self.registry = next;
        self.dirty |= dirty;
        Ok(StatusLine::new(
            Level::Info,
            "Selection changed — press Apply to deploy",
        ))
    }

    /// Fails unless the game directory is valid.
    fn require_game(&self) -> Result<()> {
        match self.game {
            GameStatus::Ready => Ok(()),
            GameStatus::Unrecognised => Err(PyluneError::Invalid(
                "game directory does not contain bin64/CrimsonDesert.exe and meta/0.papgt".into(),
            )),
            GameStatus::Missing => Err(PyluneError::Invalid("game directory not found".into())),
        }
    }

    /// Fails while the game runs: its files must not change underneath it.
    fn require_stopped(&self) -> Result<()> {
        match self.process {
            GameProcess::Stopped => Ok(()),
            GameProcess::Running => Err(PyluneError::Invalid(
                "Crimson Desert is running — close it first".into(),
            )),
        }
    }
}
