//! Nexus page actions; network work runs as background jobs.

use crate::error::{PyluneError, Result};
use crate::jobs::JobKind;
use crate::nexus::download::DownloadJob;
use crate::nexus::{handler, key, nxm};
use crate::ui::{Level, NexusAction, PyluneApp, StatusLine};

/// Accepts a bare id or a `nexusmods.com/<game>/mods/<id>` URL.
fn parse_mod_id(input: &str) -> Result<u64> {
    let trimmed = input.trim();
    let tail = trimmed.split("/mods/").nth(1).unwrap_or(trimmed);
    tail.split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|digits| digits.parse().ok())
        .ok_or_else(|| PyluneError::Invalid(format!("'{trimmed}' is not a Nexus mod id or URL")))
}

fn info(text: impl Into<String>) -> Option<StatusLine> {
    Some(StatusLine::new(Level::Info, text))
}

impl PyluneApp {
    pub(super) fn nexus_action(&mut self, action: NexusAction) -> Result<Option<StatusLine>> {
        match action {
            NexusAction::SaveKey => {
                key::save(&self.paths.nexus_key, &self.nexus.key_draft)?;
                self.nexus.key_draft.clear();
                self.nexus.user = None;
                match key::load(&self.paths.nexus_key) {
                    Some(_) => {
                        self.submit(JobKind::ValidateKey);
                        Ok(info("API key stored — validating…"))
                    }
                    None => Ok(info("API key removed")),
                }
            }
            NexusAction::Validate => {
                self.submit(JobKind::ValidateKey);
                Ok(info("Checking API key…"))
            }
            NexusAction::Fetch => {
                let id = parse_mod_id(&self.nexus.mod_id_draft)?;
                self.nexus.replace_target = None;
                self.submit(JobKind::FetchMod(id));
                Ok(info("Fetching mod…"))
            }
            NexusAction::Download(file_id) => self.download(file_id).map(Some),
            NexusAction::CheckUpdates => {
                self.submit(JobKind::CheckUpdates);
                Ok(info("Checking for updates…"))
            }
            NexusAction::OpenPage => {
                let domain = &self.config.nexus.domain;
                let url = match &self.nexus.browsing {
                    Some((mod_info, _)) => {
                        format!(
                            "https://www.nexusmods.com/{domain}/mods/{}?tab=files",
                            mod_info.mod_id
                        )
                    }
                    None => format!("https://www.nexusmods.com/{domain}/mods/"),
                };
                self.open_path(std::path::Path::new(&url))?;
                Ok(info(format!("Opened {url}")))
            }
            NexusAction::RegisterHandler => {
                let exe =
                    std::env::current_exe().map_err(|e| PyluneError::Invalid(e.to_string()))?;
                handler::register(&exe)?;
                self.nexus.handler_registered = handler::is_registered();
                Ok(Some(StatusLine::new(
                    Level::Success,
                    "Pylune now handles nxm:// links",
                )))
            }
        }
    }

    fn download(&mut self, file_id: u64) -> Result<StatusLine> {
        let (mod_info, _) = self
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
            self.registry.mods.iter().any(|m| {
                &m.name == name
                    && m.nexus
                        .as_ref()
                        .is_some_and(|n| n.mod_id == mod_info.mod_id)
            })
        });
        self.submit(JobKind::Download(DownloadJob {
            mod_id: mod_info.mod_id,
            file_id,
            nxm: None,
            replace,
        }));
        Ok(StatusLine::new(Level::Info, "Download started…"))
    }

    /// Queues a download received as an `nxm://` link.
    pub(super) fn handle_nxm(&mut self, url: &str) -> StatusLine {
        let parsed =
            nxm::parse(url).and_then(|link| match link.domain == self.config.nexus.domain {
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
                self.submit(JobKind::Download(job));
                StatusLine::new(Level::Info, "Downloading from nxm:// link…")
            }
            Err(err) => StatusLine::new(Level::Error, err.to_string()),
        }
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
