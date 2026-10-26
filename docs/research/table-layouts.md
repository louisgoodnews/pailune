# Research: deriving the game's table layouts (A-01, format 3)

Owner decision (2026-10-26): derive Pylune's own layouts from the game instead of shipping
community schemas, so field definitions match the game exactly. This page records what the
game reveals and the method. Everything below was measured read-only on build 2.3.2.

## What the data says

* 134 tables in `0008/gamedata/binarystaticinfo__/bin/`: `*.staticinfobody` plus a
  `*.staticinfoheader` index; 128 parse with Pylune's index reader, 296 155 records in total.
* No schema ships with the data. Records are `key | u32 name_len | name | payload`; the
  payload has no type information.
* 24 tables have fixed-size payloads (e.g. `categoryinfo` 7 bytes, `gimmickgateinfo` 21,
  `actionpointinfo` 230, `characterappearanceindexinfo` 17); the rest vary, up to 1.5 MB per
  record (`questinfo`).

## What the executable says

* The data sections of `CrimsonDesert.exe` contain `Class` / ` _field` string pairs (likely
  serializer messages): 513 classes with their field names, e.g. 115 for `ItemInfo`.
* Nothing in the data sections points at these strings; the engine registers its reflection
  at runtime in code. That code is in protected sections (`.xtext`, `.xtls`), and Pylune does
  not analyse it — only plain data is read, nothing is executed, decrypted or modified.
* Sizes agree with names for small tables: `CategoryInfo` = `_isBlocked` (1) + three category
  indexes (2 each) = 7 bytes; `GimmickGateInfo` = `_isBlocked` (1) + `_position` (12) +
  `_rotation` (4) + `_fieldInfo` (4) = 21. `_key` and `_stringKey` are the record header.
* The string order matches the serialization order in runs: an `ItemInfo` record starts with
  `_isBlocked` (byte 0), `_maxStackCount` (u64 = 100), `_itemName` (localisation reference
  `"9448928051312"`), and `_brokenItemPrefixString` (`…313`) follows shortly after — the same
  order as the strings. Other parts of the record do not follow the string order directly.

## Method

1. **Names**: read the `Class _field` pairs from the player's own executable at runtime (no
   game data is redistributed), cached per game build.
2. **Layout inference per table**: candidate types per field from its name and context
   (`_is*` → bool, `*Key`/`*Info` → key of the referenced table, `*List` → `u32 count` +
   elements, localisation strings, `_position` → 3 floats, nested `*Data` classes recursively);
   the order from the string runs, refined by search.
3. **Verification, not guessing**: a layout is accepted only if it decodes **every record of
   the table with no byte left over**, and every reference field points at an existing key of
   its table. Fields whose order cannot be proven (same-sized neighbours) are marked and not
   offered for editing.
4. **Per build**: layouts are tagged with the game version (`0.paver`) and re-derived after
   updates; a table whose layout no longer verifies disables its format-3 patches with a clear
   message instead of writing wrong bytes.
5. **Format 3**: `set` (and raw `old/new`) on fields with verified layouts; engine names
   (`_maxStackCount`) and their snake_case forms (`max_stack_count`) are accepted.

## Order of work

Fixed-size tables first (proves the pipeline), then the tables mods target most — `iteminfo`,
`dropsetinfo`, `skill`, `storeinfo`, `characterinfo`, `buffinfo` — one at a time, each with its
own verification report.
