# Pylune roadmap

Derived from [audit.md](audit.md) (IDs `A-xx`). Phases are ordered by risk: data safety
first, then making mods actually work in-game, then convenience. Day-to-day status lives in
[board.md](board.md).

## Phase 1 — Data safety *(must precede any real use)*

Goal: whatever happens (crash, close, second instance), Pylune can always revert exactly what it
deployed.

- A-02 Write-ahead link journal; symlinks into staging are always Pylune's own; block window
  close while exclusive jobs run.
- A-03 Process-wide `flock` single-instance lock; atomic spool for `nxm://` hand-off (also A-25).
- A-05 Delete redeploys whenever the manifest links into the mod.
- A-06 Pending changes = stored registry revision vs deployed revision.

**Exit criteria:** killing Pylune at any point of Apply/Unlink/Clean Slate leaves a state that
the next Unlink fully reverts (tested).

## Phase 2 — Reliable staging & downloads

- A-04 Download client without a global timeout.
- A-07 Rename-only folder swap; per-mod tolerant scan.
- A-08 Unique work, `.part` and staging names; no-replace rename.
- A-09 Case-insensitive path resolution against the game directory.
- A-20 Scan outside the write transaction.
- A-24 `.part` cleanup, mirror fallback, expiry check, size check.

## Phase 3 — Mods that work in-game (PAZ pipeline)

- A-01 Reverse-engineer and implement PAMT/PAZ reading (compression, encryption),
  PAPGT/PATHC generation, a Pylune overlay group, JSON patch application and RAW file
  packing. Verify against DMM output on a copy of the game files, never the live install.
- Until done: show "unsupported format" badges instead of silently skipping files.

## Phase 4 — Game lifecycle & setup

- A-14 Steam library detection and a first-run wizard.
- A-15 Game update detection and a guided recovery flow.
- A-18 Visible startup errors and config protection; A-26 config snapshot safety.

## Phase 5 — Nexus done right

- A-12 `nxm` confirmation; A-13 handler forwarding; A-16 file-chain update check.
- A-21 Rate limits and AUP registration; A-23 secret redaction.
- A-17 **Collections** (design in [audit.md](audit.md#nexus-collections)).

## Phase 6 — Launch

- A-10 Steam Linux Runtime / Steam-managed launch; A-11 wrapper commands; A-27 detection
  and zombies.

## Phase 7 — UX, packaging, quality

- A-19 FIFO exclusive lane and debounced analysis; A-22 stale UI and button enablement.
- A-28 Search, drag-and-drop, shortcuts, variant picker, save backup, profile export.
- A-29 Packaging and CI; A-30 log retention; A-31 tests; A-32 spec style; A-33 layout.
