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
    unsupported), inserts into tables, XML/HTML/CSS patch files, new texture ids,
    pack-level conflict reporting.

## Next

- [ ] A-21 Rate limits + AUP registration — *Phase 5*
- [ ] A-24 Download size/MD5 verification, resume — *Phase 2*

## Backlog

- [ ] A-28 Search, drag-and-drop, shortcuts, variants, save backup, profile export — *Phase 7*
- [ ] A-29 Packaging, CI, icon, LICENSE — *Phase 7*
- [ ] A-30 Log retention — *Phase 7*
- [ ] A-31 Test coverage (UI actions, mocked Nexus HTTP) — *Phase 7*
- [ ] A-32 Spec style fixes — *Phase 7*
- [ ] A-33 Small-window layout — *Phase 7*

## Done

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
