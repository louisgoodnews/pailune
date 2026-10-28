//! State transitions triggered by [`Action`]s, grouped by feature.

mod backups;
mod collections;
mod data_browser;
mod deploy;
mod guide;
mod modlist;
mod mods;
mod nexus;
mod poll;
mod profiles;
mod save_edit;

pub use save_edit::changes as save_changes;
mod settings;

use super::{Action, Confirm, JobState, JobView, Level, PailuneApp, StatusLine};
use crate::backup::snapshot;
use crate::db;
use crate::error::Result;
use crate::jobs::{JobId, JobKind};
use crate::logging::{Event, journal};
use crate::mod_engine::model::Registry;
use crate::steam::locate;

impl StatusLine {
    /// A status line with the given severity.
    pub fn new(level: Level, text: impl Into<String>) -> Self {
        Self {
            level,
            text: text.into(),
        }
    }
}

impl PailuneApp {
    /// Applies one action and records its outcome in the status bar.
    pub fn dispatch(&mut self, action: Action) {
        let outcome: Result<Option<StatusLine>> = match action {
            Action::Mod(action) => self.mod_action(action),
            Action::Deploy(action) => self.deploy_action(action),
            Action::Profile(action) => self.profile_action(action).map(Some),
            Action::Nexus(action) => self.nexus_action(action),
            Action::Setup(action) => self.setup_action(action),
            Action::Backup(action) => self.backup_action(action),
            Action::Guide(action) => self.guide_action(action),
            Action::Notice(action) => self.notice_action(action),
            Action::ModList(action) => self.modlist_action(action),
            Action::DataBrowser(action) => self.data_browser_action(action),
            Action::SwitchTab(tab) => {
                self.tab = tab;
                // The game writes saves while Pailune runs: read them fresh.
                let fresh = match tab {
                    super::Tab::Saves => self.reload(),
                    super::Tab::DataBrowser if !self.data_browser.loaded => {
                        self.load_data_browser_tables();
                        Ok(())
                    }
                    _ => Ok(()),
                };
                fresh.and_then(|()| self.refresh_log()).map(|()| None)
            }
            Action::BrowseGameDir => Ok(Some(self.browse_game_dir())),
            Action::SaveSettings => self.save_settings().map(Some),
            Action::RefreshLog => self.refresh_log().map(|()| None),
            Action::OpenLogFolder => self.open_path(&self.paths.logs.clone()).map(|()| None),
            Action::CopyLog => {
                let count = self.log.events.len();
                self.window.clipboard = Some(crate::logging::as_text(&self.log.events));
                Ok(Some(StatusLine::new(
                    Level::Success,
                    format!("Copied {count} log entries to the clipboard"),
                )))
            }
            Action::Copy(text) => {
                self.window.clipboard = Some(text);
                Ok(Some(StatusLine::new(
                    Level::Success,
                    "Copied to the clipboard",
                )))
            }
            Action::Quit => {
                self.window.quit = true;
                Ok(None)
            }
            // Deletes are undoable, so they ask only when the user wants it (R-04).
            Action::Ask(Confirm::DeleteMod(name)) if !self.config.behavior.confirm_deletes => {
                self.mod_action(super::ModAction::Delete(name))
            }
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
        self.loader_off = locate::loader_disabled(self.loader, self.config.game.app_id);
        let manifest = db::links::load(&self.conn).unwrap_or_default();
        self.deployed_links = manifest.links.len();
        self.dirty =
            db::mods::pending(&self.conn, &self.registry, self.deployed_links).unwrap_or(true);
        self.backup = snapshot::status(&self.info, &manifest);
        self.archives = crate::paz::status::inspect(&self.info.game_dir);
        self.plugins = crate::mod_engine::plugins::scan(
            &self.info.bin64_dir,
            &manifest.links,
            &self.paths.staging,
        );
    }

    /// Opens (`Some`) or closes (`None`) a confirmation dialog.
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
    ///
    /// At most one analysis is in flight: further requests only mark it for a
    /// re-run, so rapid toggling does not flood the pool (A-19).
    pub(super) fn request_conflicts(&mut self) {
        if self.analysis_active() {
            self.mods.conflicts_rerun = true;
            return;
        }
        self.mods.conflict_generation += 1;
        let generation = self.mods.conflict_generation;
        self.submit(JobKind::AnalyzeConflicts(generation));
    }

    /// Whether a conflict analysis is queued or running.
    pub fn analysis_active(&self) -> bool {
        self.jobs.iter().any(|j| {
            matches!(j.kind, JobKind::AnalyzeConflicts(_))
                && matches!(j.state, JobState::Queued | JobState::Running)
        })
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
        self.mods.trash = crate::mod_engine::trash::list(&self.paths.trash);
        self.backups = crate::app_backup::create::list(&self.paths.backups);
        self.save_backups = crate::save_backup::list(&self.paths.save_backups);
        self.save_slots = crate::save_backup::save_dir(self.info.prefix_dir.as_deref())
            .map(|dir| {
                // Finish an interrupted save restore before reading the slots.
                if let Err(err) = crate::save_backup::recover(&dir) {
                    tracing::warn!(%err, "could not recover an interrupted save restore");
                }
                crate::savefile::slots::slots(&dir)
            })
            .unwrap_or_default();
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
        // Keep the details panel in step with updates and re-imports (A-22).
        if let Some(name) = self.mods.selected.clone() {
            let details = crate::mod_engine::info::details(
                &self.paths.staging.join(&name),
                self.variant_of(&name).as_deref(),
            )?;
            self.mods.details = Some((name, details));
        }
        self.refresh_log()
    }

    /// The chosen variant of a staged mod.
    pub(super) fn variant_of(&self, name: &str) -> Option<String> {
        self.registry
            .mods
            .iter()
            .find(|m| m.name == name)
            .and_then(|m| m.variant.clone())
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
        let command = crate::platform::open_command(path.as_os_str());
        crate::launch::run::spawn(command).map_err(|e| {
            crate::error::PailuneError::Invalid(format!("could not open {}: {e}", path.display()))
        })
    }
}
