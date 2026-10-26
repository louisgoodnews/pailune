# Architecture

Pylune is one Rust binary (edition 2024). This page explains how its parts fit together; the
code's rustdoc (`cargo doc --document-private-items`) has the details of every item.

## Layers

```
 ┌───────────────────────────────────────────────────────────────┐
 │ ui/            egui front-end: state, views, actions, guide   │  UI thread
 ├───────────────────────────────────────────────────────────────┤
 │ jobs/          worker pool + exclusive lane, job tasks        │  worker threads
 ├───────────────────────────────────────────────────────────────┤
 │ mod_engine/  symlink/  overlay/  patch/  nexus/  launch/      │  domain logic
 │ app_backup/  save_backup/  backup/  profiles/                 │
 ├───────────────────────────────────────────────────────────────┤
 │ paz/  steam/  db/  config/  logging/  instance/  error        │  formats, storage, platform
 └───────────────────────────────────────────────────────────────┘
```

Lower layers never call upper ones. Domain modules are plain functions over data (mostly pure:
a plan is computed, then executed), which keeps them testable without a window or a game.

## Modules

| Module | Responsibility |
|--------|----------------|
| `main.rs` | Paths, config load (invalid files kept aside), logging, single-instance bootstrap, database open, window. |
| `config/` | `config.toml` model (`Config` and the `prefs` sections), atomic save, XDG paths (`DataPaths`), Steam auto-detection. |
| `db/` | SQLite: versioned migrations, mods (registry + revisions), links journal, profiles, events, collections, key/value. |
| `mod_engine/` | Staging: import (zip/7z/rar/loose), classification, placement rules, link plans, registry merge/order, variants, conflicts, trash, mod info/readmes. |
| `symlink/` | Executes link plans: `.bak` displacement, write-ahead journal, exact undeploy. |
| `paz/` | Read/write of the game's archive formats (PAPGT, PAMT, PAZ, PATHC, PAVER), ChaCha20, LZ4, DDS. See [paz-format.md](paz-format.md). |
| `overlay/` | Builds Pylune's archive group (`0100`) and a rebuilt `meta/0.papgt`/`0.pathc` from enabled mods; output is deployed as symlinks. |
| `patch/` | Byte-patch JSON and CSS patch/merge files applied to archived game files. |
| `backup/` | Snapshot of the game's `meta/` files to `.bak`; detection of game updates and foreign archive folders. |
| `app_backup/` | Backups of Pylune itself (settings, database, optionally mod files) and restore. |
| `save_backup/` | Backups of the game's save folder in the Proton prefix. |
| `nexus/` | REST v1 + GraphQL v2 client, downloads (resume, MD5), `nxm://` parsing and handler registration, update chains, collections, rate limits, API key storage. |
| `launch/` | Launch options parsing, Proton/Steam Linux Runtime discovery, starting and watching the game process. |
| `steam/` | Steam library discovery, VDF parser, game/prefix/loader detection. |
| `profiles/` | Saved mod setups (enabled set, order, launch options), JSON export/import. |
| `platform/` | Operating-system differences behind one set of functions (Steam roots, linking, opening, Steam client, process check, private files, `AppData/Local`, Proton or not); `linux.rs` is the reference, `windows.rs`/`macos.rs` are built by CI. |
| `cli/` | Command line: `ops.rs` (the `Op` set shared with MCP), `read.rs` (read-only, straight from the database), `engine.rs` (changing operations through a headless `PyluneApp` holding the instance lock), clap front-end. |
| `mcp/` | Model Context Protocol server over stdio (JSON-RPC 2.0); `tools/table.rs` lists the tools, `tools/mod.rs` maps calls to `Op`s. |
| `instance.rs` | `flock` single-instance lock and the spool directory through which a second start hands over `nxm://` links or a focus request. |
| `logging/` | Daily log files (`tracing`), the per-mod event journal and per-mod log files. |
| `jobs/` | Job kinds, the pool, the job context (config snapshot, DB connection, progress, game lock), and `tasks/` implementing each job. |
| `ui/` | `PyluneApp` state (`mod.rs`, `state.rs`), user intents (`intent.rs`), state transitions (`actions/`), rendering (`views/`), onboarding (`guide/`), theme, shortcuts. |

## Threads and processes

- **UI thread.** egui calls `PyluneApp::logic` and `ui` every frame. Views read state and return
  `Action`s; `dispatch` applies them (`ui/actions/`). Anything slower than a few milliseconds is
  submitted as a job. A ticker thread requests a repaint every second, so polling continues while
  the window is hidden.
- **Job pool** (`jobs/pool.rs`). `max_parallel` shared workers run downloads, imports, Nexus
  queries and conflict analysis. A separate single-thread **exclusive lane** runs every job that
  touches the game folder or Pylune's state as a whole (Apply, Unlink, Clean Slate, delete,
  backups) strictly in submission order. Each job gets a `JobContext` with a snapshot of the
  config and its own database connection; panics are caught and reported as failures.
- **Polling** (`ui/actions/poll.rs`). Once per second the UI drains job events, checks whether
  the game process runs, and reads the spool for requests from secondary instances.
- **Single instance** (`instance.rs`). The first Pylune holds a `flock`. A second start (e.g. the
  browser opening an `nxm://` link) writes the request into the spool directory and exits.

## Main flows

**Import.** `ModAction::ImportPaths` → `JobKind::Import` per source → archive extracted into a
hidden work folder in staging → atomically renamed to the mod's folder → registry rescanned and
merged in a write transaction → install event journaled.

**Apply.** `DeployAction::Apply` → `JobKind::Apply` (exclusive) → refuse if the game was updated
since the metadata backup → undeploy everything recorded in the links table → `plan::build`
computes link operations from the enabled mods in load order (later wins) → `overlay` packs
archive content into group `0100`, rebuilds `0.papgt`/`0.pathc` and adds their links →
`symlink::deploy` records each link in the database *before* creating it, moving originals to
`.bak` → the deployed registry revision is stored, so "pending changes" is simply
`registry revision ≠ deployed revision`.

**Unlink / Clean Slate.** Links are removed in reverse order and `.bak` originals restored, each
forgotten only *after* it was reverted (write-ahead). Clean Slate additionally restores `meta/`
from `.bak`, sets foreign archive folders aside and disables every mod.

**Play.** Optional Apply first (the launch waits for that job), save-game backup, then start via
Steam (`steam -applaunch`), the `steam://` handler, or Proton directly through the Steam Linux
Runtime when launch options contain environment variables or a wrapper command (e.g.
`gamemoderun %command%`).

**Nexus download.** A `nxm://` link (via the spool) is validated (key, expiry, game) and shown
for confirmation → `JobKind::Download` resolves mirrors, downloads with HTTP range resume,
checks size and MD5 → imports like a local archive and records the Nexus origin.

## Errors, logging, persistence

- One error type (`error::PyluneError`, `thiserror`); I/O errors carry their path. Actions turn
  errors into a red status line; jobs report them in the jobs panel and the Log tab.
- `tracing` writes daily log files; domain events (install, deploy, backup …) go to the
  `events` table and per-mod log files, shown in the Log tab.
- Settings live in `config.toml` (user-editable), application state in SQLite (WAL mode, short
  transactions, `BEGIN IMMEDIATE` for read-modify-write). See [data-model.md](data-model.md).

## Testing

`cargo test` runs offline: unit tests next to the code, and `src/tests/` with integration tests
that build a fake game folder, fake Steam library and XDG data folder in a temp dir. They cover
crash safety (interrupting a deploy between journal writes), staging, jobs, the overlay, JSON
patches, a mock Nexus HTTP server, backups, and the UI driven headlessly through `dispatch`.
Ignored tests read a real game installation, talk to the real Nexus API, or render screenshots
with the GPU. CI runs `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test`.
