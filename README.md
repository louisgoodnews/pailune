# Pylune

Native Linux mod manager for **Crimson Desert** (Steam / Proton), written in Rust with egui.

## How it works

- **Staging:** imported mods (folders, `.zip`, loose `.asi`/`.dll`/`.json`) are copied to
  `~/.local/share/pylune/staging/<mod>/`. Sources are never modified.
- **Symlink deployment:** *Apply* links every enabled mod into the game directory, in load order
  (later mods win conflicts). Every link is recorded in `manifest.json`.
- **`.bak` safety:** a file shadowed by a mod is renamed to `<file>.bak` and restored on unlink.
  `meta/0.papgt`, `0.pathc` and `0.paver` are snapshotted to `.bak` on first start.
- **Clean Slate:** removes all links, restores `meta/` from `.bak` and renames foreign archive
  folders above `0035` to `NNNN.bak` (restorable from the side panel).

## Playing

**▶ PLAY** starts the game through Steam (`steam://rungameid/3321460`), so Proton and your launch
options apply. With *Apply pending changes before Play* enabled, unapplied mod changes are deployed
first. While `CrimsonDesert.exe` runs, Apply / Unlink / Clean Slate are blocked.

## Profiles

A profile stores which mods are enabled and their load order. Save the current selection under a
name, then load, overwrite or delete it on the *Profiles* page. Loading marks changes; press
*Apply* (or *Play*) to deploy them.

## Nexus Mods

1. Paste your personal API key (<https://www.nexusmods.com/users/myaccount?tab=api>) on the
   *Nexus* page. It is stored in `~/.local/share/pylune/nexus_api_key` with mode `0600`.
2. **Premium:** enter a mod id or URL, *Fetch*, and download any file directly.
3. **Free accounts:** click *Make Pylune the nxm:// handler*, then use the website's
   "Mod Manager Download" button. The browser starts `pylune nxm://…`, which hands the link to the
   running window (or opens one).
4. *Check updates* compares installed Nexus mods with their current version; *⬆* in the mod list
   fetches the files and the next download replaces the staged mod in place.

Only `.zip` archives are extracted; `.7z`/`.rar` files must be unpacked and imported as a folder.

## File placement

| Mod contains                          | Linked to                          |
|---------------------------------------|------------------------------------|
| `bin64/…`, `meta/…`, `0036/…` (NNNN)  | same path inside the game folder   |
| `*.asi`, `*.dll`, `*.ini` elsewhere   | `bin64/<file name>`                |
| `*.json` elsewhere                    | `<JSON mod folder>/<relative path>` (default `bin64/mods`) |
| anything else (readmes, images)       | ignored                            |

ASI mods need a loader (`winmm.dll`, `version.dll` or `dinput8.dll`) in `bin64/` and the Steam
launch option shown in the side panel, e.g. `WINEDLLOVERRIDES="winmm=n,b" %command%`.

## Build

```sh
cargo run --release
cargo test
```
