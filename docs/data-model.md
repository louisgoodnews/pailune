# Data model

Everything Pailune stores, where it lives, and what each part means.

## Files on disk

| Path | Contents |
|------|----------|
| `~/.config/pailune/config.toml` | User settings (below). Rewritten atomically; an unreadable file is kept as `config.toml.invalid` and defaults are used. |
| `~/.local/share/pailune/pailune.sqlite3` | Application state (below). WAL mode. |
| `…/staging/<mod>/` | One folder per installed mod, exactly as extracted. Hidden `.pailune-*` folders are work areas of running imports. |
| `…/overlay/` | Pailune's built archive group (`0100/0.pamt`, `0.paz`), rebuilt `0.papgt`, `0.pathc`. Linked into the game by Apply. |
| `…/trash/<time>-<mod>/` | Deleted mods with `entry.json` (name, time, source, variant, Nexus origin). Newest `[mods] trash_keep` kept. |
| `…/backups/pailune-<time>-<kind>-<n>.zip` | Pailune backups (below). |
| `…/save-backups/saves-<time>-<n>.zip` | Save-game backups (below). |
| `…/exports/<mod>-v<version>.zip` | Packaged mod distribution zip archives ready for Nexus Mods. |
| `…/downloads/` | Nexus downloads in progress (`.part`, resumable). |
| `…/logs/pailune.log.<date>` | Daily application logs, newest `[logging] keep_days` kept. |
| `…/logs/mods/<mod>.log` | Per-mod history: installs, file lists, deployments, patch results. |
| `…/nexus_api_key` | Nexus API key, mode `0600`; never in backups or logs. |
| `…/spool/*.req`, `…/pailune.lock` | Requests from a second start; single-instance lock. |
| `~/.local/share/applications/pailune-nxm.desktop` | `nxm://` handler registration (when enabled). |

In the **game folder** Pailune only creates symlinks, renames shadowed originals to `<file>.bak`,
snapshots `meta/0.papgt`, `0.pathc` and `0.paver` to `.bak` on first start, and (Clean Slate)
renames foreign archive folders above `0035` to `NNNN.bak`.

## `config.toml`

| Section | Key | Default | Meaning |
|---------|-----|---------|---------|
| `[game]` | `dir` | detected | Game folder (contains `bin64/`, `meta/`). |
| | `app_id` | `3321460` | Steam app id. |
| `[ui]` | `theme` | `"dark"` | `"system"`, `"dark"` or `"light"`. |
| | `scale` | `1.0` | Interface zoom, 0.8 – 2.0. |
| | `density` | `"comfortable"` | Or `"compact"`. |
| | `reduced_motion` | `false` | Disable animations. |
| `[behavior]` | `confirm_deletes` | `false` | Ask before deleting a mod (deletes are undoable). |
| | `check_updates_on_start` | `false` | Check Nexus for updates at start (needs a key). |
| | `show_guide` | `true` | Open the getting-started guide at start. |
| `[mods]` | `enable_new` | `false` | Enable newly installed mods. |
| | `trash_keep` | `20` | Deleted mods kept restorable. |
| `[backup]` | `auto_daily` | `true` | Daily automatic Pailune backup. |
| | `keep_auto` | `10` | Automatic Pailune backups kept. |
| | `saves_before_play` | `true` | Back up save games before every launch. |
| | `keep_saves` | `10` | Automatic save backups kept. |
| `[nexus]` | `domain` | `"crimsondesert"` | Game domain on nexusmods.com. |
| | `api_url` | — | API root override (tests and debugging only; file-only). |
| `[launch]` | `apply_before_launch` | `true` | Apply pending changes before Play. |
| | `default_options` | `""` | Steam-style launch options when the active profile has none. |
| `[jobs]` | `max_parallel` | 2 – 4 | Shared worker threads (restart to apply). |
| `[logging]` | `level` | `"info"` | `tracing` filter; `RUST_LOG` overrides (restart to apply). |
| | `keep_days` | `14` | Daily log files kept. |

Missing keys take their defaults, so older files keep working.

## SQLite schema

Migrations live in `db/schema.rs` and are tracked with `PRAGMA user_version` (currently 5).

| Table | Columns | Meaning |
|-------|---------|---------|
| `mods` | `name` (PK), `position`, `enabled`, `kind`, `file_count`, `paz_files`, `installed_at`, `source`, `nexus_mod_id`, `nexus_file_id`, `nexus_version`, `variant` | The mod list in load order (`position`; later wins). `kind` is the detected category; `paz_files` counts files that need archive handling; `variant` the chosen option folder. |
| `links` | `seq` (PK), `target` (unique), `source`, `backup`, `stamp_size`, `stamp_mtime` | Write-ahead journal of every link in the game folder, in creation order, with the `.bak` path of a displaced original; hard links and copies (Windows fallback, D-11) carry a size/mtime stamp. |
| `created_dirs` | `seq`, `path` | Folders Pailune created in the game folder, removed again on unlink. |
| `profiles` | `name` (PK), `launch_options` | Saved setups. |
| `profile_slots` | `profile`, `position`, `mod_name`, `enabled` | A profile's mod order and enabled flags. |
| `collections` | `slug` (PK), `name`, `revision`, `installed_at` | Installed Nexus collections. |
| `collection_members` | `slug`, `position`, `mod_id`, `file_id`, `optional`, `selected`, `title`, `size`, `mod_name` | Members in the curator's order; `mod_name` once installed. |
| `events` | `id`, `ts`, `severity`, `kind`, `mod_name`, `message` | Event journal behind the Log tab (trimmed to 20 000). |
| `kv` | `key`, `value` | Small values: `registry_revision`, `deployed_revision` (pending changes = they differ), `active_profile`, `previous_nxm_handler`. |

Event kinds: `install`, `update`, `remove`, `deploy`, `unlink`, `reset`, `backup`, `download`,
`launch`, `profile`, `settings`, `job`.

## Backup archives

**Pailune backup** (`app_backup/`): a zip with `manifest.json`
(`format: 1`, `created_at`, `kind`: `settings`|`full`, `label`: `auto`|`manual`|`before restore`,
`mods`, `app_version`), `pailune.sqlite3` (online SQLite backup), `config.toml`, and for full
backups `staging/**`. Restoring requires nothing deployed, takes a `before restore` backup, restores
the database online and keeps the replaced staging folder as `staging-before-restore-<time>`.

**Save backup** (`save_backup/`): a zip with `manifest.json` (`format: 1`, `kind: "saves"`,
`created_at`, `label`: `before play`|`manual`|`before restore`, `files`, `app_version`) and
`save/**`, a copy of `<prefix>/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save/`.

**Mod list** (`modlist/`, B-12): JSON with `kind: "pailune-modlist"`, `format: 1`, `game`
(Nexus domain), `exported_at`, `app_version` and `mods` in load order; each mod has `name`,
`enabled`, optional `variant`, `source`, optional `nexus` (`mod_id`, `file_id`, `version`) and
optional `collections` (slugs). Files for another game or a newer format are rejected.

## Game archive formats

See [paz-format.md](paz-format.md): pack group table `meta/0.papgt` (first listed group wins),
per-group `0.pamt` indexes and `.paz` data, `meta/0.pathc` texture table, `0.paver` version,
ChaCha20 + LZ4, partial DDS.
