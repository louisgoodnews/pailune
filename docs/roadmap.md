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

## Phase 15 — Pylune as a modding tool (D-13)

Mods are created as ordinary staged mods, so everything Pylune does for installed mods (Apply,
undo, conflicts, logs) works for them while they are being made.

### B-16 JSON mod creation (first)

1. **New JSON mod** — *Create mod* asks for name, author, version and description and starts an
   empty mod in staging; it opens in the JSON editor. *(S)*
2. **Data browser** — read-only view of the game's data tables from the archives: search
   records by name or key, show fields with values where the layout is verified, raw bytes
   otherwise. *(M)*
3. **Change from the browser** — pick a field (or select bytes) and enter a new value; Pylune adds
   the change with the original bytes filled in (format 2 `entry` + `rel_offset`, or format 3
   `set` where the field is proven). *(M)*
4. **Check against the game** — dry run on the installed version: every change found, original
   bytes match, no overlaps; shown per change in the editor. *(S–M)*
5. **Test loop** — enable, Apply and launch from the editor; after a game update, re-check and
   offer to rebase offsets by record name. *(M)*
6. **Package** — zip with `modinfo`, readme and version for Nexus upload; version bump. *(S)*

### B-17 ASI mod creation (second)

1. **Research and decision record** — language and toolchain (Rust `cdylib` or C++ via MinGW for
   `x86_64-pc-windows-gnu`, built on Linux), the ASI loader contract, logging, what a template
   can offer without knowledge of the game's code. *(Research)*
2. **Templates** — *Create ASI plugin* scaffolds a project (entry point, INI settings, log file)
   outside the game folder. *(M)*
3. **Build and deploy** — build from Pylune, stage the result as a mod, Apply, launch. *(M)*
4. **Debug loop** — show the plugin's log, crash and loader messages in Pylune; rebuild and
   redeploy with one action. *(M)*
5. **Package** — DLL/ASI, INI and readme as an installable mod. *(S)*

## Owner tasks

- A-21 Register Pylune with Nexus Mods (API acceptable use policy) before a public release.
- Real-world tests: a JSON mod (e.g. MEGA STACKS) and a collection install.
