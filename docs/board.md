# Pailune board

Work items from [audit.md](audit.md), scheduled by [roadmap.md](roadmap.md) and prioritized in [priority-list.md](priority-list.md).
Move a line between columns when its status changes.

## In progress

- (nothing — waiting on the owner's in-game test batch 1)

## Next (small first)


## Backlog (large)

- [ ] D-14 in-place route: only for files the engine reads outside overlays (per DMM: dialogue
  audio, weapon/socket XML). Parts built and tested (`paz::pamt_edit`, `inplace::store`), not
  wired; wire when such a file type is confirmed to need it — *M*

- [ ] A-01 format 3.1 (phase 2): nested fields (`prefab_data_list[*].…`) and list values that
  change a record's length (table rewrite with new index offsets). Needed by *Character Creator*'s
  "Equip All Armor"; needs the full ItemInfo layout first — *L*
- [ ] A-01 compressed type-1 `.pac` meshes (2 914 in the game): decode for reading and patching — *M*

- [ ] B-16 JSON mod creation (steps 2–6): data browser → change from the browser → check
  against the game → test loop → package (roadmap phase 15) — *L*
- [ ] B-17 ASI mod creation (D-13), after B-16: research + decision record first, then templates,
  build/deploy, debug loop, packaging — *L*

- [ ] B-07 (step 4) Save editing beyond fixed-size values: adding/removing items and list
  entries (needs a body writer that rebuilds sizes and the table of contents) — *L*
- [ ] B-08 (step 2) Windows/macOS: test on real installs (needs a Windows and a macOS machine); macOS app bundle with `nxm://` URL type — *L*
- [ ] A-01 format 3, phase 2: names for large tables. Shapes are derivable (ItemInfo prefix done),
  names are not from data alone (message lists are incomplete). Next: in-game confirmation of the
  most-modded fields with the owner (stack sizes, prices, drop rates, cooldowns) — *L*

## Owner

- [ ] A-01 In-game test batch 1, **third run** — test 1 (arrow stacks) CONFIRMED 2026-10-26
  (101 arrows in a stack) after per-table groups; still open: tests 2–4 (hunting amounts, drop
  rates, crate coins). Previous note: (first run 2026-10-26: no effect — overlay held only
  table bodies; headers are now packed too, see `paz-format.md` §9.1). `local/test-mods/`, see its `TESTING.md`: arrow stacks
  (`ItemInfo._maxStackCount`), hunting amounts (drop-entry amount A/B = min/max?), drop rates
  (drop-entry rate). Also the first in-game check of patched data tables through the overlay.
- [ ] A-21 (rest) Register Pailune with Nexus (AUP) before a public release

## Done

- [x] B-16 JSON mod creation (step 1: new JSON mod wizard): *Create mod* modal asks for title,
  author, version, description; stages an empty format-3 JSON mod with `modinfo`; selects and opens
  it in Pailune's JSON editor (`ui/views/create_mod.rs`, `ui/actions/mods.rs`).
- [x] A-01 format 3.1 (phase 1: predicate matching by `match`): `Intent.matches` evaluates field
  predicates across candidate records (scalars, string, numeric tolerances, array IN-matching)
  and applies changes across all matching records; `value::decode` extracts typed fields
  (`bool`, `int`, `float`, `vec3`, `str`, `ref`, `hash`) from table payloads (`src/tables/`).
- [x] Post-extraction permission sanitization in `mod_engine/archive.rs`: normalizes restrictive
  archive permissions (`u+rwX`), guaranteeing user traversal/read/write on extracted directories
  (`0o700`) and files (`0o600`/`0o700`) across zip, 7z, and rar. Prevents OS error 13 walk errors.
- [x] Saves tab (slots, save backups, embedded editor); strict archive-index check on every Apply
  (`paz::pamt_check`, passes all vanilla groups); JMM compared (`research/jmm-comparison.md`).
- [x] B-07 (step 3) Save editing (approved 2026-10-26): fixed-size values edited in the viewer,
  checked against the current bytes, re-encoded (fresh nonce, HMAC), read back before an atomic
  replace; exclusive job, refused while the game runs, all saves backed up first. Verified in
  memory on all 5 real saves (exactly the edited byte changes). Copper-coin shortcut.
- [x] Value-level conflicts between JSON patches (`overlay/value_conflicts.rs`): overlapping byte
  ranges after resolving against vanilla, named by record; later mod overrides earlier (`apply_over`);
  cached vanilla scan (2.3 s → 21 ms on repeat).
- [x] Alternative JSON files as options (`mod_engine/alternatives.rs`): files of one mod that change
  the same values are a choice (titles from `modinfo`, natural order); only the chosen one deploys.
- [x] JSON mod viewer/editor (details panel): changes table with numbers and hex, raw JSON with
  live checking, Save marks the setup for Apply (`src/patch/document.rs`, `ui/views/json_editor/`).
- [x] Code review of the recent features (27 findings, three passes, all fixed with tests):
  FOMOD and mod paths cannot leave their folders; XML/HTML patch engine edge cases; overlapping
  or length-changing table patches refused; interrupted save restores recovered; hostile saves
  and installers bounded; CLI/MCP report failed jobs and release the lock between calls; hard
  links re-stamped after settings edits; Windows helpers without console windows; mod lists
  match several files of one Nexus mod one-to-one and keep later changes; plugins sharing a
  name in a FOMOD stay apart; a broken FOMOD deploys nothing and says why.
- [x] A-01 archive pipeline (phase 3 of the original plan): read-only PAZ reader, overlay group
  verified in game, textures with rebuilt PATHC, byte-patch JSON, CSS/XML/HTML patches, archive
  conflicts — details in `paz-format.md` and the entries below.
- [x] A-01 format 3: raw `old`/`new` intents on every data table (only record boundaries needed),
  checked on the real item table; recursive inference engine with nested classes
  (`tools/table-layouts/engine.py`).
- [x] A-01 format 3, phase 1 (`src/tables/`, `data/table-layouts.json`, `tools/table-layouts/`):
  field names from the deserializer's messages in the executable; layouts inferred by exact fit +
  minimum description length with per-field proof; 21 tables / 38 editable fields, each verified
  on every record of the installed game (Rust test). Format-3 mods: `set` by field name (engine or
  snake_case names) and raw `old`/`new`, resolved into verified byte changes; the layout is
  re-verified against the game on every Apply and refused when the build differs; unproven
  fields and other operations are refused with a reason.
- [x] A-01 research: table layouts can be derived from the game (`docs/research/table-layouts.md`):
  513 classes with field names in the executable's data sections, sizes and order verified on
  sample records; method and order of work set (owner decision 2026-10-26: own layouts).
- [x] B-13/D-12 ASI plugin management (`mod_engine/plugins.rs`): plugins in `bin64/`,
  `scripts/`, `plugins/` with their owning mod (from the link journal, so copies count too) on
  the Diagnostics page; `.ini` editor in the details panel (writes the staged file, which the
  deployed link shows; refuses paths outside staging); game-update banner warns that plugins may
  break.
- [x] B-07 own save decoder (`src/savefile/`): SAVE container (key, ChaCha20, HMAC-SHA256, LZ4),
  self-describing body (schema, table of contents, exact object walk for every field kind),
  readable values and trees. Verified on all 10 real saves: HMACs valid, byte-identical
  re-encryption, 1 881/1 881 objects decoded exactly. Read-only slot list and inspector
  (Settings › Backups › Save games), `pailune saves` / `pailune save <slot> --find …`, MCP tools
  `list_saves` / `read_save`. What saves hold is documented in `docs/save-format.md`.
- [x] D-12 accepted: no script runtime; ASI plugin management instead (added to Backlog).
- [x] B-08 Windows GUI subsystem (no console window behind the app); command-line runs re-attach
  to the parent terminal (`AttachConsole`). Compiled by CI only; unverified on Windows.
- [x] A-01 HTML patches (`src/patch/html/`): `<html-patch>` operations (`set` incl.
  `class="+a -b"`, `remove`, `replace`, `inner`, `append`, `prepend`, `before`, `after`) and
  comment directives (`@replace … @end`, `@set-attr`, `@add-class`) with CSS-lite selectors;
  void elements handled; spliced into the original text like XML.
- [x] A-01 new textures: a `.dds` the game does not have gets its own PATHC entry (default format
  class) and is packed like a replacement; covered by the overlay test. With XML patches, mods can
  reference new textures from the game's UI XML.
- [x] A-01 XML patch/merge files (`src/patch/xml/`): lenient span-keeping reader for the game's
  XML (BOM, multi-root fragments, comments, `</>` shorthand — surveyed on 1 084 real files),
  XPath subset plus `#id`/`.class`/`find`+`key`, JSON and `<xml-patch>` operations, identity-based
  `<xml-merge>`; edits are spliced into the original text (untouched bytes and CRLF kept);
  applied in load order with CSS patches, results per mod in the Log tab.
- [x] Fixed a race in mod-list import: the missing-mods list was computed from the in-memory
  registry, which can lag one poll behind an install; now computed inside the write transaction.
- [x] A-28 FOMOD installers (`src/fomod/`, `ui/views/installer.rs`): `ModuleConfig.xml` in any
  case and UTF-8/UTF-16; required files, steps with visibility conditions, all group types,
  plugin types incl. dependency types, condition flags, file/folder installs with priorities,
  conditional installs, schema ordering. Choices are stored in the mod's variant field
  (`fomod:<json>`), so planning, conflicts, overlay, mod lists, trash and backups use them; until
  the user chooses, the installer's defaults apply. A broken installer is logged and ignored.
- [x] B-08 Windows `nxm://` registration (`nexus/winreg.rs`): per-user protocol key via
  `reg.exe`, detection, forwarding links for other games to the previous handler (e.g. Vortex);
  parsing of localised `reg query` output and command templating unit-tested on Linux. CI uploads
  Windows/macOS release binaries for tags.
- [x] D-11 deployment without symlinks: platform reports symlink / hard link / copy; non-symlinks
  get a size+mtime stamp journaled after placing (schema v5); unlink removes only matching files;
  interrupted runs fall back to content equality unless the `.bak` rename was still pending.
  Tested with forced copies and hard links, crash test extended to copies.
- [x] B-07 research: save container documented (`docs/save-format.md`) — header verified on real
  saves, payload is LZ4 + ChaCha20 with HMAC-SHA256 (archive key scheme ruled out by experiment;
  community library `crimson-rs` documents the key scheme).
- [x] B-13 research (`docs/research/script-bridge.md`): no script files among 2 052 523 archived
  files; native mods are ASI plugins with per-patch pattern hooks; proposal D-12.
- [x] B-14 command line and MCP (`src/cli/`, `src/mcp/`): 25 commands with `--json` and exit
  codes, `--yes` for irreversible ones; read-only commands read the database (work with the
  window open), changing ones drive a headless `PailuneApp` under the instance lock (same rules as
  the buttons); `pailune mcp` serves 24 tools over stdio (MCP 2025-06-18, tool annotations,
  `confirm: true` for destructive tools, errors as `isError` results). Tested: CLI parsing,
  an MCP session installing/ordering/applying mods, and a smoke test of the binary in isolated
  folders (which caught `--json status` opening the window — fixed).
- [x] B-08 step 1 platform layer (`src/platform/{linux,windows,macos}.rs`): Steam roots,
  linking (Windows: symlink → hard link → copy), opening paths/URLs, Steam client, process check,
  private files, `AppData/Local` for saves, Proton or native launch (new `Direct` launch on
  Windows). `nxm://` registration reports "not supported yet" outside Linux. CI builds on
  Windows and macOS (tests informative). Linux behaviour unchanged (real Steam detection
  re-checked). Windows/macOS code is unverified until CI runs.
- [x] B-12 portable mod lists (`src/modlist/`): versioned JSON (order, flags, options, source,
  Nexus origin, collections); import matches by Nexus id then name, reorders and flags, lists
  missing mods (download all for Premium, Nexus pages otherwise) and re-applies the list after
  every install so late mods take their place; Profiles page buttons, guide topic, user guide.
- [x] B-15 *Move to number* in the details panel (`MoveTo::Position`, 1-based, clamped), Enter or
  *Move*; guide topic and user guide updated.
- [x] B-09/B-10/B-11 `CONTRIBUTING.md` (setup, conventions, checks, commits, game-data rules,
  license), `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1, contact louisgoodnews95@gmail.com),
  GitHub issue forms (bug report, feature request; blank issues off) and a PR template.
- [x] B-08 decision: Windows and macOS are goals — vision updated, open decision D-11 for the
  per-platform deployment method.
- [x] B-05 documentation: `docs/README.md` (index), `vision.md`, `architecture.md` (layers,
  modules, threads, flows, testing), `data-model.md` (files, every `config.toml` key, SQLite
  schema, backup formats), `extending.md` (conventions and recipes), `decisions.md` (D-01 … D-10),
  `user-guide.md`; rustdoc on every item. Writing it surfaced a gap — R-01 promised a pending-
  changes banner that only existed as status-bar text — now a banner with *Apply now*.
- [x] B-04 UI/UX rework against `docs/ux-principles.md` (every main screen rendered and reviewed
  in both themes): Clean slate moved into a *More* menu (R-06); confirmation buttons name their
  action and texts are plain language (R-05, R-08); Apply is the filled primary only while changes
  are pending (R-10); text no smaller than 11.5 pt, larger checkboxes (R-13, R-25); one vocabulary
  (Enable/Disable); quieter, separated Delete; teaching empty states for profiles and Nexus;
  shortcuts in tooltips; button labels pick black or white for contrast; contrast test for both
  palettes (it caught light-theme badge colours that had not been applied); rustdoc on every UI
  helper (rest of A-32).
- [x] A-29 AppImage: `packaging/appimage.sh` assembles an AppDir (binary, desktop entry, SVG and
  256 px icons, AppStream metadata) and packs it with `appimagetool`; CI builds it on Ubuntu 22.04
  for tags `v*` (glibc ≥ 2.35; a local CachyOS build needs glibc 2.44) with a pinned, checksummed
  appimagetool 1.9.1. The `nxm://` handler registers the AppImage file itself (`$APPIMAGE`), not
  its changing mount point. AppDir layout verified locally; the final packing step was not run
  here.
- [x] A-28 (part) save-game backups (`src/save_backup/`): the save folder in the Proton prefix
  (`…/AppData/Local/Pearl Abyss/CD/save/<id>/slot<N>/`, verified on the installed game) is zipped
  before every Play (default on, newest 10 kept) and on request; restore backs up the current
  saves first and swaps folders by rename; Settings › Backups › Save games. Read-only check on
  the real saves: 13 files, byte-identical after extraction.
- [x] B-03 onboarding (`src/ui/guide/`, `src/ui/views/guide.rs`): a non-modal guide window opens
  on first start with three core ideas (R-18) and five real steps checked against the app's state
  (find game → add mod → enable → apply → play) with progress and a head start (R-17), buttons that
  perform each step, a finish state (R-23); searchable "How do I…" topics (17, incl. glossary and
  shortcuts); ❓ Guide button and F1; teaching empty mod list (R-19); finished guides stop opening
  at start.
- [x] Rendering checks: bundled emoji font (icons no longer depend on system fonts), a test that
  every UI symbol has a glyph, and on-demand screenshots of key screens in both themes
  (`PYLUNE_SHOTS=<dir> cargo test screens -- --ignored`).
- [x] B-02 settings menu (`src/ui/views/settings/`, `src/config/prefs.rs`): eight categories
  with one-line help per option, constrained controls (R-07), Save/Discard bar, per-page reset;
  appearance (theme incl. System, scale, density, reduced motion) previewed live; WCAG-checked
  light and dark palettes; new `[ui]`, `[behavior]`, `[mods]`, `[backup]` sections and
  `[logging] keep_days`, all wired (deletes skip the dialog unless asked — Undo exists).
- [x] UX principles (`docs/ux-principles.md`): 29 rules with evidence and references.
- [x] B-01 backups (`src/app_backup/`): one `.zip` with manifest, `config.toml` and an online
  SQLite snapshot (mod list, load order, flags, variants, profiles, collections, journal), full
  backups add the mod files; daily automatic backup (newest 10 kept); restore needs nothing
  deployed, takes a safety backup first, restores the database online, keeps the replaced
  staging folder aside; Settings page lists, restores and deletes backups.
- [x] B-06 deleted mods move to a trash with their metadata (origin, source, variant); Undo in
  the status bar and a "Recently deleted" list restore them; the newest 20 are kept.
- [x] A-24 downloads resume with HTTP ranges (up to 4 attempts) and are verified with Nexus'
  `md5_search` (a hash registered for another file fails; an unknown hash is noted).
- [x] A-32 rustdoc on every function and constant outside the UI views (views follow in B-04);
  deepest non-UI nesting (parallel update check) flattened.
- [x] A-28 (part) multi-variant mods: option folders detected (≥ 2 sibling folders with game
  layouts), only the chosen option (default: first) deployed, shared root files always; picker in
  the details panel; choice stored (schema v4).
- [x] A-31 UI actions tested through a headless `PailuneApp` (import/apply/delete, profiles,
  settings validation, nxm key rule, collection pending downloads); Nexus download, install,
  update check, rate limits and key rejection tested against a local mock API (`[nexus] api_url`).
- [x] A-29 (part) dual license MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
- [x] A-29 (part) release profile (thin LTO, stripped), GitHub Actions CI (fmt, clippy -D
  warnings, tests), desktop launcher + SVG icon + `packaging/install.sh`, window icon,
  CHANGELOG.
- [x] A-28 (part) mod search (Ctrl+F), shortcuts, drag-and-drop import, profile export/import.
- [x] A-33 mod table scrolls horizontally on narrow windows.
- [x] A-30 log files kept 14 days, journal trimmed to 20 000 events, deleted mods' logs retired.
- [x] A-21 `X-RL-*` budget recorded after every API/GraphQL call and shown on the Nexus page;
  calls stop with a clear message when the hourly budget is nearly spent.
- [x] A-24 (part) downloads are checked against Nexus' exact file size before staging.
- [x] A-17 Nexus collections: v2 GraphQL fetch (slug, URL or `nxm://…/collections/<slug>/revisions/<n>`),
  review dialog with optional members, install in the curator's load order (Premium: parallel
  downloads; free: pending list with links, matching `nxm://` downloads accepted automatically),
  enable/disable/uninstall (keeps mods shared with other collections), revision updates that
  touch only changed members. Not yet: the collection archive (curator rules, installer choices,
  bundled files) — needs an API key to download and is unverified.
- [x] A-02 Write-ahead link journal; stale links adopted; close refused once during
  game-folder jobs. Tested by interrupting a deploy before/after every journal write.
- [x] A-03 `flock` single-instance lock; atomic request spool; second start focuses.
- [x] A-25 nxm queue loss (replaced by the spool).
- [x] A-05 Delete redeploys whenever the manifest links into the mod.
- [x] A-06 Pending changes = stored registry revision vs deployed revision.
- [x] A-04 Download client without an overall timeout.
- [x] A-07 Rename-only swap; tolerant scan; updates of deleted mods fail.
- [x] A-08 Unique work/archive names; atomic name claim.
- [x] A-09 Case-insensitive game paths.
- [x] A-20 Scan before the write transaction.
- [x] A-14 Steam library detection (native/Flatpak/Snap) + first-run setup dialog;
  hardcoded path removed.
- [x] A-18 Invalid `config.toml` kept aside; database errors get a window; rescan errors shown.
- [x] A-15 Apply blocked after a game update; guided unlink → Steam verify → refresh backup.
- [x] A-26 Game folder cannot change while game-folder jobs are queued; JSON link folder
  removed.
- [x] A-22 Buttons disabled while the game runs / jobs queued; details refresh;
  "Analysing…" state.
- [x] A-10 Direct launch through the Steam Linux Runtime tool chain + Steam compat env.
- [x] A-11 Wrapper commands (`gamemoderun %command%`).
- [x] A-27 argv[0] game detection; children reaped; URL fallback keeps arguments.
- [x] A-12 nxm links require key/expiry; every download confirmed in a dialog.
- [x] A-13 Previous nxm handler remembered; foreign links forwarded; Exec escaped.
- [x] A-16 Update check follows the `file_updates` chain; per-mod failures; nxm updates
  replace.
- [x] A-23 Key redaction, query encoding, private key file, redacted `Debug`.
- [x] A-19 FIFO lane for game-folder jobs; one conflict analysis in flight.
- [x] A-24 (part) `.part` cleanup, mirror fallback, expiry check, unique names.
