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

### Storage
- SQLite application state (`~/.local/share/pylune/pylune.sqlite3`), TOML settings
  (`~/.config/pylune/config.toml`).
