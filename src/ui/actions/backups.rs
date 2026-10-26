//! Backup actions: create, restore, delete.

use crate::app_backup::{AUTO_LABEL, create};
use crate::config::file;
use crate::error::{PyluneError, Result};
use crate::jobs::JobKind;
use crate::save_backup;
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
            BackupAction::BackUpSaves => {
                self.submit(JobKind::BackupSaves);
                Ok(Some(StatusLine::new(Level::Info, "Backing up save games…")))
            }
            BackupAction::RestoreSaves(path) => {
                self.confirm = None;
                if self.process == crate::launch::GameProcess::Running {
                    return Err(PyluneError::Invalid(
                        "close Crimson Desert before restoring save games".into(),
                    ));
                }
                self.submit(JobKind::RestoreSaves(path));
                Ok(None)
            }
            BackupAction::InspectSave(dir) => self.open_save(dir),
            BackupAction::SaveEdits
            | BackupAction::WriteSaveEdits
            | BackupAction::DiscardSaveEdits => self.save_edit_action(action),
            BackupAction::CloseSaveView => {
                self.save_view = None;
                Ok(None)
            }
            BackupAction::OpenSaveFolder => {
                let folder = self.save_folder()?;
                self.open_path(&folder).map(|()| None)
            }
        }
    }

    /// The game's save folder, if the Proton prefix is known.
    pub(super) fn save_folder(&self) -> Result<std::path::PathBuf> {
        save_backup::save_dir(self.info.prefix_dir.as_deref()).ok_or_else(|| {
            PyluneError::Invalid(
                "the game's Proton prefix was not found — start the game once from Steam".into(),
            )
        })
    }

    /// Backs up the saves right before Play (synchronously, so the copy
    /// precedes the game). Returns a warning when that failed; the game
    /// still starts, since a missing prefix is normal before the first run.
    pub(super) fn backup_saves_before_play(&mut self) -> Option<String> {
        if !self.config.backup.saves_before_play {
            return None;
        }
        let dest = self.paths.save_backups.clone();
        let made = self.save_folder().and_then(|saves| {
            save_backup::create(&saves, &dest, save_backup::AUTO_LABEL, crate::db::now())
        });
        self.save_backups = save_backup::list(&dest);
        match made {
            Ok(_) => save_backup::prune_auto(&dest, self.config.backup.keep_saves.max(1))
                .err()
                .map(|e| format!("pruning save backups failed: {e}")),
            Err(err) => Some(format!("save games not backed up: {err}")),
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
