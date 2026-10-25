//! Blocking HTTP client for the Nexus Mods v1 API.

use super::{DownloadLink, FileList, NexusMod, NexusUser, NxmLink};
use crate::error::{PyluneError, Result};
use serde::de::DeserializeOwned;
use std::time::Duration;

const BASE: &str = "https://api.nexusmods.com/v1";
const GRAPHQL: &str = "https://api.nexusmods.com/v2/graphql";
const COLLECTION_QUERY: &str = "query Rev($slug: String!, $domain: String, $revision: Int) { \
    collectionRevision(slug: $slug, domainName: $domain, revision: $revision) { revisionNumber \
    collection { name slug } modFiles { fileId optional file { modId name version sizeInBytes \
    mod { name } } } } }";
const APP_NAME: &str = "Pylune";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Credentials and connection settings. Holds no mutable state.
#[derive(Clone)]
pub struct Client {
    /// API calls: small responses, bounded overall.
    agent: ureq::Agent,
    /// Archive downloads: no overall limit (large files take minutes), but a
    /// stalled connection still fails.
    downloads: ureq::Agent,
    key: String,
    domain: String,
}

/// Never prints the API key.
impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("domain", &self.domain)
            .field("key", &"<redacted>")
            .finish()
    }
}

/// Percent-encodes a query value (RFC 3986 unreserved characters pass).
fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b => format!("%{b:02X}"),
        })
        .collect()
}

/// Builds an agent with the Pylune user agent and the given limits.
fn agent(global: Option<Duration>, per_read: Option<Duration>) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(global)
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_recv_response(Some(Duration::from_secs(60)))
        .timeout_per_read(per_read)
        .user_agent(format!(
            "{APP_NAME}/{APP_VERSION} (Linux; {})",
            std::env::consts::ARCH
        ))
        .build();
    ureq::Agent::new_with_config(config)
}

/// Translates well-known status codes into actionable messages.
fn explain(err: ureq::Error) -> PyluneError {
    match err {
        ureq::Error::StatusCode(401) => PyluneError::Invalid("Nexus rejected the API key".into()),
        ureq::Error::StatusCode(403) => PyluneError::Invalid(
            "Nexus refused the download — free accounts must use the website's \
             \"Mod Manager Download\" button (nxm:// link)"
                .into(),
        ),
        ureq::Error::StatusCode(404) => PyluneError::Invalid("not found on Nexus Mods".into()),
        ureq::Error::StatusCode(429) => {
            PyluneError::Invalid("Nexus rate limit reached, try later".into())
        }
        other => PyluneError::Http(other),
    }
}

impl Client {
    /// Creates a client for `domain` (e.g. `crimsondesert`).
    pub fn new(key: &str, domain: &str) -> Self {
        Self {
            agent: agent(Some(Duration::from_secs(30)), None),
            downloads: agent(None, Some(Duration::from_secs(60))),
            key: key.trim().to_string(),
            domain: domain.trim().to_lowercase(),
        }
    }

    /// Performs an authenticated GET and decodes the JSON body.
    fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.agent
            .get(format!("{BASE}{path}"))
            .header("apikey", &self.key)
            .header("Application-Name", APP_NAME)
            .header("Application-Version", APP_VERSION)
            .call()
            .map_err(explain)?
            .body_mut()
            .read_json()
            .map_err(explain)
    }

    /// Checks the API key and returns the account.
    pub fn validate(&self) -> Result<NexusUser> {
        self.get_json("/users/validate.json")
    }

    /// Fetches a mod page.
    pub fn mod_info(&self, mod_id: u64) -> Result<NexusMod> {
        self.get_json(&format!("/games/{}/mods/{mod_id}.json", self.domain))
    }

    /// Lists a mod's files.
    pub fn files(&self, mod_id: u64) -> Result<FileList> {
        self.get_json(&format!("/games/{}/mods/{mod_id}/files.json", self.domain))
    }

    /// Requests CDN links; `nxm` supplies the key/expiry needed by free accounts.
    pub fn download_links(
        &self,
        mod_id: u64,
        file_id: u64,
        nxm: Option<&NxmLink>,
    ) -> Result<Vec<DownloadLink>> {
        let query = nxm
            .and_then(|link| link.key.as_ref().zip(link.expires))
            .map(|(key, expires)| format!("?key={}&expires={expires}", encode(key)))
            .unwrap_or_default();
        self.get_json(&format!(
            "/games/{}/mods/{mod_id}/files/{file_id}/download_link.json{query}",
            self.domain
        ))
    }

    /// Opens a streaming download, returning `(content length, reader)`.
    pub fn open_download(&self, uri: &str) -> Result<(Option<u64>, impl std::io::Read + use<>)> {
        let response = self.downloads.get(uri).call().map_err(explain)?;
        let length = response.body().content_length();
        Ok((length, response.into_body().into_reader()))
    }

    /// Fetches a collection revision (latest when `revision` is `None`)
    /// through the v2 GraphQL API.
    pub fn collection(
        &self,
        slug: &str,
        revision: Option<u32>,
    ) -> Result<super::collection::Collection> {
        let body = serde_json::json!({
            "query": COLLECTION_QUERY,
            "variables": { "slug": slug, "domain": self.domain, "revision": revision },
        });
        let response: serde_json::Value = self
            .agent
            .post(GRAPHQL)
            .header("apikey", &self.key)
            .header("Application-Name", APP_NAME)
            .header("Application-Version", APP_VERSION)
            .send_json(&body)
            .map_err(explain)?
            .body_mut()
            .read_json()
            .map_err(explain)?;
        super::collection::from_graphql(slug, &response).ok_or_else(|| {
            let detail = response
                .get("errors")
                .map(|e| e.to_string())
                .unwrap_or_else(|| "no such collection".into());
            PyluneError::Invalid(format!("collection {slug}: {detail}"))
        })
    }
}
