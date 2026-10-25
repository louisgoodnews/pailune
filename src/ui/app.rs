//! Bootstrapping and the eframe [`App`](eframe::App) implementation.

use super::guide::GuideState;
use super::{
    Action, Level, LogState, ModsState, NexusState, ProfilesState, PyluneApp, SettingsDraft,
    StatusLine, Tab,
};
use super::{theme, views};
use crate::backup::snapshot;
use crate::config::Config;
use crate::config::detect::{self, Discovery};
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
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        paths: DataPaths,
        config: Config,
        conn: rusqlite::Connection,
        notice: Option<String>,
    ) -> Self {
        theme::apply(&cc.egui_ctx, &config.ui);
        spawn_ticker(cc.egui_ctx.clone());
        Self::build(cc.egui_ctx.clone(), paths, config, conn, notice)
    }

    /// Builds the application state around any egui context (a window's,
    /// or a headless one in tests).
    pub fn build(
        ctx: egui::Context,
        paths: DataPaths,
        config: Config,
        mut conn: rusqlite::Connection,
        notice: Option<String>,
    ) -> Self {
        let (config, discovery) = detect::resolve_game(config, &paths.config_file);
        let now = db::now();
        let staging = paths.staging.clone();
        // Startup problems are shown, never swallowed: a failed rescan falls
        // back to the stored list and reports the error.
        let synced = registry::scan(&staging)
            .and_then(|scan| db::mods::update(&mut conn, |r| Ok(registry::merge(r, &scan, now))));
        let (registry, scan_error) = match synced {
            Ok(registry) => (registry, None),
            Err(err) => (
                db::mods::load(&conn).unwrap_or_default(),
                Some(format!("Rescan failed: {err}")),
            ),
        };
        let book = db::profiles::load(&conn).unwrap_or_default();
        let collections = db::collections::load(&conn).unwrap_or_default();
        if let Err(err) = db::events::trim(&conn, 20_000) {
            tracing::warn!(%err, "could not trim the event journal");
        }
        let notice = notice.map(|n| StatusLine::new(Level::Warning, n));
        let notice = notice.or(scan_error.map(|e| StatusLine::new(Level::Error, e)));
        let pool = JobPool::spawn(
            config.jobs.max_parallel,
            paths.clone(),
            Arc::new(tasks::run),
            Arc::new(move || ctx.request_repaint()),
        );
        let info = locate::resolve(&config);
        let mut app = Self {
            draft: SettingsDraft::from_config(&config),
            applied_ui: config.ui,
            game: locate::game_status(&info),
            loader: locate::detect_loader(&info),
            backup: snapshot::status(&info, &Default::default()),
            status: notice.unwrap_or_else(|| initial_status(&info, &discovery)),
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
                collections,
                ..NexusState::default()
            },
            log: LogState::default(),
            jobs: Vec::new(),
            launch_after: None,
            last_poll: Instant::now(),
            close_refused: false,
            focus_requested: false,
            setup_open: discovery == Discovery::NotFound,
            guide: GuideState {
                open: config.behavior.show_guide && discovery != Discovery::NotFound,
                ..GuideState::default()
            },
            archives: Default::default(),
            backups: Vec::new(),
            save_backups: Vec::new(),
            pool,
            paths,
            config,
            conn,
            registry,
            info,
        };
        app.refresh_environment();
        app.backups = crate::app_backup::create::list(&app.paths.backups);
        app.save_backups = crate::save_backup::list(&app.paths.save_backups);
        app.schedule_auto_backup();
        app.request_conflicts();
        if key::load(&app.paths.nexus_key).is_some() {
            app.submit(JobKind::ValidateKey);
            if app.config.behavior.check_updates_on_start {
                app.submit(JobKind::CheckUpdates);
            }
        }
        app
    }
}

impl PyluneApp {
    /// Applies appearance preferences when they change; on the settings
    /// page the unsaved draft is previewed live (R-29).
    fn sync_appearance(&mut self, ctx: &egui::Context) {
        let wanted = match self.tab {
            Tab::Settings => self.draft.config.ui,
            _ => self.config.ui,
        };
        if wanted != self.applied_ui {
            theme::apply(ctx, &wanted);
            self.applied_ui = wanted;
        }
        theme::sync(ctx);
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
fn initial_status(info: &SteamInfo, discovery: &Discovery) -> StatusLine {
    let ready = locate::game_status(info) == GameStatus::Ready;
    match (ready.then(|| snapshot::ensure_meta_backup(info)), discovery) {
        (None, _) => StatusLine::new(
            Level::Warning,
            "Choose the Crimson Desert folder to get started",
        ),
        (Some(Ok(_)), Discovery::Detected(dir)) => StatusLine::new(
            Level::Success,
            format!("Found Crimson Desert via Steam: {}", dir.display()),
        ),
        (Some(Ok(0)), _) => StatusLine::new(Level::Info, "Ready"),
        (Some(Ok(n)), _) => StatusLine::new(
            Level::Success,
            format!("Backed up {n} metadata file(s) to .bak"),
        ),
        (Some(Err(err)), _) => StatusLine::new(Level::Error, format!("Backup failed: {err}")),
    }
}

impl eframe::App for PyluneApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.sync_appearance(ui.ctx());
        self.guard_close(ui.ctx());
        if std::mem::take(&mut self.focus_requested) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        let mut actions: Vec<Action> = super::shortcuts::collect(ui.ctx(), self);
        super::shortcuts::drop_hint(ui.ctx());
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
        actions.extend(views::incoming::show(ui.ctx(), &self.nexus.incoming));
        let installed = self
            .nexus
            .preview
            .as_ref()
            .and_then(|p| self.nexus.collections.iter().find(|c| c.slug == p.slug));
        actions.extend(views::collection_preview::show(
            ui.ctx(),
            self.nexus.preview.as_ref(),
            installed,
        ));
        actions.extend(views::guide::show(ui.ctx(), self));
        if self.setup_open {
            actions.extend(views::setup::show(ui.ctx(), self));
        }
        actions.into_iter().for_each(|action| self.dispatch(action));
    }
}
