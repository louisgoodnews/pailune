//! Nexus page actions. Network work is delegated to the worker thread.

use crate::error::{PyluneError, Result};
use crate::nexus::download::DownloadJob;
use crate::nexus::worker::Request;
use crate::nexus::{handler, key, nxm};
use crate::ui::{Level, NexusAction, PyluneApp, StatusLine, Tab};
use std::process::Command;

/// Accepts a bare id or a `nexusmods.com/<game>/mods/<id>` URL.
fn parse_mod_id(input: &str) -> Result<u64> {
    let trimmed = input.trim();
    let tail = trimmed.split("/mods/").nth(1).unwrap_or(trimmed);
    tail.split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|digits| digits.parse().ok())
        .ok_or_else(|| PyluneError::Invalid(format!("'{trimmed}' is not a Nexus mod id or URL")))
}

impl PyluneApp {
    pub(super) fn nexus_action(&mut self, action: NexusAction) -> Result<StatusLine> {
        match action {
            NexusAction::SaveKey => self.save_key(),
            NexusAction::Validate => Ok(self.request(Request::Validate, "Checking API key…")),
            NexusAction::Fetch => {
                let id = parse_mod_id(&self.nexus.mod_id_draft)?;
                self.nexus.replace_target = None;
                Ok(self.request(Request::FetchMod(id), "Fetching mod…"))
            }
            NexusAction::Download(file_id) => self.download(file_id),
            NexusAction::CheckUpdates => {
                let refs: Vec<_> = self.nexus.meta.refs.clone().into_iter().collect();
                match refs.is_empty() {
                    true => Ok(StatusLine::new(
                        Level::Info,
                        "No mods were installed from Nexus yet",
                    )),
                    false => Ok(self.request(Request::CheckUpdates(refs), "Checking for updates…")),
                }
            }
            NexusAction::Update(name) => {
                let origin = self.nexus.meta.refs.get(&name).cloned().ok_or_else(|| {
                    PyluneError::Invalid(format!("'{name}' was not installed from Nexus"))
                })?;
                self.tab = Tab::Nexus;
                self.nexus.mod_id_draft = origin.mod_id.to_string();
                self.nexus.replace_target = Some(name.clone());
                Ok(self.request(
                    Request::FetchMod(origin.mod_id),
                    &format!("Pick the new file for '{name}'"),
                ))
            }
            NexusAction::OpenPage => self.open_page(),
            NexusAction::RegisterHandler => {
                let exe =
                    std::env::current_exe().map_err(|e| PyluneError::Invalid(e.to_string()))?;
                handler::register(&exe)?;
                self.nexus.handler_registered = handler::is_registered();
                Ok(StatusLine::new(
                    Level::Success,
                    "Pylune now handles nxm:// links",
                ))
            }
        }
    }

    /// Sends a worker request and reports it as in progress.
    fn request(&mut self, request: Request, text: &str) -> StatusLine {
        match self.worker.send(request) {
            true => {
                self.nexus.pending += 1;
                StatusLine::new(Level::Info, text)
            }
            false => StatusLine::new(Level::Error, "Nexus worker stopped — restart Pylune"),
        }
    }

    fn save_key(&mut self) -> Result<StatusLine> {
        key::save(&self.paths.nexus_key, &self.nexus.key_draft)?;
        self.nexus.key_draft.clear();
        self.nexus.user = None;
        self.configure_worker();
        match key::load(&self.paths.nexus_key) {
            Some(_) => Ok(self.request(Request::Validate, "API key stored — validating…")),
            None => Ok(StatusLine::new(Level::Info, "API key removed")),
        }
    }

    fn download(&mut self, file_id: u64) -> Result<StatusLine> {
        let (info, _) = self
            .nexus
            .browsing
            .clone()
            .ok_or_else(|| PyluneError::Invalid("no mod selected".into()))?;
        if self.nexus.user.as_ref().is_some_and(|u| !u.is_premium) {
            return Err(PyluneError::Invalid(
                "direct downloads need Nexus Premium — use \"Mod Manager Download\" on the website"
                    .into(),
            ));
        }
        let replace = self.nexus.replace_target.clone().filter(|name| {
            self.nexus
                .meta
                .refs
                .get(name)
                .is_some_and(|r| r.mod_id == info.mod_id)
        });
        let job = DownloadJob {
            mod_id: info.mod_id,
            file_id,
            nxm: None,
            replace,
        };
        Ok(self.request(Request::Download(job), "Download started…"))
    }

    /// Queues a download received as an `nxm://` link.
    pub(super) fn handle_nxm(&mut self, url: &str) -> StatusLine {
        let parsed =
            nxm::parse(url).and_then(|link| match link.domain == self.config.nexus_domain {
                true => Ok(link),
                false => Err(PyluneError::Invalid(format!(
                    "nxm link is for '{}', not this game",
                    link.domain
                ))),
            });
        match parsed {
            Ok(link) => {
                let job = DownloadJob {
                    mod_id: link.mod_id,
                    file_id: link.file_id,
                    nxm: Some(link),
                    replace: None,
                };
                self.request(Request::Download(job), "Downloading from nxm:// link…")
            }
            Err(err) => StatusLine::new(Level::Error, err.to_string()),
        }
    }

    fn open_page(&self) -> Result<StatusLine> {
        let domain = &self.config.nexus_domain;
        let url = match &self.nexus.browsing {
            Some((info, _)) => format!(
                "https://www.nexusmods.com/{domain}/mods/{}?tab=files",
                info.mod_id
            ),
            None => format!("https://www.nexusmods.com/{domain}/mods/"),
        };
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| PyluneError::Invalid(format!("xdg-open failed: {e}")))?;
        Ok(StatusLine::new(Level::Info, format!("Opened {url}")))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_mod_id;

    #[test]
    fn mod_ids_from_urls() {
        assert_eq!(parse_mod_id(" 42 ").unwrap(), 42);
        assert_eq!(
            parse_mod_id("https://www.nexusmods.com/crimsondesert/mods/77?tab=files").unwrap(),
            77
        );
        assert!(parse_mod_id("abc").is_err());
    }
}
