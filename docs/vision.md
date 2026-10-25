# Vision

## Why Pylune exists

*Crimson Desert* runs on Linux through Steam and Proton, but its mod tooling targets Windows:
managers copy files into the game folder, edit the game's archives in place, or need a Windows
runtime of their own. On Linux that means broken installs after a game update, no clean way back
to an unmodded game, and fiddly launch options for script loaders.

Pylune is a native Linux mod manager for this one game. It aims to make modding **safe to try
and easy to undo**.

## Who it is for

Players on Linux, often on a Steam Deck or a desktop distribution, who are comfortable
downloading mods but are not experts in archives, symlinks or Proton. They want to play soon,
and they care a great deal about not breaking their game or their saves.

## Goals

1. **Nothing irreversible.** The game folder only ever receives symlinks; originals are kept as
   `.bak`; every link is journaled before it is made, so *Unlink all* restores the game exactly,
   even after a crash. The game's own archives are never edited.
2. **Understandable.** One obvious path (add → enable → apply → play), plain language, a guide
   that teaches by doing, and visible state ("Changes not applied").
3. **Complete for this game.** Loose files, ASI/DLL plugins, files inside the game's PAZ
   archives (via Pylune's own archive group), textures, byte-patch JSON and CSS patches, mods
   with options, Nexus Mods downloads, updates and collections.
4. **Native and light.** A single Rust binary with an egui interface; no Windows runtime, no
   background service, no telemetry. Network access only for Nexus features the user starts.
5. **Recoverable everywhere.** Undo for deletes, daily backups of Pylune's state, save-game
   backups before every launch.

## Non-goals

- Other games. Formats, paths and defaults are specific to *Crimson Desert*; generality would
  cost clarity. (The archive and overlay code is separable should that ever change.)
- Windows or macOS builds.
- Editing mods. Pylune installs and orders them; making them is a job for other tools.
- Bypassing Nexus Mods' download rules (Premium vs. free accounts are respected).

## Principles in practice

- **Safety over speed.** Game-folder changes run one at a time in submission order; Apply is
  blocked after a game update until the user has followed the recovery steps.
- **Evidence-based interface.** Design rules come from HCI, psychology and pedagogy research and
  are written down in [ux-principles.md](ux-principles.md) with references.
- **Small, readable code.** Files under 250 lines, data separated from logic, enums and `match`
  instead of nested conditionals, rustdoc on everything. See [extending.md](extending.md).

## Where it is going

Open work is tracked in [board.md](board.md) and ordered in [roadmap.md](roadmap.md). The large
remaining items are more patch formats inside the archives (XML/HTML patches, new texture ids,
table inserts, field-name JSON) and installer scripts for mods with many options.
