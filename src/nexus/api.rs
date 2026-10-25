//! Blocking HTTP client for the Nexus Mods v1 API.

use super::{DownloadLink, FileList, NexusMod, NexusUser, NxmLink};
use crate::error::{PyluneError, Result};
use serde::de::DeserializeOwned;
use std::time::Duration;

const BASE: &str = "https://api.nexusmods.com/v1";
const APP_NAME: &str = "Pylune";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Credentials and connection settings. Holds no mutable state.
#[derive(Debug, Clone)]
pub struct Client {
    agent: ureq::Agent,
    key: String,
    domain: String,
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
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent(format!("{APP_NAME}/{APP_VERSION} (Linux)"))
            .build();
        Self {
            agent: ureq::Agent::new_with_config(config),
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
            .map(|(key, expires)| format!("?key={key}&expires={expires}"))
            .unwrap_or_default();
        self.get_json(&format!(
            "/games/{}/mods/{mod_id}/files/{file_id}/download_link.json{query}",
            self.domain
        ))
    }

    /// Opens a streaming download, returning `(content length, reader)`.
    pub fn open_download(&self, uri: &str) -> Result<(Option<u64>, impl std::io::Read + use<>)> {
        let response = self.agent.get(uri).call().map_err(explain)?;
        let length = response.body().content_length();
        Ok((length, response.into_body().into_reader()))
    }
}
