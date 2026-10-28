# Pailune Priority List

This document establishes the current order of execution for open work items from [board.md](board.md) and [roadmap.md](roadmap.md).

Prioritization follows the project's guiding principle: **compatibility and blockers first, small before large (S → M → L), and proven data safety before feature expansion.**

---

## Priority Overview

| Rank | ID | Item | Size | Primary Rationale |
|:---:|:---|:---|:---:|:---|
| **P1** | **A-01** | **Format 3.1 Table Patches (Phase 1 `match` done; Phase 2 nested fields)** | **L** (phased) | Resolves active warnings and failed patch applications seen on popular mods (*Character Creator*). |
| **P1** | **A-01** | **In-Game Test Batch 1 (Tests 2–4 Verification)** | **Owner** | Completes validation of table overlay patching in game (hunting, drops, coin crates). |
| **P2** | **A-01** | **Compressed Type-1 `.pac` Meshes** | **M** | Decodes the remaining 2,914 compressed meshes in the archives for complete inspection & patching. |
| **P3** | **B-07** | **Variable-Length Save Editing (Step 4)** | **L** | Extends the verified fixed-size save editor to add/remove inventory items and dynamic lists. |
| **P3** | **D-14** | **In-Place Route for Non-Overlay Game Files** | **M** | Already built and tested (`inplace::store`), ready to wire if dialogue audio/weapon XML mods require it. |
| **P3** | **B-17** | **ASI Mod Creation: Research & Templates** | **Research / M** | Scaffolds native ASI plugin development with compiler toolchain and debug loop. |
| **P4** | **Polish** | **Pre-Release Polish: GUI Redesign & Final Code Refactoring** | **M** | Aesthetic redesign, typography, visual hierarchy, and performance/memory optimization pass. |
| **P5** | **B-08** | **Windows & macOS Real-Install Testing** | **L** | Platform validation requiring dedicated external test hardware and macOS packaging. |
| **P5** | **A-21** | **Nexus Mods API Registration** | **Admin** | Administrative prerequisite before public general release. |

---

## Detailed Action Items & Rationales

### Tier 1: Immediate & Compatibility (P1)

#### 1. A-01 — Format 3.1 Table Patches (`match` record selector & nested fields)
* **Status:** Phase 1 (`match` predicate evaluation) completed and verified; Phase 2 (nested field traversal & table rewriting) in Backlog.
* **Size:** *L* (phased).
* **Rationale:**
  * Predicate selection is now functional across tables. Phase 2 extends layout resolution to nested record arrays (`prefab_data_list[*].tribe_gender_list`) and table length rebuilds.
* **Target Areas:**
  * [`src/tables/format3.rs`](file:///home/louisgoodnews/Dokumente/pailune/src/tables/format3.rs): Nested field navigation and table re-indexing.

#### 2. Owner Task — In-Game Test Batch 1 (Tests 2–4 Verification)
* **Status:** Owner column in `board.md` (Test 1 Arrow Stacks confirmed; tests 2–4 pending).
* **Size:** Owner validation.
* **Rationale:**
  * Re-verifies in-game that the per-table archive group architecture correctly injects multi-table modifications without engine side-effects.
  * Validates hunting drops, drop rates, and crate coin rewards defined in [`local/test-mods/TESTING.md`](file:///home/louisgoodnews/Dokumente/pailune/local/test-mods/TESTING.md).

---

### Completed Features

#### B-18 — Pailune Modding SDK (Phase 16)
* **Status:** Completed & verified with automated tests.
* **Size:** *L*.
* **Implementation Summary:**
  * Project generation via CLI/MCP (`spawn_mod_project`, `export_mod_project`) and GUI modal (`SpawnProjectPrompt`).
  * Scaffolds new standalone mod projects (ASI or JSON) with metadata (`modinfo.json`), bundled libraries and build configurations (Rust `cdylib` with `Cargo.toml`, `lib.rs`, `config.ini`, `.cargo/config.toml` targeting MinGW for Windows cross-compilation; format-3 JSON `.field.json` schema), and generated Markdown documentation (`README.md`, `DEVELOPMENT.md`).
  * Integrated export pipeline (`crate::sdk::export_project`) to package projects into distribution `.zip` archives under `~/.local/share/pailune/exports` or directly stage them into Pailune.

---

### Tier 4: Engine Completeness & Utility Depth (P4)

#### 5. A-01 — Compressed Type-1 `.pac` Meshes
* **Size:** *M*.
* **Rationale:**
  * Decodes the remaining 2,914 3D mesh assets stored in compressed type-1 containers.

#### 6. B-07 (Step 4) — Variable-Length Save Editing
* **Size:** *L*.
* **Rationale:**
  * Expands the verified save editor to support adding and removing inventory items and dynamic lists with table-of-contents recalculation.

#### 7. D-14 — In-Place Route for Non-Overlay Game Files
* **Size:** *M*.
* **Rationale:**
  * Pre-built machinery (`inplace::store`) ready for non-overlay audio or socket XML mods if needed.

#### 8. B-17 — ASI Mod Scaffolding Toolkit
* **Size:** *Research / M*.
* **Rationale:**
  * Scaffolds native ASI plugin development with compiler toolchain and debug loop.

---

### Tier 5: Pre-Release Polish (P5)

#### 9. GUI Aesthetic Redesign & Pre-Release Refactoring (Phase 17)
* **Size:** *M*.
* **Rationale:**
  * Complete aesthetic review: unified visual hierarchy, refined typography, responsive pane layout, consistent iconography, and WCAG AA contrast conformance.
  * Comprehensive pre-release code refactoring, performance profiling, and memory footprint check.

---

### Tier 6: General Release & Distribution (P6)

#### 10. B-08 (Step 2) — Windows & macOS Target Verification
* **Size:** *L*.
* **Rationale:**
  * Cross-platform testing on real Windows and macOS machines, code signing, and macOS app bundling.

#### 11. A-21 — Nexus Mods API Registration
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

#### 7. D-14 — In-Place Route for Non-Overlay Files
* **Size:** *M*.
* **Rationale:**
  * Underlying machinery (`inplace::store` and `paz::pamt_edit`) is fully built and tested.
  * Remains on standby until an active mod targets non-overlay audio or socket XML assets that the game engine strictly refuses to read from archive overlays.

#### 8. B-17 — ASI Mod Creation Toolkit
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

