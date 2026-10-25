//! State transitions triggered by [`Action`]s, grouped by feature.

mod deploy;
mod mods;
mod nexus;
mod poll;
mod profiles;
mod settings;

use super::{Action, Confirm, JobState, JobView, Level, PyluneApp, StatusLine};
use crate::backup::snapshot;
use crate::db;
use crate::error::Result;
use crate::jobs::{JobId, JobKind};
use crate::logging::{Event, journal};
use crate::mod_engine::model::Registry;
use crate::steam::locate;

impl StatusLine {
    pub fn new(level: Level, text: impl Into<String>) -> Self {
        Self {
            level,
            text: text.into(),
        }
    }
}

impl PyluneApp {
    /// Applies one action and records its outcome in the status bar.
    pub fn dispatch(&mut self, action: Action) {
        let outcome: Result<Option<StatusLine>> = match action {
            Action::Mod(action) => self.mod_action(action),
            Action::Deploy(action) => self.deploy_action(action),
            Action::Profile(action) => self.profile_action(action).map(Some),
            Action::Nexus(action) => self.nexus_action(action),
            Action::Setup(action) => self.setup_action(action),
            Action::SwitchTab(tab) => {
                self.tab = tab;
                self.refresh_log().map(|()| None)
            }
            Action::BrowseGameDir => Ok(Some(self.browse_game_dir())),
            Action::SaveSettings => self.save_settings().map(Some),
            Action::RefreshLog => self.refresh_log().map(|()| None),
            Action::OpenLogFolder => self.open_path(&self.paths.logs.clone()).map(|()| None),
            Action::Ask(confirm) => Ok(self.ask(Some(confirm))),
            Action::Dismiss => Ok(self.ask(None)),
        };
        match outcome {
            Ok(Some(line)) => self.status = line,
            Ok(None) => {}
            Err(err) => self.status = StatusLine::new(Level::Error, err.to_string()),
        }
        self.refresh_environment();
    }

    /// Re-probes the game directory, loader and backup state.
    pub fn refresh_environment(&mut self) {
        self.info = locate::resolve(&self.config);
        self.game = locate::game_status(&self.info);
        self.loader = locate::detect_loader(&self.info);
        let manifest = db::links::load(&self.conn).unwrap_or_default();
        self.deployed_links = manifest.links.len();
        self.dirty =
            db::mods::pending(&self.conn, &self.registry, self.deployed_links).unwrap_or(true);
        self.backup = snapshot::status(&self.info, &manifest);
    }

    fn ask(&mut self, confirm: Option<Confirm>) -> Option<StatusLine> {
        self.confirm = confirm;
        None
    }

    /// Queues a background job and adds it to the jobs panel.
    pub(super) fn submit(&mut self, kind: JobKind) -> JobId {
        let id = self.pool.submit(kind.clone(), &self.config);
        self.jobs.push(JobView {
            id,
            title: kind.title(),
            kind,
            state: JobState::Queued,
            progress: None,
            finished_at: None,
        });
        id
    }

    /// Starts a conflict analysis; older pending results will be ignored.
    pub(super) fn request_conflicts(&mut self) {
        self.mods.conflict_generation += 1;
        let generation = self.mods.conflict_generation;
        self.submit(JobKind::AnalyzeConflicts(generation));
    }

    /// Atomically changes the registry and marks the deployment stale.
    pub(super) fn update_registry(
        &mut self,
        f: impl FnOnce(&Registry) -> Result<Registry>,
    ) -> Result<()> {
        self.registry = db::mods::update(&mut self.conn, f)?;
        self.request_conflicts();
        Ok(())
    }

    /// Re-reads everything jobs may have changed.
    pub(super) fn reload(&mut self) -> Result<()> {
        self.registry = db::mods::load(&self.conn)?;
        self.profiles.book = db::profiles::load(&self.conn)?;
        let selected_gone = self
            .mods
            .selected
            .as_ref()
            .is_some_and(|name| self.registry.mods.iter().all(|m| &m.name != name));
        if selected_gone {
            self.mods.selected = None;
            self.mods.details = None;
            self.mods.readme = None;
        }
        self.refresh_log()
    }

    /// Loads recent journal events for the Log tab.
    pub(super) fn refresh_log(&mut self) -> Result<()> {
        let filter = self
            .mods
            .selected
            .as_deref()
            .filter(|_| self.log.only_selected);
        self.log.events = db::events::recent(&self.conn, 500, filter)?;
        Ok(())
    }

    /// Writes an event to the journal from the UI thread.
    pub(super) fn journal(&self, event: Event) {
        journal::record(&self.conn, &self.paths.mod_logs, &event);
    }

    /// Opens a file or folder with the desktop's default application.
    pub(super) fn open_path(&self, path: &std::path::Path) -> Result<()> {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(drop)
            .map_err(|e| crate::error::PyluneError::Invalid(format!("xdg-open failed: {e}")))
    }
}
