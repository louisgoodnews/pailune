//! How each staged file reaches the game.

use std::path::{Component, Path};

/// Mod content Pailune cannot deploy yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PazFormat {
    /// Game files whose location inside the archives is unknown.
    Unplaced,
}

/// Where a single staged file belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Linked at the same path below `bin64/`.
    Mirror,
    /// Flattened into `bin64/` (ASI plugins, DLLs, their INIs).
    Bin64,
    /// Packed into Pailune's archive group at its virtual path.
    Pack,
    /// Part of a prebuilt archive group (`NNNN/*.pamt`, `*.paz`) shipped by the mod.
    PrebuiltGroup,
    /// A byte-patch JSON mod applied to game data inside the archives.
    JsonPatch,
    /// A CSS/XML patch or merge file, or an HTML patch, applied to an archived game file.
    TextPatch,
    /// Not deployable yet.
    NeedsPaz(PazFormat),
    /// Documentation, screenshots and mod metadata.
    Skip,
}

impl Placement {
    /// Short note on how the file is used, for logs and the file list.
    pub fn note(self) -> &'static str {
        match self {
            Placement::Mirror | Placement::Bin64 => "linked",
            Placement::Pack => "packed into Pailune's archive group",
            Placement::PrebuiltGroup => "prebuilt archive group, registered by Pailune",
            Placement::JsonPatch => "JSON patch, applied into Pailune's archive group",
            Placement::TextPatch => "CSS/XML patch, merged into the game's file",
            Placement::NeedsPaz(PazFormat::Unplaced) => {
                "game file with unknown location — not deployed"
            }
            Placement::Skip => "not deployed",
        }
    }
}

/// Returns `true` for four-digit archive folder names such as `0036`.
pub fn is_archive_dir(name: &str) -> bool {
    name.len() == 4 && name.bytes().all(|b| b.is_ascii_digit())
}

/// Mod metadata files of the community formats; never deployed.
const MANIFESTS: [&str; 3] = ["manifest.json", "mod.json", "modinfo.json"];

/// Extensions of documentation and preview files that are never deployed.
const DOCS: [&str; 11] = [
    "txt", "md", "nfo", "pdf", "png", "jpg", "jpeg", "webp", "gif", "url", "rtf",
];

/// Lowercase path components.
fn lower_parts(rel: &Path) -> Vec<String> {
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(name) => Some(name.to_string_lossy().to_lowercase()),
            _ => None,
        })
        .collect()
}

/// The archive path of a game file: after a leading `NNNN/` group folder or
/// after a RAW mod's `files/NNNN/` segment (lowercase, `/`-separated).
pub fn virtual_path(rel: &Path) -> Option<String> {
    let parts = lower_parts(rel);
    let start = match parts.first() {
        Some(first) if is_archive_dir(first) => Some(1),
        _ => parts
            .windows(2)
            .position(|w| w[0] == "files" && is_archive_dir(&w[1]))
            .map(|i| i + 2),
    }?;
    Some(parts[start..].join("/")).filter(|p| !p.is_empty())
}

/// The game file a `*.css.patch`, `*.css.merge`, `*.xml.patch`,
/// `*.xml.merge` or `*.html.patch` file changes.
pub fn text_target(rel: &Path) -> Option<String> {
    let vpath = virtual_path(rel)?;
    if let Some(t) = vpath.strip_suffix(".html.patch") {
        return Some(format!("{t}.html"));
    }
    [".css", ".xml"].iter().find_map(|ext| {
        [".patch", ".merge"].iter().find_map(|kind| {
            vpath
                .strip_suffix(&format!("{ext}{kind}"))
                .map(|t| format!("{t}{ext}"))
        })
    })
}

/// Decides the [`Placement`] of a file from its path relative to the mod root.
pub fn placement(rel: &Path) -> Placement {
    let parts = lower_parts(rel);
    let (Some(first), Some(name)) = (parts.first(), parts.last()) else {
        return Placement::Skip;
    };
    let ext = rel
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match (first.as_str(), ext.as_str(), virtual_path(rel)) {
        ("bin64", _, _) => Placement::Mirror,
        // Game metadata is build-specific; Pailune generates it.
        ("meta", _, _) => Placement::Skip,
        (dir, "paz" | "pamt", _) if is_archive_dir(dir) && parts.len() == 2 => {
            Placement::PrebuiltGroup
        }
        _ if text_target(rel).is_some() => Placement::TextPatch,
        (_, _, Some(_)) => Placement::Pack,
        _ if MANIFESTS.contains(&name.as_str()) => Placement::Skip,
        (_, "asi" | "dll" | "ini", None) => Placement::Bin64,
        (_, "json", None) => Placement::JsonPatch,
        (_, ext, None) if DOCS.contains(&ext) => Placement::Skip,
        _ => Placement::NeedsPaz(PazFormat::Unplaced),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_paths() {
        let vp = |p: &str| virtual_path(Path::new(p));
        assert_eq!(vp("Mod/files/0012/UI/a.xml").as_deref(), Some("ui/a.xml"));
        assert_eq!(vp("0012/ui/xml/b.html").as_deref(), Some("ui/xml/b.html"));
        assert_eq!(vp("readme.txt"), None);
        assert_eq!(vp("0012"), None);
    }
}
