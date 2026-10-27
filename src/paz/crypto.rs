//! ChaCha20 entry encryption (`docs/paz-format.md` §7.2).

use super::hash::pa_hash;
use crate::error::{PailuneError, Result};
use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};

/// XORed with the seed to form the key base.
const KEY_BASE: u32 = 0x6061_6263;
/// Per-word deltas of the 32-byte key.
const KEY_DELTAS: [u32; 8] = [
    0x0000_0000,
    0x0A0A_0A0A,
    0x0C0C_0C0C,
    0x0606_0606,
    0x0E0E_0E0E,
    0x0A0A_0A0A,
    0x0606_0606,
    0x0202_0202,
];

/// Seed derived from the lowercase file name (basename only).
pub fn seed(vpath: &str) -> u32 {
    let base = vpath.rsplit('/').next().unwrap_or(vpath).to_lowercase();
    pa_hash(base.as_bytes())
}

/// The 32-byte key for a seed.
pub fn key(seed: u32) -> [u8; 32] {
    let kb = seed ^ KEY_BASE;
    let mut key = [0u8; 32];
    KEY_DELTAS
        .iter()
        .enumerate()
        .for_each(|(i, d)| key[i * 4..i * 4 + 4].copy_from_slice(&(kb ^ d).to_le_bytes()));
    key
}

/// Encrypts or decrypts `buf` in place (the operation is symmetric).
///
/// The initial block counter is the seed itself; the IETF variant has a
/// 32-bit counter, and no file of the current game crosses its end, so a
/// would-be overflow is reported instead of silently diverging.
pub fn apply(vpath: &str, buf: &mut [u8]) -> Result<()> {
    let seed = seed(vpath);
    let blocks = buf.len().div_ceil(64) as u64;
    if u64::from(seed) + blocks > u64::from(u32::MAX) + 1 {
        return Err(PailuneError::Archive(format!(
            "{vpath}: ChaCha20 counter would overflow"
        )));
    }
    let nonce: Vec<u8> = (0..3).flat_map(|_| seed.to_le_bytes()).collect();
    let key = key(seed);
    let mut cipher = ChaCha20::new(
        key.as_slice().try_into().expect("32-byte key"),
        nonce.as_slice().try_into().expect("12-byte nonce"),
    );
    cipher.seek(u64::from(seed) * 64);
    cipher.apply_keystream(buf);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Example from lazorr410's notes, quoted in the spec.
    #[test]
    fn derives_the_documented_key() {
        let seed = seed("ui/rendererconfigurationmaterial.xml");
        assert_eq!(seed, 0xAF3D_CEF3);
        // The spec lists the key as a byte dump.
        let expected = "90ac5ccf9aa656c59ca050c396aa5ac99ea252c19aa656c596aa5ac992ae5ecd";
        let hex: String = key(seed).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, expected);
        let mut data = b"round trip".to_vec();
        apply("a/b.xml", &mut data).unwrap();
        apply("a/b.xml", &mut data).unwrap();
        assert_eq!(data, b"round trip");
    }
}
