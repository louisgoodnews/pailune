# Pailune Priority List

This document establishes the current order of execution for open work items from [board.md](board.md) and [roadmap.md](roadmap.md).

Prioritization follows the project's guiding principle: **compatibility and blockers first, small before large (S → M → L), and proven data safety before feature expansion.**

---

## Priority Overview

| Rank | ID | Item | Size | Primary Rationale |
|:---:|:---|:---|:---:|:---|
| **P1** | **Visual-1** | **Visual Mod Creator Phase 1: Core Node Graph Engine & Compiler** | **M** | Data structures, graph JSON schema, and compiler targeting format-3 table patches & XML descriptors. |
| **P2** | **Visual-2** | **Visual Mod Creator Phase 2: Interactive Node Canvas GUI in egui** | **M** | Node graph canvas in egui, draggable nodes, color-coded sockets, bezier wires, live diff preview. |
| **Owner** | **A-21** | **Nexus Mods API Registration** | **Admin** | Administrative prerequisite before public general release (Nexus Mods API Acceptable Use Policy). |
| *Deferred* | *B-08* | *Windows & macOS Target Testing* | *L* | Optional / deferred until dedicated Windows/macOS test hardware is available. |

---

## Detailed Action Items & Rationales

### Visual Modding Studio (Roadmap Phase 19)

#### Visual-1 — Core Node Graph Engine & Compiler
* **Size:** *M*.
* **Rationale:**
  * Define visual graph models (`Graph`, `Node`, `Socket`, `Connection`), JSON serialization, and deterministic compiler transforming visual rules into verified Format-3.1 table patches (`.field.json`) and XML merge descriptors.
  * Designed in [`docs/design/visual-mod-creator.md`](file:///home/louisgoodnews/Dokumente/pailune/docs/design/visual-mod-creator.md).

#### Visual-2 — Interactive Node Canvas GUI in egui
* **Size:** *M*.
* **Rationale:**
  * Visual node graph interface with pannable/zoomable canvas, draggable nodes, typed socket connections, real-time live diff against the Game Data Browser, and one-click templates (*Mega Stacks*, *Loot Bounty*, *Crate Rewards*).

---

## Completed Features

#### Linux Release Packaging (AppImage & Tarball)
* **Status:** Completed & verified.
* **Size:** *S*.
* **Implementation Summary:**
  * Created automated Linux release distribution pipeline in [`packaging/dist.sh`](file:///home/louisgoodnews/Dokumente/pailune/packaging/dist.sh).
  * Generates stripped release distribution tarball (`target/dist/pailune-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`) containing binary, licenses, README, and desktop assets, along with `SHA256SUMS`.
  * Updated [`packaging/appimage.sh`](file:///home/louisgoodnews/Dokumente/pailune/packaging/appimage.sh) to stage `Pailune.AppDir` with 256x256 high-resolution icon conversion via `rsvg-convert` and `.DirIcon` link, and builds standalone `.AppImage` when `appimagetool` is available.
  * Enhanced [`packaging/install.sh`](file:///home/louisgoodnews/Dokumente/pailune/packaging/install.sh) with dual-mode installation (installs from either release tarball or git repo to `~/.local/bin`, `~/.local/share/applications`, and `~/.local/share/icons`).
  * Unified packaging assets under Pailune naming: [`packaging/pailune.desktop`](file:///home/louisgoodnews/Dokumente/pailune/packaging/pailune.desktop) with `MimeType=x-scheme-handler/nxm;`, [`packaging/pailune.svg`](file:///home/louisgoodnews/Dokumente/pailune/packaging/pailune.svg), and [`packaging/pailune.appdata.xml`](file:///home/louisgoodnews/Dokumente/pailune/packaging/pailune.appdata.xml).

#### A-01 — Table Schema Expansion (DropSetInfo)
* **Status:** Completed & verified with automated tests.
* **Size:** *M*.
* **Implementation Summary:**
  * Added `DropSetInfo` (`dropsetinfo`) to shipped [`data/table-layouts.json`](file:///home/louisgoodnews/Dokumente/pailune/data/table-layouts.json) with verified 14-byte header (`_isBlocked`, `_headerPadding`) and 64-byte `_dropInfoList` struct list.
  * Mapped proven subfields: `_entryType` (u8), `_itemKey` (u32 ref), `_dropRate` (u32, 1,000,000 = 100%), `_minDropCount` (u64), and `_maxDropCount` (u64).
  * Implemented structured `list:struct` and `struct` decoding in [`src/tables/value.rs`](file:///home/louisgoodnews/Dokumente/pailune/src/tables/value.rs) for human-readable JSON output in the Game Data Browser.
  * Verified format-3 intent resolution (both indexed `_dropInfoList[0]` and wildcard `_dropInfoList[*]`) in [`src/tables/tests.rs`](file:///home/louisgoodnews/Dokumente/pailune/src/tables/tests.rs).

#### Phase 17 — Pre-Release Polish: GUI Redesign & Final Code Refactoring
* **Status:** Completed & verified with automated tests.
* **Size:** *M*.
* **Implementation Summary:**
  * Modernized visual widget hierarchy, typography, and theme styling in `src/ui/theme.rs`.
  * Refined badges with subtle tint fill, responsive border, and softened 5px corners while maintaining full WCAG AA contrast compliance (ratio >= 4.5:1).
  * Added `card_frame` container with palette-aware background and borders, unified across Collections, Profiles, and Data Browser panels.
  * Cleared all compiler warnings (annotated public API list item editing methods in `src/savefile/edit.rs` and cleaned unused imports).
  * Updated deployment summary text in `src/jobs/tasks/deploy.rs` to accurately reflect archive overlay and in-place routing rather than falsely claiming files were skipped.
  * Updated archive file descriptions in `src/ui/views/mod_details.rs`.
  * 100% test pass rate across 176 automated tests.

#### B-17 — ASI Mod Creation: Research & Templates
* **Status:** Completed & verified with automated tests.
* **Size:** *Research / M*.
* **Implementation Summary:**
  * Produced architecture research ([`docs/research/asi-mods.md`](file:///home/louisgoodnews/Dokumente/pailune/docs/research/asi-mods.md)) and architecture decision record ([`D-15`](file:///home/louisgoodnews/Dokumente/pailune/docs/decisions.md)) covering the Ultimate ASI Loader proxy DLL contract, threading rules avoiding Windows loader locks, pattern scanning, and MinGW static CRT linking for cross-compiling Windows x64 DLLs from Linux.
  * Extended SDK project generator to support both Rust `cdylib` and C++ MinGW projects (`Makefile`, `src/main.cpp`) with built-in IDA-style byte pattern scanning (`find_pattern` / `FindPattern`) and memory patch helpers (`patch_memory` / `PatchMemory`).
  * Implemented build runner (`crate::sdk::build_project`) and staging pipeline (`crate::sdk::stage_built_project`) executing native builds and staging directly into `~/.local/share/pailune/staging/<name>/bin64/`.
  * Implemented debug loop log reader (`crate::sdk::read_plugin_log`) tailing `<name>.log` from `bin64/`.
  * Exposed CLI commands (`build-project`, `plugin-log`) and MCP tools (`build_mod_project`, `read_plugin_log`).

#### A-01 — In-Game Test Batch 1 (Tests 1–4 Verification)
* **Status:** Completed & confirmed in-game.
* **Size:** Owner validation.
* **Implementation Summary:**
  * In-game verification confirmed that table overlay patching operates accurately for arrow stacks (`ItemInfo._maxStackCount`), hunting amounts, drop rates, and crate coin rewards without unintended side effects.

#### D-14 — In-Place Route for Non-Overlay Game Files
* **Status:** Completed & verified with automated tests.
* **Size:** *M*.
* **Implementation Summary:**
  * Implemented classifier (`is_inplace_candidate` in `src/inplace/classify.rs`) for dialogue audio (`.wem`, `.bnk`, `.pck`, `sound/`, `audio/`) and weapon/socket placement XML (`gamedata/item/socket/*.xml`, `gamedata/character/weapon/*.xml`).
  * Enforced strict boundary keeping data tables (`*.pabgb`, `*.pabgh`, `*.paseq`, etc.) on the overlay route.
  * Implemented in-place build pipeline (`src/inplace/build.rs`): packs entries into 16-byte aligned entries, stores content-addressed `<sha256>.paz` in `game-backup/archives/`, links as `<group>/<k>.paz`, updates `0.pamt` with `pamt_edit::redirect`, and updates `pamt_crc` in `0.papgt`.
  * Wired full backup lifecycle: base snapshot (0) taken once per game build, version snapshots (1..N) recorded on each apply, and atomic reversion to pristine vanilla on `unlink_all` and `clean_slate`.
  * Integrated seamlessly with deployment pipeline in `src/jobs/tasks/deploy.rs`, supporting mixed in-place and overlay deployments.

#### B-07 (Step 4) — Variable-Length Save Editing
* **Status:** Completed & verified with automated tests.
* **Size:** *L*.
* **Implementation Summary:**
  * Extended `src/savefile/edit.rs` to support arbitrary variable-length edits, dynamic list item modifications (deletion, duplication, addition for kinds 6/7), and counted items (kinds 1/3).
  * Implemented automatic recalculation of enclosing embedded object `size: u32` trailers, top-level TOC entry offsets and sizes, TOC header `stream_size`, and container header (`uncompressed`, `payload`, fresh nonce, and HMAC-SHA256).
  * Added UI list controls in `src/ui/views/save_view.rs` (Duplicate and Delete actions on list items) integrated into the staging and confirmation pipeline.
  * Verified with unit and UI integration tests (`variable_length_edit_shifts_subsequent_objects_and_toc`, `list_item_operations_delete_duplicate_add`, and UI action flow).

#### A-01 — Format 3.1 Table Patches (Phase 2 Nested Fields & Table Rewriting)
* **Status:** Completed & verified with automated tests.
* **Size:** *L* (phased).
* **Implementation Summary:**
  * Added recursive nested field path parsing and resolution (`resolve_target_spans`) supporting property traversal (`parent.child`), array indexing (`list[0].field`), and wildcards (`list[*].field`).
  * Supported variable length field encoding in `super::value::encode` including empty arrays (`new: []` -> `[0, 0, 0, 0]`) and typed lists.
  * Implemented table header/index offset rewriting (`crate::patch::table::rewrite_index`) and wired it into `apply_over` (`ApplyReport.rewritten_header`) and `overlay/patches.rs` so subsequent record offsets shift accurately and the companion header index is packed into the overlay group.
  * Added partial layout heuristic search in `filter_by_matches` for unmapped fields (e.g. `equip_type_info`).

#### A-01 — Compressed Type-1 `.pac` Meshes
* **Status:** Completed & verified with automated tests against local game files.
* **Size:** *M*.
* **Implementation Summary:**
  * Implemented full decoding in `src/paz/pac.rs` (`extract::decode`).
  * Parses the 80-byte `.pac` file header (`PAR ` magic, version, timestamp, up to 8 section size pairs `(stored_size, orig_size)`).
  * Decompresses each compressed section with LZ4 block decompression (`lz4_flex::block::decompress`), updates `stored_size = orig_size` in the decoded header, and outputs the clean uncompressed `.pac` mesh format.

#### B-18 — Pailune Modding SDK (Phase 16)
* **Status:** Completed & verified with automated tests.
* **Size:** *L*.
* **Implementation Summary:**
  * Project generation via CLI/MCP (`spawn_mod_project`, `export_mod_project`) and GUI modal (`SpawnProjectPrompt`).
  * Scaffolds new standalone mod projects (ASI or JSON) with metadata (`modinfo.json`), bundled libraries and build configurations (Rust `cdylib` with `Cargo.toml`, `lib.rs`, `config.ini`, `.cargo/config.toml` targeting MinGW for Windows cross-compilation; format-3 JSON `.field.json` schema), and generated Markdown documentation (`README.md`, `DEVELOPMENT.md`).
  * Integrated export pipeline (`crate::sdk::export_project`) to package projects into distribution `.zip` archives under `~/.local/share/pailune/exports` or directly stage them into Pailune.

---

### Tier 5: General Release & Distribution (P1–P2)

#### 1. B-08 (Step 2) — Windows & macOS Target Verification (P1)
* **Size:** *L*.
* **Rationale:**
  * Cross-platform testing on real Windows and macOS machines, code signing, and macOS app bundling.

#### 2. A-21 — Nexus Mods API Registration (P2)
* **Size:** *Admin*.
* **Rationale:**
  * Formal API agreement with Nexus Mods prior to public general release.
* **Target Areas:**
  * [`src/tables/format3.rs`](file:///home/louisgoodnews/Dokumente/pailune/src/tables/format3.rs): Implement predicate matching logic for `Intent::matches`.
  * [`src/tables/mod.rs`](file:///home/louisgoodnews/Dokumente/pailune/src/tables/mod.rs): Extend layout resolution to evaluate sub-records and nested array offsets.

#### 2. Owner Task — In-Game Test Batch 1 (Tests 2–4 Verification)
* **Status:** Owner column in `board.md` (Test 1 Arrow Stacks confirmed; tests 2–4 pending).
* **Size:** Owner validation.
* **Rationale:**
  * Re-verifies in-game that the per-table archive group architecture correctly injects multi-table modifications without engine side-effects.
  * Validates hunting drops, drop rates, and crate coin rewards defined in [`local/test-mods/TESTING.md`](file:///home/louisgoodnews/Dokumente/pailune/local/test-mods/TESTING.md).

---

### Tier 2: Strategic Feature Driver — Phase 15 Modding Tools (P2)

#### 3. B-16 (Steps 1 & 2) — New JSON Mod Wizard & Game Data Browser
* **Status:** Roadmap Phase 15.
* **Size:** *S* (Wizard) + *M* (Data Browser).
* **Rationale:**
  * Pailune already possesses deep archive introspection, class layout definitions (`data/table-layouts.json`), and binary decoders.
  * Exposing a read-only browser for game data tables allows creators to explore vanilla game variables without external unpackers, transforming Pailune from a mod manager into an integrated workbench.
* **Target Areas:**
  * [`src/ui/views/`](file:///home/louisgoodnews/Dokumente/pailune/src/ui/views/): New browser view displaying records, field names, types, and formatted values.
  * [`src/tables/`](file:///home/louisgoodnews/Dokumente/pailune/src/tables/): High-level table reader API suitable for UI pagination and searching.

#### 4. B-16 (Steps 3 & 4) — Change Creator & Game Dry-Run Verification
* **Status:** Roadmap Phase 15.
* **Size:** *S–M*.
* **Rationale:**
  * Allows a user to click a field in the data browser, input a new value, and automatically generate a verified patch intent with the correct original bytes.
  * Reuses Pailune's safety verification engine to guarantee that generated patches never conflict or fail on the user's game version.

---

### Tier 3: Engine Completeness & Utility Depth (P3)

#### 5. A-01 — Compressed Type-1 `.pac` Meshes
* **Size:** *M*.
* **Rationale:**
  * Approximately 2,914 3D meshes in game archives use a type-1 compression container.
  * Decoding these meshes completes the archive extraction engine and enables future model/mesh swapping and inspection tools.
* **Target Areas:**
  * [`src/paz/`](file:///home/louisgoodnews/Dokumente/pailune/src/paz/): Add decompression routines for type-1 mesh payloads.

#### 6. B-07 (Step 4) — Variable-Length Save Editing
* **Size:** *L*.
* **Rationale:**
  * Step 3 (fixed-size value editing like money/experience) is complete and verified in memory on all saves.
  * Full save editing requires modifying array sizes and inserting/deleting records, which demands a body writer capable of updating chunk lengths and the container table of contents.
* **Target Areas:**
  * [`src/savefile/`](file:///home/louisgoodnews/Dokumente/pailune/src/savefile/): Body writer and schema recalculator.

---

### Tier 4: Contingent & Subsequent Projects (P4)

#### 4. B-17 — ASI Mod Creation Toolkit
* **Size:** *Research / L*.
* **Rationale:**
  * Follows Phase 15 JSON mod creation. Scaffolds C++/Rust native code projects with MinGW toolchains, loader hooking contracts, and integrated log viewing.

---

### Tier 5: Distribution & Release (P5)

#### 9. B-08 (Step 2) — Windows & macOS Target Verification
* **Size:** *L*.
* **Rationale:**
  * Cross-platform traits and headless architecture are in place. Real-world execution requires physical Windows and macOS testbeds, codesigning, and `.app` bundle packaging.

#### 10. A-21 — Nexus Mods API Registration
* **Size:** *Admin*.
* **Rationale:**
  * Formal API agreement with Nexus Mods prior to a wider public release.

