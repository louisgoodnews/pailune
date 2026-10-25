//! State transitions triggered by [`Action`]s.

use super::{Action, Confirm, Level, PyluneApp, StatusLine};
use crate::backup::{reset, snapshot};
use crate::config::store;
use crate::error::{PyluneError, Result};
use crate::mod_engine::model::{ModEntry, Registry};
use crate::mod_engine::{install, plan, registry};
use crate::steam::{GameStatus, locate};
use crate::symlink::{Report, deploy, undeploy};
use std::path::{Path, PathBuf};

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
            Action::Ask(confirm) => Ok(self.ask(confirm)),
            Action::Dismiss => Ok(self.ask_none()),
        };
        match outcome {
            Ok(line) => self.status = line,
            Err(err) => self.status = StatusLine::new(Level::Error, err.to_string()),
        }
        self.refresh_environment();
    }

    /// Re-probes the game directory, loader and backup state.
    pub fn refresh_environment(&mut self) {
        self.info = locate::resolve(&self.config);
        self.game = locate::game_status(&self.info);
        self.loader = locate::detect_loader(&self.info);
        self.backup = snapshot::status(&self.info, &self.manifest);
    }

    fn ask(&mut self, confirm: Confirm) -> StatusLine {
        self.confirm = Some(confirm);
        self.status.clone()
    }

    fn ask_none(&mut self) -> StatusLine {
        self.confirm = None;
        self.status.clone()
    }

    fn set_registry(&mut self, next: Registry, dirty: bool) -> Result<StatusLine> {
        store::save(&self.paths.registry, &next)?;
        self.registry = next;
        self.dirty |= dirty;
        Ok(StatusLine::new(
            Level::Info,
            "Selection changed — press Apply to deploy",
        ))
    }

    fn require_game(&self) -> Result<()> {
        match self.game {
            GameStatus::Ready => Ok(()),
            GameStatus::Unrecognised => Err(PyluneError::Invalid(
                "game directory does not contain bin64/CrimsonDesert.exe and meta/0.papgt".into(),
            )),
            GameStatus::Missing => Err(PyluneError::Invalid("game directory not found".into())),
        }
    }

    fn import(&mut self, picked: Option<PathBuf>) -> Result<StatusLine> {
        let Some(source) = picked else {
            return Ok(StatusLine::new(Level::Info, "Import cancelled"));
        };
        let name = install::import(&source, &self.paths.staging)?;
        let next = registry::sync(&self.registry, &self.paths.staging)?;
        self.set_registry(next, false)?;
        Ok(StatusLine::new(
            Level::Success,
            format!("Staged '{name}' — enable it and press Apply"),
        ))
    }

    fn remove(&mut self, index: usize) -> Result<StatusLine> {
        self.confirm = None;
        let was_enabled = self.registry.mods.get(index).is_some_and(|m| m.enabled);
        let next = registry::removed(&self.registry, &self.paths.staging, index)?;
        self.set_registry(next, false)?;
        match was_enabled && !self.manifest.links.is_empty() {
            true => self.apply_mods(),
            false => Ok(StatusLine::new(Level::Success, "Mod removed from staging")),
        }
    }

    /// Rebuilds the deployment from scratch: unlink everything, link enabled mods.
    fn apply_mods(&mut self) -> Result<StatusLine> {
        self.require_game()?;
        undeploy::undeploy(&mut self.manifest)?;
        store::save(&self.paths.manifest, &self.manifest)?;
        snapshot::ensure_meta_backup(&self.info)?;
        let link_plan = plan::build(&self.config, &self.paths.staging, &self.registry)?;
        let result = deploy::deploy(&link_plan, &mut self.manifest);
        store::save(&self.paths.manifest, &self.manifest)?;
        let report = result?;
        self.dirty = false;
        let line = report_line("Applied", &report);
        Ok(match link_plan.conflicts {
            0 => line,
            n => StatusLine::new(
                Level::Warning,
                format!("{} — {n} file conflict(s), later mods won", line.text),
            ),
        })
    }

    fn unlink_all(&mut self) -> Result<StatusLine> {
        let result = undeploy::undeploy(&mut self.manifest);
        store::save(&self.paths.manifest, &self.manifest)?;
        self.dirty = self.registry.mods.iter().any(|m| m.enabled);
        result.map(|report| report_line("Unlinked", &report))
    }

    fn clean_slate(&mut self) -> Result<StatusLine> {
        self.confirm = None;
        self.require_game()?;
        let result = reset::clean_slate(&self.info, &mut self.manifest);
        store::save(&self.paths.manifest, &self.manifest)?;
        let disabled = self.registry.mods.iter().map(|m| ModEntry {
            enabled: false,
            ..m.clone()
        });
        let next = Registry {
            mods: disabled.collect(),
        };
        self.set_registry(next, false)?;
        self.dirty = false;
        result.map(|report| report_line("Clean slate restored", &report))
    }

    fn refresh_backup(&mut self) -> Result<StatusLine> {
        self.require_game()?;
        match self.manifest.links.is_empty() {
            false => Err(PyluneError::Invalid(
                "unlink all mods before refreshing the backup".into(),
            )),
            true => snapshot::refresh_meta_backup(&self.info).map(|n| {
                StatusLine::new(Level::Success, format!("Backed up {n} metadata file(s)"))
            }),
        }
    }

    fn browse_game_dir(&mut self) -> StatusLine {
        let start = Path::new(&self.draft.game_dir);
        let picked = rfd::FileDialog::new()
            .set_directory(start.parent().unwrap_or(start))
            .pick_folder();
        if let Some(dir) = picked {
            self.draft.game_dir = dir.display().to_string();
        }
        StatusLine::new(Level::Info, "Press Save to use the new settings")
    }

    fn save_settings(&mut self) -> Result<StatusLine> {
        let next = self.draft.to_config()?;
        match (
            next.game_dir != self.config.game_dir,
            self.manifest.links.is_empty(),
        ) {
            (true, false) => Err(PyluneError::Invalid(
                "unlink all mods before changing the game directory".into(),
            )),
            _ => {
                store::save(&self.paths.config, &next)?;
                self.config = next;
                Ok(StatusLine::new(Level::Success, "Settings saved"))
            }
        }
    }

    fn rescan(&mut self) -> Result<StatusLine> {
        let next = registry::sync(&self.registry, &self.paths.staging)?;
        self.set_registry(next, false)?;
        Ok(StatusLine::new(
            Level::Info,
            format!("{} mod(s) in staging", self.registry.mods.len()),
        ))
    }
}
