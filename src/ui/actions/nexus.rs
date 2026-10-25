//! Nexus page actions; network work runs as background jobs.

use crate::db;
use crate::error::{PyluneError, Result};
use crate::jobs::JobKind;
use crate::nexus::download::DownloadJob;
use crate::nexus::{handler, key, nxm};
use crate::ui::{Incoming, Level, NexusAction, PyluneApp, StatusLine};

/// Key/value entry remembering the `nxm://` handler Pylune replaced.
const PREVIOUS_HANDLER: &str = "previous_nxm_handler";

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
            NexusAction::FetchCollection
            | NexusAction::ToggleMember(_)
            | NexusAction::InstallCollection
            | NexusAction::ClosePreview
            | NexusAction::UpdateCollection(_)
            | NexusAction::CollectionEnabled(..)
            | NexusAction::UninstallCollection(_)
            | NexusAction::OpenMemberPage(..) => self.collection_action(action),
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
            NexusAction::AcceptIncoming(index) => self.resolve_incoming(index, true),
            NexusAction::RejectIncoming(index) => self.resolve_incoming(index, false),
            NexusAction::RegisterHandler => {
                let exe =
                    std::env::current_exe().map_err(|e| PyluneError::Invalid(e.to_string()))?;
                if exe.to_string_lossy().contains("/target/") {
                    tracing::warn!(exe = %exe.display(), "registering a development build as nxm handler");
                }
                let previous = handler::register(&exe)?;
                if let Some(previous) = &previous {
                    db::set_value(&self.conn, PREVIOUS_HANDLER, Some(previous))?;
                }
                self.nexus.handler_registered = handler::is_registered();
                let text = match previous {
                    Some(p) => {
                        format!("Pylune now handles nxm:// links; links for other games go to {p}")
                    }
                    None => "Pylune now handles nxm:// links".into(),
                };
                Ok(Some(StatusLine::new(Level::Success, text)))
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
            collection: None,
        }));
        Ok(StatusLine::new(Level::Info, "Download started…"))
    }

    /// Holds an `nxm://` link for confirmation and looks up its names.
    pub(super) fn handle_nxm(&mut self, url: &str) -> StatusLine {
        let domain = &self.config.nexus.domain;
        let collection = nxm::parse_collection(url);
        let foreign = nxm::parse(url).is_ok_and(|link| &link.domain != domain)
            || collection.as_ref().is_some_and(|c| &c.domain != domain);
        if foreign {
            return self.forward_nxm(url);
        }
        // A collection link only fetches the list; installing needs a click.
        if let Some(link) = collection {
            self.submit(JobKind::FetchCollection {
                slug: link.slug,
                revision: link.revision,
            });
            self.focus_requested = true;
            return StatusLine::new(
                Level::Info,
                "Collection link from the browser — review it in the dialog",
            );
        }
        let checked = nxm::parse(url)
            .and_then(|link| nxm::validate(&link, &self.config.nexus.domain).map(|()| link));
        match checked {
            // The user already chose this file by installing its collection.
            Ok(link) if self.pending_member(&link).is_some() => {
                let slug = self.pending_member(&link);
                let job = DownloadJob {
                    mod_id: link.mod_id,
                    file_id: link.file_id,
                    nxm: Some(link),
                    replace: None,
                    collection: slug,
                };
                self.submit(JobKind::Download(job));
                StatusLine::new(Level::Info, "Downloading a collection member…")
            }
            Ok(link) => {
                self.submit(JobKind::DescribeFile {
                    mod_id: link.mod_id,
                    file_id: link.file_id,
                });
                self.nexus.incoming.push(Incoming {
                    link,
                    summary: None,
                });
                self.focus_requested = true;
                StatusLine::new(
                    Level::Info,
                    "Download request from the browser — confirm it in the dialog",
                )
            }
            Err(err) => StatusLine::new(Level::Error, err.to_string()),
        }
    }

    /// Hands a link for another game to the handler Pylune replaced.
    fn forward_nxm(&self, url: &str) -> StatusLine {
        let previous = db::get_value(&self.conn, PREVIOUS_HANDLER).ok().flatten();
        match previous.map(|p| handler::forward(&p, url).map(|()| p)) {
            Some(Ok(p)) => StatusLine::new(
                Level::Info,
                format!("Passed a link for another game to {p}"),
            ),
            Some(Err(err)) => StatusLine::new(Level::Error, err.to_string()),
            None => StatusLine::new(
                Level::Warning,
                "Ignored an nxm link for another game (no other handler known)",
            ),
        }
    }

    /// Starts or drops a confirmed incoming download.
    fn resolve_incoming(&mut self, index: usize, accept: bool) -> Result<Option<StatusLine>> {
        if index >= self.nexus.incoming.len() {
            return Ok(None);
        }
        let incoming = self.nexus.incoming.remove(index);
        if !accept {
            return Ok(Some(StatusLine::new(
                Level::Info,
                "Download request ignored",
            )));
        }
        let link = incoming.link;
        // "Update…" on a free account: the next file of that mod replaces it.
        let replace = self.nexus.replace_target.clone().filter(|name| {
            self.registry.mods.iter().any(|m| {
                &m.name == name && m.nexus.as_ref().is_some_and(|n| n.mod_id == link.mod_id)
            })
        });
        let job = DownloadJob {
            mod_id: link.mod_id,
            file_id: link.file_id,
            nxm: Some(link),
            replace,
            collection: None,
        };
        self.submit(JobKind::Download(job));
        Ok(Some(StatusLine::new(Level::Info, "Download started…")))
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
