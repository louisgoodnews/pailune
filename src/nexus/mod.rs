//! Nexus Mods REST (v1) connector.
//!
//! Authentication uses the personal API key from
//! <https://www.nexusmods.com/users/myaccount?tab=api>. Direct downloads need
//! a Premium account; free accounts download through `nxm://` links that the
//! website's "Mod Manager Download" button hands to Pylune.

pub mod api;
pub mod collection;
pub mod download;
pub mod handler;
pub mod key;
pub mod limits;
pub mod nxm;
pub mod updates;
pub mod winreg;

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
    /// Exact size, when Nexus reports it.
    #[serde(default)]
    pub size_in_bytes: Option<u64>,
}

/// Envelope of `/mods/{id}/files.json`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FileList {
    #[serde(default)]
    pub files: Vec<NexusFile>,
    /// Successor chain: `old_file_id` was superseded by `new_file_id`.
    #[serde(default)]
    pub file_updates: Vec<FileUpdate>,
}

/// One link of a file's update chain.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FileUpdate {
    pub old_file_id: u64,
    pub new_file_id: u64,
    #[serde(default)]
    pub new_file_name: String,
}

/// Names and size of one file, for confirming a download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSummary {
    pub mod_id: u64,
    pub file_id: u64,
    pub mod_name: String,
    pub file_name: String,
    pub version: String,
    pub size_kb: u64,
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
