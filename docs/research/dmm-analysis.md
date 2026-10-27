# Definitive Mod Manager (DMM) — how it reaches the game

Analysed 2026-10-26 with the owner's permission: DMM 3.7.0 (Rust/Tauri, Windows build) by
targeted disassembly in a scratch folder (never run, nothing committed), cross-checked with
DMM 1.5.5.3's own logs and backup files in `~/.local/share/DMM/`. Labels: **[code]** seen in
the disassembly, **[log]** DMM's logs/backups, **[text]** DMM's own messages or changelog (the
author's claims, not proof of engine behaviour).

## Data tables: one overlay group per table [code][text]

JSON byte patches and field-level mods never go in place. Each changed table becomes its own
group, `dmmv3_<table>` (e.g. `dmmv3_iteminfo`), under the vanilla folder path
(`gamedata/binarystaticinfo__/bin`), holding the patched body **and its header** (rebuilt, or
the vanilla header unchanged when no offset moves); `.paseq` tables take their `.paseqc` along.
DMM re-parses body and header and refuses inconsistent pairs. Bodies keep the vanilla flags
(re-encrypted when the vanilla entry was); headers are stored uncompressed.
**A stack-size mod that worked through DMM therefore worked through an overlay group.**

## Registering a group in `0.papgt` [code]

Rebuilt from a clean copy: the new group's name is put **first in the string table** (all other
name offsets shift), the entry is **inserted at index 0** with `is_optional = 0` and
`lang_mask = 0x3FFF`; header unknowns are kept, the count grows by one, the checksum is
recomputed. All of DMM's groups end up before the vanilla groups. (JMM uses the same mask.)

## In place: replacements of existing non-table files [code][log]

XML, HTML, CSS, DDS, PAC, audio from loose mods ("to kill the 'works in other managers, CTDs on
DMM' bug class", changelog 1.4.0):

* the new entry is appended to the **owning group's `0.paz`** (even if the vanilla entry lived in
  another archive); a vanilla slot in `0.paz` is overwritten only if the new data fits, with the
  original bytes journaled;
* the record is patched (offset, sizes, `paz_index = 0`, flags inherited from vanilla); the
  archive table's `paz_crc` (over the whole `0.paz`) and size, the index checksum and the group's
  `pamt_crc` in `0.papgt` are updated; a self-check verifies sizes and bounds;
* new files are appended into their inferred owning group with a new record (the index is
  rebuilt); files without an owning group are skipped.

## Backups and restore [code][log]

`backups/vault/<build>/`: every group's `0.pamt`, `0.papgt`, `0.pathc`, `0.paver` (with
sha256) and, per group written in place, a **full copy of the vanilla `0.paz`** plus a journal
`"DMMJ" | u8 1 | u64 base_size | u32 n | n × (u64 offset | u32 len | bytes)`. Every mount
starts by restoring vanilla: truncate `0.paz` to `base_size`, replay the journal, restore the
index files; refused while the game runs. Build changes are detected through `0.paver`, an
embedded table of vanilla index hashes per build (2.01.00–2.03.02) and Steam's update state.

## DMM's claims about the engine [text] (unverified)

Cutscene dialogue and weapon/socket placement XML are "overlay-blind" (hence in place); loose
`.pabgb`/`.pabgh` on disk outrank every archive group; a group without a `0.papgt` entry is
never read; an older table in another numbered folder may win.

## Consequences for Pailune

1. Data tables: one group per table with its header, registered like DMM (name first, index 0,
   mask `0x3FFF`), header stored uncompressed — implemented 2026-10-26.
2. The in-place route (D-14) is needed only for files the engine reads outside overlays
   (per DMM: dialogue audio, weapon/socket XML). Pailune's design keeps the vanilla `.paz` files
   untouched (an added archive per group instead of appending to `0.paz` and copying 900 MB).
