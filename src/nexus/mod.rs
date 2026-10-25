//! Nexus Mods REST (v1) connector.
//!
//! Authentication uses the personal API key from
//! <https://www.nexusmods.com/users/myaccount?tab=api>. Direct downloads need
//! a Premium account; free accounts download through `nxm://` links that the
//! website's "Mod Manager Download" button hands to Pylune.

pub mod api;
pub mod download;
pub mod handler;
pub mod key;
pub mod nxm;

use serde::Deserialize;

/// The account behind the API key (`/users/validate.json`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NexusUser {
    pub user_id: u64,
    pub name: String,
    #[serde(default)]
    pub is_premium: bool,
}

/// Mod page information (`/games/{domain}/mods/{id}.json`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NexusMod {
    pub mod_id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
}

/// One downloadable file of a mod.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NexusFile {
    pub file_id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub category_name: Option<String>,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub size_kb: u64,
}

/// Envelope of `/mods/{id}/files.json`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FileList {
    #[serde(default)]
    pub files: Vec<NexusFile>,
}

/// One CDN mirror returned by `download_link.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DownloadLink {
    #[serde(default)]
    pub short_name: String,
    #[serde(rename = "URI")]
    pub uri: String,
}

/// A parsed `nxm://<domain>/mods/<mod>/files/<file>?key=…&expires=…` link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NxmLink {
    pub domain: String,
    pub mod_id: u64,
    pub file_id: u64,
    pub key: Option<String>,
    pub expires: Option<u64>,
}
