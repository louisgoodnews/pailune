//! Per-frame housekeeping: worker responses, game process, nxm queue.

use crate::config::store;
use crate::instance;
use crate::launch::run;
use crate::mod_engine::registry;
use crate::nexus::worker::Response;
use crate::ui::{Level, PyluneApp, StatusLine};
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_secs(1);

impl PyluneApp {
    /// Handles everything that happened since the last frame.
    pub fn poll(&mut self) {
        let responses = self.worker.poll();
        responses.into_iter().for_each(|r| self.on_response(r));
        if self.last_poll.elapsed() < POLL_INTERVAL {
            return;
        }
        self.last_poll = Instant::now();
        self.process = run::game_process();
        instance::drain(&self.paths.nxm_queue)
            .iter()
            .for_each(|url| self.status = self.handle_nxm(url));
    }

    fn on_response(&mut self, response: Response) {
        let finished = !matches!(response, Response::Progress { .. });
        if finished {
            self.nexus.pending = self.nexus.pending.saturating_sub(1);
        }
        let line = match response {
            Response::User(user) => {
                let tier = if user.is_premium {
                    "Premium"
                } else {
                    "free account"
                };
                let line = StatusLine::new(
                    Level::Success,
                    format!("Signed in as {} ({tier})", user.name),
                );
                self.nexus.user = Some(user);
                line
            }
            Response::Mod(info, files) => {
                let line = StatusLine::new(
                    Level::Info,
                    format!("'{}' — {} file(s)", info.name, files.len()),
                );
                self.nexus.browsing = Some((info, files));
                line
            }
            Response::Progress { file, done, total } => {
                self.nexus.progress = Some((file, done, total));
                return;
            }
            Response::Installed(installed) => self.on_installed(installed),
            Response::Updates(outdated) => {
                self.nexus.updates = outdated.into_iter().collect();
                match self.nexus.updates.len() {
                    0 => StatusLine::new(Level::Success, "All Nexus mods are up to date"),
                    n => StatusLine::new(Level::Warning, format!("{n} mod update(s) available")),
                }
            }
            Response::Failed(message) => StatusLine::new(Level::Error, message),
        };
        if finished {
            self.nexus.progress = None;
        }
        self.status = line;
    }

    fn on_installed(&mut self, installed: crate::nexus::download::Installed) -> StatusLine {
        let name = installed.name;
        self.nexus
            .meta
            .refs
            .insert(name.clone(), installed.reference);
        self.nexus.updates.remove(&name);
        self.nexus.replace_target = None;
        let saved = store::save(&self.paths.nexus_meta, &self.nexus.meta)
            .and_then(|()| registry::sync(&self.registry, &self.paths.staging))
            .and_then(|next| self.set_registry(next, false));
        let deployed = self
            .registry
            .mods
            .iter()
            .any(|m| m.name == name && m.enabled);
        self.dirty |= deployed;
        match saved {
            Ok(_) if deployed => {
                StatusLine::new(Level::Success, format!("Updated '{name}' — press Apply"))
            }
            Ok(_) => StatusLine::new(
                Level::Success,
                format!("Installed '{name}' — enable it and press Apply"),
            ),
            Err(err) => StatusLine::new(Level::Error, err.to_string()),
        }
    }
}
