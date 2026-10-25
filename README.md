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
