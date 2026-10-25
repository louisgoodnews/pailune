//! `pa_hash`: Bob Jenkins' lookup3 `hashlittle` with seed `0xC5EDE`.
//!
//! One function covers the PAPGT/PAMT/PAZ checksums, folder and texture
//! path hashes and the ChaCha20 key seed (`docs/paz-format.md` §1.1).

/// The seed every Crimson Desert hash uses.
pub const SEED: u32 = 0x000C_5EDE;

fn mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *a = a.wrapping_sub(*c);
    *a ^= c.rotate_left(4);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= a.rotate_left(6);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= b.rotate_left(8);
    *b = b.wrapping_add(*a);
    *a = a.wrapping_sub(*c);
    *a ^= c.rotate_left(16);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= a.rotate_left(19);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= b.rotate_left(4);
    *b = b.wrapping_add(*a);
}

fn final_mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *c ^= *b;
    *c = c.wrapping_sub(b.rotate_left(14));
    *a ^= *c;
    *a = a.wrapping_sub(c.rotate_left(11));
    *b ^= *a;
    *b = b.wrapping_sub(a.rotate_left(25));
    *c ^= *b;
    *c = c.wrapping_sub(b.rotate_left(16));
    *a ^= *c;
    *a = a.wrapping_sub(c.rotate_left(4));
    *b ^= *a;
    *b = b.wrapping_sub(a.rotate_left(14));
    *c ^= *b;
    *c = c.wrapping_sub(b.rotate_left(24));
}

/// Three little-endian words of a 12-byte block (zero-padded).
fn words(block: &[u8]) -> (u32, u32, u32) {
    let mut padded = [0u8; 12];
    padded[..block.len()].copy_from_slice(block);
    let w = |i: usize| u32::from_le_bytes([padded[i], padded[i + 1], padded[i + 2], padded[i + 3]]);
    (w(0), w(4), w(8))
}

/// `hashlittle(data, SEED)`.
pub fn pa_hash(data: &[u8]) -> u32 {
    let init = 0xDEAD_BEEFu32
        .wrapping_add(data.len() as u32)
        .wrapping_add(SEED);
    let (mut a, mut b, mut c) = (init, init, init);
    // All full blocks but the last are mixed; the last (1..=12 bytes) is finalised.
    let tail_start = match data.len() {
        0 => return c,
        n => (n - 1) / 12 * 12,
    };
    data[..tail_start].as_chunks::<12>().0.iter().for_each(|block| {
        let (x, y, z) = words(block);
        a = a.wrapping_add(x);
        b = b.wrapping_add(y);
        c = c.wrapping_add(z);
        mix(&mut a, &mut b, &mut c);
    });
    let (x, y, z) = words(&data[tail_start..]);
    a = a.wrapping_add(x);
    b = b.wrapping_add(y);
    c = c.wrapping_add(z);
    final_mix(&mut a, &mut b, &mut c);
    c
}

#[cfg(test)]
mod tests {
    use super::pa_hash;

    /// Vectors from `docs/paz-format.md` §1.1 (verified against the game).
    #[test]
    fn matches_the_documented_vectors() {
        assert_eq!(pa_hash(b""), 0xDEBA_1DCD);
        assert_eq!(pa_hash(b"gamedata"), 0xAA22_65B1);
        assert_eq!(pa_hash(b"rendererconfigurationmaterial.xml"), 0xAF3D_CEF3);
        assert_eq!(pa_hash(b"paconfig.txt"), 0xA1A8_E934);
        assert_eq!(pa_hash(&[2, 0, 3, 0, 2, 0]), 0xCB19_CCE3);
    }
}
