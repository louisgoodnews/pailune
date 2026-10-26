# Pylune board

Work items from [audit.md](audit.md), scheduled by [roadmap.md](roadmap.md).
Move a line between columns when its status changes.

## In progress

- [ ] A-01 PAZ pipeline — *Phase 3* — see [paz-format.md](paz-format.md).
  - done: spec; read-only `src/paz/` (pa_hash, PAVER, PAPGT parse/serialize, PAMT,
    ChaCha20, LZ4, extraction), verified read-only on the installed game (34 groups,
    2 052 523 files, real XML decrypted); PAZ-only content flagged instead of linked;
    inactive archive folders reported.
  - done: overlay writer (`src/paz/write.rs`, `src/overlay/`): RAW and group-folder files are
    packed into group `0100` with the shadowed file's flags; prebuilt mod groups re-registered
    under free numbers; `0.papgt` rebuilt from the verified vanilla backup with overlays first;
    deployed as symlinks; Unlink restores vanilla byte for byte. Tested end to end on a
    fixture and dry-run against the installed game.
  - **verified in game (2026-10-25, build 2.3.2):** an overlay group listed first in
    `0.papgt` overrides the vanilla file ("first group wins") — `local/Pylune Overlay Test`
    turned the title screen copyright line red and large.
  - done: textures — partial-DDS codec (`src/paz/dds/`) and `0.pathc` reader/writer
    (`src/paz/pathc.rs`); replaced textures keep the vanilla format class, PATHC is rebuilt
    from the vanilla backup and deployed with the overlay. Verified read-only on 302 real
    textures (templates and chunk sizes match PATHC; 253 re-encode byte for byte; PATHC
    round-trips byte for byte). Optional in-game check: `local/Pylune Texture Test`.
  - done: byte-patch JSON (`src/patch/`): dialect-tolerant parser, table index reader
    (`.pabgh`/`.staticinfoheader`, verified on the real item table: 6 816 records),
    entry and offset anchors, signature anchors, original-byte verification, strict vs.
    `allow_partial_apply`, later mods win; game files resolved by path, renamed extension
    or unique file name; patched files packed with vanilla flags; results journaled per
    mod. Dry run on the real item table changes exactly the targeted byte.
  - not yet: field-name JSON (format 3, needs per-table schemas — reported as
    unsupported), inserts into tables, XML/HTML patch files, new texture ids.
  - done: CSS patch/merge files (`*.css.patch`, `*.css.merge`: @merge/@replace/@add/@remove),
    applied in load order on top of shipped or vanilla stylesheets; untouched rules stay
    byte-identical (checked on the real title stylesheet).
  - done: conflicts between packed archive files are reported like linked ones
    (`[archive]/<path>` in the details panel).

## Next (small first)

- [ ] B-15 Move a mod to a load-order number — *S*
- [ ] B-12 Portable mod lists (export/import with variants, Nexus origins, collections) — *S–M*
- [ ] B-14 Headless core + CLI, then MCP server — *M–L*
- [ ] B-08 (step 1) `Platform` trait around Linux-specific code — *M*
- [ ] B-07 (research) Save format: read-only decoding, `docs/save-format.md` — *L*
- [ ] B-13 (research) Script bridge report + decision record

## Backlog (large)

- [ ] B-07 (step 2) Save viewer, then editing — *L*
- [ ] B-08 (step 2) Windows support — *L*
- [ ] B-13 (step 2) Script bridge installation — *L*
- [ ] A-01 (rest) XML/HTML patches, new texture ids, table inserts, field-name JSON — *L*
- [ ] A-28 (rest) FOMOD-style installer scripts — *L*

## Owner

- [ ] A-21 (rest) Register Pylune with Nexus (AUP) before a public release

## Done

- [x] B-09/B-10/B-11 `CONTRIBUTING.md` (setup, conventions, checks, commits, game-data rules,
  license), `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1, contact louisgoodnews95@gmail.com),
  GitHub issue forms (bug report, feature request; blank issues off) and a PR template.
- [x] B-08 decision: Windows and macOS are goals — vision updated, open decision D-11 for the
  per-platform deployment method.
- [x] B-05 documentation: `docs/README.md` (index), `vision.md`, `architecture.md` (layers,
  modules, threads, flows, testing), `data-model.md` (files, every `config.toml` key, SQLite
  schema, backup formats), `extending.md` (conventions and recipes), `decisions.md` (D-01 … D-10),
  `user-guide.md`; rustdoc on every item. Writing it surfaced a gap — R-01 promised a pending-
  changes banner that only existed as status-bar text — now a banner with *Apply now*.
- [x] B-04 UI/UX rework against `docs/ux-principles.md` (every main screen rendered and reviewed
  in both themes): Clean slate moved into a *More* menu (R-06); confirmation buttons name their
  action and texts are plain language (R-05, R-08); Apply is the filled primary only while changes
  are pending (R-10); text no smaller than 11.5 pt, larger checkboxes (R-13, R-25); one vocabulary
  (Enable/Disable); quieter, separated Delete; teaching empty states for profiles and Nexus;
  shortcuts in tooltips; button labels pick black or white for contrast; contrast test for both
  palettes (it caught light-theme badge colours that had not been applied); rustdoc on every UI
  helper (rest of A-32).
- [x] A-29 AppImage: `packaging/appimage.sh` assembles an AppDir (binary, desktop entry, SVG and
  256 px icons, AppStream metadata) and packs it with `appimagetool`; CI builds it on Ubuntu 22.04
  for tags `v*` (glibc ≥ 2.35; a local CachyOS build needs glibc 2.44) with a pinned, checksummed
  appimagetool 1.9.1. The `nxm://` handler registers the AppImage file itself (`$APPIMAGE`), not
  its changing mount point. AppDir layout verified locally; the final packing step was not run
  here.
- [x] A-28 (part) save-game backups (`src/save_backup/`): the save folder in the Proton prefix
  (`…/AppData/Local/Pearl Abyss/CD/save/<id>/slot<N>/`, verified on the installed game) is zipped
  before every Play (default on, newest 10 kept) and on request; restore backs up the current
  saves first and swaps folders by rename; Settings › Backups › Save games. Read-only check on
  the real saves: 13 files, byte-identical after extraction.
- [x] B-03 onboarding (`src/ui/guide/`, `src/ui/views/guide.rs`): a non-modal guide window opens
  on first start with three core ideas (R-18) and five real steps checked against the app's state
  (find game → add mod → enable → apply → play) with progress and a head start (R-17), buttons that
  perform each step, a finish state (R-23); searchable "How do I…" topics (17, incl. glossary and
  shortcuts); ❓ Guide button and F1; teaching empty mod list (R-19); finished guides stop opening
  at start.
- [x] Rendering checks: bundled emoji font (icons no longer depend on system fonts), a test that
  every UI symbol has a glyph, and on-demand screenshots of key screens in both themes
  (`PYLUNE_SHOTS=<dir> cargo test screens -- --ignored`).
- [x] B-02 settings menu (`src/ui/views/settings/`, `src/config/prefs.rs`): eight categories
  with one-line help per option, constrained controls (R-07), Save/Discard bar, per-page reset;
  appearance (theme incl. System, scale, density, reduced motion) previewed live; WCAG-checked
  light and dark palettes; new `[ui]`, `[behavior]`, `[mods]`, `[backup]` sections and
  `[logging] keep_days`, all wired (deletes skip the dialog unless asked — Undo exists).
- [x] UX principles (`docs/ux-principles.md`): 29 rules with evidence and references.
- [x] B-01 backups (`src/app_backup/`): one `.zip` with manifest, `config.toml` and an online
  SQLite snapshot (mod list, load order, flags, variants, profiles, collections, journal), full
  backups add the mod files; daily automatic backup (newest 10 kept); restore needs nothing
  deployed, takes a safety backup first, restores the database online, keeps the replaced
  staging folder aside; Settings page lists, restores and deletes backups.
- [x] B-06 deleted mods move to a trash with their metadata (origin, source, variant); Undo in
  the status bar and a "Recently deleted" list restore them; the newest 20 are kept.
- [x] A-24 downloads resume with HTTP ranges (up to 4 attempts) and are verified with Nexus'
  `md5_search` (a hash registered for another file fails; an unknown hash is noted).
- [x] A-32 rustdoc on every function and constant outside the UI views (views follow in B-04);
  deepest non-UI nesting (parallel update check) flattened.
- [x] A-28 (part) multi-variant mods: option folders detected (≥ 2 sibling folders with game
  layouts), only the chosen option (default: first) deployed, shared root files always; picker in
  the details panel; choice stored (schema v4).
- [x] A-31 UI actions tested through a headless `PyluneApp` (import/apply/delete, profiles,
  settings validation, nxm key rule, collection pending downloads); Nexus download, install,
  update check, rate limits and key rejection tested against a local mock API (`[nexus] api_url`).
- [x] A-29 (part) dual license MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
- [x] A-29 (part) release profile (thin LTO, stripped), GitHub Actions CI (fmt, clippy -D
  warnings, tests), desktop launcher + SVG icon + `packaging/install.sh`, window icon,
  CHANGELOG.
- [x] A-28 (part) mod search (Ctrl+F), shortcuts, drag-and-drop import, profile export/import.
- [x] A-33 mod table scrolls horizontally on narrow windows.
- [x] A-30 log files kept 14 days, journal trimmed to 20 000 events, deleted mods' logs retired.
- [x] A-21 `X-RL-*` budget recorded after every API/GraphQL call and shown on the Nexus page;
  calls stop with a clear message when the hourly budget is nearly spent.
- [x] A-24 (part) downloads are checked against Nexus' exact file size before staging.
- [x] A-17 Nexus collections: v2 GraphQL fetch (slug, URL or `nxm://…/collections/<slug>/revisions/<n>`),
  review dialog with optional members, install in the curator's load order (Premium: parallel
  downloads; free: pending list with links, matching `nxm://` downloads accepted automatically),
  enable/disable/uninstall (keeps mods shared with other collections), revision updates that
  touch only changed members. Not yet: the collection archive (curator rules, installer choices,
  bundled files) — needs an API key to download and is unverified.
- [x] A-02 Write-ahead link journal; stale links adopted; close refused once during
  game-folder jobs. Tested by interrupting a deploy before/after every journal write.
- [x] A-03 `flock` single-instance lock; atomic request spool; second start focuses.
- [x] A-25 nxm queue loss (replaced by the spool).
- [x] A-05 Delete redeploys whenever the manifest links into the mod.
- [x] A-06 Pending changes = stored registry revision vs deployed revision.
- [x] A-04 Download client without an overall timeout.
- [x] A-07 Rename-only swap; tolerant scan; updates of deleted mods fail.
- [x] A-08 Unique work/archive names; atomic name claim.
- [x] A-09 Case-insensitive game paths.
- [x] A-20 Scan before the write transaction.
- [x] A-14 Steam library detection (native/Flatpak/Snap) + first-run setup dialog;
  hardcoded path removed.
- [x] A-18 Invalid `config.toml` kept aside; database errors get a window; rescan errors shown.
- [x] A-15 Apply blocked after a game update; guided unlink → Steam verify → refresh backup.
- [x] A-26 Game folder cannot change while game-folder jobs are queued; JSON link folder
  removed.
- [x] A-22 Buttons disabled while the game runs / jobs queued; details refresh;
  "Analysing…" state.
- [x] A-10 Direct launch through the Steam Linux Runtime tool chain + Steam compat env.
- [x] A-11 Wrapper commands (`gamemoderun %command%`).
- [x] A-27 argv[0] game detection; children reaped; URL fallback keeps arguments.
- [x] A-12 nxm links require key/expiry; every download confirmed in a dialog.
- [x] A-13 Previous nxm handler remembered; foreign links forwarded; Exec escaped.
- [x] A-16 Update check follows the `file_updates` chain; per-mod failures; nxm updates
  replace.
- [x] A-23 Key redaction, query encoding, private key file, redacted `Debug`.
- [x] A-19 FIFO lane for game-folder jobs; one conflict analysis in flight.
- [x] A-24 (part) `.part` cleanup, mirror fallback, expiry check, unique names.
