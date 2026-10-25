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

## Phase 8 — Small items (S)

- A-32 Spec style fixes (rustdoc gaps, deep nesting, `match` on bool).
- A-24 MD5 verification and resumable downloads.
- B-06 Recoverable deletes (mod trash).

## Phase 9 — Medium features (M)

- B-01 Backup and restore (manual and automatic).
- B-02 Full settings menu (categories, validation, reset).
- B-03 Onboarding: guided first run and built-in guide.
- A-28 Save-game backup.
- A-29 AppImage packaging.

## Phase 10 — Large reworks (L)

- B-04 Evidence-based UI/UX rework (principles in `ux-principles.md`).
- B-05 Full documentation (vision, architecture, extension guides, decisions).
- A-01 rest: XML/HTML patches, new texture ids, table inserts, field-name JSON (format 3).
- A-28 FOMOD-style installer scripts.

## Owner tasks

- A-21 Register Pylune with Nexus Mods (API acceptable use policy) before a public release.
- Real-world tests: a JSON mod (e.g. MEGA STACKS) and a collection install.
