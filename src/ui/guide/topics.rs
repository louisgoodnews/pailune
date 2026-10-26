//! "How do I…" topics: short, task-oriented answers (minimalist
//! instruction, R-20), and the search over them.

/// One guide entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Topic {
    pub title: &'static str,
    /// Extra words people may search for.
    pub keywords: &'static str,
    /// Short paragraphs or steps.
    pub body: &'static [&'static str],
}

/// All topics, most common tasks first.
pub const TOPICS: &[Topic] = &[
    Topic {
        title: "Install a mod from a file",
        keywords: "import zip 7z rar folder archive drag drop add",
        body: &[
            "Mods tab › Import archives / files… (Ctrl+I) or Import folder…, or drop files onto the window.",
            "Supported: folders, .zip, .7z, .rar, and loose .asi, .dll or .json files.",
            "The mod is copied into Pylune's staging folder; your download is not changed.",
        ],
    },
    Topic {
        title: "Install mods from Nexus Mods",
        keywords: "nexus download api key nxm mod manager button browser",
        body: &[
            "Nexus tab › paste your personal API key (nexusmods.com › your account › API keys) › Save key.",
            "Press \"Make Pylune the nxm:// handler\" once; then the website's \"Mod manager download\" button sends files to Pylune.",
            "Each download asks for confirmation before it starts.",
        ],
    },
    Topic {
        title: "Install a collection",
        keywords: "collection modlist list curator revision",
        body: &[
            "Nexus tab › Collections › paste the collection's address or slug › Fetch.",
            "Review the members, untick optional ones, then Install. Mods are ordered like the curator's list.",
            "Premium accounts download everything at once; free accounts click each member's link on the website and Pylune accepts the matching downloads.",
        ],
    },
    Topic {
        title: "Enable, disable and order mods",
        keywords: "load order priority checkbox move up down enable disable win",
        body: &[
            "Tick a mod's checkbox to enable it; untick to disable. Nothing reaches the game until Apply.",
            "Mods lower in the list win when two change the same file. Use the arrows in the details panel to move a mod, or type its new number into \"Move to number\".",
        ],
    },
    Topic {
        title: "Apply, Unlink and what happens to game files",
        keywords: "apply deploy symlink unlink bak original safe",
        body: &[
            "Apply links enabled mods into the game folder. A replaced original is renamed to .bak.",
            "Unlink all removes every link and restores the .bak originals, leaving the game as it was.",
            "Mods inside the game's archives are packed into Pylune's own archive group; the game's archives are never edited.",
        ],
    },
    Topic {
        title: "Choose an option of a mod",
        keywords: "variant option choice fomod folder",
        body: &[
            "Some mods ship several versions in separate folders. Select the mod; the details panel offers a choice.",
            "Only the chosen option is deployed. Press Apply after changing it.",
            "Mods with a FOMOD installer show its pages in the details panel; the recommended choices are used until you change them.",
        ],
    },
    Topic {
        title: "Find out why a mod has no effect",
        keywords: "conflict overwrite lose win not working",
        body: &[
            "A ⚔ badge shows how many files a mod wins and loses against others.",
            "Select the mod and open its conflict list: files it loses are replaced by a mod lower in the list. Move it further down to make it win.",
            "Check the Log tab for messages about this mod.",
        ],
    },
    Topic {
        title: "Update mods",
        keywords: "update new version check newer",
        body: &[
            "Nexus tab › Check updates. Mods with a newer file are marked.",
            "Select a mod › Update: Nexus mods show their file list, other mods ask for a replacement archive.",
            "Settings › Behavior can check for updates every time Pylune starts.",
        ],
    },
    Topic {
        title: "Undo a delete",
        keywords: "undo deleted trash restore recover removed",
        body: &[
            "Press Undo in the status bar right after deleting, or open \"Recently deleted\" below the mod list.",
            "How many deleted mods are kept is set in Settings › Mods.",
        ],
    },
    Topic {
        title: "Switch between mod setups",
        keywords: "profile setup preset save load launch options",
        body: &[
            "Profiles tab › type a name › Save current as stores which mods are enabled and their order.",
            "Load switches to a profile; press Apply afterwards. Each profile can have its own launch options.",
        ],
    },
    Topic {
        title: "Share or move my whole setup",
        keywords: "mod list export import share copy another computer reinstall missing",
        body: &[
            "Profiles › Export mod list… saves the order, enabled mods, options and Nexus origins to a file.",
            "Import mod list… applies it: installed mods are ordered as listed; missing ones are listed with ways to get them and take their place once installed.",
        ],
    },
    Topic {
        title: "Use ASI or script mods",
        keywords: "asi loader dll version winmm dinput8 launch options proton",
        body: &[
            "ASI mods need a loader such as version.dll in bin64/. The header shows whether one is installed.",
            "Settings › Diagnostics shows the launch options Proton needs for the loader; set them as default options or per profile.",
        ],
    },
    Topic {
        title: "Back up and restore Pylune",
        keywords: "backup restore safety settings mod list",
        body: &[
            "Settings › Backups › Back up now (or Full backup, which also contains the mod files).",
            "A daily automatic backup is on by default. Restoring needs all mods unlinked and takes a safety backup first.",
        ],
    },
    Topic {
        title: "Protect my save games",
        keywords: "save savegame slot progress backup restore cloud",
        body: &[
            "Pylune copies the game's save files every time you press Play (Settings › Backups).",
            "Settings › Backups › Save games: Back up saves now, or Restore… an older copy. The current saves are backed up first.",
            "If Steam reports a cloud conflict after a restore, keep the local files.",
        ],
    },
    Topic {
        title: "After a game update",
        keywords: "game update patch broken version verify",
        body: &[
            "When the game changed, Pylune blocks Apply and shows a banner with the steps:",
            "1. Unlink all › 2. Verify files in Steam › 3. Refresh backup › Apply again.",
        ],
    },
    Topic {
        title: "Start over with a clean game",
        keywords: "clean slate reset vanilla remove all",
        body: &[
            "Clean Slate removes all links, restores the game's original metadata and sets other tools' archive folders aside.",
            "Your mods stay in Pylune, disabled. Set-aside folders can be restored in Settings › Diagnostics.",
        ],
    },
    Topic {
        title: "Report a problem",
        keywords: "log bug error debug report crash",
        body: &[
            "Log tab shows what Pylune did, per mod. Settings › Advanced › Open log folder has the full log files.",
            "Set the log level to debug in Settings › Advanced and restart for more detail.",
        ],
    },
    Topic {
        title: "Keyboard shortcuts",
        keywords: "keys shortcut hotkey keyboard",
        body: &[
            "F1 guide · Ctrl+F search mods · Ctrl+I import · Ctrl+Enter apply · F5 rescan · Delete removes the selected mod.",
        ],
    },
    Topic {
        title: "Words used in Pylune",
        keywords: "glossary staging overlay profile collection term meaning",
        body: &[
            "Staging: Pylune's folder holding your installed mods.",
            "Apply / Unlink: put links to enabled mods into the game / take them out again.",
            "Load order: the list order; lower mods win conflicts.",
            "Overlay: Pylune's own archive group that carries mods for files inside the game's archives.",
            "Profile: a saved set of enabled mods, order and launch options.",
            "Collection: a mod list from Nexus Mods, installed as one unit.",
        ],
    },
];

/// Whether every word of `query` appears in the topic (case-insensitive).
fn matches(topic: &Topic, words: &[String]) -> bool {
    let text = format!(
        "{} {} {}",
        topic.title,
        topic.keywords,
        topic.body.join(" ")
    )
    .to_lowercase();
    words.iter().all(|w| text.contains(w.as_str()))
}

/// Topics matching the search text; all topics for an empty query.
pub fn search(query: &str) -> Vec<&'static Topic> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    TOPICS.iter().filter(|t| matches(t, &words)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_uses_every_word() {
        assert_eq!(search("").len(), TOPICS.len());
        let found: Vec<_> = search("UNDO trash").iter().map(|t| t.title).collect();
        assert_eq!(found, ["Undo a delete"]);
        assert!(search("no such words here").is_empty());
    }
}
