//! Nexus collections: preview, install, update, enable, uninstall.

use crate::db;
use crate::error::{PyluneError, Result};
use crate::jobs::JobKind;
use crate::mod_engine::registry;
use crate::nexus::NxmLink;
use crate::nexus::collection::{self, Collection, Member};
use crate::nexus::download::DownloadJob;
use crate::ui::{Level, NexusAction, PyluneApp, StatusLine};

/// An informational status line.
fn info(text: impl Into<String>) -> Result<Option<StatusLine>> {
    Ok(Some(StatusLine::new(Level::Info, text)))
}

impl PyluneApp {
    /// Handles the collection-related Nexus actions.
    pub(super) fn collection_action(&mut self, action: NexusAction) -> Result<Option<StatusLine>> {
        match action {
            NexusAction::FetchCollection => {
                let slug = collection::parse_slug(&self.nexus.collection_draft)
                    .ok_or_else(|| PyluneError::Invalid("enter a collection slug or URL".into()))?;
                self.submit(JobKind::FetchCollection {
                    slug,
                    revision: None,
                });
                info("Fetching collection…")
            }
            NexusAction::UpdateCollection(slug) => {
                self.submit(JobKind::FetchCollection {
                    slug,
                    revision: None,
                });
                info("Checking for a newer revision…")
            }
            NexusAction::ToggleMember(position) => {
                if let Some(m) = self
                    .nexus
                    .preview
                    .as_mut()
                    .and_then(|c| c.members.get_mut(position))
                {
                    m.selected = !m.optional || !m.selected;
                }
                Ok(None)
            }
            NexusAction::ClosePreview => {
                self.nexus.preview = None;
                Ok(None)
            }
            NexusAction::InstallCollection => self.install_preview().map(Some),
            NexusAction::CollectionEnabled(slug, enabled) => {
                let names = self.member_names(&slug);
                self.update_registry(|r| {
                    Ok(names
                        .iter()
                        .fold(r.clone(), |r, n| registry::with_enabled(&r, n, enabled)))
                })?;
                info(format!(
                    "{} mod(s) {} — press Apply",
                    names.len(),
                    if enabled { "enabled" } else { "disabled" }
                ))
            }
            NexusAction::UninstallCollection(slug) => self.uninstall(&slug).map(Some),
            NexusAction::OpenMemberPage(mod_id, file_id) => {
                let url = format!(
                    "https://www.nexusmods.com/{}/mods/{mod_id}?tab=files&file_id={file_id}",
                    self.config.nexus.domain
                );
                self.open_path(std::path::Path::new(&url))?;
                info(
                    "Click \"Mod Manager Download\" on the page; Pylune picks the file up automatically",
                )
            }
            _ => Ok(None),
        }
    }

    /// The installed collection with this slug.
    fn installed(&self, slug: &str) -> Option<&Collection> {
        self.nexus.collections.iter().find(|c| c.slug == slug)
    }

    /// Staged mod names of a collection, in curator order.
    fn member_names(&self, slug: &str) -> Vec<String> {
        self.installed(slug).map_or_else(Vec::new, |c| {
            c.members
                .iter()
                .filter_map(|m| m.mod_name.clone())
                .collect()
        })
    }

    /// Premium accounts download directly; free accounts via the website.
    pub(super) fn can_download(&self) -> bool {
        self.nexus.user.as_ref().is_some_and(|u| u.is_premium)
    }

    /// Queues the download of one collection member.
    fn download_member(&mut self, slug: &str, member: &Member, replace: Option<String>) {
        let job = DownloadJob {
            mod_id: member.mod_id,
            file_id: member.file_id,
            nxm: None,
            replace,
            collection: Some(slug.into()),
        };
        self.submit(JobKind::Download(job));
    }

    /// Installs the previewed collection, or updates the installed one.
    fn install_preview(&mut self) -> Result<StatusLine> {
        let latest = self
            .nexus
            .preview
            .take()
            .ok_or_else(|| PyluneError::Invalid("no collection selected".into()))?;
        let others: Vec<Collection> = self
            .nexus
            .collections
            .iter()
            .filter(|c| c.slug != latest.slug)
            .cloned()
            .collect();
        let (plan, stored) = match self.installed(&latest.slug).cloned() {
            Some(installed) => {
                let plan = collection::plan_update(&installed, &latest, &others);
                // Keep what is staged for members that did not change.
                let members = latest
                    .members
                    .iter()
                    .map(|m| {
                        let old = installed.members.iter().find(|o| o.mod_id == m.mod_id);
                        let unchanged = old.filter(|o| o.file_id == m.file_id);
                        Member {
                            mod_name: unchanged.and_then(|o| o.mod_name.clone()),
                            selected: m.selected || old.is_some_and(|o| o.selected),
                            ..m.clone()
                        }
                    })
                    .collect();
                (plan, Collection { members, ..latest })
            }
            None => {
                let download = latest
                    .members
                    .iter()
                    .filter(|m| m.selected)
                    .map(|m| (m.clone(), None))
                    .collect();
                (
                    collection::UpdatePlan {
                        download,
                        remove: Vec::new(),
                    },
                    latest,
                )
            }
        };
        db::collections::save(&mut self.conn, &stored)?;
        let premium = self.can_download();
        if premium {
            plan.download
                .iter()
                .for_each(|(m, replace)| self.download_member(&stored.slug, m, replace.clone()));
        }
        plan.remove.iter().for_each(|name| {
            self.submit(JobKind::Remove(name.clone()));
        });
        self.nexus.collections = db::collections::load(&self.conn)?;
        let n = plan.download.len();
        Ok(match premium {
            true => StatusLine::new(
                Level::Info,
                format!("{}: downloading {n} mod(s)…", stored.name),
            ),
            false => StatusLine::new(
                Level::Warning,
                format!(
                    "{}: {n} mod(s) to download — open each on Nexus (Collections section) and click \"Mod Manager Download\"",
                    stored.name
                ),
            ),
        })
    }

    /// Removes the collection and the mods no other collection uses.
    fn uninstall(&mut self, slug: &str) -> Result<StatusLine> {
        self.confirm = None;
        let target = self
            .installed(slug)
            .cloned()
            .ok_or_else(|| PyluneError::Invalid(format!("no collection {slug}")))?;
        let remove = collection::exclusive_mods(&target, &self.nexus.collections);
        remove.iter().for_each(|name| {
            self.submit(JobKind::Remove(name.clone()));
        });
        db::collections::delete(&self.conn, slug)?;
        self.nexus.collections = db::collections::load(&self.conn)?;
        Ok(StatusLine::new(
            Level::Info,
            format!("Uninstalling {} ({} mod(s))…", target.name, remove.len()),
        ))
    }

    /// A fetched collection: shown for review (install or update).
    pub(super) fn on_collection(&mut self, fetched: Collection) -> StatusLine {
        let text = match self.installed(&fetched.slug) {
            Some(c) if c.revision == fetched.revision => {
                format!("{} is up to date (revision {})", c.name, c.revision)
            }
            Some(c) => format!(
                "{}: revision {} available (installed {})",
                c.name, fetched.revision, c.revision
            ),
            None => format!(
                "{}: {} mod(s) — choose optional mods and install",
                fetched.name,
                fetched.members.len()
            ),
        };
        self.nexus.preview = Some(fetched);
        StatusLine::new(Level::Info, text)
    }

    /// After a member download: refresh and apply the curator's order.
    pub(super) fn after_member_installed(&mut self, slug: &str) -> Result<()> {
        self.nexus.collections = db::collections::load(&self.conn)?;
        let names = self.member_names(slug);
        self.update_registry(|r| Ok(registry::arranged(r, &names)))
    }

    /// The collection that waits for this file, if any (free accounts).
    pub(super) fn pending_member(&self, link: &NxmLink) -> Option<String> {
        self.nexus.collections.iter().find_map(|c| {
            c.members
                .iter()
                .any(|m| {
                    m.selected
                        && m.mod_name.is_none()
                        && m.mod_id == link.mod_id
                        && m.file_id == link.file_id
                })
                .then(|| c.slug.clone())
        })
    }
}
