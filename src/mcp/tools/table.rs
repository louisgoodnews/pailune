//! The MCP tools Pylune offers (data only).

/// One tool: name, description, input schema, and hints for clients.
pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    /// JSON schema properties (all listed ones are required unless noted).
    pub params: &'static [(&'static str, &'static str, &'static str)],
    pub read_only: bool,
    pub destructive: bool,
}

/// `(name, JSON type, description)` shorthand.
const NAME: (&str, &str, &str) = ("name", "string", "Mod name as shown by list_mods");

/// Every tool, read-only ones first.
pub const TOOLS: &[Tool] = &[
    Tool {
        name: "status",
        description: "Game folder, number of mods, enabled mods, deployed links, whether changes are pending, active profile.",
        params: &[],
        read_only: true,
        destructive: false,
    },
    Tool {
        name: "list_mods",
        description: "Installed mods in load order (position 1 loads first; later mods win file conflicts), with enabled flag, kind, variant and Nexus origin.",
        params: &[],
        read_only: true,
        destructive: false,
    },
    Tool {
        name: "list_profiles",
        description: "Saved profiles (sets of enabled mods and order) and the active one.",
        params: &[],
        read_only: true,
        destructive: false,
    },
    Tool {
        name: "read_log",
        description: "Recent journal events (installs, deployments, warnings, errors), newest first.",
        params: &[
            ("mod_name", "string", "Optional: only this mod"),
            (
                "limit",
                "integer",
                "Optional: number of events (default 50)",
            ),
        ],
        read_only: true,
        destructive: false,
    },
    Tool {
        name: "list_backups",
        description: "Pylune backups (settings, mod list, optionally mod files) and save-game backups.",
        params: &[],
        read_only: true,
        destructive: false,
    },
    Tool {
        name: "list_saves",
        description: "The game's save slots with character level, save time and mission key (read-only).",
        params: &[],
        read_only: true,
        destructive: false,
    },
    Tool {
        name: "read_save",
        description: "Everything one save slot contains (character, inventory, quests, knowledge, …) as a tree, read-only. Use `find` to keep only matching branches; full saves are large.",
        params: &[
            ("slot", "string", "Slot name or number, e.g. slot1 or 1"),
            (
                "find",
                "string",
                "Optional: text to search for, e.g. _level or ItemSaveData",
            ),
        ],
        read_only: true,
        destructive: false,
    },
    Tool {
        name: "enable_mods",
        description: "Enable mods. Nothing reaches the game until apply.",
        params: &[("names", "array", "Mod names")],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "disable_mods",
        description: "Disable mods. Takes effect in the game after apply.",
        params: &[("names", "array", "Mod names")],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "set_all_mods",
        description: "Enable or disable every mod.",
        params: &[(
            "enabled",
            "boolean",
            "true to enable all, false to disable all",
        )],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "move_mod",
        description: "Move a mod to a 1-based load-order position (clamped to the list).",
        params: &[NAME, ("position", "integer", "Target position, 1 = first")],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "set_variant",
        description: "Choose the option folder of a mod that ships several variants.",
        params: &[NAME, ("variant", "string", "Option folder name")],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "install_mods",
        description: "Install mods from local archives (.zip/.7z/.rar), folders or loose .asi/.dll/.json files. They are added disabled unless configured otherwise.",
        params: &[("paths", "array", "Absolute paths")],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "delete_mod",
        description: "Delete a mod. It moves to Pylune's trash and can be restored from the window.",
        params: &[NAME],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "apply",
        description: "Link the enabled mods into the game in load order. Originals are kept as .bak.",
        params: &[],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "unlink_all",
        description: "Remove every link from the game folder and restore the original files.",
        params: &[],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "clean_slate",
        description: "Put the game back to its original state: unlink, restore metadata, set aside other tools' archive folders, disable every mod. Requires confirm: true.",
        params: &[("confirm", "boolean", "Must be true")],
        read_only: false,
        destructive: true,
    },
    Tool {
        name: "save_profile",
        description: "Save the current enabled mods and order as a profile.",
        params: &[NAME],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "load_profile",
        description: "Switch to a saved profile (call apply afterwards).",
        params: &[NAME],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "export_mod_list",
        description: "Write the whole setup (order, flags, options, Nexus origins) to a JSON file.",
        params: &[("path", "string", "Absolute target path")],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "import_mod_list",
        description: "Apply a mod list file to the installed mods; reports mods that are not installed.",
        params: &[("path", "string", "Absolute path of the list")],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "create_backup",
        description: "Back up settings, mod list and profiles (full: also every mod file).",
        params: &[("full", "boolean", "Optional: include mod files")],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "restore_backup",
        description: "Restore a Pylune backup (all mods must be unlinked; a safety backup is taken first). Requires confirm: true.",
        params: &[
            ("path", "string", "Backup file from list_backups"),
            ("confirm", "boolean", "Must be true"),
        ],
        read_only: false,
        destructive: true,
    },
    Tool {
        name: "backup_saves",
        description: "Back up the game's save files.",
        params: &[],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "check_updates",
        description: "Check Nexus Mods for newer files of installed mods (needs a stored API key).",
        params: &[],
        read_only: false,
        destructive: false,
    },
    Tool {
        name: "launch_game",
        description: "Start Crimson Desert through Steam (pending changes are applied first if configured; saves are backed up).",
        params: &[],
        read_only: false,
        destructive: false,
    },
];
