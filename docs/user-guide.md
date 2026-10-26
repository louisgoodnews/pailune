# Pylune user guide

Pylune installs and manages *Crimson Desert* mods on Linux (Steam with Proton). This guide covers
everything from installation to troubleshooting. Inside the app, **❓ Guide** (or **F1**) shows the
short version with a searchable list of tasks.

## Three things to know

1. **Mods are kept in Pylune's own folder.** Installing, enabling and ordering mods never touches
   the game.
2. **Apply links the enabled mods into the game.** Files a mod replaces are kept as `.bak`.
3. **Unlink all puts the game back exactly as it was.**

## Install Pylune

- **AppImage:** download `Pylune-<version>-x86_64.AppImage`, make it executable
  (`chmod +x`, or *Properties → Allow executing*), and start it. Keep it in a fixed place
  (e.g. `~/Applications`) — browser downloads are routed to the file you register.
- **From source:** install Rust, then in the project folder run `cargo build --release` and
  `packaging/install.sh` (puts `pylune` into `~/.local/bin` and adds a menu entry).

## First start

Pylune looks for Crimson Desert in your Steam libraries (normal, Flatpak and Snap Steam). If it
cannot find the game, it asks for the folder that contains `bin64/` and `meta/`. It then takes a
backup of the game's metadata files and opens the getting-started guide, which walks you through
your first mod: **add a mod → enable it → Apply → Play**.

## Add mods

- **From a file:** *Mods → Import archives / files…* (Ctrl+I) for `.zip`, `.7z`, `.rar`, `.asi`,
  `.dll` or `.json`; *Import folder…* for unpacked mods; or drop files onto the window. Several
  imports run in parallel.
- **From Nexus Mods:** on the *Nexus* tab paste your personal API key (nexusmods.com → your
  account → API keys) and press *Save key*. Press *Make Pylune the nxm:// handler* once; from then
  on the website's **Mod manager download** buttons send files to Pylune, which asks before each
  download. You can also enter a mod's number or address and press *Fetch* to see its files.
- **Collections:** *Nexus → Collections*, paste the collection's address and *Fetch*. Review the
  list (optional mods can be unticked), then *Install*. Premium accounts download everything at
  once; with a free account, open each member's page from the list and click the website's
  download button — Pylune accepts the matching files automatically.

Installed mods appear at the end of the mod list, disabled (unless *Settings → Mods → Enable new
mods* is on).

## Enable, order and apply

- Tick a mod's checkbox to enable it. Nothing changes in the game yet; a banner says
  **Changes not applied**.
- **Load order:** when two mods change the same file, the one **lower in the list wins**. Click a
  mod to open its details and use *Top*, *⏶*, *⏷*, *Bottom*, or type a number into **Move to
  number** and press *Move* (Enter works too).
- **Apply** (Ctrl+Enter) links the enabled mods into the game. The status line tells you what
  happened; problems are listed on the *Log* tab.
- **Unlink all** removes every link and restores the originals.

### What Pylune can install

| Mod contains | What Pylune does |
|--------------|------------------|
| Files under `bin64/` | Linked at the same place. |
| `.asi`, `.dll`, `.ini` plugins | Linked into `bin64/` (needs an ASI loader, see below). |
| Game files for the archives (`files/NNNN/…`, `NNNN/…`, textures `.dds`) | Packed into Pylune's own archive group, which the game reads first. |
| Prebuilt archive groups (`NNNN/0.pamt`, `*.paz`) | Registered under a free group number. |
| Byte-patch JSON mods | Applied to the game file inside the archives; each change is checked against the original bytes. |
| `*.css.patch`, `*.css.merge` | Applied to the game's stylesheets. |
| Readmes, screenshots | Kept for you to read (details panel), not deployed. |

### Mods with options

Some mods ship several versions in separate folders. The details panel then offers a choice;
only the chosen option is deployed. Press *Apply* after changing it.

### Conflicts

A ⚔ badge shows how many files a mod wins and loses against others. The details panel lists them;
move a mod further down to make it win.

## Play

**▶ PLAY** starts the game through Steam with the active profile's launch options (or the default
ones from *Settings → Game & launch*). Pending changes are applied first, and your save games are
backed up before the game starts.

**ASI/script mods** need a loader such as `version.dll` in `bin64/` — the header shows whether
one is installed. *Settings → Diagnostics* shows the launch options Proton needs for it (e.g.
`WINEDLLOVERRIDES="version=n,b" %command%`); put them into *Default launch options* or a profile.
Wrapper commands like `gamemoderun %command%` work too.

## Profiles

A profile remembers which mods are enabled, their order, and its own launch options. *Profiles → type a name → Save current as*. *Load* switches to a profile (then press
*Apply*). Profiles can be exported to a `.json` file and imported elsewhere.

## Updates

*Nexus → Check updates* marks mods with a newer file on Nexus (or turn on *Settings → Behavior →
Check for mod updates at start*). Select a mod and press *Update…*: Nexus mods show their files,
the download replaces the mod in place; other mods ask for a replacement archive.

## Deleting and undo

*Delete* (or the Delete key) moves a mod to **Recently deleted**. Press **↩ Undo** in the status bar
or restore it from the list below the mods. The newest 20 deleted mods are kept (*Settings →
Mods*). If you prefer a question before each delete, turn on *Settings → Behavior → Ask before
deleting a mod*.

## Backups

- **Pylune backups** (*Settings → Backups*): settings, mod list, order, options, profiles and
  collections; a *Full backup* also contains the mod files. One is taken automatically each day.
  Restoring needs all mods unlinked and takes a safety backup first.
- **Save games:** copied before every Play and on request (*Back up saves now*). *Restore…* puts
  an older copy back after backing up the current saves. If Steam then reports a cloud conflict,
  keep the local files.

## After a game update

When the game's files change, Pylune blocks *Apply* and shows a yellow banner with three steps:
**1. Unlink all → 2. Verify files in Steam → 3. Refresh backup**. Then apply again. This keeps
Pylune's backups in step with the new game version.

## Start over

*More → Clean slate…* unlinks and disables every mod, restores the game's original metadata and
sets aside archive folders that other tools added (restorable in *Settings → Diagnostics*). Your
mods stay in Pylune.

## Settings

| Page | What you can change |
|------|---------------------|
| Appearance | Theme (System/Dark/Light), size 80–200 %, density, reduce motion — previewed live. |
| Behavior | Ask before deleting, update check at start, show the guide at start. |
| Mods | Enable new mods, how many deleted mods to keep. |
| Game & launch | Game folder, Steam app id, apply before Play, default launch options. |
| Nexus Mods | Game domain (the API key is on the Nexus tab). |
| Backups | Daily backup, how many to keep, save backups before Play, the backup lists. |
| Advanced | Parallel jobs, log level, days of logs (take effect after a restart). |
| Diagnostics | Game version and archives, Proton prefix, ASI launch options, metadata backup, file locations. |

Press **Save** to keep changes or **Discard** to drop them; each page can be reset to defaults.

## Keyboard shortcuts

| Keys | Action |
|------|--------|
| F1 | Open the guide |
| Ctrl+F | Search mods |
| Ctrl+I | Import files |
| Ctrl+Enter | Apply |
| F5 | Rescan the staging folder |
| Delete | Delete the selected mod (undoable) |

## Troubleshooting

| Problem | What to do |
|---------|------------|
| A mod has no effect | Check its ⚔ conflicts and move it lower; check the *Log* tab for skipped files or failed patches; ASI mods need a loader and launch options. |
| "Changes not applied" stays | Press *Apply*; if it is blocked, read the status line (game running, a job running, or a game update). |
| The game does not start | Start it once from Steam so the Proton prefix exists; check *Settings → Diagnostics*. |
| Browser downloads do not reach Pylune | *Nexus → Make Pylune the nxm:// handler* again (needed after moving the AppImage). |
| Something went wrong after an update | Follow the yellow banner; or *More → Clean slate…*. |
| Reporting a bug | *Settings → Advanced → Open log folder*; set the log level to `debug` and restart for detail. |

## Where Pylune keeps its files

Settings in `~/.config/pylune/config.toml`; everything else in `~/.local/share/pylune/` (mods in
`staging/`, backups in `backups/` and `save-backups/`, logs in `logs/`). See
[data-model.md](data-model.md) for the full list.

## Removing Pylune

Press **Unlink all** first (the game is then exactly as before), then delete the AppImage or
`~/.local/bin/pylune`, and optionally `~/.config/pylune/`, `~/.local/share/pylune/` and
`~/.local/share/applications/pylune*.desktop`.
