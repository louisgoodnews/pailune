//! Mod management actions: selection, flags, order, import, update, delete.

use crate::error::{PyluneError, Result};
use crate::jobs::JobKind;
use crate::mod_engine::{info, registry};
use crate::ui::{Level, ModAction, PyluneApp, StatusLine, Tab};
use std::path::PathBuf;

/// Extensions offered in the import dialogs.
const MOD_FILES: [&str; 7] = ["zip", "7z", "rar", "asi", "dll", "json", "ini"];

impl PyluneApp {
    pub(super) fn mod_action(&mut self, action: ModAction) -> Result<Option<StatusLine>> {
        match action {
            ModAction::Select(name) => self.select(name).map(|()| None),
            ModAction::SetEnabled(name, enabled) => {
                self.update_registry(|r| Ok(registry::with_enabled(r, &name, enabled)))?;
                Ok(Some(changed()))
            }
            ModAction::SetAll(enabled) => {
                self.update_registry(|r| Ok(registry::with_all(r, enabled)))?;
                Ok(Some(changed()))
            }
            ModAction::Move(name, to) => {
                self.update_registry(|r| Ok(registry::moved(r, &name, to)))?;
                Ok(Some(changed()))
            }
            ModAction::Delete(name) => {
                self.confirm = None;
                self.submit(JobKind::Remove(name.clone()));
                Ok(Some(StatusLine::new(
                    Level::Info,
                    format!("Deleting '{name}'…"),
                )))
            }
            ModAction::Update(name) => self.update_mod(name).map(Some),
            ModAction::OpenFolder(name) => self
                .open_path(&self.paths.staging.join(name))
                .map(|()| None),
            ModAction::ShowReadme(rel) => self.show_readme(rel).map(|()| None),
            ModAction::CloseReadme => {
                self.mods.readme = None;
                Ok(None)
            }
            ModAction::ImportFolder => Ok(Some(
                self.import(rfd::FileDialog::new().pick_folder().into_iter().collect()),
            )),
            ModAction::ImportFiles => Ok(Some(
                self.import(
                    rfd::FileDialog::new()
                        .add_filter("Mods", &MOD_FILES)
                        .pick_files()
                        .unwrap_or_default(),
                ),
            )),
            ModAction::Rescan => {
                let now = crate::db::now();
                let staging = self.paths.staging.clone();
                self.update_registry(|r| registry::sync(r, &staging, now))?;
                Ok(Some(StatusLine::new(
                    Level::Info,
                    format!("{} mod(s) in staging", self.registry.mods.len()),
                )))
            }
        }
    }

    fn select(&mut self, name: String) -> Result<()> {
        let details = info::details(&self.paths.staging.join(&name))?;
        self.mods.details = Some((name.clone(), details));
        self.mods.readme = None;
        self.mods.selected = Some(name);
        match self.log.only_selected {
            true => self.refresh_log(),
            false => Ok(()),
        }
    }

    fn show_readme(&mut self, rel: PathBuf) -> Result<()> {
        let name = self
            .mods
            .selected
            .clone()
            .ok_or_else(|| PyluneError::Invalid("no mod selected".into()))?;
        let text = info::read_text(&self.paths.staging.join(name).join(&rel))?;
        self.mods.readme = Some((rel, text));
        Ok(())
    }

    /// Queues one import job per source; they run in parallel.
    fn import(&mut self, sources: Vec<PathBuf>) -> StatusLine {
        let count = sources.len();
        sources.into_iter().for_each(|source| {
            self.submit(JobKind::Import {
                source,
                replace: None,
            });
        });
        match count {
            0 => StatusLine::new(Level::Info, "Import cancelled"),
            n => StatusLine::new(
                Level::Info,
                format!("Importing {n} mod(s) in the background…"),
            ),
        }
    }

    /// Nexus mods open their file list; others ask for a replacement file.
    fn update_mod(&mut self, name: String) -> Result<StatusLine> {
        let entry = self
            .registry
            .mods
            .iter()
            .find(|m| m.name == name)
            .cloned()
            .ok_or_else(|| PyluneError::Invalid(format!("no mod named '{name}'")))?;
        match entry.nexus {
            Some(origin) => {
                self.tab = Tab::Nexus;
                self.nexus.mod_id_draft = origin.mod_id.to_string();
                self.nexus.replace_target = Some(name.clone());
                self.submit(JobKind::FetchMod(origin.mod_id));
                Ok(StatusLine::new(
                    Level::Info,
                    format!("Pick the new file for '{name}'"),
                ))
            }
            None => {
                let picked = rfd::FileDialog::new()
                    .set_title(format!("Replacement for {name}"))
                    .add_filter("Mods", &MOD_FILES)
                    .pick_file();
                Ok(match picked {
                    Some(source) => {
                        self.submit(JobKind::Import {
                            source,
                            replace: Some(name.clone()),
                        });
                        StatusLine::new(Level::Info, format!("Updating '{name}'…"))
                    }
                    None => StatusLine::new(Level::Info, "Update cancelled"),
                })
            }
        }
    }
}

fn changed() -> StatusLine {
    StatusLine::new(Level::Info, "Selection changed — press Apply to deploy")
}
