//! Bootstrapping and the eframe [`App`](eframe::App) implementation.

use super::{Action, Level, NexusState, ProfilesState, PyluneApp, SettingsDraft, StatusLine, Tab};
use super::{theme, views};
use crate::backup::snapshot;
use crate::config::{Config, store};
use crate::launch::run;
use crate::mod_engine::{model::Registry, registry};
use crate::nexus::{NexusMeta, handler, worker};
use crate::steam::{GameStatus, SteamInfo, locate};
use crate::symlink::Manifest;
use eframe::egui;
use std::time::{Duration, Instant};

impl PyluneApp {
    /// Loads persisted state, starts the Nexus worker and takes the first backup.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let paths = store::data_paths().expect("Pylune needs a writable data directory");
        let config: Config = store::load_or_default(&paths.config).unwrap_or_default();
        let manifest: Manifest = store::load_or_default(&paths.manifest).unwrap_or_default();
        let saved: Registry = store::load_or_default(&paths.registry).unwrap_or_default();
        let registry = registry::sync(&saved, &paths.staging).unwrap_or(saved);
        let meta: NexusMeta = store::load_or_default(&paths.nexus_meta).unwrap_or_default();
        let profiles = ProfilesState {
            book: store::load_or_default(&paths.profiles).unwrap_or_default(),
            name_draft: String::new(),
        };
        let ctx = cc.egui_ctx.clone();
        let worker = worker::spawn(
            paths.staging.clone(),
            paths.downloads.clone(),
            Box::new(move || ctx.request_repaint()),
        );
        spawn_ticker(cc.egui_ctx.clone());
        let info = locate::resolve(&config);
        let mut app = Self {
            draft: SettingsDraft::from_config(&config),
            game: locate::game_status(&info),
            loader: locate::detect_loader(&info),
            backup: snapshot::status(&info, &manifest),
            status: initial_status(&info),
            confirm: None,
            dirty: false,
            tab: Tab::Mods,
            process: run::game_process(),
            nexus: NexusState {
                meta,
                handler_registered: handler::is_registered(),
                ..NexusState::default()
            },
            last_poll: Instant::now(),
            profiles,
            worker,
            paths,
            config,
            registry,
            manifest,
            info,
        };
        app.refresh_environment();
        app.connect_nexus();
        app
    }
}

/// Wakes the app once per second, even while its window is hidden or
/// occluded, so the game process and the nxm queue keep being polled.
fn spawn_ticker(ctx: egui::Context) {
    std::thread::spawn(move || {
        std::iter::repeat_with(|| std::thread::sleep(Duration::from_secs(1)))
            .for_each(|()| ctx.request_repaint());
    });
}

/// Takes the first-run `.bak` snapshot when the game is present.
fn initial_status(info: &SteamInfo) -> StatusLine {
    let ready = locate::game_status(info) == GameStatus::Ready;
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
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
    }

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
                .show(ui, |ui| match self.tab {
                    Tab::Mods => views::mod_list::show(ui, self),
                    Tab::Profiles => views::profiles::show(ui, self),
                    Tab::Nexus => views::nexus::show(ui, self),
                })
                .inner,
            views::confirm::show(ui.ctx(), self.confirm.as_ref(), &self.registry),
        ]
        .into_iter()
        .flatten()
        .collect();
        actions.into_iter().for_each(|action| self.dispatch(action));
    }
}
