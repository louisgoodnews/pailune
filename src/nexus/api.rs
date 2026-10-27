//! Blocking HTTP client for the Nexus Mods v1 API.

use super::{DownloadLink, FileList, NexusMod, NexusUser, NxmLink};
use crate::error::{PailuneError, Result};
use serde::de::DeserializeOwned;
use std::time::Duration;

/// The public Nexus Mods API.
pub const DEFAULT_API: &str = "https://api.nexusmods.com";
/// GraphQL query for a collection revision and its mod files.
const COLLECTION_QUERY: &str = "query Rev($slug: String!, $domain: String, $revision: Int) { \
    collectionRevision(slug: $slug, domainName: $domain, revision: $revision) { revisionNumber \
    collection { name slug } modFiles { fileId optional file { modId name version sizeInBytes \
    mod { name } } } } }";
/// Sent as `Application-Name`, as the Nexus API guidelines ask.
const APP_NAME: &str = "Pailune";
/// Sent as `Application-Version`.
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
    /// API root, e.g. `https://api.nexusmods.com` (overridable for tests).
    api: String,
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

/// Builds an agent with the Pailune user agent and the given limits.
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
fn explain(err: ureq::Error) -> PailuneError {
    match err {
        ureq::Error::StatusCode(401) => PailuneError::Invalid("Nexus rejected the API key".into()),
        ureq::Error::StatusCode(403) => PailuneError::Invalid(
            "Nexus refused the download — free accounts must use the website's \
             \"Mod Manager Download\" button (nxm:// link)"
                .into(),
        ),
        ureq::Error::StatusCode(404) => PailuneError::Invalid("not found on Nexus Mods".into()),
        ureq::Error::StatusCode(429) => {
            PailuneError::Invalid("Nexus rate limit reached, try later".into())
        }
        other => PailuneError::Http(other),
    }
}

impl Client {
    /// Creates a client for `domain` (e.g. `crimsondesert`) on the public API.
    #[cfg(test)]
    pub fn new(key: &str, domain: &str) -> Self {
        Self::with_api(key, domain, DEFAULT_API)
    }

    /// A client for a different API root (`[nexus] api_url`, tests).
    pub fn with_api(key: &str, domain: &str, api: &str) -> Self {
        Self {
            agent: agent(Some(Duration::from_secs(30)), None),
            downloads: agent(None, Some(Duration::from_secs(60))),
            key: key.trim().to_string(),
            domain: domain.trim().to_lowercase(),
            api: api.trim_end_matches('/').to_string(),
        }
    }

    /// Performs an authenticated GET and decodes the JSON body.
    fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        super::limits::allow().map_err(PailuneError::Invalid)?;
        let mut response = self
            .agent
            .get(format!("{}/v1{path}", self.api))
            .header("apikey", &self.key)
            .header("Application-Name", APP_NAME)
            .header("Application-Version", APP_VERSION)
            .call()
            .map_err(explain)?;
        super::limits::record(response.headers());
        response.body_mut().read_json().map_err(explain)
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

    /// Opens a streaming download from byte `offset` (resume when > 0).
    /// Returns the remaining length, whether the server honoured the range
    /// (HTTP 206), and the body reader.
    pub fn open_download(
        &self,
        uri: &str,
        offset: u64,
    ) -> Result<(Option<u64>, bool, impl std::io::Read + use<>)> {
        let request = self.downloads.get(uri);
        let request = match offset {
            0 => request,
            from => request.header("Range", format!("bytes={from}-")),
        };
        let response = request.call().map_err(explain)?;
        let partial = response.status() == 206;
        let length = response.body().content_length();
        Ok((length, partial, response.into_body().into_reader()))
    }

    /// Files whose content has this MD5, as `(mod id, file id)` pairs; an
    /// unknown hash gives an empty list.
    pub fn md5_search(&self, md5: &str) -> Result<Vec<(u64, u64)>> {
        let path = format!("/games/{}/mods/md5_search/{md5}.json", self.domain);
        let hits: Vec<serde_json::Value> = match self.get_json(&path) {
            Ok(hits) => hits,
            Err(PailuneError::Invalid(msg)) if msg.contains("not found") => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };
        Ok(hits
            .iter()
            .filter_map(|h| {
                let mod_id = h.get("mod")?.get("mod_id")?.as_u64()?;
                Some((mod_id, h.get("file_details")?.get("file_id")?.as_u64()?))
            })
            .collect())
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
        super::limits::allow().map_err(PailuneError::Invalid)?;
        let mut reply = self
            .agent
            .post(format!("{}/v2/graphql", self.api))
            .header("apikey", &self.key)
            .header("Application-Name", APP_NAME)
            .header("Application-Version", APP_VERSION)
            .send_json(&body)
            .map_err(explain)?;
        super::limits::record(reply.headers());
        let response: serde_json::Value = reply.body_mut().read_json().map_err(explain)?;
        super::collection::from_graphql(slug, &response).ok_or_else(|| {
            let detail = response
                .get("errors")
                .map(|e| e.to_string())
                .unwrap_or_else(|| "no such collection".into());
            PailuneError::Invalid(format!("collection {slug}: {detail}"))
        })
    }
}
