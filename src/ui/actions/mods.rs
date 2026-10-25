//! Staging, deployment and backup actions.

use super::report_line;
use crate::backup::{reset, snapshot};
use crate::config::store;
use crate::error::{PyluneError, Result};
use crate::mod_engine::model::{ModEntry, Registry};
use crate::mod_engine::{install, plan, registry};
use crate::symlink::{deploy, undeploy};
use crate::ui::{Level, PyluneApp, StatusLine};
use std::path::PathBuf;

impl PyluneApp {
    pub(super) fn import(&mut self, picked: Option<PathBuf>) -> Result<StatusLine> {
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

    pub(super) fn remove(&mut self, index: usize) -> Result<StatusLine> {
        self.confirm = None;
        let entry = self.registry.mods.get(index).cloned();
        let next = registry::removed(&self.registry, &self.paths.staging, index)?;
        self.set_registry(next, false)?;
        if let Some(name) = entry.as_ref().map(|m| &m.name) {
            self.nexus.meta.refs.remove(name);
            self.nexus.updates.remove(name);
            store::save(&self.paths.nexus_meta, &self.nexus.meta)?;
        }
        match entry.is_some_and(|m| m.enabled) && !self.manifest.links.is_empty() {
            true => self.apply_mods(),
            false => Ok(StatusLine::new(Level::Success, "Mod removed from staging")),
        }
    }

    /// Rebuilds the deployment from scratch: unlink everything, link enabled mods.
    pub(super) fn apply_mods(&mut self) -> Result<StatusLine> {
        self.require_game()?;
        self.require_stopped()?;
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

    pub(super) fn unlink_all(&mut self) -> Result<StatusLine> {
        self.require_stopped()?;
        let result = undeploy::undeploy(&mut self.manifest);
        store::save(&self.paths.manifest, &self.manifest)?;
        self.dirty = self.registry.mods.iter().any(|m| m.enabled);
        result.map(|report| report_line("Unlinked", &report))
    }

    pub(super) fn clean_slate(&mut self) -> Result<StatusLine> {
        self.confirm = None;
        self.require_game()?;
        self.require_stopped()?;
        let result = reset::clean_slate(&self.info, &mut self.manifest);
        store::save(&self.paths.manifest, &self.manifest)?;
        let disabled = self.registry.mods.iter().map(|m| ModEntry {
            enabled: false,
            ..m.clone()
        });
        self.set_registry(
            Registry {
                mods: disabled.collect(),
            },
            false,
        )?;
        self.dirty = false;
        result.map(|report| report_line("Clean slate restored", &report))
    }

    pub(super) fn refresh_backup(&mut self) -> Result<StatusLine> {
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

    pub(super) fn rescan(&mut self) -> Result<StatusLine> {
        let next = registry::sync(&self.registry, &self.paths.staging)?;
        self.set_registry(next, false)?;
        Ok(StatusLine::new(
            Level::Info,
            format!("{} mod(s) in staging", self.registry.mods.len()),
        ))
    }
}
