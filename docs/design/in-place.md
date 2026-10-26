# In-place deployment with base and version backups (D-14)

Status: design, 2026-10-26. Details of the game-side behaviour are confirmed against the JMM
analysis and an in-game test before implementation.

## Goals

1. Patched game files reach the game the way DMM's working route does: through the index of the
   group that holds the vanilla file.
2. Nothing irreversible: vanilla can always be restored exactly, without Steam's verification.
3. Every change is a version that can be rolled back to.
4. The game's large archives (`NNNN/*.paz`, ~0.9 GB each) are never rewritten.

## What changes in the game folder

| File | Change | Revert |
|------|--------|--------|
| `NNNN/K.paz` (K = next free index of the group) | **New**: Pylune's packed entries, a symlink into Pylune's data | remove the link |
| `NNNN/0.pamt` | Rewritten: records of patched files point to `K.paz` (offset, sizes, flags); the archive table lists `K.paz` with its checksum and size; header checksum updated | restore from backup |
| `meta/0.papgt` | `pamt_crc` of each changed group and the file checksum updated | restore from backup |
| `meta/0.pathc` | Only for textures (as today) | restore from backup |

Vanilla `.paz` files keep their bytes; vanilla records of files no mod touches keep theirs.
Entries for files the game does not have stay in Pylune's overlay group (if the control test
shows overlays work) or are added to the group's index (folder and name tries rebuilt).

## Backups

* **Base backup** (once per game build, before the first in-place change): the vanilla
  `0.pamt` of every group Pylune may change, `meta/0.papgt`, `meta/0.pathc`, plus the size and
  SHA-256 of every `.paz` of those groups. Stored under `~/.local/share/pylune/game-backup/base/`
  with the build fingerprint (`meta/0.paver`). Index files are small (KB–MB), so all groups'
  indexes can be kept.
* **Version backup** (every Apply that changes the in-place state): the written `0.pamt` files,
  `0.papgt`, `0.pathc`, Pylune's `K.paz` files and a manifest (which mods, which entries, hashes
  of everything written) under `game-backup/versions/<n>/`. Pylune's archive files are content
  addressed, so unchanged ones are shared between versions.
* Retention: base forever (until the game updates); versions per setting (default 10), the
  deployed one always kept.

## Operations

* **Apply**: build `K.paz` for each changed group from vanilla bytes (base backup + vanilla
  archives), write the new indexes to a staging folder, verify them with `paz::pamt_check` and by
  reading every patched entry back through the new index, journal the plan, then replace the
  index files (atomic rename) and link `K.paz`. Store the version.
* **Roll back to version N**: the same replacement with version N's files.
* **Revert to vanilla**: restore the base files, remove Pylune's archive links. Unlink all and
  Clean slate do this too.
* **Safety checks before any write**: the game is not running; the live index files equal either
  the base or the last version Pylune wrote (hash) — otherwise something else changed them (game
  update, Steam verification, another manager) and Pylune refuses and offers to refresh the base;
  the `.paz` sizes and hashes in the base still match.

## Game updates and Steam verification

A changed `0.paver` or index hash means the base is stale: Pylune stops, asks to unlink (which
restores what it can), verify in Steam, and take a new base. Steam's verification alone also
returns the game to vanilla (Pylune's extra `K.paz` is not a Steam file and is removed on
revert).

## Open questions (answered by the JMM analysis / in-game tests)

* Does the game accept an additional `.paz` in a vanilla group? (Vanilla groups already have
  several; the index lists them with checksums.)
* Encryption and compression of rewritten entries (inherit vanilla flags, as today).
* New files: overlay or index insertion.
