//! Per-frame housekeeping: job events, game process, nxm queue.

use crate::instance;
use crate::jobs::{JobEvent, JobKind, JobOutput, JobUpdate};
use crate::launch::run;
use crate::ui::{JobState, Level, PyluneApp, StatusLine};
use std::time::{Duration, Instant};

/// How often the game process and the request spool are checked.
const POLL_INTERVAL: Duration = Duration::from_secs(1);
/// How long finished jobs stay visible in the jobs panel.
const FINISHED_LINGER: Duration = Duration::from_secs(8);

impl PyluneApp {
    /// Handles everything that happened since the last frame.
    pub fn poll(&mut self) {
        let events = self.pool.poll();
        events.into_iter().for_each(|e| self.on_job_event(e));
        self.jobs
            .retain(|j| j.finished_at.is_none_or(|t| t.elapsed() < FINISHED_LINGER));
        if self.last_poll.elapsed() < POLL_INTERVAL {
            return;
        }
        self.last_poll = Instant::now();
        self.close_refused &= self.game_job_active();
        self.process = run::game_process();
        let requests = instance::drain(&self.paths.spool);
        requests.iter().for_each(|request| match request.as_str() {
            instance::FOCUS_REQUEST => self.focus_requested = true,
            url => self.status = self.handle_nxm(url),
        });
    }

    /// Updates the jobs panel and reacts to a finished job.
    fn on_job_event(&mut self, event: JobEvent) {
        let Some(view) = self.jobs.iter_mut().find(|j| j.id == event.id) else {
            return;
        };
        let result = match event.update {
            JobUpdate::Started => {
                view.state = JobState::Running;
                return;
            }
            JobUpdate::Progress { done, total } => {
                view.progress = Some((done, total));
                return;
            }
            JobUpdate::Finished(result) => result,
        };
        view.state = if result.is_ok() {
            JobState::Succeeded
        } else {
            JobState::Failed
        };
        view.finished_at = Some(Instant::now());
        let kind = view.kind.clone();
        if matches!(kind, JobKind::AnalyzeConflicts(_))
            && std::mem::take(&mut self.mods.conflicts_rerun)
        {
            self.request_conflicts();
        }
        match result {
            Ok(output) => self.on_success(event.id, kind, output),
            Err(message) => {
                if self.launch_after == Some(event.id) {
                    self.launch_after = None;
                }
                if let JobKind::DescribeFile { mod_id, file_id } = kind {
                    self.lookup_failed(mod_id, file_id, &message);
                }
                self.status = StatusLine::new(Level::Error, message);
            }
        }
        if let Err(err) = self.reload() {
            self.status = StatusLine::new(Level::Error, err.to_string());
        }
        self.refresh_environment();
    }

    /// Applies a successful job's output to the UI state.
    fn on_success(&mut self, id: u64, kind: JobKind, output: JobOutput) {
        let changes_mods = matches!(
            kind,
            JobKind::Import { .. }
                | JobKind::Download(_)
                | JobKind::Remove(_)
                | JobKind::CleanSlate
        );
        if changes_mods {
            self.request_conflicts();
        }
        match (&kind, output) {
            (_, JobOutput::Collection(fetched)) => self.status = self.on_collection(fetched),
            (_, JobOutput::Conflicts(generation, report)) => {
                if generation == self.mods.conflict_generation {
                    self.mods.conflicts = report;
                }
            }
            (_, JobOutput::User(user)) => {
                let tier = if user.is_premium {
                    "Premium"
                } else {
                    "free account"
                };
                self.status = StatusLine::new(
                    Level::Success,
                    format!("Signed in as {} ({tier})", user.name),
                );
                self.nexus.user = Some(user);
            }
            (_, JobOutput::Mod(mod_info, files)) => {
                self.status = StatusLine::new(
                    Level::Info,
                    format!("'{}' — {} file(s)", mod_info.name, files.len()),
                );
                self.nexus.browsing = Some((mod_info, files));
            }
            (_, JobOutput::Updates { outdated, failed }) => {
                self.nexus.updates = outdated.into_iter().collect();
                let failures = match failed {
                    0 => String::new(),
                    n => format!(" ({n} check(s) failed — see the Log tab)"),
                };
                self.status = match self.nexus.updates.len() {
                    0 => StatusLine::new(
                        Level::Success,
                        format!("All Nexus mods are up to date{failures}"),
                    ),
                    n => StatusLine::new(
                        Level::Warning,
                        format!("{n} mod update(s) available{failures}"),
                    ),
                };
            }
            (_, JobOutput::Described(summary)) => {
                self.nexus
                    .incoming
                    .iter_mut()
                    .filter(|i| {
                        i.link.mod_id == summary.mod_id && i.link.file_id == summary.file_id
                    })
                    .for_each(|i| i.summary = Some(summary.clone()));
            }
            (kind, JobOutput::Done(message)) => {
                self.status = StatusLine::new(Level::Success, message);
                self.after_done(id, kind);
            }
        }
    }

    /// Follow-ups of finished file jobs.
    fn after_done(&mut self, id: u64, kind: &JobKind) {
        match kind {
            JobKind::Remove(name) => {
                let trash = crate::mod_engine::trash::list(&self.paths.trash);
                self.mods.undo = trash.into_iter().find(|i| &i.name == name).map(|i| i.id);
            }
            JobKind::Import {
                replace: Some(name),
                ..
            } => self.mark_updated(name.clone()),
            JobKind::Download(job) => {
                if let Some(name) = job.replace.clone() {
                    self.mark_updated(name);
                }
                if let Some(slug) = job.collection.clone()
                    && let Err(err) = self.after_member_installed(&slug)
                {
                    self.status = StatusLine::new(Level::Error, err.to_string());
                }
            }
            _ => {}
        }
        if self.launch_after == Some(id) {
            self.launch_after = None;
            self.status = self
                .launch_now()
                .unwrap_or_else(|e| StatusLine::new(Level::Error, e.to_string()));
        }
    }

    /// Clears update hints once a mod was replaced. Whether a redeploy is
    /// needed follows from the stored registry revision.
    fn mark_updated(&mut self, name: String) {
        self.nexus.updates.remove(&name);
        self.nexus.replace_target = None;
    }
}

impl PyluneApp {
    /// Whether a job that changes the game directory is queued or running.
    pub fn game_job_active(&self) -> bool {
        self.jobs.iter().any(|j| {
            matches!(j.state, JobState::Queued | JobState::Running)
                && j.kind.access() == crate::jobs::Access::Exclusive
        })
    }

    /// Refuses the first close request while a game-folder job runs, so the
    /// job is not killed halfway; a second request closes anyway.
    pub fn guard_close(&mut self, ctx: &eframe::egui::Context) {
        let requested = ctx.input(|i| i.viewport().close_requested());
        if !requested || !self.game_job_active() || self.close_refused {
            return;
        }
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::CancelClose);
        self.close_refused = true;
        self.status = StatusLine::new(
            Level::Warning,
            "A job is changing the game folder — wait for it, or close again to quit anyway \
             (the next Unlink/Apply repairs an interrupted run)",
        );
    }

    /// Shows the ids (and why names are missing) instead of a lookup spinner.
    fn lookup_failed(&mut self, mod_id: u64, file_id: u64, message: &str) {
        let summary = crate::nexus::FileSummary {
            mod_id,
            file_id,
            mod_name: format!("Nexus mod {mod_id}"),
            file_name: format!("file {file_id} (names unavailable: {message})"),
            version: String::new(),
            size_kb: 0,
        };
        self.nexus
            .incoming
            .iter_mut()
            .filter(|i| i.link.mod_id == mod_id && i.link.file_id == file_id && i.summary.is_none())
            .for_each(|i| i.summary = Some(summary.clone()));
    }
}
