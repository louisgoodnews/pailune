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
**Decision.** Build Pylune's own group (`0100`) from mod files and a rebuilt `meta/0.papgt` that
lists it first; deploy both as symlinks. Verified in game: the first listed group wins.
**Why.** The game's archives stay untouched, so unlinking restores vanilla byte for byte and game
updates cannot be corrupted by modded archives.

## D-04 SQLite for state, TOML for settings

**Decision.** Settings a user may edit by hand go to `config.toml`; everything Pylune manages
(mod list, links, profiles, journal) goes to SQLite.
**Why.** TOML is readable and diffable; SQLite gives transactions, concurrent access from job
threads (WAL) and online backups. Splitting them keeps a broken hand edit from touching state.

## D-05 One exclusive lane for game-folder jobs

**Decision.** Jobs that change the game folder or Pylune's whole state run on one dedicated
thread in submission order; everything else runs on a small shared pool.
**Why.** Ordering is what users expect ("delete, then apply"), two deploys can never interleave,
and long downloads never block an Apply.

## D-06 egui for the interface

**Decision.** Immediate-mode egui/eframe.
**Why.** One native binary without a web runtime, fast to build and test (the whole app runs
headless in tests, and screens can be rendered offscreen), and simple state handling: views are
functions of `PyluneApp` that return intents.
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

## D-11 Deployment per platform (open)

**Context.** Windows and macOS became goals (2026-10-26). D-01 relies on Unix symlinks; on
Windows they need Developer Mode or administrator rights.
**Direction.** Keep one journaled deployment model with a per-platform method: symlinks on Linux
and macOS; on Windows symlinks when permitted, otherwise hard links (same volume) or verified
copies. All platform specifics sit behind a `Platform` trait (B-08 step 1). To be finalised with
the Windows port.
