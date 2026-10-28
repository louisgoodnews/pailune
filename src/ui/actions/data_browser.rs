//! Data Browser actions: listing tables from archives, loading table bodies,
//! and inspecting individual records.

use crate::error::Result;
use crate::ui::PailuneApp;
use crate::ui::intent::DataBrowserAction;

impl PailuneApp {
    /// Dispatches Data Browser actions.
    pub(super) fn data_browser_action(
        &mut self,
        action: DataBrowserAction,
    ) -> Result<Option<crate::ui::StatusLine>> {
        match action {
            DataBrowserAction::RefreshTables => {
                self.load_data_browser_tables();
                Ok(None)
            }
            DataBrowserAction::SelectTable(stem) => {
                self.select_data_browser_table(&stem);
                Ok(None)
            }
            DataBrowserAction::SelectRecord(key) => {
                self.select_data_browser_record(key);
                Ok(None)
            }
            DataBrowserAction::SetTargetMod(name) => {
                self.data_browser.target_mod = Some(name);
                Ok(None)
            }
            DataBrowserAction::StartFieldEdit(edit) => {
                self.data_browser.active_edit = Some(edit);
                Ok(None)
            }
            DataBrowserAction::CancelFieldEdit => {
                self.data_browser.active_edit = None;
                Ok(None)
            }
            DataBrowserAction::AddChangeToMod {
                mod_name,
                table_stem,
                record_name,
                record_key,
                field_name,
                new_value,
                is_raw,
                rel_offset,
                original_hex,
            } => {
                self.data_browser.active_edit = None;
                self.add_browser_change_to_mod(
                    &mod_name,
                    &table_stem,
                    &record_name,
                    record_key,
                    field_name.as_deref(),
                    &new_value,
                    is_raw,
                    rel_offset,
                    &original_hex,
                )
            }
        }
    }

    /// Adds a field or byte change directly from the Data Browser to a staged JSON mod.
    #[allow(clippy::too_many_arguments)]
    pub fn add_browser_change_to_mod(
        &mut self,
        mod_name: &str,
        table_stem: &str,
        record_name: &str,
        record_key: u64,
        field_name: Option<&str>,
        new_value: &str,
        is_raw: bool,
        rel_offset: usize,
        original_hex: &str,
    ) -> Result<Option<crate::ui::StatusLine>> {
        let clean = crate::mod_engine::install::sanitize_name(mod_name);
        if clean.is_empty() {
            return Err(crate::error::PailuneError::Invalid(
                "Mod name cannot be empty".into(),
            ));
        }
        let mod_dir = self.paths.staging.join(&clean);
        std::fs::create_dir_all(&mod_dir).map_err(|e| {
            crate::error::PailuneError::Invalid(format!("Could not create mod directory: {e}"))
        })?;

        // Find existing JSON patch file in staging folder, or create new format-3 file
        let mut target_file = None;
        if let Ok(entries) = std::fs::read_dir(&mod_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|ext| ext == "json") {
                    target_file = Some(p);
                    break;
                }
            }
        }

        let file_path = target_file.unwrap_or_else(|| mod_dir.join(format!("{clean}.field.json")));
        let mut doc: serde_json::Value = if file_path.exists() {
            let text = std::fs::read_to_string(&file_path).map_err(|e| {
                crate::error::PailuneError::Invalid(format!("Could not read mod file: {e}"))
            })?;
            serde_json::from_str(&text).map_err(|e| {
                crate::error::PailuneError::Invalid(format!("Invalid JSON in mod file: {e}"))
            })?
        } else {
            serde_json::json!({
                "format": 3,
                "modinfo": {
                    "title": clean,
                    "version": "1.0",
                    "description": ""
                },
                "targets": []
            })
        };

        let game_file = format!("gamedata/binarystaticinfo__/bin/{table_stem}.staticinfobody");

        if !is_raw && let Some(field) = field_name {
            // Format-3 field intent
            let parsed_val: serde_json::Value = if let Ok(n) = new_value.trim().parse::<i64>() {
                serde_json::json!(n)
            } else if let Ok(f) = new_value.trim().parse::<f64>() {
                serde_json::json!(f)
            } else if new_value.trim() == "true" {
                serde_json::json!(true)
            } else if new_value.trim() == "false" {
                serde_json::json!(false)
            } else if let Ok(val) = serde_json::from_str::<serde_json::Value>(new_value) {
                val
            } else {
                serde_json::json!(new_value.trim())
            };

            // Ensure format 3 targets
            if doc.get("targets").is_none() {
                doc["targets"] = serde_json::json!([]);
            }
            let targets = doc["targets"].as_array_mut().ok_or_else(|| {
                crate::error::PailuneError::Invalid("targets is not an array".into())
            })?;

            let target_idx = match targets.iter().position(|t| {
                t.get("file")
                    .or_else(|| t.get("target"))
                    .and_then(|f| f.as_str())
                    == Some(&game_file)
            }) {
                Some(idx) => idx,
                None => {
                    targets.push(serde_json::json!({
                        "file": game_file,
                        "intents": []
                    }));
                    targets.len() - 1
                }
            };

            let intents = targets[target_idx]["intents"]
                .as_array_mut()
                .ok_or_else(|| {
                    crate::error::PailuneError::Invalid("intents is not an array".into())
                })?;

            let existing_idx = intents.iter().position(|i| {
                i.get("field").and_then(|f| f.as_str()) == Some(field)
                    && (i.get("key").and_then(|k| k.as_u64()) == Some(record_key)
                        || (!record_name.is_empty()
                            && i.get("entry").and_then(|e| e.as_str()) == Some(record_name)))
            });

            match existing_idx {
                Some(idx) => {
                    intents[idx]["new"] = parsed_val;
                }
                None => {
                    let entry_val = if record_name.is_empty() {
                        serde_json::Value::Null
                    } else {
                        serde_json::json!(record_name)
                    };
                    intents.push(serde_json::json!({
                        "entry": entry_val,
                        "key": record_key,
                        "field": field,
                        "op": "set",
                        "new": parsed_val
                    }));
                }
            }
        } else {
            // Raw byte change
            let clean_hex: String = new_value.chars().filter(|c| !c.is_whitespace()).collect();
            if crate::tables::format3::is_format3(&doc) {
                if doc.get("targets").is_none() {
                    doc["targets"] = serde_json::json!([]);
                }
                let targets = doc["targets"].as_array_mut().ok_or_else(|| {
                    crate::error::PailuneError::Invalid("targets is not an array".into())
                })?;
                let target_idx = match targets.iter().position(|t| {
                    t.get("file")
                        .or_else(|| t.get("target"))
                        .and_then(|f| f.as_str())
                        == Some(&game_file)
                }) {
                    Some(idx) => idx,
                    None => {
                        targets.push(serde_json::json!({
                            "file": game_file,
                            "intents": []
                        }));
                        targets.len() - 1
                    }
                };
                let intents = targets[target_idx]["intents"]
                    .as_array_mut()
                    .ok_or_else(|| {
                        crate::error::PailuneError::Invalid("intents is not an array".into())
                    })?;
                let entry_val = if record_name.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::json!(record_name)
                };
                intents.push(serde_json::json!({
                    "entry": entry_val,
                    "key": record_key,
                    "op": "replace",
                    "old": original_hex,
                    "new": clean_hex
                }));
            } else {
                // Format 2 patches
                if doc.get("patches").is_none() {
                    doc["patches"] = serde_json::json!([]);
                }
                let patches = doc["patches"].as_array_mut().ok_or_else(|| {
                    crate::error::PailuneError::Invalid("patches is not an array".into())
                })?;
                let patch_idx = match patches.iter().position(|p| {
                    p.get("game_file")
                        .or_else(|| p.get("file"))
                        .and_then(|f| f.as_str())
                        == Some(&game_file)
                }) {
                    Some(idx) => idx,
                    None => {
                        patches.push(serde_json::json!({
                            "game_file": game_file,
                            "changes": []
                        }));
                        patches.len() - 1
                    }
                };
                let changes = patches[patch_idx]["changes"]
                    .as_array_mut()
                    .ok_or_else(|| {
                        crate::error::PailuneError::Invalid("changes is not an array".into())
                    })?;
                let orig_val = match original_hex.is_empty() {
                    true => serde_json::Value::Null,
                    false => serde_json::json!(original_hex),
                };
                changes.push(serde_json::json!({
                    "entry": record_name,
                    "rel_offset": rel_offset,
                    "original": orig_val,
                    "patched": clean_hex
                }));
            }
        }

        let json_text = crate::patch::document::pretty(&doc);
        std::fs::write(&file_path, json_text).map_err(|e| {
            crate::error::PailuneError::Invalid(format!("Could not write mod file: {e}"))
        })?;

        let now = crate::db::now();
        let staging = self.paths.staging.clone();
        let scan = crate::mod_engine::registry::scan(&staging)?;
        self.update_registry(|r| Ok(crate::mod_engine::registry::merge(r, &scan, now)))?;
        let _ = crate::db::mods::record_install(&self.conn, &clean, "modified", None, now);
        self.select(clean.clone())?;

        let label = if is_raw {
            format!("raw bytes @+{rel_offset}")
        } else {
            field_name.unwrap_or("field").to_string()
        };

        self.notices.push(
            crate::ui::Level::Success,
            "Change added to mod",
            &format!("Added change on {record_name} ({label}) to mod '{clean}'"),
        );

        Ok(Some(crate::ui::StatusLine::new(
            crate::ui::Level::Success,
            format!("Added change on {record_name} to mod '{clean}'"),
        )))
    }

    /// Loads the list of available static info tables from archive group 0008.
    pub fn load_data_browser_tables(&mut self) {
        if self.game != crate::steam::GameStatus::Ready {
            self.data_browser.error = Some("Game installation not found or not ready.".into());
            self.data_browser.loaded = true;
            return;
        }
        match crate::tables::browser::list_tables(&self.info.game_dir) {
            Ok(tables) => {
                self.data_browser.tables = tables;
                self.data_browser.error = None;
                self.data_browser.loaded = true;
            }
            Err(err) => {
                self.data_browser.error = Some(err.to_string());
                self.data_browser.loaded = true;
            }
        }
    }

    /// Loads and parses the records of a selected table.
    pub fn select_data_browser_table(&mut self, stem: &str) {
        self.data_browser.selected_table = Some(stem.to_string());
        self.data_browser.selected_record = None;
        self.data_browser.inspected = None;
        self.data_browser.record_filter.clear();

        match crate::tables::browser::read_table(&self.info.game_dir, stem) {
            Ok((records, body)) => {
                self.data_browser.records = records;
                self.data_browser.body = body;
                self.data_browser.error = None;
                // Auto-select the first record if available
                if let Some(first) = self.data_browser.records.first().cloned() {
                    let inspected = crate::tables::browser::inspect_record(
                        stem,
                        &first,
                        &self.data_browser.body,
                    );
                    self.data_browser.selected_record = Some(first.key);
                    self.data_browser.inspected = Some(inspected);
                }
            }
            Err(err) => {
                self.data_browser.records.clear();
                self.data_browser.body.clear();
                self.data_browser.error = Some(format!("Could not load table {stem}: {err}"));
            }
        }
    }

    /// Selects and decodes an individual record by key.
    pub fn select_data_browser_record(&mut self, key: u64) {
        self.data_browser.selected_record = Some(key);
        let Some(stem) = self.data_browser.selected_table.as_deref() else {
            return;
        };
        if let Some(rec) = self.data_browser.records.iter().find(|r| r.key == key) {
            let inspected =
                crate::tables::browser::inspect_record(stem, rec, &self.data_browser.body);
            self.data_browser.inspected = Some(inspected);
        }
    }
}
