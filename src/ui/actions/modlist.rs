//! Mod list export and import (B-12).

use crate::error::{PyluneError, Result};
use crate::jobs::JobKind;
use crate::logging::{Event, EventKind, Severity};
use crate::modlist::{self, apply::apply};
use crate::nexus::download::DownloadJob;
use crate::ui::state::ListImport;
use crate::ui::{Level, ModListAction, PyluneApp, StatusLine};

impl PyluneApp {
    /// Handles mod list intents.
    pub(super) fn modlist_action(&mut self, action: ModListAction) -> Result<Option<StatusLine>> {
        match action {
            ModListAction::Export => self.export_list(),
            ModListAction::Import => self.import_list(),
            ModListAction::DownloadMissing => self.download_missing().map(Some),
            ModListAction::OpenMissing(index) => self.open_missing(index).map(|()| None),
            ModListAction::CloseMissing => {
                self.mods.list_import = None;
                Ok(None)
            }
        }
    }

    fn export_list(&mut self) -> Result<Option<StatusLine>> {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Pylune mod list", &["json"])
            .set_file_name("pylune-modlist.json")
            .save_file()
        else {
            return Ok(None);
        };
        let list = modlist::capture(
            &self.registry,
            &self.nexus.collections,
            &self.config.nexus.domain,
            crate::db::now(),
        );
        modlist::write(&path, &list)?;
        let text = format!("Exported {} mod(s) to {}", list.mods.len(), path.display());
        self.journal(Event::new(
            Severity::Info,
            EventKind::Profile,
            None,
            text.clone(),
        ));
        Ok(Some(StatusLine::new(Level::Success, text)))
    }

    fn import_list(&mut self) -> Result<Option<StatusLine>> {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Pylune mod list", &["json"])
            .pick_file()
        else {
            return Ok(None);
        };
        let list = modlist::read(&path, &self.config.nexus.domain)?;
        self.apply_list(list).map(Some)
    }

    /// Applies a list to the installed mods and remembers what is missing.
    pub fn apply_list(&mut self, list: modlist::ModList) -> Result<StatusLine> {
        // Computed inside the write transaction, from the stored list: the
        // in-memory copy may lag behind a job that just installed a mod.
        let mut outcome = None;
        self.update_registry(|r| {
            let applied = apply(&list, r);
            let next = applied.registry.clone();
            outcome = Some(applied);
            Ok(next)
        })?;
        let outcome = outcome.expect("update ran the closure");
        let missing = outcome.missing.len();
        self.mods.list_import = (missing > 0).then_some(ListImport {
            list,
            missing: outcome.missing,
        });
        let text = format!("Mod list applied to {} mod(s)", outcome.matched);
        Ok(match missing {
            0 => StatusLine::new(Level::Success, format!("{text} — press Apply")),
            n => StatusLine::new(
                Level::Warning,
                format!("{text}; {n} not installed — see the list of missing mods"),
            ),
        })
    }

    /// After an install: re-apply the pending list so new mods take their place.
    pub(super) fn reapply_pending_list(&mut self) {
        let Some(pending) = self.mods.list_import.take() else {
            return;
        };
        if let Err(err) = self.apply_list(pending.list) {
            self.status = StatusLine::new(Level::Error, err.to_string());
        }
    }

    fn download_missing(&mut self) -> Result<StatusLine> {
        if !self.can_download() {
            return Err(PyluneError::Invalid(
                "direct downloads need a Premium account — open each mod's page instead".into(),
            ));
        }
        let jobs: Vec<DownloadJob> = self
            .mods
            .list_import
            .iter()
            .flat_map(|i| i.missing.iter())
            .filter_map(|m| m.nexus.as_ref())
            .map(|n| DownloadJob {
                mod_id: n.mod_id,
                file_id: n.file_id,
                nxm: None,
                replace: None,
                collection: None,
            })
            .collect();
        let count = jobs.len();
        jobs.into_iter().for_each(|job| {
            self.submit(JobKind::Download(job));
        });
        Ok(StatusLine::new(
            Level::Info,
            format!("Downloading {count} missing mod(s)…"),
        ))
    }

    fn open_missing(&mut self, index: usize) -> Result<()> {
        let nexus = self
            .mods
            .list_import
            .as_ref()
            .and_then(|i| i.missing.get(index))
            .and_then(|m| m.nexus.clone())
            .ok_or_else(|| PyluneError::Invalid("this mod did not come from Nexus".into()))?;
        let url = format!(
            "https://www.nexusmods.com/{}/mods/{}?tab=files&file_id={}",
            self.config.nexus.domain, nexus.mod_id, nexus.file_id
        );
        self.open_path(std::path::Path::new(&url))
    }
}
