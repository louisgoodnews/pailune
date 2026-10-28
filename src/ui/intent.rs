//! Every user intent the views can emit, grouped by feature. Views return
//! these; [`super::actions`] turns them into state changes and jobs.

use crate::mod_engine::model::MoveTo;
use std::path::PathBuf;

/// Main-area page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Mods,
    /// Save slots, save backups and the save editor.
    Saves,
    /// Read-only game data table browser.
    DataBrowser,
    Profiles,
    Nexus,
    Log,
    Settings,
}

/// Destructive actions that need an explicit confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirm {
    CleanSlate,
    DeleteMod(String),
    DeleteProfile(String),
    UninstallCollection(String),
    RestoreBackup(PathBuf),
    RestoreSaves(PathBuf),
    /// Write the values typed into the save viewer (`n` changes).
    EditSave(usize),
}

/// Mod management intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModAction {
    Select(String),
    SetEnabled(String, bool),
    SetAll(bool),
    Move(String, MoveTo),
    Delete(String),
    /// Nexus mods: pick a newer file; others: pick a replacement archive.
    Update(String),
    OpenFolder(String),
    ShowReadme(PathBuf),
    CloseReadme,
    ImportFolder,
    ImportFiles,
    /// Files or folders dropped onto the window.
    ImportPaths(Vec<PathBuf>),
    /// Choose the option folder of a multi-variant mod.
    SetVariant(String, String),
    /// Write a file of an installed mod: settings (`.ini`) or JSON patches
    /// (an absolute path inside staging).
    SaveModFile(PathBuf, String),
    /// Bring a deleted mod back from the trash.
    RestoreDeleted(String),
    /// Delete a trash entry for good.
    PurgeDeleted(String),
    /// Open the wizard to create a new JSON mod in staging.
    CreateModPrompt,
    /// Close the create mod wizard.
    CloseCreateMod,
    /// Create a new JSON mod in staging with metadata.
    CreateMod {
        name: String,
        author: String,
        version: String,
        description: String,
    },
    /// Open the dialog to package a mod for distribution.
    PackageModPrompt(String),
    /// Close the package mod dialog.
    ClosePackageMod,
    /// Package a mod into an installable distribution archive.
    PackageMod(crate::mod_engine::package::PackageMetadata),
    Rescan,
}

/// Game-directory intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployAction {
    Apply,
    UnlinkAll,
    CleanSlate,
    RestoreOverlays,
    RefreshBackup,
    Launch,
    /// Ask Steam to verify the game files (after a game update).
    VerifyGame,
}

/// Profile page intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileAction {
    /// Save the current selection under this name (new or overwrite).
    Save(String),
    Load(String),
    Delete(String),
    /// Store the edited launch options of this profile.
    SaveOptions(String),
    /// Write a profile to a JSON file chosen by the user.
    Export(String),
    /// Read a profile from a JSON file.
    Import,
}

/// Nexus page intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NexusAction {
    SaveKey,
    Validate,
    Fetch,
    Download(u64),
    CheckUpdates,
    OpenPage,
    RegisterHandler,
    /// Start the confirmed incoming download at this index.
    AcceptIncoming(usize),
    /// Drop the incoming download at this index.
    RejectIncoming(usize),
    /// Look up the collection in the draft field.
    FetchCollection,
    /// Flip whether an optional member of the previewed collection is installed.
    ToggleMember(usize),
    /// Install (or update to) the previewed collection.
    InstallCollection,
    ClosePreview,
    /// Fetch the newest revision of an installed collection for review.
    UpdateCollection(String),
    /// Enable or disable every installed member.
    CollectionEnabled(String, bool),
    UninstallCollection(String),
    /// Open a member's file page on Nexus (free accounts download there).
    OpenMemberPage(u64, u64),
}

/// First-run setup dialog intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupAction {
    /// Search Steam's libraries again.
    Detect,
    /// Save the chosen folder and close the dialog if it is valid.
    Confirm,
    /// Close the dialog without a valid game folder.
    Later,
}

/// Every user intent the views can emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Mod(ModAction),
    Deploy(DeployAction),
    Profile(ProfileAction),
    Nexus(NexusAction),
    Setup(SetupAction),
    Backup(BackupAction),
    Guide(super::guide::GuideAction),
    ModList(ModListAction),
    SwitchTab(Tab),
    BrowseGameDir,
    SaveSettings,
    RefreshLog,
    OpenLogFolder,
    /// Copy the shown log entries to the clipboard.
    CopyLog,
    /// Close Pailune.
    Quit,
    /// Put text on the clipboard.
    Copy(String),
    Notice(super::notify::NoticeAction),
    Ask(Confirm),
    Dismiss,
    DataBrowser(DataBrowserAction),
}

/// Game data browser intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataBrowserAction {
    SelectTable(String),
    SelectRecord(u64),
    RefreshTables,
    SetTargetMod(String),
    StartFieldEdit(super::state::ActiveFieldEdit),
    CancelFieldEdit,
    AddChangeToMod {
        mod_name: String,
        table_stem: String,
        record_name: String,
        record_key: u64,
        field_name: Option<String>,
        new_value: String,
        is_raw: bool,
        rel_offset: usize,
        original_hex: String,
    },
}

/// Portable mod list intents (B-12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModListAction {
    /// Write the current list to a file chosen by the user.
    Export,
    /// Read a list and apply it to the installed mods.
    Import,
    /// Download every missing mod that came from Nexus (Premium).
    DownloadMissing,
    /// Open the Nexus page of the missing mod at this index.
    OpenMissing(usize),
    /// Stop tracking the imported list.
    CloseMissing,
}

/// Backup intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupAction {
    /// Back up settings and the mod list (`full`: with mod files).
    Create {
        full: bool,
    },
    /// Restore a backup (after confirmation).
    Restore(PathBuf),
    /// Pick a backup file elsewhere and restore it (after confirmation).
    RestoreFromFile,
    Delete(PathBuf),
    OpenFolder,
    /// Back up the game's save files now.
    BackUpSaves,
    /// Put a save backup back in place (after confirmation).
    RestoreSaves(PathBuf),
    /// Open the game's save folder.
    OpenSaveFolder,
    /// Show the contents of a save slot (read-only).
    InspectSave(PathBuf),
    CloseSaveView,
    /// Ask before writing the typed values.
    SaveEdits,
    /// Write them (confirmed).
    WriteSaveEdits,
    DiscardSaveEdits,
}
