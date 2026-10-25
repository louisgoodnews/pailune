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

- [ ] A-32 Spec style fixes — *S*
- [ ] A-24 (rest) MD5 verification, resumable downloads — *S*
- [ ] B-06 Recoverable deletes (mod trash) — *S*
- [ ] B-01 Backup and restore (mods, profiles, mod lists, settings) — *M*
- [ ] B-02 Full settings menu — *M*
- [ ] B-03 Onboarding: guided first run + built-in guide — *M*
- [ ] A-28 (rest) Save-game backup — *M*
- [ ] A-29 (rest) AppImage — *M*

## Backlog (large)

- [ ] B-04 Evidence-based UI/UX rework — *L*
- [ ] B-05 Full documentation: vision, architecture, code, extension guides — *L*
- [ ] A-01 (rest) XML/HTML patches, new texture ids, table inserts, field-name JSON — *L*
- [ ] A-28 (rest) FOMOD-style installer scripts — *L*

## Owner

- [ ] A-21 (rest) Register Pylune with Nexus (AUP) before a public release

## Done

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
