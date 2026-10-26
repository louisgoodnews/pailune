# Save-game format (B-07)

Status 2026-10-26: **header understood and verified on real saves; payload decoding not
implemented** (needs an owner decision, see the end). Verified read-only on build 2.3.2.

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
- Pylune already backs up saves before every launch (A-28); any editor must work on copies
  and keep that backup.

## Options (owner decision needed)

1. **Depend on `crimson-rs`** (MIT, Rust, actively maintained, per-patch tables) for decoding,
   with attribution; Pylune adds a read-only viewer first, editing later. Fastest, but adds a
   large dependency and the game's key reaches Pylune through it.
2. **Implement our own decoder** from the documented format. More work; the key still has to be
   taken from the game or from community sources.
3. **Integrate external editors** instead (launch them on a copy of a save, then offer to
   restore). No crypto in Pylune at all.

Recommendation: start with option 1 as a **read-only inspector** (version, slot summary, play
time, location, character), keeping editing behind a later decision.

## Sources

- crimson-rs, `src/save/mod.rs` (MIT): https://github.com/bbfox0703/crimson-rs
- NattKh, Crimson Desert save editor: https://github.com/NattKh/CRIMSON-DESERT-SAVE-EDITOR-AND-GAME-MODS
- Save editors on Nexus Mods: https://www.nexusmods.com/crimsondesert/mods/20,
  https://www.nexusmods.com/crimsondesert/mods/3395
