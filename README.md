# Pylune

Native Linux mod manager for **Crimson Desert** (Steam / Proton), written in Rust with egui.

## First start

Pylune finds Crimson Desert through Steam's library configuration. If that fails, a setup
dialog asks for the game folder. After a game update, a banner guides you through
unlink → verify files in Steam → refresh backup before mods can be applied again.

## Where things live

| What | Path |
|------|------|
| Settings | `~/.config/pylune/config.toml` |
| Application state (mods, load order, links, profiles, event journal) | `~/.local/share/pylune/pylune.sqlite3` |
| Staged mods | `~/.local/share/pylune/staging/<mod>/` |
| Application log (daily) | `~/.local/share/pylune/logs/pylune.log.YYYY-MM-DD` |
| Per-mod log (installs, updates, deployments, file lists) | `~/.local/share/pylune/logs/mods/<mod>.log` |
| Save-game backups | `~/.local/share/pylune/save-backups/saves-<time>-….zip` |
| Nexus API key (mode `0600`) | `~/.local/share/pylune/nexus_api_key` |

## How it works

- **Staging:** mods (folders, `.zip`, `.7z`, `.rar`, loose `.asi`/`.dll`/`.json`) are copied to the
  staging folder. Sources are never modified. Archive entries escaping the target are rejected.
- **Symlink deployment:** *Apply* links every enabled mod into the game directory in load order
  (later mods win). Every link is recorded in the database, so reverting is exact.
- **`.bak` safety:** a file shadowed by a mod is renamed to `<file>.bak` and restored on unlink.
  `meta/0.papgt`, `0.pathc` and `0.paver` are snapshotted to `.bak` on first start.
- **Clean Slate:** removes all links, restores `meta/` from `.bak` and renames foreign archive
  folders above `0035` to `NNNN.bak` (restorable in *Settings*).

## Getting started

On first start Pylune opens a short guide: three ideas to know, then five steps you do for real
(find the game, add a mod, enable it, apply, play) with buttons for each. The guide's
*How do I…* page answers common tasks and can be searched; reopen it with **❓ Guide** or **F1**.

## Settings

The *Settings* tab groups every option into categories, each with a one-line explanation:
**Appearance** (System/Dark/Light theme, size 80–200 %, compact or comfortable density, reduced
motion — previewed live), **Behavior** (ask before deleting, update check at start, the guide),
**Mods** (enable new mods, how many deleted mods stay restorable), **Game & launch**,
**Nexus Mods**, **Backups** (daily automatic backup, how many are kept), **Advanced** (parallel
jobs, log level, days of logs) and **Diagnostics** (archive state, Proton, metadata backup,
file locations). Changes are saved with *Save* or dropped with *Discard*; every page can be
reset to its defaults. Everything is stored in `config.toml` (sections `[game]`, `[ui]`,
`[behavior]`, `[mods]`, `[backup]`, `[nexus]`, `[launch]`, `[jobs]`, `[logging]`).

## Background jobs

Imports, downloads, Nexus queries, update checks and conflict analysis run on a worker pool
(`[jobs] max_parallel`, default 2–4) and appear in the jobs panel. Selecting several archives
imports them in parallel. Jobs that change the game directory (Apply, Unlink, Clean Slate,
Delete, backups) run one at a time. Every failure is written to the Log tab.

## Mod management

Click a mod to open its details panel:

- activate / deactivate, move to top / up / down / bottom (also *Enable all* / *Disable all*)
- **Update**: Nexus mods open their file list; the next download replaces the mod in place.
  Other mods ask for a replacement archive or file.
- **Delete** (redeploys the remaining mods when needed), **Open folder**
- **File conflicts**: which files this mod overrides and which are overridden, and by whom.
  The list shows `⚔ won/lost` badges.
- **Options**: archives with several alternative folders (e.g. `Human Female/0009/…` and
  `Dwarf Female/0009/…`) show an option picker; only the chosen option is deployed.
- **Readme viewer** for `readme`, `changelog`, `.txt`, `.md` and `.nfo` files, plus the full file list.

## Profiles

A profile stores which mods are enabled, their load order and **its own launch options**
(Steam syntax, e.g. `WINEDLLOVERRIDES="winmm=n,b" %command% -dx12`).

- Plain arguments: the game starts via `steam -applaunch 3321460 <args>`.
- Environment variables (`KEY=value`) or wrappers (`gamemoderun %command%`): Pylune starts
  the game itself through the same tool chain Steam uses (Steam Linux Runtime → Proton,
  read from `toolmanifest.vdf`). The Steam client must be running.

If the active profile has no options, `[launch] default_options` is used.

## Nexus Mods (`crimsondesert`)

1. Paste your personal API key (<https://www.nexusmods.com/users/myaccount?tab=api>) on the
   *Nexus* page.
2. **Premium:** enter a mod id or URL, *Fetch*, and download any file directly.
3. **Free accounts:** click *Make Pylune the nxm:// handler*, then use the website's
   "Mod Manager Download" button. Pylune asks for confirmation before every download, and
   accepts only links carrying the website's download key. Links for other games are passed
   to the handler Pylune replaced (e.g. MO2 or Vortex).
4. **Collections:** enter a collection slug or URL (or use the website's collection install
   button) and review the mod list; optional mods can be ticked. Premium accounts download all
   members in parallel; free accounts get a list of pending mods with links to their Nexus file
   pages, and the "Mod Manager Download" links are picked up automatically. Members are placed
   in the curator's load order; *Update* fetches the newest revision and changes only what
   changed; *Uninstall* keeps mods another collection still uses.
5. *Check updates* compares installed Nexus mods with their current version (several requests in
   parallel).

## Backups

*Settings → Backups* saves your settings, mod list (order, enabled flags, chosen options),
profiles and collections into one `.zip` in `~/.local/share/pylune/backups/`; a **full backup**
also contains every mod file. One automatic backup is taken per day (the newest 10 are kept).
Restoring needs the mods to be unlinked, takes a safety backup first and keeps the replaced mod
folder aside. Deleted mods go to *Recently deleted* on the Mods page and can be restored (or
undone right away from the status bar). The Nexus API key is never part of a backup.

**Save games:** every time you press *Play*, Pylune copies the game's save files (from
`<Proton prefix>/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save/`) to
`~/.local/share/pylune/save-backups/` (the newest 10 automatic copies are kept). *Settings →
Backups → Save games* backs them up on request and restores an older copy; the current saves
are backed up first. If Steam reports a cloud conflict afterwards, keep the local files.

## File placement

| Mod contains                                   | Result                                   |
|------------------------------------------------|------------------------------------------|
| `bin64/…`                                      | linked at the same path                  |
| `*.asi`, `*.dll`, `*.ini` elsewhere            | linked into `bin64/`                     |
| RAW `files/NNNN/<path>` or `NNNN/<path>` game files, including `.dds` textures | packed into **Pylune's archive group** (encrypted/compressed like the game file they replace; textures are also registered in a rebuilt `0.pathc`) |
| prebuilt groups `NNNN/0.pamt` + `*.paz`        | registered under a free group number     |
| byte-patch JSON mods (`"patches": [{ "game_file", "changes" }]`) | applied to the game file, packed into Pylune's archive group; every change is checked against the original bytes, results per mod in the Log tab |
| `meta/*` shipped by a mod                      | ignored — Pylune generates `0.papgt` from your vanilla backup |
| CSS patch files (`<file>.css.patch`, `<file>.css.merge`) in RAW layouts | merged into the stylesheet in load order (`@merge`, `@replace`, `@add`, `@remove`) |
| field-name JSON (format 3), XML/HTML patch files, game files with unknown location | not supported yet — reported in the UI/Log |
| readmes, images, `manifest.json`               | not deployed (shown in the details panel) |

### The archive overlay

The game only loads content from PAZ archives listed in `meta/0.papgt`. On *Apply*, Pylune
builds its own archive group (`0100`) in `~/.local/share/pylune/overlay/`, writes a new
`0.papgt` that lists it **first** (from the vanilla backup, after checking it against every
installed archive index), and links both into the game folder. Vanilla archives are never
modified; *Unlink all* restores the original `0.papgt` (and `0.pathc`) byte for byte. Format details:
[docs/paz-format.md](docs/paz-format.md).

ASI mods need a loader (`winmm.dll`, `version.dll` or `dinput8.dll`) in `bin64/` plus
`WINEDLLOVERRIDES="<loader>=n,b"` in Steam's launch options or a profile's launch options.

## Build and install

```sh
cargo build --release
packaging/install.sh       # binary to ~/.local/bin, launcher and icon for your desktop
cargo run --release
cargo test                 # offline suite
cargo test -- --ignored    # also talks to api.nexusmods.com
```

`tests/fixtures/sample.rar` comes from the `unrar` crate (MIT / Apache-2.0).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. Unless you explicitly state otherwise, any
contribution intentionally submitted for inclusion in Pylune shall be dual licensed as above,
without any additional terms or conditions.
