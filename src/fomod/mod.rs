//! FOMOD installers (board item A-28): `fomod/ModuleConfig.xml`, the wizard
//! format most Nexus Mods installers use. A FOMOD mod stays in staging as
//! extracted; the user's choices are stored like a variant
//! ([`Selection::encode`]) and [`resolve`] maps the chosen files onto their
//! destinations whenever the mod is planned, analysed or deployed.
//!
//! Supported: required files, install steps with visibility conditions,
//! option groups of every type, plugin types (incl. dependency types),
//! condition flags, file/folder installs with priorities, conditional file
//! installs. Game/script-extender version conditions count as met.

mod parse;
pub mod resolve;

pub use parse::parse;
pub use resolve::resolve;

use crate::error::{IoContext, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Prefix marking a FOMOD selection in the mod's variant field.
pub const PREFIX: &str = "fomod:";

/// The whole installer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub name: String,
    /// Always installed.
    pub required: Vec<FileInstall>,
    pub steps: Vec<Step>,
    pub conditional: Vec<ConditionalInstall>,
}

/// One wizard page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Step {
    pub name: String,
    /// Shown only when this holds (flags set by earlier steps).
    pub visible: Option<Condition>,
    pub groups: Vec<Group>,
}

/// How many plugins of a group may be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    ExactlyOne,
    AtMostOne,
    AtLeastOne,
    All,
    Any,
}

/// A set of options on a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub kind: GroupKind,
    pub plugins: Vec<Plugin>,
}

/// How a plugin is offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginKind {
    Required,
    Recommended,
    Optional,
    CouldBeUsable,
    NotUsable,
}

/// One option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plugin {
    pub name: String,
    pub description: String,
    /// Image path inside the mod, if any.
    pub image: Option<String>,
    pub files: Vec<FileInstall>,
    /// Flags set when chosen.
    pub flags: Vec<(String, String)>,
    /// Default type, and types that apply when their condition holds.
    pub kind: PluginKind,
    pub kind_rules: Vec<(Condition, PluginKind)>,
}

/// A file or folder to install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInstall {
    /// Relative to the mod root (as written in the XML, `\` or `/`).
    pub source: String,
    /// Relative to the game layout; empty means "same as source" for files,
    /// the root for folders.
    pub destination: String,
    pub folder: bool,
    /// Higher wins when two installs write the same file.
    pub priority: i32,
}

/// Files installed when a condition holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalInstall {
    pub condition: Condition,
    pub files: Vec<FileInstall>,
}

/// A dependency expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    All(Vec<Condition>),
    Any(Vec<Condition>),
    /// Flag has this value (empty value: flag unset or empty).
    Flag(String, String),
    /// A file in the game is `Active`/`Inactive`/`Missing`.
    File(String, String),
    /// Game, script extender or manager versions: treated as met.
    Always,
}

/// The user's choices: `(step, group, plugin)` names.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub picks: Vec<(String, String, String)>,
}

impl Selection {
    /// Whether this plugin is chosen.
    pub fn has(&self, step: &str, group: &str, plugin: &str) -> bool {
        self.picks
            .iter()
            .any(|(s, g, p)| s == step && g == group && p == plugin)
    }

    /// The variant-field form: `fomod:<json>`.
    pub fn encode(&self) -> String {
        format!(
            "{PREFIX}{}",
            serde_json::to_string(self).unwrap_or_default()
        )
    }

    /// Reads a variant field; `None` when it holds no FOMOD selection.
    pub fn decode(variant: Option<&str>) -> Option<Self> {
        variant
            .and_then(|v| v.strip_prefix(PREFIX))
            .and_then(|json| serde_json::from_str(json).ok())
    }
}

/// `fomod/ModuleConfig.xml` in any letter case.
pub fn config_path(mod_root: &Path) -> Option<PathBuf> {
    let find = |dir: &Path, name: &str| {
        std::fs::read_dir(dir)
            .ok()?
            .filter_map(|e| e.ok())
            .find_map(|e| {
                (e.file_name().to_string_lossy().eq_ignore_ascii_case(name)).then(|| e.path())
            })
    };
    find(mod_root, "fomod").and_then(|dir| find(&dir, "moduleconfig.xml"))
}

/// Loads the installer of a mod, if it has one.
pub fn load(mod_root: &Path) -> Result<Option<Config>> {
    let Some(path) = config_path(mod_root) else {
        return Ok(None);
    };
    let bytes = std::fs::read(&path).at(&path)?;
    parse(&decode_text(&bytes)).map(Some)
}

/// XML text from UTF-8 or UTF-16 (with byte-order mark) bytes.
pub fn decode_text(bytes: &[u8]) -> String {
    let utf16 = |data: &[u8], le: bool| {
        let units: Vec<u16> = data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| match le {
                true => u16::from_le_bytes([c[0], c[1]]),
                false => u16::from_be_bytes([c[0], c[1]]),
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, true),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, false),
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

#[cfg(test)]
pub mod tests;
