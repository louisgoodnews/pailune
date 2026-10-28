//! Data Browser actions: listing tables from archives, loading table bodies,
//! and inspecting individual records.

use crate::error::Result;
use crate::ui::intent::DataBrowserAction;
use crate::ui::PailuneApp;

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
        }
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

