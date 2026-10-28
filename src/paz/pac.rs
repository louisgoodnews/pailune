//! `.pac` mesh files in compression type 1 ("partial", §7.5).
//!
//! Stored `.pac` files have an 80-byte header: magic `PAR `, version, timestamp,
//! and up to 8 section pairs `(stored_size, orig_size)` at `16..80`.
//! Sections with `stored_size < orig_size` are LZ4-block compressed.
//! Decoding decompresses each section, replaces `stored_size` in the header
//! with `orig_size`, and produces the standard uncompressed `.pac` mesh file.

use crate::error::{PailuneError, Result};

pub const HEADER_SIZE: usize = 80;

/// Decodes a stored type-1 `.pac` mesh file into its standard uncompressed representation.
pub fn decode(stored: &[u8], orig_size: usize) -> Result<Vec<u8>> {
    if stored.len() == orig_size {
        return Ok(stored.to_vec());
    }
    if stored.len() < HEADER_SIZE || &stored[0..4] != b"PAR " {
        return Err(PailuneError::Archive("not a valid .pac mesh file".into()));
    }

    let mut out = stored[..HEADER_SIZE].to_vec();
    let mut pos = HEADER_SIZE;

    for s in 0..8 {
        let pair_offset = 16 + s * 8;
        let s_stored = u32::from_le_bytes(
            stored[pair_offset..pair_offset + 4]
                .try_into()
                .map_err(|_| PailuneError::Archive("truncated .pac header".into()))?,
        ) as usize;
        let s_orig = u32::from_le_bytes(
            stored[pair_offset + 4..pair_offset + 8]
                .try_into()
                .map_err(|_| PailuneError::Archive("truncated .pac header".into()))?,
        ) as usize;

        if s_stored == 0 {
            continue;
        }

        if pos + s_stored > stored.len() {
            return Err(PailuneError::Archive("truncated .pac section".into()));
        }

        let slice = &stored[pos..pos + s_stored];
        if s_stored < s_orig {
            let decomp = lz4_flex::block::decompress(slice, s_orig)
                .map_err(|e| PailuneError::Archive(format!(".pac LZ4 section {s}: {e}")))?;
            out.extend_from_slice(&decomp);
        } else {
            out.extend_from_slice(slice);
        }

        // In the uncompressed file's header, stored_size equals orig_size
        out[pair_offset..pair_offset + 4].copy_from_slice(&(s_orig as u32).to_le_bytes());
        pos += s_stored;
    }

    // Trailing bytes (if any)
    if pos < stored.len() {
        out.extend_from_slice(&stored[pos..]);
    }

    if out.len() != orig_size {
        return Err(PailuneError::Archive(format!(
            ".pac decoded to {} bytes, expected {orig_size}",
            out.len()
        )));
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_compressed_pac_mesh() {
        let mut header = vec![0u8; HEADER_SIZE];
        header[0..4].copy_from_slice(b"PAR ");
        header[4..8].copy_from_slice(&0x01000903u32.to_le_bytes());

        let sec0_raw = b"SECTION_0_VERTICES_DATA_ABCDEF".repeat(5);
        let sec0_comp = lz4_flex::block::compress(&sec0_raw);

        let sec1_raw = b"SECTION_1_INDICES_DATA_123456".repeat(3);
        let sec1_comp = lz4_flex::block::compress(&sec1_raw);

        // Stamp sections in header
        // Section 0
        header[16..20].copy_from_slice(&(sec0_comp.len() as u32).to_le_bytes());
        header[20..24].copy_from_slice(&(sec0_raw.len() as u32).to_le_bytes());
        // Section 1
        header[24..28].copy_from_slice(&(sec1_comp.len() as u32).to_le_bytes());
        header[28..32].copy_from_slice(&(sec1_raw.len() as u32).to_le_bytes());

        let mut stored = header.clone();
        stored.extend_from_slice(&sec0_comp);
        stored.extend_from_slice(&sec1_comp);

        let orig_size = HEADER_SIZE + sec0_raw.len() + sec1_raw.len();
        let decoded = decode(&stored, orig_size).expect("decode pac");

        assert_eq!(decoded.len(), orig_size);
        assert_eq!(&decoded[0..4], b"PAR ");
        assert_eq!(
            &decoded[HEADER_SIZE..HEADER_SIZE + sec0_raw.len()],
            &sec0_raw[..]
        );
        assert_eq!(&decoded[HEADER_SIZE + sec0_raw.len()..], &sec1_raw[..]);

        // Verify updated header in decoded file has stored_size == orig_size
        let s0_s = u32::from_le_bytes(decoded[16..20].try_into().unwrap()) as usize;
        let s0_o = u32::from_le_bytes(decoded[20..24].try_into().unwrap()) as usize;
        assert_eq!(s0_s, s0_o);
        assert_eq!(s0_o, sec0_raw.len());
    }

    #[test]
    fn uncompressed_pac_passes_through() {
        let plain = b"PAR \x03\x09\x00\x01data".repeat(10);
        let decoded = decode(&plain, plain.len()).unwrap();
        assert_eq!(decoded, plain);
    }
}
