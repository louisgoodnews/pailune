# Crimson Desert archive formats and community mod formats: implementation spec for Pylune

Status: research document, written 2026-10-25 against the user's install
(`/mnt/nvme0n1/nvme0n1p1/louisgoodnews/steamapps/common/Crimson Desert`,
`meta/0.paver` = version **2.3.2**). It is meant to be enough to implement
Pylune's own overlay archive group, so mods stop depending on loose files.
The game ignores loose files.

Legend:

* **VERIFIED**: checked byte for byte against the user's real files with
  scripts (see §12 "Verification log").
* **SOURCE**: taken from one or more community tools (see §13) and
  consistent with the real files, but not independently proven on a running game.
* **UNVERIFIED**: claimed by a tool or inferred, not checked. Treat it as a
  hypothesis and test it before relying on it.

All integers are **little-endian**. "u8/u16/u32" are unsigned. Offsets are
in bytes from the start of the structure named.

---

## 0. TL;DR for the implementer

1. One hash function covers almost everything: **`pa_hash(bytes) =
   lookup3::hashlittle(bytes, initval = 0x000C5EDE)`** (Bob Jenkins'
   lookup3, the primary `c` result). It is used for the PAPGT file CRC,
   the PAMT CRCs, the PAZ CRCs, the PAMT folder hashes, the PATHC path
   hashes, the 0.paver checksum and the ChaCha20 key seed. VERIFIED on every
   meta file and on all 35 `0.pamt` files on disk.
2. The VFS root is `meta/0.papgt`, a list of *pack groups* (directories such
   as `0000`…`0035`). Each group has `0.pamt` (index) plus `0.paz`, `1.paz`, …
   (data). The PAPGT stores `pa_hash(pamt[12..])` for every group, and its own
   header stores `pa_hash(papgt[12..])`. Change any PAMT and you must refresh
   that group's PAPGT entry and the PAPGT file CRC.
3. **An overlay group** is a new directory (`<name>/0.pamt`, `<name>/0.paz`)
   plus one new PAPGT entry. JSON Mod Manager (JMM) and CDUMM insert it as
   entry **index 0** of the PAPGT ("first match wins"). The shadowed vanilla
   group's PAMT is **not** modified (CDUMM even restores it to vanilla).
   **VERIFIED IN GAME (2026-10-25, build 2.3.2):** Pylune's overlay group `0100`,
   listed first in a rebuilt PAPGT, replaced `ui/xml/gamemain/title/titleview.css`
   (flags 0x32) on the title screen.
4. File entries in a PAZ are aligned to 16 bytes and zero padded, and the PAZ
   file length is padded to 16 too. Per-entry `flags`: low nibble =
   compression (0 none, 1 DDS "partial", 2 LZ4 block), next nibble =
   encryption (3 = ChaCha20, key and IV derived from the lowercase basename).
   VERIFIED.
5. Textures (`.dds`) also depend on `meta/0.pathc`, the game's
   "TextureHeaderCollection". It maps `pa_hash("/" + vpath)` to a 148-byte
   DDS header template and four per-mip chunk sizes. Replacing or adding a DDS
   generally means patching PATHC as well. Texture mods are the hardest part.
6. Recommended first milestone (§11.4): a read-only `paz` crate (PAPGT, PAMT,
   PATHC parsers, extraction with ChaCha20 and LZ4), validated against copies
   of the user's meta files. Then an overlay writer for non-texture files.

---

## 1. Common primitives

### 1.1 `pa_hash` (lookup3 hashlittle, initval 0xC5EDE)

Standard `hashlittle()` from lookup3.c, byte-oriented (the endian-neutral
path: read 12-byte blocks as three LE u32, zero-pad the tail block), with
`initval = 0x000C5EDE`:

```
a = b = c = 0xDEADBEEF + len + 0x000C5EDE        (wrapping u32)
while len > 12:
    a += le32(k[0..4]); b += le32(k[4..8]); c += le32(k[8..12])
    mix(a,b,c)        # rot 4,6,8,16,19,4 (standard lookup3 mix)
    k += 12; len -= 12
if len == 0: return c                  # NOTE: no final() on an empty tail
zero-pad the remaining 1..12 bytes to 12; a += le32(t[0..4]); b += le32(t[4..8]); c += le32(t[8..12])
final(a,b,c)         # rot 14,11,25,16,4,14,24 (standard lookup3 final)
return c
```

Notes:
* MrIkso's `PaChecksum` writes the init as `len - 0x2145E233`. That is the
  same thing, because `0xDEADBEEF + 0xC5EDE ≡ -0x2145E233 (mod 2^32)`.
  MrIkso's version returns 0 for empty input, while real lookup3 returns
  `0xDEBA1DCD`. The game uses **0xDEBA1DCD** for the empty root folder name
  (see §4.6), so implement real lookup3.
* The input length must be known up front (it seeds the state). Streaming a
  large PAZ is still possible when its total size is known.

Test vectors (VERIFIED with both a Python and a Rust implementation):

| input (bytes)                         | `pa_hash`    |
|---------------------------------------|--------------|
| `""` (empty)                          | `0xDEBA1DCD` |
| `"gamedata"`                          | `0xAA2265B1` |
| `"rendererconfigurationmaterial.xml"` | `0xAF3DCEF3` |
| `"paconfig.txt"`                      | `0xA1A8E934` |
| `02 00 03 00 02 00` (paver version)   | `0xCB19CCE3` |
| current `meta/0.papgt[12..]`          | `0x2FB574BF` |

Rust: write the ~40 lines yourself (that is what all the tools do). The
`hashers`/`lookup3` crates on crates.io exist, but check their exact
semantics (byte-wise LE reading, empty-input result) against the vectors
above before using one.

### 1.2 Path conventions

* Virtual paths are lowercase, `/`-separated, **no leading slash** inside
  PAMT (`ui/texture/image/x.dds`). PATHC hashes them **with** a leading slash
  (`/ui/texture/image/x.dds`), see §6.
* Names are stored as UTF-8 with u8 length prefixes. Max segment length is
  255 bytes. The longest full path on this install is 190 bytes.

---

## 2. On-disk layout

```
<game>/
  bin64/CrimsonDesert.exe
  meta/0.papgt      group table (VFS root)          679 B
  meta/0.pathc      texture header collection       7 094 584 B
  meta/0.paver      game version + checksum         10 B
  meta/0.papk       32 opaque bytes (unknown; never touch)
  0000/0.pamt 0000/0.paz … 0000/35.paz             pack group 0000 (36 PAZ files)
  …
  0035/…
  dmmsa/0.pamt dmmsa/0.paz dmmsa/.dmm_owned        DMM leftover overlay (NOT registered any more)
```

Executable strings confirm how paths are built: `%s/meta/%s`, `0.papgt`,
`%s/%s/%s` → `<root>/<groupName>/0.pamt`, and `%s/%s/%d.paz` →
`<root>/<groupName>/<pazIndex>.paz` (decimal, no zero padding). Error strings
`packMeta Crc` and `. file(%u) papgt(%u) pamt(%u)` show the loader comparing
the PAPGT's stored CRC with the PAMT's CRC. The group name is an arbitrary
string taken from the PAPGT string table, so non-numeric names like `dmmsa`
are representable (SOURCE: DMM used `dmmsa`).

State of the user's install (VERIFIED):
* `meta/0.papgt`, `meta/0.pathc`, `meta/0.paver` are dated 2026-09-28 and are
  **vanilla for game 2.3.2**. A Steam update overwrote DMM's modified
  PAPGT/PATHC. The current PAPGT has **no `dmmsa` entry**, so the game no
  longer loads `dmmsa/` at all; it is dead weight that can be deleted.
* DMM's `~/.local/share/DMM/backups/papgt_clean.bin` and `pathc_clean.bin`
  are vanilla files of an **older** build (1.15.0, paver
  `01 00 0f 00 00 00 …`), captured 2026-07-26. They cannot be diffed against
  DMM's output (that output is gone), only against the current vanilla.
* DMM's July mount also wrote straight into vanilla `0009/0.paz`,
  `0012/0.paz` and their `0.pamt` (13 "in-place" injections). The Steam
  update replaced those files (all current PAMT CRCs match the current
  PAPGT).

---

## 3. `meta/0.paver` (10 bytes), VERIFIED

| off | size | field     | current value |
|-----|------|-----------|---------------|
| 0   | u16  | major     | 2 |
| 2   | u16  | minor     | 3 |
| 4   | u16  | patch     | 2 |
| 6   | u32  | checksum  | `pa_hash(bytes[0..6])` = `0xCB19CCE3` |

Pylune should **read it, never write it**. Use `(major, minor, patch,
checksum)` as the "game build fingerprint" for invalidating caches and
vanilla snapshots after game updates. DMM's log showed `01000f000000e188846a`
for the July build (1.15.0).

---

## 4. `meta/0.papgt`: pack group tree, VERIFIED

### 4.1 Layout

```
Header (12 bytes)
  0x00 u32  unknown0       build-specific, preserve verbatim (current 0x000012FA; 1.15.0 had 0x00000D5F)
  0x04 u32  file_crc       pa_hash(file[0x0C .. EOF])
  0x08 u8   group_count    N (so max 255 groups)
  0x09 u16  unknown1       0x0002 in both builds seen; preserve
  0x0B u8   zero           0
Entries (N × 12 bytes) starting at 0x0C
  +0  u8   is_optional     1 = may be absent on disk (DLC / language-pack slot)
  +1  u16  lang_mask       language bit mask (0x7FFF = all, in build 2.3.2)
  +3  u8   zero            0
  +4  u32  name_offset     byte offset into the string table
  +8  u32  pamt_crc        must equal the first u32 of <group>/0.pamt (= pa_hash(pamt[12..]))
String table
  u32  string_table_size  (S)
  S bytes  NUL-terminated ASCII names, concatenated
EOF = 0x0C + 12·N + 4 + S          (no trailing data)
```

The names are **variable length** (`dmmsa\0` is 6 bytes). MrIkso's
`PapgtReader` assumes fixed 5-byte names; do not copy that assumption.
JMM and CDUMM rebuild the string table from scratch.

`lang_mask` bits (from LukeFZ/pycrimson via MrIkso): KOR=1<<0, ENG=1<<1,
JPN=1<<2, RUS=1<<3, TUR=1<<4, SPA_ES=1<<5, SPA_MX=1<<6, FRE=1<<7, GER=1<<8,
ITA=1<<9, POL=1<<10, POR_BR=1<<11, ZHO_TW=1<<12, ZHO_CN=1<<13. Build 2.3.2
adds bit 14 (group `0033`, lang `0x4000`), so "all" is now **0x7FFF**. The
tools' default for new entries is still 0x3FFF, which leaves out bit 14.
Pylune should copy the mask of a vanilla all-language entry such as `0000`.
UNVERIFIED: whether the game drops a group whose mask misses the user's
language bit (that is the likely meaning).

### 4.2 Current vanilla content (2.3.2), VERIFIED

39 groups: `0000`–`0017`, `0019`–`0033`, `0035`–`0040`. (`0018` and `0034`
do not exist.)

| group(s) | is_optional | lang_mask | on disk |
|----------|-------------|-----------|---------|
| 0000–0004, 0007–0017 | 0 | 0x7FFF | yes |
| 0005 / 0006 | 0 | 0x0001 / 0x0002 | yes (KOR / ENG data) |
| 0019–0032 | 0 | 1<<0 … 1<<13 | yes (per-language packs) |
| 0033 | 0 | 0x4000 | yes |
| 0035 | 0 | 0x2000 | yes |
| 0036–0040 | **1** | 0x0080, 0x0100, 0x0004, 0x0800, 0x0020 | **no**: optional voice-pack placeholders (FRE, GER, JPN, POR_BR, SPA_ES) |

So in 2.3.2 a PAPGT may legitimately list groups that are missing, as long
as they are `is_optional=1`. In the older 1.15.0 file the language packs
were `is_optional=1` while present. The flag means "may be missing", not
"is a language pack". CDUMM reports (SOURCE) that an overlay placed in one
of the 0036–0040 slots inherits the optional/language gating and is
silently never mounted. **Never reuse a name that the vanilla PAPGT
lists.**

Header worked example (current file, first 0x24 bytes):

```
00000000: fa12 0000 bf74 b52f 2702 0000 00ff 7f00  unknown0=0x12FA crc=0x2FB574BF N=0x27(39) unk1=0x0002 0
00000010: 0000 0000 3753 8e72 00ff 7f00 0500 0000  entry0: opt=0 lang=0x7FFF name@0 ("0000") crc=0x728E5337
00000020: fdd0 09b6 …                               entry1: … name@5 ("0001") crc=0xB609D0FD
… string table size at 0x1E0 = 0x000000C3 (195 = 39 × 5), then "0000\0" "0001\0" …
```

`pa_hash(file[12..]) = 0x2FB574BF` = header CRC (VERIFIED), and every
`pamt_crc` equals the corresponding on-disk `0.pamt` header (VERIFIED for all
34 present groups).

### 4.3 Diff: DMM's `papgt_clean.bin` (1.15.0) against the current vanilla (2.3.2)

This is not a DMM before/after diff (see §2), but it shows what changes
between builds:
* unknown0 0x0D5F → 0x12FA; group count 33 → 39 (added `0033`, `0036`–`0040`).
* All `pamt_crc` values changed (every PAMT is rebuilt by patches).
* Language groups went from `is_optional=1` to `0`. The "all" mask was
  already 0x7FFF in 1.15.0, so it did not change.

DMM's own record of what it did (`~/.local/share/DMM/backups/mount_log.txt`,
`mount_log.jsonl`): `PAPGT_REBUILD preserved=33 dmm_dropped=0
foreign_dropped=0 dmm_added=1`, `[PAPGT_DMM_ADDED] dmmsa (CRC=0x14F04663)`.
`0x14F04663` is exactly the header CRC of the `dmmsa/0.pamt` still on disk
(VERIFIED). DMM's insertion position is not logged. UNVERIFIED.

### 4.4 Registering a new group (algorithm)

```
read vanilla papgt (from Pylune's snapshot, not the live file if it may be dirty)
entries = parse(); remove any previous Pylune entry
new = { is_optional: 0, lang_mask: entries["0000"].lang_mask (0x7FFF), zero: 0,
        name: "<pylune group>", pamt_crc: le32(new_pamt[0..4]) }
entries.insert(0, new)                     # JMM + CDUMM: index 0 = highest priority
rebuild string table: for each entry in order, name_offset = len(table); table += name + "\0"
out = unknown0 | 0 | u8(len(entries)) | unknown1 | zero
    | entries (12 B each) | u32(len(table)) | table
out[4..8] = le32(pa_hash(out[12..]))
write atomically (write temp + rename) after backing up the vanilla file
```

Also refresh `pamt_crc` for any group whose PAMT you changed. Pylune's design
should never change vanilla PAMTs.

---

## 5. `NNNN/0.pamt`: pack meta (group index), VERIFIED

All 35 PAMTs on disk (34 vanilla plus `dmmsa`) parse to exactly EOF with
this layout, and their header CRCs verify.

### 5.1 Layout

```
Header
  0x00 u32  pamt_crc        pa_hash(file[0x0C .. EOF])
  0x04 u32  paz_count       P
  0x08 u32  unknown         0x610E0232 for most groups, 0 for 0004/0005/0006/0035; copy 0x610E0232 for overlays (JMM/CDUMM/DMM do)
PAZ table (P × 12 bytes)
  +0 u32 paz_index          0..P-1 (file name "<paz_index>.paz")
  +4 u32 paz_crc            pa_hash(whole <paz_index>.paz)        (VERIFIED on 0003, 0013, 0016, dmmsa)
  +8 u32 paz_size           file size of that .paz in bytes
Folder-name block
  u32 dir_block_size
  dir_block_size bytes      trie nodes (§5.2)
File-name block
  u32 name_block_size
  name_block_size bytes     trie nodes (§5.2)
Folder table
  u32 folder_count
  folder_count × 16 bytes:
    +0  u32 folder_hash     pa_hash(full folder path bytes, no leading/trailing '/'); root = pa_hash("") = 0xDEBA1DCD
    +4  u32 name_offset     node offset in folder-name block (0xFFFFFFFF for the root/empty folder)
    +8  u32 first_file      index of first file record of this folder
    +12 u32 file_count      number of consecutive file records
File table
  u32 file_count
  file_count × 20 bytes:
    +0  u32 name_offset     node offset in file-name block
    +4  u32 paz_offset      byte offset of the entry inside <paz_index>.paz
    +8  u32 stored_size     bytes occupied in the PAZ (after compression/encryption) ("comp_size")
    +12 u32 orig_size       decompressed size ("decomp_size")
    +16 u16 paz_index       which .paz of this group
    +18 u16 flags           low nibble: compression; bits 4..7: encryption (§7.1)
EOF
```

(lazorr410's parser reads `+16` as one u32 `flags` and takes `flags & 0xFF`
as the PAZ index and `(flags >> 16) & 0xF` as the compression. That is the
same bytes.)

Because offsets are u32, a single PAZ cannot exceed 4 GiB. Vanilla PAZ
files are about 0.9 GB each.

### 5.2 Name tries (both blocks)

Each node is `u32 parent_offset` (0xFFFFFFFF = none), `u8 len`, then `len`
bytes of UTF-8. The full string of a node is the concatenation of all
ancestors' fragments plus its own, root first:

```
fn full_name(block, off) = if off == 0xFFFFFFFF { "" }
                           else { full_name(block, parent(off)) + fragment(off) }
```

Fragments are **arbitrary prefixes, not path segments** (a radix trie):
* vanilla `0003` file names: node `"abyss"` (parent none) has children
  `"dissolve.material"` and `"multitextured3.material"`;
* vanilla folder nodes: `"material"`, then `"/dist"` (parent = "material");
* DMM's `dmmsa` folder block: `"ui"` → `"/"` → `"texture"` → `"/image"` → `"/customizeimage"`.

Readers must only concatenate. Writers may use any decomposition. The
simplest valid choice is the one JMM and CDUMM use: one node per folder path
segment (`"ui"`, `"/texture"`, …) and one flat node per file name
(parent 0xFFFFFFFF). The game accepts it (SOURCE).

### 5.3 Ordering and invariants observed in vanilla, VERIFIED

* The folder table is sorted by **folder name** (byte order), not by hash.
* Inside a folder the file records are sorted by file name. Folders cover
  the file table contiguously: `first_file` of folder i+1 = `first_file + file_count` of folder i,
  and the total equals `file_count`.
* Vanilla registers **every intermediate folder** (for example
  `gamedata/binarygimmickchart__` with `file_count = 0`). JMM, CDUMM and DMM
  overlays register only folders that contain files, and they work (SOURCE).
* Files directly at the root use folder name `""`: `name_offset = 0xFFFFFFFF`,
  `folder_hash = 0xDEBA1DCD` (see `0016`).
* PAZ offsets are always multiples of 16.

Pylune's writer should follow the vanilla conventions anyway (sorted
folders, sorted names, contiguous ranges). It costs nothing. Whether to
include intermediate folders is optional.

### 5.4 Worked example: `0013/0.pamt` (174 bytes, complete)

```
00: f3d9ba0e                pamt_crc 0x0EBAD9F3 (= papgt entry for 0013)
04: 01000000                paz_count 1
08: 32020e61                unknown 0x610E0232
0c: 00000000 8245715e 50241000   paz[0]: index 0, crc 0x5E714582, size 0x102450 (1 057 872)
18: 18000000                dir_block_size 24
1c: ffffffff 08 "aiscript"           node@0  -> "aiscript"
29: 00000000 06 "/bin__"             node@13 -> "aiscript/bin__"
34: 26000000                name_block_size 38
38: ffffffff 0b "aichart.pai"        node@0
48: ffffffff 11 "pathfindtable.pai"  node@16
5e: 02000000                folder_count 2
62: dc45ade9 00000000 00000000 00000000   hash("aiscript")=0xE9AD45DC, node 0, files [0,0)
72: c4826ad3 0d000000 00000000 02000000   hash("aiscript/bin__")=0xD36A82C4, node 13, files [0,2)
82: 02000000                file_count 2
86: 00000000 00000000 81f60f00 bd5c3600 0000 0200   aichart.pai: paz 0 @0, stored 1 046 145, orig 3 562 685, flags 0x0002 (LZ4)
9a: 10000000 90f60f00 b62d0000 87230300 0000 0200   pathfindtable.pai: @0x0FF690, stored 11 702, orig 205 703, LZ4
```

Note that `0x0FF690 = align16(0 + 1 046 145)`. The DMM overlay
`dmmsa/0.pamt` (22 449 bytes, 399 DDS files, one folder) was **rebuilt byte
for byte** from its parsed contents with this layout (VERIFIED, §12).

### 5.5 Resolving a virtual path in one group

```
dir, base = rsplit_once(vpath, '/')  (dir = "" for root files)
h = pa_hash(dir)
for each folder record with folder_hash == h and full_name(dir_block, name_offset) == dir:
    for i in first_file .. first_file+file_count:
        if full_name(name_block, files[i].name_offset) == base: return files[i]
```

Build a `HashMap<String, FileRecord>` once per group (2.05 M files across
the install; about 1 s in native code). Compare case-insensitively when
matching mod paths, but write lowercase names.

---

## 6. `meta/0.pathc`: texture header collection, VERIFIED

The executable names it `TextureHeaderCollection`. It is a global
(not per-group) index of every DDS texture, keyed by path hash. There is
**no checksum** in the file: header u32s 0 and 1 are 0 in both builds. CDUMM
once wrote a PAPGT-style CRC there and broke every texture mod (SOURCE,
comment in `pathc_handler.py`).

### 6.1 Layout

```
Header (0x1C bytes = 7 × u32)
  0x00 u32 unknown0            0
  0x04 u32 unknown1            0   (NOT a CRC; leave as read)
  0x08 u32 record_size         0x94 = 148 (DDS header incl. DX10 extension)
  0x0C u32 record_count        R   (2.3.2: 0x28E = 654; 1.15.0: 0x284 = 644)
  0x10 u32 hash_count          H   (2.3.2: 0x472CB = 291 531; 1.15.0: 282 858)
  0x14 u32 collision_count     C   (12 in both)
  0x18 u32 collision_blob_size B   (0x2DC = 732)
DDS template records   R × record_size     at 0x1C
Key hashes             H × u32             sorted ascending, unique (binary search)
Map entries            H × 20 bytes, parallel to key hashes:
   +0  u32 selector
   +4  u32 m1   +8 u32 m2   +12 u32 m3   +16 u32 m4     (per-mip chunk sizes, §7.4)
Collision entries      C × 24 bytes:
   +0 u32 path_offset (into blob)  +4 u32 record_index  +8..+24 m1..m4
Collision blob         B bytes, NUL-terminated full paths ("/ui/texture/…dds")
EOF
```

* **Key** = `pa_hash(lowercase("/" + vpath))`, for example
  `pa_hash("/ui/texture/icon/itemicon_kebail_leather_armor_iii.dds")`.
  VERIFIED: 5000 of 5000 sampled `.dds` paths from `0000/0.pamt` are present.
* **selector**: if `selector >> 16 == 0xFFFF` then
  `record_index = selector & 0xFFFF`. Otherwise the hash collides, and the
  selector is `(end << 24) | (start << 16) | 0xFFFF`, where collision entries
  `[start, end)` hold the candidates, to be disambiguated by full path.
  VERIFIED: six colliding hashes, each with 2 paths, selectors
  0x0200FFFF, 0x0402FFFF, … 0x0C0AFFFF.
* **Template record** = the file's DDS header (128 bytes, or 148 bytes with
  DX10), zero-padded to 148, with `reserved1[0..11]` (bytes 32..75)
  zeroed. Byte 124 (`dwReserved2`, called "last4" by the tools) holds a
  format class (observed 4, 12, 13, 15; JMM's table: DXT1→12, DXT3/5→15,
  BC4/BC5→4). Records are deduplicated: many textures share one record.
* `m1..m4` equal exactly the `reserved1[0..4]` values stamped in that
  texture's PAZ-stored header (VERIFIED on 40 of 40 samples).

### 6.2 What DMM changed

From the DMM logs: `PATHC_WRITE_DBG … writing 283257 map-entries, 645
records`, and `DX10 DDS PATHC patched: <file> → idx=644, vpath=/ui/…` for
each overlay DDS. Against `pathc_clean.bin` (H = 282 858, R = 644), DMM
**appended one template record** (index 644, shared by all 399 identical
256×256 1-mip textures) and **inserted 399 new key/map pairs** in sorted
order (282 858 + 399 = 283 257). That matches `dmmsa/0.pamt` exactly (399
files). None of these keys are in the vanilla PATHC (VERIFIED). So DMM used
the overlay for **new** textures, and PATHC had to learn about them.

### 6.3 PATHC update algorithm (add or replace texture `vpath`)

```
rec = dds_header(new_file)[0..148] padded; rec[32..76] = 0; rec[124..128] = last4
idx = records.position(rec) or push(rec)
m   = chunk sizes as stored in the overlay payload (§7.4)
h   = pa_hash(lower("/" + vpath))
if h in hashes (binary search):
    if selector is a collision selector: update the matching collision entry (by path)
    else: map[i] = (0xFFFF0000 | idx, m1..m4)
else: insert h and its map entry at the sorted position
rewrite header counts; serialize; no checksum to fix
```

Keep the vanilla PATHC as the base for every rebuild and re-apply all of
Pylune's textures. Never edit incrementally on top of an unknown live file.

---

## 7. `NNNN/N.paz`: data files, VERIFIED

### 7.1 Container

* No header and no footer. Entries are raw blobs at `paz_offset`, each
  starting at a multiple of **16**. The gap after each entry is filled with
  **zero bytes**, and the file length is padded up to a multiple of 16
  (dmmsa: last entry ends at 4 678 212, file is 4 678 224; 0016: 3 846 →
  3 856).
* The PAMT records `paz_crc = pa_hash(entire .paz)` and `paz_size`.
  UNVERIFIED: whether the game checks `paz_crc` at runtime (DMM and CDUMM
  always recompute it). Always write correct values.

`flags` (u16 at file-record +18):

| bits | meaning | values seen (whole install, 2 052 922 files) |
|------|---------|-----------|
| 0–3 | compression: 0 none, 1 DDS partial (§7.4), 2 LZ4 block; MrIkso lists 3 = zlib, 4 = QuickLZ, lazorr lists 3 = custom, 4 = zlib (UNVERIFIED, never observed) | 0: 1 259 604; 2: 365 205; 1: 324 110 |
| 4–7 | encryption: 0 none, 3 ChaCha20; MrIkso lists 1 = ICE, 2 = AES (never observed) | 0x30: 54 780; 0x32: 49 223 |

So `0x0032` = ChaCha20 + LZ4, `0x0030` = ChaCha20 + stored. Decode order:
**read stored_size bytes → decrypt (if enc = 3) → decompress (by comp)**.
Encode order is the reverse: compress, then encrypt.

Encryption is decided by the flag, **not by extension**. Encrypted
extensions on this install include `.pami` (70 582), `.pac_xml`, `.xml`,
`.app_xml`, `.prefabdata_xml`, `.paccd_xml`, `.uianiminit`, `.paloc`,
`.material`, `.save`, `.pars`, `.css`, `.html`, `.spline*`, `.thtml`,
`.technique`, `.txt`, … (counts in §12). CDUMM moved from an extension list
to flag-based detection for this reason. For a replacement file, **inherit
the flags of the vanilla entry it shadows**. For new files, use the flags of
vanilla siblings with the same extension.

### 7.2 ChaCha20 (VERIFIED: decrypted real `.txt`, `.material` and `.xml`
entries in 0016/0003/0008; Rust `chacha20 0.10` matches)

```
seed  = pa_hash(lowercase(basename(vpath)).utf8)      # basename only: "abyssdissolve.material"
iv16  = le32(seed) × 4                                # 16 bytes
kb    = seed ^ 0x60616263
key32 = concat(le32(kb ^ d) for d in
          [0x00000000,0x0A0A0A0A,0x0C0C0C0C,0x06060606,0x0E0E0E0E,0x0A0A0A0A,0x06060606,0x02020202])
cipher: ChaCha20, 20 rounds, OpenSSL/"cryptography" 16-byte-IV convention:
        iv16[0..4] = initial 32-bit block counter (= seed), iv16[4..16] = 96-bit nonce
```

Example (lazorr410): `rendererconfigurationmaterial.xml` → seed
`0xAF3DCEF3`, key `90ac5ccf 9aa656c5 9ca050c3 96aa5ac9 9ea252c1 9aa656c5 96aa5ac9 92ae5ecd`.

Rust (verified by test program, §12):
```rust
use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
let mut c = chacha20::ChaCha20::new(&key.into(), &nonce12.into()); // nonce12 = le32(seed)×3
c.seek(seed as u64 * 64);                                          // counter = seed
c.apply_keystream(&mut buf);
```
Edge case: the IETF variant has a 32-bit counter. If `seed + ceil(len/64)`
exceeds 2^32, OpenSSL carries into the next state word, while the
`chacha20` IETF type errors or panics. `chacha20::ChaCha20Legacy` (feature
`legacy`, 64-bit counter) with `nonce8 = le32(seed)×2` and
`seek(((seed as u128) | (seed as u128) << 32) * 64)` reproduces OpenSSL's
carry behaviour and produced identical output in the test. No file on this
install hits the edge (VERIFIED scan of all 104 003 encrypted entries).
UNVERIFIED: whether the game's own implementation carries. It cannot
matter today.

### 7.3 LZ4 (VERIFIED)

Plain **LZ4 block** format: no frame, no size prefix. The decompressed size
is `orig_size` from the PAMT. `lz4_flex::block::decompress(buf, orig_size)`
works. Re-compressing with `lz4_flex::block::compress` gives different
(slightly larger) output than the game's encoder (3 881 vs 3 846 bytes for
`paconfig.txt`). That is irrelevant for an overlay, where sizes are free,
and fatal for "same-size in-place" patching, which Pylune should not do.

### 7.4 Compression type 1: DDS "partial" (VERIFIED on 1 600 sampled vanilla textures)

The stored payload is the DDS file with its first mip levels optionally
LZ4-compressed per mip, and the chunk sizes stamped into the DDS header:

```
stored = header (128 B, or 148 B if FourCC == "DX10")
       + chunk_0 + chunk_1 + … + chunk_{n-1}      n = min(4, mipCount)
       + raw bytes of mips n.. (unchanged)
header.reserved1[0..4] (bytes 32..48) = m1..m4 = stored size of chunk_i
header.reserved1[4..11] (bytes 48..76) = 0
header.dwReserved2 (byte 124)          = "last4" format class
chunk_i is LZ4-block(mip_i) if m_i < raw_size(mip_i), else raw mip_i
PAMT: orig_size = full standard DDS size; stored_size = len(stored)
```

Observed special case: for **1-mip** textures, vanilla writes `m1 = stored
size of mip 0` and `m2 = raw size of mip 0`, with m3 = m4 = 0 (e.g. raw
`(131072, 131072, 0, 0)`, LZ4 `(23294, 262144, 0, 0)`). DMM's and CDUMM's
writer (ported from JMM `BuildPartialDdsPayload`) produces that shape. For
mipCount 2–4 the vanilla semantics of `m2..m4` are UNVERIFIED (none in the
samples).

`raw_size(mip)` uses BC block math: `max(1,(w+3)/4) · max(1,(h+3)/4) ·
block_bytes`, with block_bytes 8 for DXT1/BC1/BC4/ATI1, 16 for
DXT3/DXT5/BC2/BC3/BC5/ATI2/BC6H/BC7 (DXGI 70–84, 94–99). Non-block formats
(FourCC 0, uncompressed) also exist with type 1 (raw chunk sizes).

**Simplest correct writer (recommended for Pylune):** write every chunk
*raw* (`m_i = raw_size(mip_i)`, `stored_size = orig_size`). That is what
about 90 % of vanilla textures do. Then for a same-format, same-dimension
replacement, `m1..m4` and the header template are **identical to the vanilla
PATHC entry**, and PATHC needs no change. That last part is UNVERIFIED in
game but follows from the observed invariants. For new textures, or changed
format or size, update PATHC (§6.3).

---

## 8. `meta/0.papk` (32 bytes)

Opaque, high-entropy, unchanged by every mod manager studied. Purpose
unknown (possibly a key or signature). **Never modify.**

---

## 9. Building a Pylune overlay group

### 9.1 What the existing managers do

| tool | overlay dir name | PAPGT position | shadowed vanilla PAMT | notes |
|------|------------------|----------------|-----------------------|-------|
| JMM (0xNobodyYT) | `0036`+ (first free number) | **index 0** (`entries.Insert(0, …)`), lang 0x3FFF | untouched | flags of shadowed entry inherited; LZ4 by default |
| CDUMM | first number above every dir and every PAPGT name (≥ 0041 on 2.x) | **new dirs first** ("game loads PAPGT in order, first match wins") | **restored to vanilla** before building the overlay | adds companion `.pabgh`/`.staticinfoheader` for every modified table body; marker file `_cdumm_overlay.marker` |
| MrIkso CrimsonDesertTools | `0254` | appended at the end | untouched | test tool; stores files uncompressed; fixed-5-byte names |
| DMM 1.5.5.3 | `dmmsa` (+ `.dmm_owned` marker) | not logged | used *both*: overlay for **new** files, plus in-place append into vanilla `0009`/`0012` PAZ/PAMT for **replacements** | patched PATHC for overlay DDS |

The important question was whether override comes from PAPGT order or from
editing the original PAMT. JMM and CDUMM, written independently, both
replace vanilla files by putting the overlay first in the PAPGT and leaving
vanilla PAMTs alone. DMM's in-place route for replacements is a design
choice (comments mention streaming and voice-mirror concerns), not evidence
that overlays cannot override. **UNVERIFIED in game by this research.** The
first Pylune milestone that writes anything must include a manual in-game
test (§11.4).

DMM log hints at per-type caveats (SOURCE, UNVERIFIED):
* "customization-meta" XMLs (`character/descriptors/customizationmeta/*.xml`)
  were written **in-place, uncompressed and unencrypted** ("no 0x0032
  re-encrypt") instead of into the overlay;
* new UI images were forced to a "self-contained" texture class (last4
  0x0F) and registered in PATHC.

**Data tables need their header in the overlay (2026-10-26).** Test batch 1 packed only the
patched `*.staticinfobody` files: the overlay held exactly the intended bytes, listed first in
the PAPGT, yet none of the three tests had an effect in game. CDUMM packs the companion
`.staticinfoheader`/`.pabgh` with every modified body; the likely reason is that the game reads a
table's body from the group where it found the header. Pylune now packs the (unchanged) header
with every patched body. **UNVERIFIED in game** until test batch 1 is repeated.

### 9.2 Recommended Pylune procedure

Naming: use a **numeric** name above every number in the vanilla PAPGT and
every existing directory, for example `0050`, and remember it in Pylune's
DB. Mark it with a `.pylune_owned` file. Non-numeric names (`dmmsa`) are
representable, but CDUMM's rescan logic only recognises 4-digit names, and
other tools may delete unknown directories. Never reuse 0036–0040.

```
inputs: vanilla snapshot of meta/0.papgt, meta/0.pathc (+ paver fingerprint),
        resolved mod file list: [(vpath, bytes)], last mod wins on conflicts

1. Guard: live meta/0.paver must equal the snapshot's paver; otherwise the game was
   updated → re-snapshot vanilla (only if live PAPGT has no foreign entries) and rebuild.
2. For each vpath:
     v = lookup in vanilla groups (PAPGT order, first hit)          # §5.5
     flags = v.flags if v else flags of a same-extension sibling (fallback: 0x0002)
     payload = encode(bytes, flags)          # .dds → type 1 raw-chunk (§7.4); else LZ4 if comp=2;
                                             # then ChaCha20 if enc=3 (key from basename)
     place at align16(cursor) in 0.paz; zero-pad
     if .dds: queue PATHC update with header template + m1..m4
   If a mod ships a table body (.pabgb / .staticinfobody), also ship the matching
   header (.pabgh / .staticinfoheader), patched or vanilla (CDUMM rule).
3. Pad 0.paz to 16; paz_crc = pa_hash(0.paz); paz_size = len.
4. Build 0.pamt (§5.1): paz_count=1, unknown=0x610E0232, folder trie per segment,
   flat file-name nodes, folders sorted, files sorted per folder, contiguous ranges,
   folder_hash = pa_hash(folder path). pamt_crc = pa_hash(pamt[12..]).
5. PAPGT: vanilla + new entry at index 0 (§4.4), lang = vanilla "all" mask.
6. PATHC: vanilla + queued texture updates (§6.3) (skip if no textures changed).
7. Write order (crash safety): <dir>/0.paz → <dir>/0.pamt → meta/0.pathc → meta/0.papgt
   (each via temp file + fsync + rename). PAPGT last: until it is written the game
   still sees a consistent vanilla state.
Uninstall: restore vanilla 0.papgt and 0.pathc from snapshot, then delete <dir>.
```

Size limit: one PAZ per overlay is fine up to 4 GiB (u32 offsets). Beyond
that, use multiple PAZ files (`paz_index` 1, 2, …) in the same group.

### 9.3 Known gotchas

* Steam "verify files" or game updates rewrite `meta/*`. The overlay
  directory survives but is no longer registered (exactly what happened to
  `dmmsa`). Detect it via paver and the PAPGT contents, then offer
  "re-apply".
* Foreign managers (DMM, CDUMM, JMM) may also own the PAPGT. Detect
  non-vanilla entries (not in the snapshot, or `dmmsa`, `.dmm_owned`,
  `_cdumm_overlay.marker`) and refuse to proceed, or warn, instead of
  silently dropping them.
* The current install has an orphaned `dmmsa/` (4.7 MB). It is harmless but
  can be removed with the user's consent.

---

## 10. Community mod formats

Everything in this section is SOURCE: it comes from reading the CDUMM,
UJMM (0xNobodyYT), MacJMM (Enki013) and NattKh code and fixtures. It was
not checked against real mods, because the user's `~/.local/share/DMM/mods`
holds only empty `_lang`/`_language` folders. All formats that change
existing game data operate on the **decrypted and decompressed plaintext of
one PAMT entry**, identified by its virtual path. All of them end up as
overlay entries in Pylune (§9).

### 10.1 Byte-patch JSON ("JSON mod", JMM v1/v2, "format": 2)

```jsonc
{
  "format": 2,                               // informational; nobody branches on it
  "name": "...", "version": "...", "description": "...", "author": "...",
  "modinfo": { "title", "name", "version", "author", "description",
               "allow_partial_apply": true }, // optional JMM-style nesting (preferred if present)
  "patches": [
    { "game_file": "gamedata/skill.pabgb",   // VFS path; UJMM also accepts "file"
      "signature": "0900000043…",            // optional hex anchor: offsets relative to END of first match
      "changes": [
        { "offset": 497051,                  // int, "0x…" hex, or decimal string (see coercion below)
          "entry": "Skill_CrowWing",         // optional v2 anchor: record name in the table
          "rel_offset": 116,                 // with entry: offset from end of the record name
          "label": "[Flight] CrowWing_hover -10.0 -> -5.000",   // "[Group] …" = preset group
          "original": "f0d8ffff",            // hex, verified before writing
          "patched":  "78ecffff" },
        { "offset": "0x0", "type": "insert", "bytes": "deadbeef" }   // insert (CDUMM/UJMM only)
      ] } ]
}
```

(The first change is a real excerpt of
`MacJMM/mods/stamina_json_v1.02.00/stamina_v1.02.00_50pct.json`, a mod with
140 changes.)

* Offset coercion: CDUMM uses `int(s, 0)`, then falls back to hex for bare
  strings such as `"1A"`. UJMM treats a digit-only string as decimal and a
  string with a–f letters as hex. Accept ints, `0x…` hex and decimal
  strings. Treat other strings as hex only if they contain a–f letters.
  `original`/`patched` may contain spaces or a `0x` prefix, so strip them.
  The aliases `old`/`new`/`bytes`, `op` for `type` and `relative_offset` for
  `rel_offset` also exist.
* `game_file` resolution: exact lowercase VFS path across all groups, then
  basename only. MacJMM and UJMM only look in `0008`. Alias `.pabgb`↔
  `.staticinfobody` and `.pabgh`↔`.staticinfoheader`: since the 2026-09-04
  update the user's install only has the new names (VERIFIED:
  `gamedata/binarystaticinfo__/bin/iteminfo.staticinfobody`). Without the
  alias every table mod silently does nothing.
* **Table index (`.pabgh`/`.staticinfoheader`)**: `count` (u16 for most
  tables, u32 for some, rarely u8), then `count × (key[key_size] + u32
  offset_into_body)`. Key size is usually 4 (VERIFIED for iteminfo: u16
  6 816, 8-byte rows); 2 for storeinfo/inventory-like tables, and 8 and 12
  also occur. Derive it by tiling: `len == cs + count·(ks+4)` with
  offsets starting at 0 and strictly increasing.
* **Body record (`.pabgb`)**: `key[key_size] | u32 name_len | name bytes |
  (usually) 0x00 | payload`. Some tables (ItemInfo) have no NUL, and some
  (RegionInfo) have no name header.
* **`entry` + `rel_offset`** → `abs = record_start + key_size + 4 +
  name_len + rel_offset` (anchored right after the name characters,
  **not** after the NUL). The entry is found by name, first match wins.
  CDUMM additionally supports `record_key` + `record_rel_offset` (from the
  record start).
* Apply algorithm (minimum viable, close to UJMM):
  1. Start from vanilla bytes.
  2. For each change, compute the offset (entry anchor first, otherwise
     `offset`).
  3. Sort the changes, then apply them while tracking size shifts from
     inserts and size-changing replaces.
  4. For a replace, require `data[off..off+len(original)] == original`. If
     the bytes already equal `patched`, count the change as applied.
     Otherwise skip it and report.
  5. On data tables, any mismatch rejects the whole mod unless
     `allow_partial_apply`.
  6. Inserts into a table body require shifting every index offset at or
     after the insert position (`fixup_pabgh_after_inserts`). UJMM refuses
     these; Pylune can too, at first.
  CDUMM adds many drift heuristics (signature retry, fallback offsets,
  pattern scan). Skip them.
* Conflicts: apply all enabled mods to the same vanilla buffer in priority
  order. The later mod wins on overlapping bytes. Report overlaps.
* Write-back in Pylune: the patched plaintext becomes an overlay entry,
  re-compressed and re-encrypted per the vanilla flags. CDUMM also ships
  the matching header file whenever a body is touched.

Complexity: **M**. It needs `paz::vfs` extraction, `patch::table` and the
overlay writer.

### 10.2 Format 3 / `.field.json` / `.cdmod` (field-name JSON)

These are one format under three names. `.cdmod` is a zip of
`manifest.json` plus `patches/semantic.json`, with the keys renamed:
`targets[].operations[]`→`intents[]`, `path`→`field`, `selector.key`→`key`,
`selector.string_key`→`entry`, `value`→`new`.

```jsonc
{ "modinfo": { "title", "version", "author", "description", "note" },
  "format": 3,                    // required marker
  "format_minor": 1,              // optional (DMM v3.1 multi-target dialect)
  "target": "dropsetinfo.pabgb", "intents": [ … ]            // either singular …
  // "targets": [ { "file": "iteminfo.pabgb", "intents": [ … ] }, … ]   // … or plural, not both
}
```

Intent shapes:

| shape | form |
|-------|------|
| set (the default op) | `{entry, key?, field, op:"set", new}`. `field` may be nested: `cooltime.a`, `gear_stat[1000000]`, `drop_default_data.add_socket_material_item_list` |
| raw replace | `{entry, old:"hex", new:"hex"}` |
| match | `{match:{field:value}, field, new}` |
| clone_record | `{op:"clone_record", source_key, new_key, new_name, patches:[{field,new}]}` |
| new_record | `{op:"new_record", template_key\|source_key, new_key, new_name, patches}` |
| delete_record | `{op:"delete_record", key}` |
| array_append | `{op:"array_append", entry, key, field, new:{…}}` |
| scale | DMM's `op:"scale"` with a `factor` |

Example (CDUMM test fixture): `{"format":3,"target":"dropsetinfo.pabgb",
"intents":[{"entry":"DropSet_Faction_Graymane","key":175001,"field":"drops",
"op":"set","new":[{"item_key":30010,"rates":1000000,"rates_100":100,"min_amt":15,"max_amt":10}]}]}`.

Implementing it requires a **per-table field layout**, and that is the hard
part:
* CDUMM `field_schema/<table>.json` (JMM v9.9.3-compatible) maps a field
  name to either `{"tid":"0xAABBCCDD","value_offset":5,"type":"i64"}` (find
  the unique 4-byte TID in the entry payload and write at `pos +
  value_offset`) or `{"rel_offset":N,"type":"u32"}`. Here the offset is
  measured from the payload start (after the NUL), a different anchor than
  §10.1. Types: i8/u8/i16/u16/i32/u32/f32/i64/u64/f64. Only `skill.json` and
  `storeinfo.json` ship.
* `schemas/pabgb_complete_schema.json` (434 classes, from IDA) is in memory
  order, not on-disk order. About 82 of 134 tables cannot be decoded from
  it. **License MPL-2.0** (see §13).
* CDUMM has hand-written whole-table writers for iteminfo, skill,
  storeinfo, dropsetinfo, characterinfo, buffinfo and others, to handle
  variable-length records and rebuild `.pabgh`.
* Conversion: intents → v2 changes `{entry, rel_offset, original,
  patched}` against vanilla bytes, then the §10.1 engine.

Pylune plan: support `set` and raw `old/new` for tables with a
field_schema-style layout, and refuse everything else with a clear message.
Complexity: **S/M** for that subset, **L** for CDUMM parity.

### 10.3 RAW / loose-file mods

Layouts to detect (CDUMM `import_handler._find_loose_file_candidates`, UJMM `LoadOverlayDirectory`):

1. `mod.json` + `files/<NNNN>/<vpath>` (the canonical layout; `files_dir`
   may rename `files`).
2. `mod.json` + game roots at the top level (`gamedata/`, `ui/`,
   `character/`, `sequencer/`, …), or numbered dirs under `game_files/`
   (JMM 9.9.2).
3. Bare `files/<NNNN>/…`.
4. Bare `<NNNN>/<root>/…`.
5. Variant pack: `mod.json` plus at least two subfolders, each containing
   `<NNNN>/…`. The user's own example is DMM's "Character Creator":
   `Character Creator/Human Female/0009/character/…` and `…/0012/ui/texture/…`,
   with `mod.json`, `CharacterCreatorHead.asi` and `Female Animations.field.json`
   at the pack root.

Manifest: `mod.json` (UJMM also reads `modinfo.json` and `manifest.json`) =
`{"modinfo": {title|name, version, author, description, files_dir?,
conflict_mode?: "normal"|"override", target_language?, force_inplace?}}`.
Manifest files are never packed.

Mapping: strip `files/` and the `NNNN/` prefix; the rest is the VFS path.
The `NNNN` hint is advisory. Match case-insensitively: first the exact
path, then a unique PAMT path ending in `/<path>`, then a unique basename
(PAMTs sometimes "flatten" folders). Without `NNNN`, search all vanilla
groups.

Compiled group mods (a mod ships `NNNN/0.pamt` + `NNNN/*.paz`):
* CDUMM: for a number of 36 or more, remap to a fresh dir. For a vanilla
  number, decompose entry by entry and re-emit through the overlay. It also
  verifies and fixes the PAMT CRC.
* UJMM copies the group verbatim into the next free number and registers
  it at PAPGT index 0. The risk is landing on 0036–0040.
* MacJMM always uses `0036`, which **collides with the 2.x reserved
  slot**.
* Pylune: decompose into entries (read with `paz::pamt`) and merge them into
  its single overlay. That avoids multiple groups and slot fights.

Complexity: **M** (detection + VFS resolution + overlay writer).

### 10.4 Browser/UI overlay folders ("Crimson Browser")

* Manifest: `manifest.json` or `modinfo.json`, up to 3 levels deep:
  `{"format": "crimson_browser_mod_v1" | "crimson_sharp_mod_v1", "id",
  "files_dir": "files", "patches_dir"?, title/version/author/description}`.
  A manifest without `format` but with a non-empty `files_dir` is accepted.
  `patches_dir` holds §10.1 JSONs (a bare `{game_file, changes}` gets
  wrapped).
* Files: `files/NNNN/<vpath>` (e.g. `files/0012/ui/xml/gamemain/play/minimaphudview2.css`),
  plus optional `tex/`, `textures/`, `texture/` siblings. They are handled
  exactly like RAW (§10.3).
* DMM "browser mods" are just `<Pack>/<Variant>` folders with bare
  `NNNN/<vpath>` trees (`mounted.json: "active_browser_mods":
  ["Character Creator/Human Female"]`).
* `.dds` inside them needs the type-1 payload (§7.4) and PATHC registration
  (§6.3). New textures without PATHC entries render invisible ("black
  circles", CDUMM).

Complexity: **M** (layout S; DDS + PATHC M).

### 10.5 XML / HTML / CSS patch and merge files

The target is the patch file's own path with `.patch` or `.merge` stripped
(`ui/…/foo.xml.patch` → `ui/…/foo.xml`). Extract the vanilla file, apply the
patch, then put the result in the overlay.

* `*.xml.patch`, in either of two forms:
  * JSON: `{"operations":[{"op":"replace|add|add-before|add-after|remove|set-attr|remove-attr",
    "xpath":"//foo","value":"…","attribute":"name"}]}`;
  * XML: `<xml-patch>` with children `set`, `unset`, `insert`/`add`,
    `insert-before`, `insert-after`, `replace`, `delete`/`remove`.
  
  The target is given by `target="xpath"`, by `at="#id"`/`".cls"` (UJMM), or
  by `find="tag" key="K"`. Quirks to preserve: keep the BOM, wrap
  multi-root fragments in a sentinel root, expand CD's `</>` shorthand
  close tag, and disable entity expansion.
* `*.xml.merge` (`<xml-merge>`): match elements by identity attribute,
  priority `Key > Name > Id > key > name > id`, then any `*_key`/`*_id`/`*_name`.
  Copy attributes onto the match. `__delete="true"` removes the match.
  Unmatched elements are appended.
* `*.html.patch`: ops `<set at="#btn" class="+active -disabled"/>`,
  `remove`, `replace`, `inner`, `append`, `prepend`, `before`, `after`, and
  comment directives `<!-- @replace selector="…" --> … <!-- @end -->`,
  `@set-attr`, `@add-class`. Selectors are CSS-lite (tag, #id, .class,
  [attr=v], descendant).
* `*.css.patch`: comment-directive modes `/* @merge */` (the default,
  property by property), `/* @replace */`, `/* @add */`,
  `/* @remove ".sel" */`. `*.css.merge`: every rule merges.

Status (2026-10-26): XML patch and merge are implemented in `src/patch/xml/` (an own
span-keeping reader instead of a DOM, since the game's XML is not well-formed: 69 of 1 084
sampled files fail a strict parser, 24 use `</>`, all start with a BOM); HTML patches in
`src/patch/html/` with the same span-keeping reader (void elements aware).

Complexity: XML **M** (needs an XML DOM with XPath), HTML **M/L**
(lenient parser), CSS **S**. These are ports of JMM (MPL-2.0) behaviour:
re-implement them from this description, not by translating code.

### 10.6 What Pylune needs, by format

| format | Pylune needs | complexity |
|--------|--------------|------------|
| RAW / browser folders (non-DDS) | layout detection, VFS resolve, overlay | M (first) |
| Byte-patch JSON | vfs extract, table index, apply engine, overlay | M |
| DDS in RAW/browser | type-1 encoder, PATHC writer | M |
| XML/HTML/CSS patch | DOM libraries + op engines | M–L |
| Field JSON / Format 3 | per-table schemas/writers | S/M subset, L full |

---

## 11. Implementation plan for Pylune

### 11.1 Module breakdown (new crate or `src/paz/`, `src/patch/`)

| module | responsibility | size |
|--------|----------------|------|
| `paz::hash` | `pa_hash` (lookup3, 0xC5EDE), test vectors from §1.1 | S |
| `paz::crypto` | key/IV derivation, ChaCha20 apply (IETF and seek, Legacy fallback for counter overflow) | S |
| `paz::lz4` | thin wrapper over `lz4_flex::block` | S |
| `paz::dds` | DDS header parse, mip sizes, type-1 encode/decode (§7.4), last4 table | M |
| `paz::papgt` | parse, serialize, insert/remove entry, CRC | S |
| `paz::pamt` | parse (tries, folders, files), serialize (writer per §5.1/§9.2), lookup index | M |
| `paz::pathc` | parse, serialize, sorted insert/update, collision handling | M |
| `paz::vfs` | group order from PAPGT, `vpath → (group, record)`, `extract(vpath) → bytes` | M |
| `paz::overlay` | build overlay PAZ+PAMT from `[(vpath, bytes)]`, PATHC and PAPGT update, atomic write, uninstall | M |
| `paz::snapshot` | vanilla snapshot of meta files keyed by paver; foreign-entry detection | S |
| `patch::json` | byte-patch JSON (§10) applied to extracted bytes | M |
| `patch::table` | `.pabgh`/`.staticinfoheader` index (u16 count + (u32 key, u32 offset)), entry+rel_offset addressing | S |
| `patch::field` | `.field.json` / Format 3 (needs per-table schemas) | L |

Crates: `lz4_flex` (0.11, pure Rust, verified), `chacha20` (0.10 with
`legacy` feature, verified), `memmap2` (optional, for reading PAZ ranges;
plain `File::seek` + `read_exact` is fine), `tempfile` (already a dev
dependency). lookup3 should be self-written (about 40 lines). Pylune's
existing `serde_json` covers the JSON formats.

### 11.2 Test strategy

* Copy the user's small meta files into `tests/fixtures/` (they are game
  data, so keep them out of git if the repo is public, or gate the tests on
  an env var pointing at a local fixture dir): `0.papgt`, `0.paver`,
  `0.pathc` (7 MB), `0013/0.pamt` (174 B), `0016/0.pamt`, `0003/0.pamt`,
  `dmmsa/0.pamt`, plus DMM's `papgt_clean.bin` and `pathc_clean.bin` as a
  second build.
* Golden tests:
  1. `pa_hash` vectors (§1.1).
  2. PAPGT: parse, then `pa_hash(file[12..]) == header.crc`; serialize(parse(x)) == x for both builds.
  3. PAMT: for every fixture, `pamt_crc` verifies and the parse consumes
     exactly EOF. serialize(parse(dmmsa.pamt)) == bytes (keep the original trie).
     Also rebuild dmmsa with Pylune's own trie writer and check it re-parses to the same entry list.
  4. PATHC: serialize(parse(x)) == x; hashes sorted; collision selectors decode; known path present.
  5. Crypto: decrypt `0016/0.paz[0..3846]` (`paconfig.txt`, flags 0x32) → LZ4 → 6 422 bytes that
     start with `// ` and contain `gv_appIdentifier=pearlabyss.crimsondesert`. Keep a 3.8 KB fixture.
  6. DDS type 1: for a few vanilla textures, decode stored → full DDS of `orig_size`,
     re-encode raw-chunk → m1..m4 match PATHC.
  7. Overlay end-to-end in a temp "fake game dir" (copied meta + fake groups): build, re-open with
     the reader, and check that `extract(vpath)` returns the mod bytes and that the CRC chain
     (PAZ → PAMT → PAPGT) verifies.
* Optional integration test (env var `PYLUNE_GAME_DIR`, read-only): parse
  all 35 PAMTs, verify all CRCs against PAPGT, and decrypt-sample 100
  encrypted entries.
* Manual in-game acceptance tests (cannot be automated): (a) overlay
  replacing one plain XML/HTML UI file with a visible text change; (b)
  overlay replacing an existing DDS (same format) with no PATHC change;
  (c) a new DDS with a PATHC entry.

### 11.3 Riskiest unknowns (ordered)

1. **Override semantics**: the game resolving duplicates by PAPGT order
   (index 0 wins) comes from JMM/CDUMM only. It needs an in-game test.
   Fallback if it is wrong: rewrite the shadowed vanilla group's PAMT entry
   to point into an appended PAZ (DMM's in-place route), which is much more
   invasive.
2. **Texture loading**: the PATHC interaction (header template, last4
   classes, "self-contained" UI class 0x0F, mip-count-2..4 semantics). Ship
   textures after plain files work.
3. **lang_mask / is_optional semantics** for new groups (0x3FFF vs 0x7FFF;
   bit 14 language users).
4. Whether the game verifies `paz_crc` or `paz_size` at runtime (write
   them correctly anyway; it costs one hash pass).
5. Encrypted file types that DMM deliberately wrote unencrypted and
   in-place (customization-meta XMLs). Some loaders may bypass the generic
   VFS.
6. Table renames across game updates (`.pabgb/.pabgh` →
   `.staticinfobody/.staticinfoheader` on 2026-09-04): mods target the old
   names, so alias them.
7. Game updates rewrite `meta/*`, so Pylune must detect and re-apply. Never
   take a "vanilla snapshot" from a PAPGT that contains foreign entries.

### 11.4 Recommended first milestone

**M1 "read-only VFS" (no writes to the game dir):** `paz::{hash, crypto,
lz4, papgt, pamt, pathc, vfs}` plus the golden tests above, and a debug
command `pylune vfs extract <vpath>` / `pylune vfs ls <prefix>`. This
validates every format against the real install with zero risk, and it is a
prerequisite for JSON patches anyway (they patch extracted bytes).

**M2 "overlay for non-texture files":** `paz::overlay` and `paz::snapshot`,
RAW mods for XML/HTML/CSS/data files, PAPGT insert at index 0, then manual
in-game test (a). If (a) fails, revisit unknown #1 before anything else.

**M3:** byte-patch JSON mods (they become overlay entries). **M4:** DDS
and PATHC. **M5:** field JSON / Format 3.

---

## 12. Verification log (what was actually run)

Scripts lived in the research scratchpad (Python `-I` with a self-written
lookup3, ChaCha20 via `cryptography`, a hand-written LZ4 block decoder, plus
a Rust test crate using `chacha20 0.10` and `lz4_flex 0.11`). The game
directory was only read, and small meta files were copied to scratch first.

* PAPGT (current 2.3.2 and DMM `papgt_clean.bin` 1.15.0): parsed to EOF;
  `pa_hash(file[12..])` equals header CRC for both; all 34 present groups'
  `pamt_crc` equal the on-disk PAMT header; 0036–0040 absent and optional.
* PAMT: all 35 files (34 vanilla plus dmmsa) parse exactly to EOF and the
  header CRC verifies. 2 052 922 file records in total. Folder hashes
  verified (all folders of 0000, 0008, dmmsa). Folders sorted by name, files
  sorted per folder, contiguous ranges, all offsets ≡ 0 mod 16. `dmmsa/0.pamt`
  rebuilt byte for byte from its parsed contents.
* PAZ: `paz_crc == pa_hash(whole file)` for 0003, 0013, 0016, dmmsa; zero
  padding and 16-alignment confirmed.
* Crypto and LZ4: decrypted and decompressed `paconfig.txt` (0016, 0x32),
  `material/dist/*.material` (0003, 0x30/0x32), `gamedata/*.xml` (0008);
  outputs are valid text/XML. Rust IETF-and-seek and Legacy-64 ChaCha20
  produce the same plaintext as Python.
* PATHC (both builds): parsed to EOF; hashes sorted and unique; header u32 0
  and 1 are zero; 5000 of 5000 vanilla DDS paths found; 12 collision entries
  decode to 6 colliding pairs whose `pa_hash("/"+path)` match;
  m1..m4 == PAZ header reserved1 and template == header with reserved1 zeroed (40 of 40).
* DDS type 1: the stored-size formula `hs + Σ m[0..n] + Σ raw[n..]` and
  `orig_size == hs + Σ raw` held for 1 600 of 1 600 sampled textures across 0000/0009/0012/0015.
* Table index: `gamedata/binarystaticinfo__/bin/iteminfo.staticinfoheader` (0008) =
  u16 count (6 816) + 6 816 × (u32 key, u32 offset); offsets fall inside the
  6 465 724-byte LZ4-decompressed `iteminfo.staticinfobody`.
* paver: `pa_hash(first 6 bytes) == last u32`.

Not verified: anything that requires running the game (override order,
texture loading, lang gating, runtime PAZ CRC checks).

---

## 13. Sources and licenses

Only facts (formats, constants, algorithms) were taken. No code was copied
into Pylune. Format documentation is fine regardless of license; reusing
code would need the license terms (MIT requires keeping the copyright
notice).

| source | license | taken (facts only) |
|--------|---------|--------------------|
| [lazorr410/crimson-desert-unpacker](https://github.com/lazorr410/crimson-desert-unpacker) | MIT (© 2026 lazorr410) | ChaCha20 key/IV derivation, seed constant 0xC5EDE and test vector, LZ4 block, PAMT reading, in-place repack constraints |
| [MrIkso/CrimsonDesertTools](https://github.com/MrIkso/CrimsonDesertTools) | MIT (© 2026 MrIkso) | PAPGT/PAMT field names (hexpat templates), flags nibbles and enum names, `PaChecksum` (= lookup3 with 0xC5EDE), 16-byte alignment, group packing procedure. PAPGT layout credited to LukeFZ/pycrimson (not cloned) |
| [NattKh/CrimsonDesertModdingTools](https://github.com/NattKh/CrimsonDesertModdingTools) | **No LICENSE file**; README says "MIT". CDUMM's `schemas/NOTICE` attributes the pabgb schema JSON to NattKh's save-editor repo under **MPL-2.0** | PABGB/PABGH header heuristics (u16/u32 count). Do not copy its schema JSON |
| [faisalkindi/CrimsonDesert-UltimateModsManager](https://github.com/faisalkindi/CrimsonDesert-UltimateModsManager) | MIT (© 2026 faisalkindi). Contains ports of JMM code (MPL-2.0) and schema data under MPL-2.0 (`schemas/NOTICE`, `pabgb_type_overrides.json` `_meta`) | Overlay PAMT layout, PAPGT rebuild and "new dirs first", reserved 0036–0040 slots, PATHC layout and update, DDS partial payload, table rename, JSON/Format 3/RAW/browser/XML/HTML/CSS formats, `native/` Rust crate choices (`chacha20`, `lz4_flex`) |
| [0xNobodyYT/ultimate-json-mod-manager](https://github.com/0xNobodyYT/ultimate-json-mod-manager) | MIT (© 2026 0xNobodyYT) | `BuildPapgtWithMod` (insert at index 0, lang 0x3FFF), loose overlay PAMT builder, flags inheritance, byte-patch semantics, RAW/BROWSER layouts |
| [Enki013/Crimson-Desert-JSON-Mod-Manager-MacOS](https://github.com/Enki013/Crimson-Desert-JSON-Mod-Manager-MacOS) | MIT (© 2025 contributors) | Example JSON mods (format 2), `0036` overlay approach (now a reserved slot) |
| [Ekey/CD.PAZ.Tool](https://github.com/Ekey/CD.PAZ.Tool) | **No license found** (README only) | Only confirmed the 0x610E0232 PAMT constant check. Nothing else used |
| JMM (original JSON Mod Manager, via CDUMM comments) | MPL-2.0 (per CDUMM docstrings) | Origin of the DDS partial payload and XML/HTML patch semantics. Re-implement from the description; do not translate code |
| DMM 1.5.5.3 logs/backups on the user's machine | closed source | Observed behaviour: `dmmsa` overlay, PATHC +1 record/+399 hashes, in-place injections |
| User's game files (2.3.2) | game data, Pearl Abyss | All VERIFIED facts. Fixtures must stay local (do not commit game files to a public repo) |

Pylune itself declares no license file at the time of writing. If Pylune
later reuses MIT code (for example CDUMM's `native/src/*.rs`), keep the MIT
notice. MPL-2.0 material requires file-level source availability for the
copied files.
