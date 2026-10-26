//! Shown instead of the main window when Pylune cannot start safely.

use super::theme::{self, danger, muted};
use crate::config::paths::DataPaths;
use eframe::egui::{self, RichText};
use std::path::PathBuf;

/// A minimal window explaining a start-up failure.
pub struct FatalApp {
    message: String,
    database: PathBuf,
    data_dir: PathBuf,
}

impl FatalApp {
    /// A window explaining that the database could not be opened.
    pub fn new(message: String, paths: &DataPaths) -> Self {
        let data_dir = paths
            .database
            .parent()
            .map(PathBuf::from)
            .unwrap_or_default();
        tracing::error!(%message, "cannot open the database");
        Self {
            message,
            database: paths.database.clone(),
            data_dir,
        }
    }
}

impl eframe::App for FatalApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        theme::apply(ui.ctx(), &Default::default());
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(RichText::new("Pylune cannot open its database").color(danger()));
            ui.label(&self.message);
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("Database: {}", self.database.display())).color(muted()),
            );
            ui.label(
                "Nothing in the game folder was changed. If another program is using the \
                 database, close it and start Pylune again. If the file is damaged, move it \
                 aside (keep a copy) and Pylune will create a new one; the game folder can \
                 then be cleaned with Clean Slate.",
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("📂 Open data folder").clicked() {
                    let _ = crate::platform::open_command(self.data_dir.as_os_str()).spawn();
                }
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}
