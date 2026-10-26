# Pylune roadmap

Derived from [audit.md](audit.md) (IDs `A-xx`, owner requests `B-xx`). Since 2026-10-25 work is
ordered **small before large**: quick, independent items first, then medium features, then
the large reworks that build on them. Day-to-day status lives in [board.md](board.md).

## Completed phases

1. **Data safety** — write-ahead link journal, single-instance lock, exact reverts (A-02…A-06).
2. **Staging & downloads** — atomic staging, download client, case-insensitive paths (A-04…A-09, A-20).
3. **Mods in the archives** — reader, overlay (verified in game), textures, byte-patch JSON,
   CSS patches, archive conflicts (A-01 core).
4. **Game lifecycle** — Steam detection, setup dialog, update recovery (A-14, A-15, A-18, A-26).
5. **Nexus** — hardened `nxm`, handler forwarding, update chain, collections, rate limits (A-12…A-17, A-21, A-23).
6. **Launch** — Steam Linux Runtime chain, wrappers, detection (A-10, A-11, A-27).
7. **Quality** — job lanes, UI tests, packaging, CI, license, logs (A-19, A-22, A-29…A-31, A-33).

8. **Small items** — style fixes, MD5 + resumable downloads, recoverable deletes (A-32, A-24, B-06).
9. **Medium features** — backups, settings menu, onboarding, save-game backups, AppImage
   (B-01, B-02, B-03, A-28 part, A-29).
10. **Reworks** — evidence-based UI/UX rework, full documentation (B-04, B-05).

## Phase 11 — Project hygiene (S)

- B-09 `CONTRIBUTING.md`.
- B-10 `CODE_OF_CONDUCT.md`.
- B-15 Move a mod to a load-order number.
- B-11 Issue and pull request templates.

## Phase 12 — Medium features (M)

- B-12 Portable mod lists: export/import with variants, Nexus origins and collections;
  offer to download what is missing.
- B-14 Headless core + CLI, then an MCP server (`pylune mcp`) on top of it.
- B-08 (step 1) Platform layer: move Linux-specific code behind a `Platform` trait.

## Phase 13 — Research

- B-07 Save format: read-only decoding and documentation (`docs/save-format.md`).
- B-13 Script bridge: report and decision record before any code.

## Phase 14 — Remaining large items (L)

- A-01 rest: XML/HTML patches, new texture ids, table inserts, field-name JSON (format 3).
- A-28 FOMOD-style installer scripts.
- B-07 (step 2) Save viewer, then editing once re-encoding is proven byte-identical.
- B-08 (step 2) Windows and macOS support (deployment strategy per platform, packaging).
- B-13 (step 2) Bridge installation, if the research says it is viable.

## Owner tasks

- A-21 Register Pylune with Nexus Mods (API acceptable use policy) before a public release.
- Real-world tests: a JSON mod (e.g. MEGA STACKS) and a collection install.
