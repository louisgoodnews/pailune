//! Save-file key, cipher and checksum (B-07).
//!
//! The scheme was documented by the community library `crimson-rs` (MIT,
//! <https://github.com/bbfox0703/crimson-rs>); this is Pailune's own
//! implementation of it, verified by checking the HMAC of real saves.

use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// The game's base key material (31 bytes used).
const BASE: [u8; 31] = [
    0xC4, 0x1B, 0x8E, 0x73, 0x0D, 0xF2, 0x59, 0xA6, 0x37, 0xCC, 0x04, 0xE9, 0xB1, 0x2F, 0x96, 0x68,
    0xDA, 0x10, 0x7A, 0x85, 0x3E, 0x61, 0xF9, 0x22, 0x4D, 0xB8, 0x0A, 0xD7, 0x5C, 0x13, 0xEF,
];

/// Appended to the version prefix before mixing.
const SUFFIX: &[u8] = b"PRIVATE_HMAC_SECRET_CHECK";

/// Per-version prefix of the key material.
fn prefix(version: u16) -> Option<&'static [u8]> {
    match version {
        1 => Some(b"^Qgbrm/.#@`zsr]\\@rvfal#\""),
        2 => Some(b"^Pearl--#Abyss__@!!"),
        _ => None,
    }
}

/// The 32-byte key of a save version (also the HMAC key).
pub fn key(version: u16) -> Option<[u8; 32]> {
    let material: Vec<u8> = prefix(version)?.iter().chain(SUFFIX).copied().collect();
    let mut key = [0u8; 32];
    BASE.iter()
        .zip(&material)
        .enumerate()
        .for_each(|(i, (b, m))| key[i] = b ^ m);
    Some(key)
}

/// Encrypts or decrypts in place. The stored nonce is the initial block
/// counter (`u32` LE) followed by the 96-bit IETF nonce.
pub fn apply(key: &[u8; 32], nonce: &[u8; 16], buf: &mut [u8]) {
    let counter = u32::from_le_bytes([nonce[0], nonce[1], nonce[2], nonce[3]]);
    let iv: [u8; 12] = nonce[4..].try_into().expect("12 bytes");
    let mut cipher = ChaCha20::new(key.into(), &iv.into());
    cipher.seek(u64::from(counter) * 64);
    cipher.apply_keystream(buf);
}

/// HMAC-SHA256 of the compressed payload.
pub fn mac(key: &[u8; 32], data: &[u8]) -> [u8; 32] {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("any key length");
    mac.update(data);
    mac.finalize().into_bytes().into()
}
