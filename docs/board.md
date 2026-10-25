# Pylune board

Work items from [audit.md](audit.md), scheduled by [roadmap.md](roadmap.md).
Move a line between columns when its status changes.

## In progress

- [ ] A-24 Download hygiene — *Phase 2* — done: `.part` cleanup, mirror fallback, expiry
  check, unique names. Open: size/MD5 verification, resume.
- [ ] A-23 Secret redaction — *Phase 5* — done: `Client` Debug redacts the key.
  Open: redact `nxm` keys in errors, enforce key file mode.

## Next

- [ ] A-01 PAZ/PAPGT pipeline for JSON, RAW and overlay mods — *Phase 3*
  (meanwhile: "unsupported format" badges)
- [ ] A-14 Steam library detection + first-run wizard — *Phase 4*
- [ ] A-15 Game update detection and recovery flow — *Phase 4*
- [ ] A-18 Visible startup errors, config protection — *Phase 4*

## Backlog

- [ ] A-26 Config snapshot safety, JSON folder validation — *Phase 4*
- [ ] A-12 nxm confirmation dialog — *Phase 5*
- [ ] A-13 nxm handler forwarding + Exec escaping — *Phase 5*
- [ ] A-16 File-chain update check, per-mod results, nxm replace — *Phase 5*
- [ ] A-17 Nexus collections — *Phase 5*
- [ ] A-21 Rate limits + AUP registration — *Phase 5*
- [ ] A-10 Steam Linux Runtime / Steam-managed launch — *Phase 6*
- [ ] A-11 Wrapper commands in launch options — *Phase 6*
- [ ] A-27 Game detection, xdg-open args, zombie reaping — *Phase 6*
- [ ] A-19 FIFO exclusive lane, debounced conflict analysis — *Phase 7*
- [ ] A-22 Stale UI, button enablement — *Phase 7*
- [ ] A-28 Search, drag-and-drop, shortcuts, variants, save backup, profile export — *Phase 7*
- [ ] A-29 Packaging, CI, icon, LICENSE — *Phase 7*
- [ ] A-30 Log retention — *Phase 7*
- [ ] A-31 Test coverage — *Phase 7*
- [ ] A-32 Spec style fixes — *Phase 7*
- [ ] A-33 Small-window layout — *Phase 7*

## Done

- [x] A-02 Write-ahead link journal (`symlink/journal.rs`, `db::links::DbJournal`); stale
  links into staging are adopted; window close is refused once while a game-folder job runs.
  Tested by interrupting a deploy before and after every journal write.
- [x] A-03 `flock` single-instance lock; atomic spool for hand-off; a second start focuses
  the window.
- [x] A-25 nxm queue loss (replaced by the spool).
- [x] A-05 Delete redeploys whenever the manifest links into the mod.
- [x] A-06 Pending changes = stored registry revision vs deployed revision (survives
  restarts, set by Unlink, bumped by installs).
- [x] A-04 Separate download client without an overall timeout (stall limit 60 s).
- [x] A-07 Rename-only swap for updates; per-mod tolerant scan; an update of a deleted mod
  fails instead of resurrecting it.
- [x] A-08 Unique work and archive names; atomic claim of the staging name; no import from
  inside staging.
- [x] A-09 Case-insensitive resolution against existing game folders.
- [x] A-20 Staging scan runs before the write transaction.
