# CD JSON Mod Manager (JMM) compared with Pailune

Looked at 2026-10-26 from `local/test-mods/CD JSON Mod Manager/` (not committed). JMM is a
Windows .NET single-file application (80 MB) that embeds a Python engine. It was examined only
through the texts it carries (its localised messages, built-in help, and method names kept by
.NET); its code was neither decompiled nor run.

## What JMM does (from its own texts)

| Area | JMM | Pailune |
|------|-----|--------|
| JSON byte patches | Decompress the game file, apply `original`→`patched`, recompress, write into an **overlay group** | Same (`src/overlay/patches.rs`) |
| Overlay group | Folders `0036+` (`AllocateOverlayGroupNames`), entry added to `0.papgt` (`add_papgt_entry`) | `0100`+, first in `0.papgt`, deployed as symlinks |
| `0.papgt` backup | `_jmm_backups/0.papgt.original`, `0.papgt.pre-apply` to detect interrupted applies | `meta/0.papgt.bak` + write-ahead journal |
| Index/archive checksums | `RefreshPapgtPamtHashes`, `UpdatePamtPazCrc`, a `PAMT_UNKNOWN` constant | Same values written; strict check of every built index since 2026-10-26 (`paz::pamt_check`) |
| Language mask | `PAPGT_LANG_ALL` constant | `0x7fff`, as the game's 17 data groups |
| Textures | `UpdatePathcForTextures`, `UpdatePathcEntry` | Rebuilt `0.pathc` |
| Table index | `entry` + `rel_offset` resolved through the table index; `FixupPabghAfterInserts` | Same resolution; inserts into tables refused |
| Inserts | `type: "insert"` (v9.8.7+), variable-length patches | Inserts only outside tables |
| Encryption | Option "Encrypt overlay output" | Flags inherited from the replaced game file |
| Format 3 ("Field JSON v3") | Supported (`apply-field-json`) with its own field knowledge | Own derived layouts (21 tables + item stack sizes) |
| **PAZ replacement mods** | Also writes into the game's own archives, with backups ("PAZ replacement backups", "Verify Integrity" advice when backups are missing) | Never modifies game archives (D-01 spirit, overlay only) |
| ASI plugins | Copies ASI/DLL into `bin64/`, installs the loader | Links them, never installs a loader by itself |

JMM's help still names tables `gamedata/<name>.pabgb` / `.pabgh` (the pre-2.x names); none of
its readable texts mention `binarystaticinfo__` or `staticinfobody`.

## What this means for "mods have no effect" (2026-10-26)

* Pailune's deployed overlay passes every check that can be made outside the game: index
  checksum, folder hashes, sorted folders and names, contiguous ranges, archive checksums and
  sizes, 16-byte alignment, entry first in `0.papgt` with the data groups' language mask. The
  same checker passes every vanilla group.
* Save editing (direct write to the game's files) works in game; DMM's in-place route made
  *Fat Stacks* work.
* JMM uses the same overlay route as Pailune for JSON mods, but also an in-place route.

Open: whether the game reads data tables (`gamedata/binarystaticinfo__/…`) through the overlay
at all. The control test (title-screen CSS, which worked through Pailune's overlay on
2026-10-25) separates "overlay not active" from "tables not taken from overlays". If only the
tables are the problem, the alternative is an in-place route like DMM's/JMM's, which needs an
owner decision because Pailune so far never changes the game's archives.

## Decompilation (2026-10-26, authorised by the owner)

The bundle was unpacked and decompiled in a scratch folder (nothing committed, nothing run):
the C# assembly does all archive work; an embedded `CrimsonCLI.exe` (PyInstaller, Python 3.14,
with the Rust extension `crimson_rs`) handles Field JSON v3.

* **No game 2.x support.** Neither part knows `binarystaticinfo__`, `staticinfobody`,
  `0.paver` or `0.papk`. Table lookups use `gamedata/<name>.pabgb` /
  `gamedata/binary__/client/bin/…`; the CLI hard-codes group `0008` and `iteminfo.pabgb`.
  JMM therefore cannot change data tables of the current game build.
* **"PAZ replacement" is whole-file replacement**, not patching: a mod that ships a group's
  `0.paz` (and optionally `0.pamt`) has those files copied over the vanilla ones (originals
  backed up once to `_jmm_backups/paz_originals/`, never refreshed after game updates); the
  group's `pamt_crc` in `0.papgt` is updated. Byte-patch JSON always goes through an overlay.
* **Overlay:** groups `0036+` (it treats every group ≥ 36 as its own and deletes it — this
  would remove the game's optional voice-pack groups 0036–0040), inserted at index 0 with
  language mask `0x3FFF`; PAMT unknown `0x610E0232`; one trie node per path segment, flat file
  names; entries LZ4 + ChaCha20 when the vanilla entry is encrypted (Pailune: the vanilla flags).
  The companion table index is added only after size-changing inserts.
* **Checksums** as Pailune (`pa_hash`: `init = len + 0xDEBA1DCD`); a wrong PAMT checksum crashes
  the game according to JMM's own validator.

**Consequence:** JMM offers no route that is known to work for current data tables, and its
in-place route is a whole-group replacement unsuited to patches. The DMM analysis (its in-place
route made *Fat Stacks* work) decides the design of D-14.
