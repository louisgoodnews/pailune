//! Byte-patch JSON mods ("JSON mod", JMM v1/v2, `"format": 2`), per
//! `docs/paz-format.md` §10.1. A mod lists changes to the decrypted,
//! decompressed bytes of archived game files; Pailune applies them to the
//! vanilla bytes and packs the result into its overlay.

pub mod apply;
pub mod css;
pub mod document;
pub mod html;
pub mod parse;
pub mod table;
pub mod text;
pub mod xml;

#[cfg(test)]
mod apply_safety;
#[cfg(test)]
mod machine;

/// Where a change applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anchor {
    /// Absolute byte offset (relative to the end of `signature`, if given).
    Offset(u64),
    /// Offset from the end of a table record's name (v2 `entry` + `rel_offset`).
    Entry { name: String, rel_offset: u64 },
}

/// What a change does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Replace bytes; `original` is verified first when given.
    Replace {
        original: Option<Vec<u8>>,
        patched: Vec<u8>,
    },
    /// Insert bytes (not allowed inside table bodies).
    Insert { bytes: Vec<u8> },
}

/// One change of a patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub anchor: Anchor,
    pub edit: Edit,
    pub label: String,
}

/// All changes to one game file.
#[derive(Debug, Clone, PartialEq)]
pub struct FilePatch {
    /// Virtual path or bare file name as written by the mod author.
    pub game_file: String,
    /// Optional anchor bytes: offsets count from the end of their first match.
    pub signature: Option<Vec<u8>>,
    pub changes: Vec<Change>,
    /// Format-3 intents, resolved against the table when applied.
    pub fields: Vec<crate::tables::format3::Intent>,
}

/// A parsed JSON mod.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonMod {
    pub title: String,
    /// Apply the changes that match even if others do not.
    pub allow_partial: bool,
    pub patches: Vec<FilePatch>,
}

/// Outcome of applying one file patch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplyReport {
    pub applied: usize,
    /// Changes whose bytes were already in the patched state.
    pub already: usize,
    /// Labels (or offsets) of changes that could not be applied, with reasons.
    pub failed: Vec<String>,
}
