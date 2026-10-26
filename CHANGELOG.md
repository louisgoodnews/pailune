# Changelog

## Unreleased

### Mods in the game's archives
- Pylune builds its own archive group and a rebuilt `meta/0.papgt` (listed first, verified in
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
- Mod lists: export your whole setup and import it elsewhere; missing mods can be downloaded.
- Move a mod straight to a load-order number from its details panel.

### Documentation
- User guide, vision, architecture, data model, extension guide and decision records in `docs/`.

### Interface
- Reworked against documented UX principles: Clean slate in a *More* menu, dialogs name their
  action, larger text and checkboxes, consistent wording, contrast-checked colours.

### Platforms
- Operating-system specifics moved into one platform layer; Windows and macOS builds in CI.

### Packaging
- AppImage build script and release CI job; `nxm://` links point at the AppImage file.

### Save games
- Save files are backed up before every Play and on request; restore keeps a safety copy.

### Onboarding
- Getting-started guide with real, checkable steps and progress; searchable "How do I…" topics;
  ❓ Guide button and F1; the empty mod list explains what to do next.
- Icons render everywhere (emoji font bundled).

### Storage
- SQLite application state (`~/.local/share/pylune/pylune.sqlite3`), TOML settings
  (`~/.config/pylune/config.toml`).
