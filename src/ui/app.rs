//! Bootstrapping and the eframe [`App`](eframe::App) implementation.

use super::{
    Action, Level, LogState, ModsState, NexusState, ProfilesState, PyluneApp, SettingsDraft,
    StatusLine, Tab,
};
use super::{theme, views};
use crate::backup::snapshot;
use crate::config::Config;
use crate::config::paths::DataPaths;
use crate::db;
use crate::jobs::{JobKind, pool::JobPool, tasks};
use crate::launch::run;
use crate::mod_engine::registry;
use crate::nexus::{handler, key};
use crate::steam::{GameStatus, SteamInfo, locate};
use eframe::egui;
use std::sync::Arc;
use std::time::{Duration, Instant};

impl PyluneApp {
    /// Opens the database, starts the job pool and takes the first backup.
    pub fn new(cc: &eframe::CreationContext<'_>, paths: DataPaths, config: Config) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut conn = db::open(&paths.database).expect("Pylune needs a working database");
        let now = db::now();
        let staging = paths.staging.clone();
        let registry =
            db::mods::update(&mut conn, |r| registry::sync(r, &staging, now)).unwrap_or_default();
        let book = db::profiles::load(&conn).unwrap_or_default();
        let ctx = cc.egui_ctx.clone();
        let pool = JobPool::spawn(
            config.jobs.max_parallel,
            paths.clone(),
            Arc::new(tasks::run),
            Arc::new(move || ctx.request_repaint()),
        );
        spawn_ticker(cc.egui_ctx.clone());
        let info = locate::resolve(&config);
        let mut app = Self {
            draft: SettingsDraft::from_config(&config),
            game: locate::game_status(&info),
            loader: locate::detect_loader(&info),
            backup: snapshot::status(&info, &Default::default()),
            status: initial_status(&info),
            deployed_links: 0,
            confirm: None,
            dirty: false,
            tab: Tab::Mods,
            process: run::game_process(),
            mods: ModsState::default(),
            profiles: ProfilesState {
                book,
                ..ProfilesState::default()
            },
            nexus: NexusState {
                handler_registered: handler::is_registered(),
                ..NexusState::default()
            },
            log: LogState::default(),
            jobs: Vec::new(),
            launch_after: None,
            last_poll: Instant::now(),
            pool,
            paths,
            config,
            conn,
            registry,
            info,
        };
        app.refresh_environment();
        app.request_conflicts();
        if key::load(&app.paths.nexus_key).is_some() {
            app.submit(JobKind::ValidateKey);
        }
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
        None => StatusLine::new(Level::Warning, "Game not found — check the Settings tab"),
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
        let mut actions: Vec<Action> = Vec::new();
        actions.extend(
            egui::Panel::top("header")
                .show(ui, |ui| views::header::show(ui, self))
                .inner,
        );
        actions.extend(
            egui::Panel::bottom("status")
                .show(ui, |ui| views::status_bar::show(ui, self))
                .inner,
        );
        if !self.jobs.is_empty() {
            egui::Panel::bottom("jobs").show(ui, |ui| views::jobs::show(ui, self));
        }
        if self.tab == Tab::Mods && self.mods.selected.is_some() {
            let details = egui::Panel::right("details")
                .default_size(360.0)
                .resizable(true)
                .show(ui, |ui| views::mod_details::show(ui, self))
                .inner;
            actions.extend(details);
        }
        let page = egui::CentralPanel::default()
            .show(ui, |ui| match self.tab {
                Tab::Mods => views::mod_list::show(ui, self),
                Tab::Profiles => views::profiles::show(ui, self),
                Tab::Nexus => views::nexus::show(ui, self),
                Tab::Log => views::log::show(ui, self),
                Tab::Settings => views::settings::show(ui, self),
            })
            .inner;
        actions.extend(page);
        actions.extend(views::readme::show(ui.ctx(), self.mods.readme.as_ref()));
        actions.extend(views::confirm::show(ui.ctx(), self.confirm.as_ref()));
        actions.into_iter().for_each(|action| self.dispatch(action));
    }
}
