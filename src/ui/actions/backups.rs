//! Backup actions: create, restore, delete.

use crate::app_backup::{AUTO_LABEL, create};
use crate::config::file;
use crate::error::{PyluneError, Result};
use crate::jobs::JobKind;
use crate::ui::{BackupAction, Confirm, Level, PyluneApp, SettingsDraft, StatusLine};

impl PyluneApp {
    /// Handles backup actions (the work itself runs as exclusive jobs).
    pub(super) fn backup_action(&mut self, action: BackupAction) -> Result<Option<StatusLine>> {
        match action {
            BackupAction::Create { full } => {
                self.submit(JobKind::CreateBackup {
                    full,
                    label: "manual".into(),
                });
                Ok(Some(StatusLine::new(Level::Info, "Creating backup…")))
            }
            BackupAction::Restore(path) => {
                self.confirm = None;
                if self.deployed_links > 0 {
                    return Err(PyluneError::Invalid(
                        "unlink all mods before restoring a backup".into(),
                    ));
                }
                self.submit(JobKind::RestoreBackup(path));
                Ok(Some(StatusLine::new(
                    Level::Info,
                    "Restoring backup (a safety backup is taken first)…",
                )))
            }
            BackupAction::RestoreFromFile => {
                let picked = rfd::FileDialog::new()
                    .add_filter("Pylune backup", &["zip"])
                    .pick_file();
                Ok(picked.map(|path| {
                    self.confirm = Some(Confirm::RestoreBackup(path));
                    StatusLine::new(Level::Info, "Confirm the restore")
                }))
            }
            BackupAction::Delete(path) => {
                self.submit(JobKind::DeleteBackup(path));
                Ok(None)
            }
            BackupAction::OpenFolder => self.open_path(&self.paths.backups.clone()).map(|()| None),
        }
    }

    /// Queues the daily automatic backup if one is due.
    pub fn schedule_auto_backup(&mut self) {
        if self.config.backup.auto_daily && create::auto_due(&self.paths.backups, crate::db::now())
        {
            self.submit(JobKind::CreateBackup {
                full: false,
                label: AUTO_LABEL.into(),
            });
        }
    }

    /// Re-reads settings after a restore replaced `config.toml`.
    pub(super) fn reload_config(&mut self) {
        let (config, notice) = file::load_or_recover(&self.paths.config_file);
        self.draft = SettingsDraft::from_config(&config);
        self.config = config;
        if let Some(notice) = notice {
            self.status = StatusLine::new(Level::Warning, notice);
        }
    }
}
