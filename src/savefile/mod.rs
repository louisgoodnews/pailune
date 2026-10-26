//! Reading and writing the game's save files (board item B-07): the `SAVE`
//! container of `docs/save-format.md`. The payload is LZ4-compressed, then
//! ChaCha20-encrypted, with an HMAC-SHA256 over the compressed bytes.
//!
//! [`decode`] checks the HMAC before trusting anything; `encode` (tests only, for now) writes a
//! file the game accepts. Pylune only reads saves so far; writing is used to
//! prove that the format is fully understood (byte-identical round trips).

pub mod body;
pub mod crypto;
pub mod slots;
pub mod view;

use crate::error::{PyluneError, Result};

/// Largest unpacked body accepted (real saves are about 3 MB).
const MAX_BODY: usize = 256 << 20;

/// Size of the header in front of the payload.
pub const HEADER: usize = 0x80;

/// The fixed part of the header and its dynamic fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub version: u16,
    pub uncompressed: u32,
    pub payload: u32,
    pub nonce: [u8; 16],
    pub hmac: [u8; 32],
    /// The whole header as read, kept verbatim for writing back.
    pub raw: Vec<u8>,
}

/// A decoded save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFile {
    pub header: Header,
    /// The decompressed body.
    pub body: Vec<u8>,
}

fn invalid(what: &str) -> PyluneError {
    PyluneError::Invalid(format!("not a readable save: {what}"))
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Reads and checks the header.
pub fn header(bytes: &[u8]) -> Result<Header> {
    if bytes.len() < HEADER || &bytes[..4] != b"SAVE" {
        return Err(invalid("no SAVE header"));
    }
    let header = Header {
        version: u16::from_le_bytes([bytes[4], bytes[5]]),
        uncompressed: u32_at(bytes, 0x12),
        payload: u32_at(bytes, 0x16),
        nonce: bytes[0x1A..0x2A].try_into().expect("16 bytes"),
        hmac: bytes[0x2A..0x4A].try_into().expect("32 bytes"),
        raw: bytes[..HEADER].to_vec(),
    };
    match HEADER + header.payload as usize == bytes.len() {
        true => Ok(header),
        false => Err(invalid("payload size does not match the file")),
    }
}

/// The decrypted, still compressed payload, after checking its HMAC.
pub fn compressed(bytes: &[u8]) -> Result<(Header, Vec<u8>)> {
    let header = header(bytes)?;
    let key = crypto::key(header.version).ok_or_else(|| invalid("unknown save version"))?;
    let mut data = bytes[HEADER..].to_vec();
    crypto::apply(&key, &header.nonce, &mut data);
    match crypto::mac(&key, &data) == header.hmac {
        true => Ok((header, data)),
        false => Err(invalid("checksum mismatch (damaged, or a newer format)")),
    }
}

/// Decodes a save file completely.
pub fn decode(bytes: &[u8]) -> Result<SaveFile> {
    let (header, data) = compressed(bytes)?;
    // The size is outside the checksum: never trust it for an allocation
    // beyond what LZ4 can produce or a sane save needs.
    let size = header.uncompressed as usize;
    if size > data.len().saturating_mul(255) || size > MAX_BODY {
        return Err(invalid("implausible unpacked size"));
    }
    let body =
        lz4_flex::block::decompress(&data, size).map_err(|e| invalid(&format!("LZ4: {e}")))?;
    Ok(SaveFile { header, body })
}

/// Writes a save (used by tests to prove the format; editing comes later):
/// compresses `body`, recomputes sizes and HMAC, encrypts
/// with `nonce`; other header bytes are kept from `template`.
#[cfg(test)]
pub fn encode(template: &Header, body: &[u8], nonce: [u8; 16]) -> Result<Vec<u8>> {
    let key = crypto::key(template.version).ok_or_else(|| invalid("unknown save version"))?;
    let mut data = lz4_flex::block::compress(body);
    let hmac = crypto::mac(&key, &data);
    crypto::apply(&key, &nonce, &mut data);
    let mut out = template.raw.clone();
    out[0x12..0x16].copy_from_slice(&(body.len() as u32).to_le_bytes());
    out[0x16..0x1A].copy_from_slice(&(data.len() as u32).to_le_bytes());
    out[0x1A..0x2A].copy_from_slice(&nonce);
    out[0x2A..0x4A].copy_from_slice(&hmac);
    out.extend_from_slice(&data);
    Ok(out)
}

/// Re-encrypts an unchanged compressed payload (same nonce): the exact
/// original file, if the format is understood.
#[cfg(test)]
pub fn reencrypt(header: &Header, compressed: &[u8]) -> Result<Vec<u8>> {
    let key = crypto::key(header.version).ok_or_else(|| invalid("unknown save version"))?;
    let mut data = compressed.to_vec();
    crypto::apply(&key, &header.nonce, &mut data);
    Ok(header.raw.iter().copied().chain(data).collect())
}

#[cfg(test)]
mod tests;
