//! Per-frame housekeeping: job events, game process, nxm queue.

use crate::instance;
use crate::jobs::{JobEvent, JobKind, JobOutput, JobUpdate};
use crate::launch::run;
use crate::ui::{JobState, Level, PyluneApp, StatusLine};
use std::time::{Duration, Instant};

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
        self.process = run::game_process();
        let links = instance::drain(&self.paths.nxm_queue);
        links
            .iter()
            .for_each(|url| self.status = self.handle_nxm(url));
    }

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
        match result {
            Ok(output) => self.on_success(event.id, kind, output),
            Err(message) => {
                if self.launch_after == Some(event.id) {
                    self.launch_after = None;
                }
                self.status = StatusLine::new(Level::Error, message);
            }
        }
        if let Err(err) = self.reload() {
            self.status = StatusLine::new(Level::Error, err.to_string());
        }
        self.refresh_environment();
    }

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
            (_, JobOutput::Updates(outdated)) => {
                self.nexus.updates = outdated.into_iter().collect();
                self.status = match self.nexus.updates.len() {
                    0 => StatusLine::new(Level::Success, "All Nexus mods are up to date"),
                    n => StatusLine::new(Level::Warning, format!("{n} mod update(s) available")),
                };
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
            JobKind::Apply | JobKind::CleanSlate => self.dirty = false,
            JobKind::Import {
                replace: Some(name),
                ..
            } => self.mark_updated(name.clone()),
            JobKind::Download(job) => {
                if let Some(name) = job.replace.clone() {
                    self.mark_updated(name);
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

    /// A replaced mod that is enabled needs a redeploy.
    fn mark_updated(&mut self, name: String) {
        self.nexus.updates.remove(&name);
        self.nexus.replace_target = None;
        self.dirty |= self
            .registry
            .mods
            .iter()
            .any(|m| m.name == name && m.enabled);
    }
}
