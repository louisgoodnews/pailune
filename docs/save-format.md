# Save-game format (B-07)

Status 2026-10-26: **container and body fully decoded by Pailune's own implementation**
(`src/savefile/`), verified read-only on all 10 real saves of build 2.3.2: every HMAC checks out,
re-encrypting the decrypted payload reproduces each file byte for byte, and every top-level
object (1 881 in total) decodes with no byte left over.

**Editing (2026-10-26, updated 2026-10-28 for variable lengths, `src/savefile/edit.rs`):**
Supports both fixed-size in-place modifications and variable-length edits (adding, duplicating,
and deleting items in dynamic lists such as `InventorySaveData` / `ItemSaveData`, as well as
counted items of kinds 1 and 3). When lengths change:
- All enclosing embedded object `size: u32` trailers are shifted and recalculated.
- Top-level TOC entry sizes are updated.
- All subsequent TOC entry offsets are shifted.
- The TOC header `stream_size` is updated.
- The body is re-compressed (LZ4 block), container `uncompressed` and `payload` sizes updated,
  encrypted with a fresh nonce derived from SHA-256 of the old nonce and the new body, HMAC-SHA256
  recomputed, and the rewritten save decoded in memory and structural validity verified across all
  TOC entries with `body::walk::object` before atomic file replacement. Automatic backup precedes
  any save modification. Verified in memory and unit tests. Copper coins: `ItemSaveData` with
  `_itemKey = 1` (`Money_Copper`), amount in `_stackCount`. Steam Auto-Cloud keeps a copy under
  `Steam/userdata/<id>/3321460/ac/WinAppDataLocal/…`.

## Files

`AppData/Local/Pearl Abyss/CD/save/<Steam account id>/` — on Linux inside the Proton prefix
(`compatdata/3321460/pfx/drive_c/users/steamuser/…`), on Windows in `%LOCALAPPDATA%`.

| File | Size (observed) | Likely content |
|------|-----------------|----------------|
| `slot<N>/save.save` | ~830 KB (≈ 3.1 MB unpacked) | The game state of that slot. |
| `slot<N>/lobby.save` | ~500 B (≈ 520 B unpacked) | Summary shown in the load menu. |
| `steam_autocloud.vdf` | 52 B | Marks the folder for Steam Cloud (`accountid`). |

Observed slots: 0, 1, 2 and 100, 101 (the latter probably autosaves). The archives also contain
367 `.save` files — templates shipped with the game.

## Container (`SAVE`, version 2)

All fields little-endian. Offsets confirmed on our files; field meanings for the crypto part
agree with the community library `crimson-rs` (see sources).

| Offset | Size | Field |
|--------|------|-------|
| 0x00 | 4 | Magic `SAVE` |
| 0x04 | 2 | Version (2) |
| 0x06 | 2 | `0x0080` — header size / flags |
| 0x08 | 10 | Unknown (`00 00 00 00 02 00 00 00 00 00`), kept verbatim |
| 0x12 | 4 | Unpacked size |
| 0x16 | 4 | Payload size (= file size − 128) |
| 0x1A | 16 | ChaCha20 nonce: initial block counter (`u32`) + 96-bit nonce; new per write |
| 0x2A | 32 | HMAC-SHA256 over the compressed (decrypted) payload |
| 0x4A | 54 | Padding — in `lobby.save` it contains stale memory (pointer-like values) |
| 0x80 | … | Payload: LZ4 block (not frame), then ChaCha20 (IETF) |

The payload has 8.0 bits/byte of entropy (encrypted). Our own experiments confirmed that the
archive key scheme (file-name seed, `docs/paz-format.md` §7.2) does **not** decrypt saves.
According to `crimson-rs`, the key is a constant from the game combined with a per-version
string, and the same key is used for the HMAC. The decrypted body is a typed object tree with a
table of contents; its field names come from the game's data tables (`iteminfo`, `questinfo`,
`knowledgeinfo`, … — they change between game patches).

## Risks for editing

- **Integrity:** the HMAC must be recomputed; a wrong one most likely makes the game reject
  the save.
- **Version drift:** layouts and field tables change with game patches (`crimson-rs` keeps
  per-patch key tables); edits that change lengths have crashed the game loader in community
  reports.
- **Steam Cloud** can sync a broken save to every machine — and back over a restored one.
- Pailune already backs up saves before every launch (A-28); any editor must work on copies
  and keep that backup.

## Key and checksum

32-byte key = the 31-byte base material `C4 1B 8E 73 0D F2 59 A6 37 CC 04 E9 B1 2F 96 68 DA 10
7A 85 3E 61 F9 22 4D B8 0A D7 5C 13 EF` XOR the first 31 bytes of (version prefix +
`PRIVATE_HMAC_SECRET_CHECK`), followed by `00`. Version 2 prefix: `^Pearl--#Abyss__@!!`;
version 1: ``^Qgbrm/.#@`zsr]\@rvfal#"``. The same key encrypts (ChaCha20, IETF, counter =
first 4 nonce bytes) and authenticates (HMAC-SHA256 over the LZ4-compressed payload).

## Body

The decompressed body (≈3 MB for `save.save`) is self-describing:

1. **Prefix** (14 bytes): `FF FF 04 00` and 10 more bytes.
2. **Schema** (from 0x0E): `u16 tag, u16 0, u16 type_count`, then the first type's name
   (`u32` length + text); per type `u16 field_count` and per field name, type name
   (length-prefixed), `u16 kind, u16 size, u32 aux`; after each type but the last, the next
   type's name. 80–85 types in our saves.
3. **Table of contents**: `u32 0, u32 count, u32 stream_size`, then per entry `u32 type,
   u32, u32, u32 offset, u32 size` (360–408 entries).
4. **Objects**: `u16 mask_len, mask, u32 reserved`, then the fields in schema order, present
   ones per the mask. Absent arrays and lists (kinds 3, 6, 7) write one `01` byte. Present:
   kind 0/2 a scalar of `size` bytes; 1 `u32 count` + items; 3 `00`, `u32 count` + items; 4/5 an
   embedded object (`u16 mask_len, mask, u16 type, u8, u32, u32, u32 offset`, `u32 reserved`,
   fields, `u32 size` of the payload; kind 5 may have up to 8 bytes before it); 6/7 a list
   (`00`, `u32 count`, 13 bytes, then `count` embedded objects).

Times (`Ctc64`, e.g. `_generatedTime`) are Unix seconds. Keys (items, quests, characters,
factions) refer to the game's data tables; names need its localisation files.

## What a save holds

Top-level objects of a save (slot 2, build 2.3.2), by type:

| Type | Content |
|------|---------|
| `CharacterStatusSaveData` | character, faction, level, experience, skill points, HP, MP |
| `EquipmentSaveData`, `InventorySaveData`, `InventoryItemContentsSaveData` | equipped items, 18 inventories with items (key, stack count, endurance, sockets), bank data |
| `QuestSaveData`, `QuestDialogSaveData` | stage, mission, quest and gauge states, last completed quest/mission |
| `KnowledgeSaveData`, `SkillUnlockMarkSaveData`, `PlayGuideSaveData` | learned knowledge and skills, skill points, guide hints |
| `FactionSaveData`, `FriendlySaveData`, `FieldNPCSaveData` (24) | factions and rewards, NPC friendship/affinity, NPC appearance |
| `MercenaryClanSaveData` | mercenaries, farm slots, call cooldowns |
| `TransformSaveData`, `FieldSaveData`, `FogSaveData`, `DiscoveredLevelGimmickSceneObjectSaveData`, `GameData_GimmickPointData` (357) | position per field, field gimmicks, map fog, discoveries |
| `StoreSaveData`, `RoyalSupplySaveData`, `ContentsMiscSaveData` | stores, royal supply, mini-games, house, misc |
| `CustomizationSaveData` | character customisation mesh and decorations |
| `GameEventSaveData`, `GamePlayVariableSaveData`, `SubLevelSaveData`, schedule managers | world events, gameplay variables, sub-levels, NPC schedules |

`lobby.save` holds the slot summary (`LobbySaveData`): character, level, save time, display
mission and quest. Next to the slots, `initialize_input.save` (input settings, same object
serialisation without the container) and `user_engine_option_save.xml` (graphics options).

## Decisions

- 2026-10-26: Pailune implements its own decoder (owner). The scheme is credited to `crimson-rs`.
- Editing will need: re-encoding with a fresh nonce and HMAC (implemented and tested), a body
  writer (the walk already keeps exact positions), and validation per game version.

## Sources

- crimson-rs, `src/save/mod.rs` (MIT): https://github.com/bbfox0703/crimson-rs
- NattKh, Crimson Desert save editor: https://github.com/NattKh/CRIMSON-DESERT-SAVE-EDITOR-AND-GAME-MODS
- Save editors on Nexus Mods: https://www.nexusmods.com/crimsondesert/mods/20,
  https://www.nexusmods.com/crimsondesert/mods/3395
