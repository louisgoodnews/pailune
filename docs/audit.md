# Pylune audit — 2026-10-25

Audit of commit `bf76e62` before productive use. Four review passes were run in parallel
(filesystem safety, concurrency & persistence, Nexus/launch/security, product readiness),
plus a live investigation of Nexus Mods collections via the public GraphQL API.

**Verdict: not ready for productive use.** The code base is clean and modular, and
ASI/DLL deployment is safe on the happy path. But most Crimson Desert mod formats have no
effect with symlink deployment, and several failure paths can leave the game directory in a
state Pylune can no longer revert.

Severity scale: **blocker** (must fix before use) · **high** · **medium** · **low**.
Every item has an ID that [roadmap.md](roadmap.md) and [board.md](board.md) refer to.

---

## Blockers

### A-01 — JSON, RAW and overlay mods do nothing in-game
- **Where:** `src/mod_engine/plan.rs` (placement), `src/config/mod.rs` (`json_subdir`).
- **Finding:** Crimson Desert loads content only from PAZ archives indexed by `meta/0.papgt` /
  `0.pathc`. Checked against the mods in the most-revised Crimson Desert collection
  ("Crimson QOL", revision 12):
  - **JSON mods** (e.g. `MEGA_STACKS_999999.json`) are byte patches. DMM injects them into
    PAZ entries (log: `PAZ inject`, `PAPGT_REBUILD`, `PATHC_WRITE`). Pylune links them into
    `bin64/mods`, which nothing reads.
  - **RAW mods** ship `files/<group>/<virtual path>` plus `manifest.json`
    (e.g. `Better_Radial_Menus/files/0012/ui/...`). Pylune classifies them as `EMPTY`.
  - **Overlay mods** that ship `00NN/` and their own `meta/0.papgt` collide with each other
    (both take the same group number) and break after every game update (stale CRCs).
  - **ASI mods** are the only format that works.
- **Fix:** a PAZ pipeline. Read PAMT, decompress/decrypt entries, apply JSON patches and RAW
  files, write a Pylune-owned overlay group, and generate `0.papgt`/`0.pathc`. The
  generated files are still deployed as symlinks with `.bak` originals. Until this exists,
  mark unsupported formats clearly in the UI.

### A-02 — Deploy is not crash-safe; the manifest is written only at the end
- **Where:** `src/jobs/tasks/deploy.rs` (`redeploy`), `src/symlink/deploy.rs`, `src/ui/app.rs`.
- **Finding:** The manifest is saved empty after undeploy, links and `.bak` renames happen in
  memory, and the manifest is persisted only after the loop. Closing the window kills worker
  threads mid-job; a panic has the same effect. The result is orphan symlinks and stranded
  `.bak` files that Unlink and Clean Slate cannot revert. A later Apply may even back up an
  orphan symlink as the "original".
- **Fix:** write-ahead journaling (persist each record before touching the filesystem),
  treat symlinks into staging as Pylune's own, and block window close while exclusive jobs run.

### A-03 — Two instances can deploy concurrently
- **Where:** `src/instance.rs`.
- **Finding:** The PID check compares `/proc/PID/exe` with `current_exe()`. After a rebuild
  or upgrade the running binary reads `… (deleted)` (verified), so the next start or `nxm://`
  link opens a second GUI. Check-then-write is also racy. Each process has its own game lock.
- **Fix:** hold an exclusive `flock` for the whole process lifetime, and hand off through
  atomic spool files.

### A-04 — Downloads longer than 30 s always fail
- **Where:** `src/nexus/api.rs`.
- **Finding:** `timeout_global(30s)` in ureq 3 covers the response body. Reproduced: a 3 s
  global timeout fails at 3.006 s while the body is still streaming.
- **Fix:** a separate download agent without a global timeout, keeping connect and response
  timeouts.

### A-05 — Deleting a disabled-but-deployed mod leaves dangling links
- **Where:** `src/jobs/tasks/staging.rs` (`remove`).
- **Finding:** Redeploy is decided by `entry.enabled`, not by whether the manifest links into
  the mod. Links then point at deleted files and the originals stay in `.bak`.
- **Fix:** decide by `manifest.links.any(source starts_with staging/<name>)`.

### A-06 — The "pending changes" flag is unreliable
- **Where:** `src/ui/actions/poll.rs`, `src/ui/app.rs`, `src/ui/actions/deploy.rs`.
- **Finding:** `dirty` is session-only, is cleared even when the selection changed during
  Apply, and is not set by Unlink all, so Play can launch vanilla or with stale mods.
- **Fix:** keep a registry revision in the DB and record the deployed revision;
  pending = `deployed_rev != registry_rev`.

---

## High

| ID | Area | Finding | Fix |
|----|------|---------|-----|
| A-07 | Staging | `sync` during an update's `remove_dir_all` + rename window drops the mod's order, flags and Nexus ref (`install.rs` swap, `registry.rs` sync). | Swap with renames only (old → trash, new → dest, then delete the trash); tolerate per-mod scan errors. |
| A-08 | Staging | Same-name concurrent imports/downloads share `.pylune-incoming-<name>` and `downloads/<file>.part`; a non-replacing import can still delete an existing `dest`. | Unique work and `.part` names per job; no-replace final rename. |
| A-09 | Deploy | Case: a mod shipping `Bin64/x.asi` is classified as `bin64` but linked with its original case, which creates a second `Bin64/` directory on Linux. | Resolve each path component case-insensitively against existing directories. |
| A-10 | Launch | Direct Proton launch bypasses the Steam Linux Runtime (`require_tool_appid 4183110`), Steam Cloud sync and the overlay, and misses `STEAM_COMPAT_INSTALL_PATH`/`STEAM_COMPAT_LIBRARY_PATHS`. | Run through the SLR entry point, or write the launch options to Steam and launch through Steam; at least warn. |
| A-11 | Launch | Wrapper commands before `%command%` (`gamemoderun`, `mangohud`) are passed to the game as arguments. | Keep a wrapper prefix; reject wrappers on the Steam CLI path. |
| A-12 | Security | Any web page can trigger download + install via `nxm://` with no confirmation; Premium needs no key. | Confirmation dialog listing mod and file; require `key`/`expires`. |
| A-13 | Nexus | Registering the nxm handler silently takes over MO2/Vortex links; links for other games are then dropped. `Exec` quoting is not per spec. | Remember the previous handler, forward foreign domains to it, escape `Exec`. |
| A-14 | Setup | Game path is hardcoded; no Steam library detection (`libraryfolders.vdf`, `appmanifest_3321460.acf`), no first-run wizard (the spec required asking). | Auto-detect the game, plus a setup wizard when it isn't found. |
| A-15 | Updates | Game updates only show a badge. Steam "verify" may write through deployed symlinks into staged files. | Check `0.paver` at start and before Apply; guided "unlink → verify → re-snapshot". |
| A-16 | Nexus | Update check compares mod-page versions instead of the file `file_updates` chain; one 404/429 aborts the whole check; free-account `nxm` updates never replace. | Follow the file chain, collect per-mod results, honour `replace_target` for nxm. |
| A-17 | Collections | Nexus collections are not supported (see [Collections](#nexus-collections)). | New feature, after A-01. |

## Medium

| ID | Area | Finding |
|----|------|---------|
| A-18 | Startup | DB failure panics with no dialog; a sync failure shows an empty list as "Ready"; an invalid `config.toml` falls back to defaults and the next Save overwrites it. |
| A-19 | Jobs | Exclusive jobs are not FIFO (`std::sync::Mutex`) and can block every worker; every toggle starts a full conflict analysis (no debounce). |
| A-20 | DB | `sync` walks all mods inside `BEGIN IMMEDIATE`, so UI writes wait up to 10 s; one unreadable folder fails every import. |
| A-21 | Nexus | No rate-limit handling (`X-RL-*` headers, 429 backoff); a public release needs registration under the Nexus API AUP. |
| A-22 | UI | Stale details panel after updates; "No conflicts" shown while analysis is pending; Apply/Unlink/Clean Slate enabled while the game runs or exclusive jobs are queued. |
| A-23 | Security | `nxm` keys can appear in error messages and the journal; an existing key file keeps its old mode; `Client` derives `Debug` (latent API-key leak). |
| A-24 | Nexus | Expired `nxm` links give a misleading error; only the first mirror is tried; `.part` files are left behind; no resume; no size/MD5 check. |
| A-25 | Instance | The `nxm` queue can lose links (append vs rename race); a second start without arguments exits silently. |
| A-26 | Config | Jobs keep the config snapshot from submit time: a queued Apply can deploy into the old game folder after a change. JSON folder accepts `..`. |
| A-27 | Launch | Game detection is case-sensitive and matches any argv (false positives when an editor has the exe open); xdg-open fallback drops arguments; children are never reaped (zombies). |

## Low / polish

| ID | Finding |
|----|---------|
| A-28 | Missing manager features: search/filter, drag-and-drop (order and import), keyboard shortcuts, variant/FOMOD picker (multi-variant archives conflict with themselves), save backup, profile export/import. |
| A-29 | Packaging: no LICENSE, CHANGELOG, CI, window icon, launcher `.desktop`, release profile, AppImage/Flatpak. |
| A-30 | Unbounded growth: daily log files, per-mod logs survive deletion, `events` table. |
| A-31 | Tests missing for UI actions, instance hand-off, launch selection, Nexus HTTP (mocked), settings validation. |
| A-32 | Spec style: some public items lack rustdoc; two spots nest 8–9 levels; `match` on `bool` in several places. |
| A-33 | Small windows: the 8-column grid clips with the details panel open; long file lists are not virtualised. |

---

## Nexus collections

Facts, from live unauthenticated queries to `api.nexusmods.com/v2/graphql`:

- Crimson Desert supports collections: 5 published, e.g. "Visual & QoL" (38 mods, 2.3 GB) and
  "Crimson QOL" (26 mods).
- `collectionRevision(slug, revision, domainName)` → `modFiles[]` with `fileId`, `optional`,
  `updatePolicy` and `file { modId, name, version, sizeInBytes, uri }`, plus
  `externalResources` and `downloadLink`.
- In Crimson QOL, 13 of 26 entries are optional; every update policy is `latest`.
- The collection archive (load-order rules, installer choices, hashes) needs authentication
  (HTTP 401 without a key).
- The website's install button sends a collection-shaped `nxm://` link (exact format still
  to confirm); the current parser only accepts single files.
- **Dependency:** collections are mostly RAW/JSON mods, so they only become useful after A-01.

Target design — a collection is a parent mod with sub-mods:

1. `collections` table (slug, revision, name, installed_at) and `collection_members`
   (slug, mod_name, mod_id, file_id, optional, position); `mods.collection` points at the parent.
2. Install: fetch the revision and archive → optional-mod picker → one download job per member
   (parallel) → order by the collection's rules.
3. Premium downloads automatically. Free accounts get a "waiting for downloads" list that
   matches incoming `nxm://` links to pending members.
4. Mod list: a collapsible collection row; toggling it toggles all members.
5. Revision update = diff (add / remove / update members). Uninstall removes members no other
   collection or manual install uses.
