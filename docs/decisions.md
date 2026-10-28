# Decisions

Architecture decision records: the significant choices, the alternatives, and why. New
decisions are appended with the next number; superseded ones stay and point to their successor.

## D-01 Symlinks instead of copies

**Context.** Mods must reach the game folder, and players must be able to undo that exactly.
**Decision.** Deploy every file as a symlink into staging; move shadowed originals to `.bak`.
**Why.** Undo is trivial and exact, staging stays the single source of truth, disk use does not
double, and links are easy to recognise. Proton/Wine follow symlinks.
**Trade-off.** Mods must stay in staging while deployed; tools that write into the game folder
can write through links (they then change the staged copy).

## D-02 Write-ahead link journal in SQLite

**Context.** A crash between moving an original and creating a link could lose track of files.
**Decision.** Record each link before creating it and forget it only after reverting it
(`symlink/journal.rs`, `db/links.rs`).
**Why.** Every possible on-disk state is covered by the journal, so the next unlink is exact.
Tested by interrupting a deploy after every journal write (`src/tests/crash.rs`).

## D-03 Own archive group instead of editing the game's archives

**Context.** Most content lives in PAZ archives. Other tools rewrite them in place.
**Decision.** Build Pailune's own group (`0100`) from mod files and a rebuilt `meta/0.papgt` that
lists it first; deploy both as symlinks. Verified in game: the first listed group wins.
**Why.** The game's archives stay untouched, so unlinking restores vanilla byte for byte and game
updates cannot be corrupted by modded archives.

## D-04 SQLite for state, TOML for settings

**Decision.** Settings a user may edit by hand go to `config.toml`; everything Pailune manages
(mod list, links, profiles, journal) goes to SQLite.
**Why.** TOML is readable and diffable; SQLite gives transactions, concurrent access from job
threads (WAL) and online backups. Splitting them keeps a broken hand edit from touching state.

## D-05 One exclusive lane for game-folder jobs

**Decision.** Jobs that change the game folder or Pailune's whole state run on one dedicated
thread in submission order; everything else runs on a small shared pool.
**Why.** Ordering is what users expect ("delete, then apply"), two deploys can never interleave,
and long downloads never block an Apply.

## D-06 egui for the interface

**Decision.** Immediate-mode egui/eframe.
**Why.** One native binary without a web runtime, fast to build and test (the whole app runs
headless in tests, and screens can be rendered offscreen), and simple state handling: views are
functions of `PailuneApp` that return intents.
**Trade-off.** Fewer native widgets; accessibility relies on egui's AccessKit support. The emoji
font is bundled so icons do not depend on system fonts.

## D-07 Undo instead of confirmations

**Decision.** Deleting a mod moves it to a trash with Undo; dialogs only for rare, irreversible
actions, with buttons naming the action.
**Why.** Confirmations habituate and stop protecting (Anderson et al., 2015); undo protects
without interrupting (Raskin, 2000). See [ux-principles.md](ux-principles.md) R-04, R-05.

## D-08 Network only on request

**Decision.** No telemetry, no automatic network access; Nexus calls happen when the user acts
(or enables *check updates at start*). The API key is stored with mode `0600` and redacted from
logs.
**Why.** Privacy and trust; Nexus' API acceptable use policy also asks clients to keep requests
low (rate limits are tracked per call).

## D-09 Save-game backups before every launch

**Decision.** Copy the save folder synchronously right before starting the game.
**Why.** Saves are small (a few MB); a copy taken before every session makes a broken mod
recoverable at no noticeable cost, and doing it synchronously guarantees the copy predates the
game's first write.

## D-10 Dual license MIT OR Apache-2.0

**Decision.** License the code under either license, at the user's option.
**Why.** The Rust ecosystem's convention: compatible with almost every downstream use, and the
Apache option adds an explicit patent grant.

## D-14 Mods may be written into the game's archives, with backups (accepted 2026-10-26)

**Context.** Mods packed into Pailune's own archive group had no effect in game for data tables
(and the control test is still open), while save editing (writing the game's files) and DMM's
in-place route (*Fat Stacks*) work. The owner prefers not to change the game's files but decided
to do it, provided Pailune keeps a **base backup** of the vanilla state and a **version backup**
on every change, so any state — and vanilla — can be restored easily.
**Decision.** Add an in-place deployment route for archived game files (design in
[design/in-place.md](design/in-place.md)): the game's large `.paz` archives are never rewritten;
Pailune's patched entries live in an additional archive file of the affected group (deployed as a
symlink, like every other Pailune file), and only the group's index (`0.pamt`) and `meta/0.papgt`
are rewritten. The vanilla index files are backed up once (base), every Apply that changes them
stores a version, and *Revert to vanilla* / *Roll back to version N* restore them. The overlay
route stays for content that works through it; which route a file takes is decided per file type
after the in-game control test and the JMM/DMM analysis.
**Supersedes** the "the game's own archives are never edited" goal for this route only.

## D-13 Pailune also creates mods (accepted 2026-10-26)

**Context.** The vision listed "editing mods" as a non-goal. Pailune since gained what mod
authors need most: a verified reader for the game's archives and data tables, derived table
layouts, byte-checked patching and a JSON editor. The owner asked for mod creation.
**Decision.** Pailune is a mod manager *and* a modding tool. Creation starts with JSON mods
(B-16, built on the table reader and layouts), then ASI plugins (B-17, tooling around the
author's own code). The installed game stays read-only; created mods are ordinary staged mods
and deploy through the same journaled path.
**Limits.** No analysis of the game's protected code (unchanged); no bundling of game data or
third-party schemas into created mods beyond the changed bytes.
**Supersedes** the "Editing mods" non-goal in [vision.md](vision.md).

## D-12 No script runtime in Pailune (accepted 2026-10-26)

**Context.** B-13 asked whether scripts can be injected into the running game. The archives
contain no script files; native mods hook functions by pattern scanning via ASI plugins and
break with patches ([research/script-bridge.md](research/script-bridge.md)).
**Decision.** Pailune does not embed a script runtime; it manages ASI loader and plugins well
(install, status, update warnings, INI settings) and extends data patching instead.

## D-11 Deployment per platform

**Context.** Windows and macOS became goals (2026-10-26). D-01 relies on Unix symlinks; on
Windows they need Developer Mode or administrator rights.
**Decision (2026-10-26).** One journaled deployment model; the platform layer places each file
as a symlink where possible, else (Windows) a hard link, else a copy, and reports which.
Symlinks prove ownership by their target. Hard links and copies get a **stamp** (size and
modification time) journaled right after placing; unlink removes them only while the stamp
still matches, so files changed by the game or the user are never deleted. If a run stopped
between placing and stamping, identical content to the staged file counts as ours — except
when the planned `.bak` rename had not happened yet, since the file is then the original.
**Verified.** Forced copies and hard links on Linux: exact revert, changed copies left alone,
and the crash test interrupts at every journal write for symlinks and copies.
**Trade-off.** Copies use disk space and do not follow later staging changes until the next
Apply (which redeploys anyway).

## D-15 Tooling for native ASI plugin development (accepted 2026-10-28)

**Context.** B-17 asks for native ASI mod creation tooling. Game mechanics not controlled by data tables are hardcoded into the executable. Authors hook these functions via ASI proxy DLL loaders (`winmm.dll`, etc.). Developing plugins on Linux for Windows x64 requires cross-compilation toolchains, loader synchronization discipline, and log visibility.
**Decision.** Pailune provides complete native mod creation tooling: project scaffolding (Rust `cdylib` and C++ MinGW with static CRT linking and pattern scanning harnesses), background build runners targeting `x86_64-pc-windows-gnu`, staging directly to `bin64/`, one-click deploy/apply, live log monitoring from `bin64/<name>.log`, and release packaging. Pailune remains strictly a tool harness: it never analyzes, disassembles, or modifies protected executable code.

## D-16 Visual Node-Based Mod Creator (accepted 2026-10-28)

**Context.** Format-3 semantic patching (`.field.json`) enables high-level gameplay customization (stack sizes, drop rates, rewards) without calculating raw byte offsets. However, hand-authoring JSON requires knowing internal schemas, field types, and hex IDs. Non-technical users require a visual, no-code authoring workflow to create mods safely.
**Decision.** Embed a node graph editor directly into Pailune (`Tab::Creator`) built with immediate-mode egui. Graphs are constructed with typed sockets (`ItemRef`, `DropTableRef`, `Number`, `Boolean`, `Text`), validated via topological sorting with cycle detection, and compiled deterministically into Format-3 table patches (`iteminfo.pabgb`, `dropsetinfo.pabgb`, `wantedinfo.pabgb`). Projects persist graph state (`graph.json`) alongside `modinfo.json` in the mod's staging folder so projects remain visually editable, exportable to distributable ZIPs, and deployable with one click.
