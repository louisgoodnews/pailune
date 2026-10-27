# Changelog

## Unreleased

### Mods in the game's archives
- Pailune builds its own archive group and a rebuilt `meta/0.papgt` (listed first, verified in
  game on build 2.3.2), deployed as symlinks; vanilla archives are never modified.
- RAW mods (`files/NNNN/<path>`, `NNNN/<path>`), DDS textures (with a rebuilt `meta/0.pathc`),
  prebuilt archive groups and byte-patch JSON mods are supported.
- Read-only archive reader: PAPGT, PAMT, PATHC, ChaCha20, LZ4, partial DDS.

### Nexus Mods
- Collections: review, install in the curator's load order, update to new revisions,
  enable/disable, uninstall.
- `nxm://` links need the website's key and are confirmed in a dialog; links for other games go
  to the previous handler; update checks follow the file chain; rate limits are respected;
  downloads are size-checked.

### Safety and reliability
- Write-ahead link journal, single-instance lock, FIFO lane for game-folder jobs, guarded
  window close, game-update recovery banner, case-insensitive game paths.

### Everyday use
- Steam library auto-detection and first-run setup; launches through the Steam Linux Runtime
  with wrapper and environment support; profiles with launch options and JSON export/import;
  mod search, keyboard shortcuts, drag-and-drop import; Log tab with per-mod logs.

### Settings
- Categorised settings page with explanations, Save/Discard, per-page reset to defaults.
- Light and dark themes (or follow the desktop), UI scale, density, reduced motion.
- New options: confirm deletes, update check at start, enable new mods, trash size, automatic
  backups and how many to keep, days of logs kept.

### Mod management
- Field-name JSON (format 3): raw changes on every data table; `set` by name where Pailune derived and verified the layout (21 tables so far), plus the leading fields of the item table (stack sizes: mods like *Fat Stacks* work).
- ASI plugins: list with their mod on the Diagnostics page; edit plugin settings (`.ini`).
- HTML patch files (`*.html.patch`) with CSS-style selectors.
- XML patch and merge files (`*.xml.patch`, `*.xml.merge`) change the game's XML in place.
- FOMOD installers: the installer's pages appear in the details panel; choices are remembered.
- Mod lists: export your whole setup and import it elsewhere; missing mods can be downloaded.
- Move a mod straight to a load-order number from its details panel.
- Conflicts between JSON mods: mods changing the same values of a game file are listed with the
  records involved; on the same values the later mod now overrides the earlier one (its original
  bytes are checked against the unmodded game) instead of failing.
- Faster Apply and analysis: the scan of the game's archive indexes is reused while they are
  unchanged; format-3 intents are matched to records by key.
- Mods that wrap their game folders in one extra folder next to a plugin or JSON file
  (`Mod/Mod Enhanced/0009/…`) now deploy; before, those files were left out.
- New `.pac` meshes are packed (stored plain, as some game meshes are) instead of being refused
  as textures.
- Format-3.1 intents (records chosen by `match`, nested fields) are refused with a clear reason.
- JSON mods with alternative files (e.g. *Fat Stacks* 2x … 100x) offer them as options; only the
  chosen file is applied.
- JSON mod editor in the details panel: every change with its original and new value, editable as
  a number or hex (format 3: the new field value), plus the raw JSON, checked as you type.

### Documentation
- User guide, vision, architecture, data model, extension guide and decision records in `docs/`.

### Interface
- The header warns when an ASI loader is installed but Steam's launch options leave it off
  (no ASI plugin would run); clicking the warning copies the launch option.
- Notifications: pop-ups (top right) when mods are installed, applied, deleted or restored,
  backups made, scans finished or something failed; the 🔔 button keeps them for the session.
  Pop-ups can be switched off in *Settings → Behavior*.
- The mod table fills the window width; the background jobs panel is resizable and scrolls.
- *More → Quit Pailune* (Ctrl+Q) closes Pailune; the menu stays usable while the game runs.
- *Log → Copy log* copies the shown entries as text, e.g. for bug reports.
- Reworked against documented UX principles: Clean slate in a *More* menu, dialogs name their
  action, larger text and checkboxes, consistent wording, contrast-checked colours.

### Command line and agents
- `pailune <command>` for every main task, with `--json`; `pailune mcp` for AI agents (MCP).

### Platforms
- Windows: Pailune can register itself for `nxm://` links and forwards other games' links.
- Files can be deployed as hard links or copies where symlinks are not allowed (Windows), with the same exact revert.
- Operating-system specifics moved into one platform layer; Windows and macOS builds in CI.

### Packaging
- AppImage build script and release CI job; `nxm://` links point at the AppImage file.

### Save games
- New **Saves** tab next to Mods: slots and save backups on the left, the open save on the right.
- Save editing: change any number or flag of a save in the inspector (copper coins have a
  shortcut); written after a confirmation, with every save backed up first.
- Pailune reads the game's save files: slot list with level and time, a searchable read-only
  inspector, `pailune saves` / `pailune save`, MCP tools.
- Save files are backed up before every Play and on request; restore keeps a safety copy.

### Onboarding
- Getting-started guide with real, checkable steps and progress; searchable "How do I…" topics;
  ❓ Guide button and F1; the empty mod list explains what to do next.
- Icons render everywhere (emoji font bundled).

### Fixes from code review
- Paths in FOMOD installers and mods can no longer point outside the mod or game folder.
- XML/HTML patches: quoted `>` in attributes, self-closing elements gaining children, unknown
  predicates and per-parent positions handled; table patches that overlap or change a record's
  length are refused; ambiguous record names need a key.
- An interrupted save restore is undone at the next start; corrupt or hostile save files and
  installers fail cleanly instead of crashing.
- Command line and MCP: failed downloads/installs are reported as errors; MCP no longer keeps
  the window locked out between calls.
- Mod lists: several files of the same Nexus mod are matched one-to-one; installing missing mods
  later keeps changes made since the import; loading a profile drops the pending list.
- FOMOD: options sharing a name are remembered correctly; a broken installer deploys nothing and
  is shown in the details panel.
- Plugin settings editor notices files changed on disk; the save inspector's search is faster.
- Windows: no console windows flash up; links open without going through `cmd`.

### Storage
- SQLite application state (`~/.local/share/pailune/pailune.sqlite3`), TOML settings
  (`~/.config/pailune/config.toml`).
