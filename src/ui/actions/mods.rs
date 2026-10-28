//! Mod management actions: selection, flags, order, import, update, delete.

use crate::error::{PailuneError, Result};
use crate::jobs::JobKind;
use crate::mod_engine::{info, registry};
use crate::ui::{Level, ModAction, PailuneApp, StatusLine, Tab};
use std::path::PathBuf;

/// Extensions offered in the import dialogs.
const MOD_FILES: [&str; 7] = ["zip", "7z", "rar", "asi", "dll", "json", "ini"];

impl PailuneApp {
    /// Handles mod management actions.
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
            ModAction::CreateModPrompt => {
                self.mods.create_mod = Some(crate::ui::state::CreateModDraft::default());
                Ok(None)
            }
            ModAction::CloseCreateMod => {
                self.mods.create_mod = None;
                Ok(None)
            }
            ModAction::CreateMod {
                name,
                author,
                version,
                description,
            } => {
                self.mods.create_mod = None;
                let clean_name = crate::mod_engine::install::sanitize_name(&name);
                if clean_name.is_empty() {
                    return Err(PailuneError::Invalid("mod name cannot be empty".into()));
                }
                let mod_dir = self.paths.staging.join(&clean_name);
                if mod_dir.exists() {
                    return Err(PailuneError::Invalid(format!(
                        "a mod named '{clean_name}' already exists in staging"
                    )));
                }
                std::fs::create_dir_all(&mod_dir)
                    .map_err(|e| PailuneError::Invalid(format!("could not create mod directory: {e}")))?;
                let doc = serde_json::json!({
                    "format": 3,
                    "modinfo": {
                        "title": clean_name,
                        "author": author.trim(),
                        "version": match version.trim().is_empty() {
                            true => "1.0",
                            false => version.trim(),
                        },
                        "description": description.trim(),
                    },
                    "targets": []
                });
                let file_path = mod_dir.join(format!("{clean_name}.field.json"));
                let json_text = crate::patch::document::pretty(&doc);
                std::fs::write(&file_path, json_text)
                    .map_err(|e| PailuneError::Invalid(format!("could not write mod file: {e}")))?;
                let now = crate::db::now();
                let staging = self.paths.staging.clone();
                let scan = registry::scan(&staging)?;
                self.update_registry(|r| Ok(registry::merge(r, &scan, now)))?;
                crate::db::mods::record_install(&self.conn, &clean_name, "created", None, now)?;
                self.select(clean_name.clone())?;
                self.notices.push(
                    Level::Success,
                    "Mod created",
                    &format!("Created '{clean_name}' with a format-3 JSON patch document"),
                );
                Ok(Some(StatusLine::new(
                    Level::Success,
                    format!("Created mod '{clean_name}'"),
                )))
            }
            ModAction::PackageModPrompt(name) => {
                let mod_dir = self.paths.staging.join(&name);
                let mut author = String::new();
                let mut version = "1.0".to_string();
                let mut description = String::new();

                let modinfo_path = mod_dir.join("modinfo.json");
                if let Ok(text) = std::fs::read_to_string(&modinfo_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(a) = val.get("author").and_then(|v| v.as_str()) {
                            author = a.to_string();
                        }
                        if let Some(v) = val.get("version").and_then(|v| v.as_str()) {
                            version = v.to_string();
                        }
                        if let Some(d) = val.get("description").and_then(|v| v.as_str()) {
                            description = d.to_string();
                        }
                    }
                } else if let Ok(entries) = std::fs::read_dir(&mod_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().is_some_and(|e| e == "json") {
                            if let Ok(text) = std::fs::read_to_string(&p) {
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if let Some(info) = val.get("modinfo") {
                                        if let Some(a) = info.get("author").and_then(|v| v.as_str()) {
                                            author = a.to_string();
                                        }
                                        if let Some(v) = info.get("version").and_then(|v| v.as_str()) {
                                            version = v.to_string();
                                        }
                                        if let Some(d) = info.get("description").and_then(|v| v.as_str()) {
                                            description = d.to_string();
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                let bumped_version = crate::mod_engine::package::bump_version_string(&version);
                self.mods.package_mod = Some(crate::mod_engine::package::PackageMetadata {
                    name,
                    author,
                    version: bumped_version,
                    description,
                    bump_version: true,
                });
                Ok(None)
            }
            ModAction::ClosePackageMod => {
                self.mods.package_mod = None;
                Ok(None)
            }
            ModAction::PackageMod(meta) => {
                let exports_dir = self.paths.exports.clone();
                let res = crate::mod_engine::package::package_mod(
                    &self.paths.staging,
                    &meta.name,
                    &exports_dir,
                    &meta,
                )?;
                self.mods.package_mod = None;
                let size_kb = (res.total_bytes / 1024).max(1);
                let archive_name = res
                    .archive_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.notices.push(
                    Level::Success,
                    "Mod Packaged",
                    &format!(
                        "Created archive: exports/{archive_name} ({} file(s), {size_kb} KB)",
                        res.file_count,
                    ),
                );
                Ok(Some(StatusLine::new(
                    Level::Success,
                    format!("Packaged mod '{}' as exports/{archive_name}", meta.name),
                )))
            }
            ModAction::SpawnProjectPrompt => {
                self.mods.spawn_project = Some(crate::ui::state::SpawnProjectDraft::default());
                Ok(None)
            }
            ModAction::CloseSpawnProject => {
                self.mods.spawn_project = None;
                Ok(None)
            }
            ModAction::SpawnProject(draft) => {
                self.mods.spawn_project = None;
                let clean_name = crate::mod_engine::install::sanitize_name(&draft.name);
                if clean_name.is_empty() {
                    return Err(PailuneError::Invalid("project name cannot be empty".into()));
                }
                let loc = PathBuf::from(draft.location.trim());
                if !loc.is_dir() {
                    return Err(PailuneError::Invalid(format!(
                        "destination folder '{}' does not exist",
                        loc.display()
                    )));
                }

                let cfg = crate::sdk::ProjectConfig {
                    name: clean_name.clone(),
                    mod_type: draft.mod_type,
                    author: draft.author.clone(),
                    version: draft.version.clone(),
                    description: draft.description.clone(),
                    target_dir: loc,
                };
                let report = crate::sdk::scaffold_project(&cfg)?;

                if draft.stage_after {
                    let _ = crate::sdk::export_project(
                        &report.project_dir,
                        &self.paths.exports,
                        Some(&self.paths.staging),
                    );
                    let now = crate::db::now();
                    let scan = registry::scan(&self.paths.staging)?;
                    self.update_registry(|r| Ok(registry::merge(r, &scan, now)))?;
                    let _ = crate::db::mods::record_install(&self.conn, &clean_name, "created", None, now);
                    self.select(clean_name.clone())?;
                }

                let type_name = draft.mod_type.as_str().to_uppercase();
                self.notices.push(
                    Level::Success,
                    "Project Spawned",
                    &format!(
                        "Created {type_name} project '{}' at {}",
                        clean_name,
                        report.project_dir.display()
                    ),
                );

                Ok(Some(StatusLine::new(
                    Level::Success,
                    format!("Created {type_name} project '{}'", clean_name),
                )))
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
            ModAction::ImportPaths(paths) => Ok(Some(self.import(paths))),
            ModAction::SaveModFile(path, text) => {
                // Only files of staged mods; deployed links point at them.
                let escapes = path
                    .components()
                    .any(|c| c == std::path::Component::ParentDir);
                if escapes || !path.starts_with(&self.paths.staging) {
                    return Err(PailuneError::Invalid(
                        "not a file of an installed mod".into(),
                    ));
                }
                std::fs::write(&path, text)
                    .map_err(|e| PailuneError::Invalid(format!("{}: {e}", path.display())))?;
                crate::db::links::restamp_shared(&self.conn, &path)?;
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                // Packed content (JSON patches) reaches the game only through
                // the archive group Apply rebuilds.
                let packed = crate::mod_engine::plan::placement(
                    path.strip_prefix(&self.paths.staging).unwrap_or(&path),
                ) == crate::mod_engine::plan::Placement::JsonPatch;
                if packed {
                    crate::db::mods::bump_revision(&self.conn)?;
                }
                let when = match (packed, cfg!(windows)) {
                    (true, _) => "press Apply to rebuild the game's data",
                    (false, true) => "press Apply to copy it into the game",
                    (false, false) => "it applies the next time the game starts",
                };
                Ok(Some(StatusLine::new(
                    Level::Success,
                    format!("Saved {name} — {when}"),
                )))
            }
            ModAction::RestoreDeleted(id) => {
                self.mods.undo = None;
                self.submit(JobKind::RestoreDeleted(id));
                Ok(Some(StatusLine::new(Level::Info, "Restoring…")))
            }
            ModAction::PurgeDeleted(id) => {
                self.submit(JobKind::PurgeDeleted(id));
                Ok(None)
            }
            ModAction::SetVariant(name, variant) => {
                self.update_registry(|r| {
                    Ok(registry::with_variant(r, &name, Some(variant.clone())))
                })?;
                self.select(name.clone())?;
                let what = match variant.starts_with(crate::fomod::PREFIX) {
                    true => "installer options changed".to_string(),
                    false => format!("option \"{variant}\""),
                };
                Ok(Some(StatusLine::new(
                    Level::Info,
                    format!("{name}: {what} — press Apply"),
                )))
            }
            ModAction::Rescan => {
                let now = crate::db::now();
                let staging = self.paths.staging.clone();
                let scan = registry::scan(&staging)?;
                let before: Vec<String> =
                    self.registry.mods.iter().map(|m| m.name.clone()).collect();
                self.update_registry(|r| Ok(registry::merge(r, &scan, now)))?;
                let mods = &self.registry.mods;
                let added = mods.iter().filter(|m| !before.contains(&m.name)).count();
                let gone = before
                    .iter()
                    .filter(|b| !mods.iter().any(|m| &m.name == *b))
                    .count();
                let text = format!(
                    "{} mod(s) in staging — {added} new, {gone} gone",
                    mods.len()
                );
                self.notices.push(Level::Info, "Scan complete", &text);
                Ok(Some(StatusLine::new(Level::Info, text)))
            }
        }
    }

    /// Selects a mod and loads its details.
    pub(crate) fn select(&mut self, name: String) -> Result<()> {
        let details = info::details(
            &self.paths.staging.join(&name),
            self.variant_of(&name).as_deref(),
        )?;
        self.mods.details = Some((name.clone(), details));
        self.mods.readme = None;
        self.mods.selected = Some(name);
        match self.log.only_selected {
            true => self.refresh_log(),
            false => Ok(()),
        }
    }

    /// Opens a readme of the selected mod.
    fn show_readme(&mut self, rel: PathBuf) -> Result<()> {
        let name = self
            .mods
            .selected
            .clone()
            .ok_or_else(|| PailuneError::Invalid("no mod selected".into()))?;
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
            .ok_or_else(|| PailuneError::Invalid(format!("no mod named '{name}'")))?;
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

/// Status after a change to the mod list.
fn changed() -> StatusLine {
    StatusLine::new(Level::Info, "Selection changed — press Apply to deploy")
}
