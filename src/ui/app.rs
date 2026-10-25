//! Bootstrapping and the eframe [`App`](eframe::App) implementation.

use super::{Action, Level, PyluneApp, SettingsDraft, StatusLine, theme, views};
use crate::backup::snapshot;
use crate::config::{Config, store};
use crate::mod_engine::{model::Registry, registry};
use crate::steam::locate;
use crate::symlink::Manifest;
use eframe::egui;

impl PyluneApp {
    /// Loads persisted state and takes the initial metadata backup.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let paths = store::data_paths().expect("Pylune needs a writable data directory");
        let config: Config = store::load_or_default(&paths.config).unwrap_or_default();
        let manifest: Manifest = store::load_or_default(&paths.manifest).unwrap_or_default();
        let saved: Registry = store::load_or_default(&paths.registry).unwrap_or_default();
        let registry = registry::sync(&saved, &paths.staging).unwrap_or(saved);
        let info = locate::resolve(&config);
        let status = initial_status(&info);
        let mut app = Self {
            draft: SettingsDraft::from_config(&config),
            game: locate::game_status(&info),
            loader: locate::detect_loader(&info),
            backup: snapshot::status(&info, &manifest),
            confirm: None,
            dirty: false,
            paths,
            config,
            registry,
            manifest,
            info,
            status,
        };
        app.refresh_environment();
        app
    }
}

/// Takes the first-run `.bak` snapshot when the game is present and clean.
fn initial_status(info: &crate::steam::SteamInfo) -> StatusLine {
    let ready = locate::game_status(info) == crate::steam::GameStatus::Ready;
    match ready.then(|| snapshot::ensure_meta_backup(info)) {
        None => StatusLine::new(Level::Warning, "Game not found — check the settings panel"),
        Some(Ok(0)) => StatusLine::new(Level::Info, "Ready"),
        Some(Ok(n)) => StatusLine::new(
            Level::Success,
            format!("Backed up {n} metadata file(s) to .bak"),
        ),
        Some(Err(err)) => StatusLine::new(Level::Error, format!("Backup failed: {err}")),
    }
}

impl eframe::App for PyluneApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let actions: Vec<Action> = [
            egui::Panel::top("header")
                .show(ui, |ui| views::header::show(ui, self))
                .inner,
            egui::Panel::bottom("status")
                .show(ui, |ui| views::status_bar::show(ui, self))
                .inner,
            egui::Panel::right("settings")
                .default_size(320.0)
                .resizable(true)
                .show(ui, |ui| views::settings::show(ui, self))
                .inner,
            egui::CentralPanel::default()
                .show(ui, |ui| views::mod_list::show(ui, self))
                .inner,
            views::confirm::show(ui.ctx(), self.confirm, &self.registry),
        ]
        .into_iter()
        .flatten()
        .collect();
        actions.into_iter().for_each(|action| self.dispatch(action));
    }
}
