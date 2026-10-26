# Research: a script bridge into the running game (B-13)

Question: can Pylune let mods inject scripts into the running game, and how?
Status 2026-10-26: research done; **no code**; decision proposed as D-12.

## Findings

1. **The engine has no exposed scripting layer.** A read-only scan of all 2 052 523 files in the
   game's archives (build 2.3.2) found no script files — no `.lua`, `.luac`, `.py`, `.js`, `.as`,
   `.cs` or similar. Behaviour is data-driven: sequences (`.paseq`, `.paseqc`), schedules
   (`.paschedule`), gimmicks (`.binarygimmick`), stages, levels and XML. Pylune already patches
   such data through its overlay (JSON byte patches, CSS; XML patches are on the roadmap).
2. **Native code runs through ASI plugins.** The community uses the Ultimate ASI Loader (a proxy
   DLL such as `winmm.dll` or `version.dll` in `bin64/`; it is installed on the reference
   machine). Mods like JustSkip or InstantInteract are compiled C++ plugins that find game
   functions by **byte-pattern scanning** and hook them.
3. **Patterns break with patches.** Mod pages report errors like "Timer pattern not found" after
   game updates; every hook-based mod needs maintenance per patch.
4. **No community scripting framework exists** for this game (no Lua/ScriptHook equivalent was
   found). The Ultimate ASI Loader offers C++ "snippets", not a scripting language.

## What a bridge would take

An ASI plugin that embeds a script runtime (e.g. Lua or Rhai) and exposes hooks on game
functions. That requires reverse-engineering the executable for each patch (function
signatures, calling conventions, object layouts) — a separate project in C++/Rust for Windows
x64, not part of a mod manager. Risks: crashes and save corruption, breakage on every patch,
and possible conflict with the game's terms; the game is single-player and has no known
anti-cheat, but that should be confirmed before any work.

## Recommendation

- Pylune should **not** build a script runtime. It should be the best **installer and manager**
  for native plugins: detect/install the ASI loader, show which plugins are active, warn after a
  game update that hook-based plugins may break (Pylune already detects game updates), and keep
  per-plugin INI settings editable.
- If a community script bridge appears later, Pylune installs it like any ASI mod and can manage
  its script folder.
- Data-driven changes (the game's actual "scripts") are better served by extending the overlay:
  XML patches (A-01) and table edits.

## Sources

- Ultimate ASI Loader: https://github.com/ThirteenAG/Ultimate-ASI-Loader
- Crimson Desert Ultimate Mods Manager (script installers, ASI tab): https://github.com/faisalkindi/CrimsonDesert-UltimateModsManager
- Example hook-based mods: https://www.nexusmods.com/crimsondesert/mods/178,
  https://crimsondesertmods.com/justskip-v6-0/
- Modding guide: https://www.nexusmods.com/crimsondesert/mods/366
