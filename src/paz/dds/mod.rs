//! DDS textures in the game's "partial" storage (compression type 1, §7.4).
//!
//! The stored payload is the DDS file with up to four leading mips stored as
//! separate chunks (LZ4 or raw) whose sizes are stamped into the header's
//! `reserved1[0..4]`; `dwReserved2` (byte 124) holds a format class.

use crate::error::{PyluneError, Result};

/// Size of the classic DDS header including the `DDS ` magic.
const HEADER: usize = 128;
/// With the DX10 extension.
const HEADER_DX10: usize = 148;
/// Bytes of the per-texture template record in `0.pathc`.
pub const TEMPLATE_SIZE: usize = 148;

/// What the codec needs from a DDS header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DdsInfo {
    pub header_len: usize,
    pub width: u32,
    pub height: u32,
    pub mips: u32,
    /// Bytes per 4×4 block for block-compressed formats.
    pub block_bytes: Option<usize>,
    /// Bits per pixel for uncompressed formats.
    pub bits_per_pixel: Option<usize>,
    /// FourCC for legacy formats (`DXT1`…), 0 if none.
    pub four_cc: u32,
}

fn u32_at(data: &[u8], off: usize) -> Result<u32> {
    data.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| PyluneError::Archive("DDS header truncated".into()))
}

const fn fourcc(code: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*code)
}

/// Block size (bytes per 4×4 block) or bits per pixel of a DXGI format.
fn dxgi_layout(format: u32) -> Option<(Option<usize>, Option<usize>)> {
    match format {
        70..=72 | 79..=81 => Some((Some(8), None)),
        73..=78 | 82..=84 | 94..=99 => Some((Some(16), None)),
        2 => Some((None, Some(128))),
        10 | 11 => Some((None, Some(64))),
        24 | 26 | 27..=29 | 87..=88 | 90..=91 => Some((None, Some(32))),
        49 | 54 => Some((None, Some(16))),
        61 | 65 => Some((None, Some(8))),
        _ => None,
    }
}

/// Parses the parts of a DDS header the codec needs.
pub fn info(data: &[u8]) -> Result<DdsInfo> {
    if data.get(0..4) != Some(b"DDS ") {
        return Err(PyluneError::Archive("not a DDS file".into()));
    }
    let (height, width, mips) = (
        u32_at(data, 12)?,
        u32_at(data, 16)?,
        u32_at(data, 28)?.max(1),
    );
    let four_cc = u32_at(data, 84)?;
    let caps2 = u32_at(data, 112)?;
    if caps2 & 0x200 != 0 {
        return Err(PyluneError::Archive("cube maps are not supported".into()));
    }
    let (header_len, layout) = match four_cc {
        c if c == fourcc(b"DX10") => {
            if u32_at(data, 140)? > 1 {
                return Err(PyluneError::Archive(
                    "texture arrays are not supported".into(),
                ));
            }
            (HEADER_DX10, dxgi_layout(u32_at(data, 128)?))
        }
        c if c == fourcc(b"DXT1")
            || c == fourcc(b"ATI1")
            || c == fourcc(b"BC4U")
            || c == fourcc(b"BC4S") =>
        {
            (HEADER, Some((Some(8), None)))
        }
        c if [
            b"DXT2", b"DXT3", b"DXT4", b"DXT5", b"ATI2", b"BC5U", b"BC5S",
        ]
        .iter()
        .any(|f| c == fourcc(f)) =>
        {
            (HEADER, Some((Some(16), None)))
        }
        0 => (HEADER, Some((None, Some(u32_at(data, 88)? as usize)))),
        _ => (HEADER, None),
    };
    let (block_bytes, bits_per_pixel) =
        layout.ok_or_else(|| PyluneError::Archive("unsupported DDS pixel format".into()))?;
    Ok(DdsInfo {
        header_len,
        width,
        height,
        mips,
        block_bytes,
        bits_per_pixel,
        four_cc,
    })
}

/// Raw size in bytes of mip level `level`.
pub fn mip_size(dds: &DdsInfo, level: u32) -> usize {
    let w = (dds.width >> level).max(1) as usize;
    let h = (dds.height >> level).max(1) as usize;
    match (dds.block_bytes, dds.bits_per_pixel) {
        (Some(block), _) => w.div_ceil(4).max(1) * h.div_ceil(4).max(1) * block,
        (None, Some(bpp)) => (w * bpp).div_ceil(8) * h,
        (None, None) => 0,
    }
}

/// Number of leading mips stored as separate chunks.
fn chunk_count(dds: &DdsInfo) -> usize {
    dds.mips.min(4) as usize
}

/// Chunk sizes `m1..m4` for a texture whose chunks are all stored raw.
pub fn raw_chunk_sizes(dds: &DdsInfo) -> [u32; 4] {
    let raw = |i: usize| mip_size(dds, i as u32) as u32;
    match dds.mips {
        // Vanilla quirk for single-mip textures: (stored, raw, 0, 0).
        1 => [raw(0), raw(0), 0, 0],
        _ => std::array::from_fn(|i| if i < chunk_count(dds) { raw(i) } else { 0 }),
    }
}

/// Encodes a standard DDS file for the archive with raw chunks, stamping
/// the chunk sizes and the format class. Returns the stored bytes and m1..m4.
pub fn encode_raw(data: &[u8], format_class: u32) -> Result<(Vec<u8>, [u32; 4])> {
    let dds = info(data)?;
    let m = raw_chunk_sizes(&dds);
    let mut stored = data.to_vec();
    stored[32..76].fill(0);
    m.iter()
        .enumerate()
        .for_each(|(i, v)| stored[32 + i * 4..36 + i * 4].copy_from_slice(&v.to_le_bytes()));
    stored[124..128].copy_from_slice(&format_class.to_le_bytes());
    Ok((stored, m))
}

/// Decodes a stored partial DDS back to a standard DDS file.
pub fn decode(stored: &[u8], orig_size: usize) -> Result<Vec<u8>> {
    let dds = info(stored)?;
    let m: Vec<usize> = (0..4)
        .map(|i| u32_at(stored, 32 + i * 4).map(|v| v as usize))
        .collect::<Result<_>>()?;
    let mut out = stored[..dds.header_len].to_vec();
    out[32..76].fill(0);
    let mut cursor = dds.header_len;
    (0..chunk_count(&dds)).try_for_each(|i| {
        let raw = mip_size(&dds, i as u32);
        let chunk = stored
            .get(cursor..cursor + m[i])
            .ok_or_else(|| PyluneError::Archive("DDS chunk truncated".into()))?;
        match m[i] < raw {
            true => out.extend(
                lz4_flex::block::decompress(chunk, raw)
                    .map_err(|e| PyluneError::Archive(format!("DDS LZ4: {e}")))?,
            ),
            false => out.extend(&chunk[..raw]),
        }
        cursor += m[i];
        Ok::<(), PyluneError>(())
    })?;
    out.extend(&stored[cursor.min(stored.len())..]);
    match out.len() == orig_size {
        true => Ok(out),
        false => Err(PyluneError::Archive(format!(
            "DDS decoded to {} bytes, expected {orig_size}",
            out.len()
        ))),
    }
}

/// The `0.pathc` template record for a stored header: padded to 148 bytes,
/// `reserved1` zeroed, format class kept.
pub fn template(stored_header: &[u8]) -> [u8; TEMPLATE_SIZE] {
    let mut record = [0u8; TEMPLATE_SIZE];
    let n = stored_header.len().min(TEMPLATE_SIZE);
    record[..n].copy_from_slice(&stored_header[..n]);
    record[32..76].fill(0);
    record
}

/// Format class for brand-new textures (observed classes per format).
pub fn default_class(dds: &DdsInfo) -> u32 {
    match (dds.four_cc, dds.block_bytes) {
        (c, _) if c == fourcc(b"DXT1") => 12,
        (c, _) if c == fourcc(b"DXT3") || c == fourcc(b"DXT5") => 15,
        (c, _)
            if [b"ATI1", b"ATI2", b"BC4U", b"BC5U"]
                .iter()
                .any(|f| c == fourcc(f)) =>
        {
            4
        }
        _ => 15,
    }
}

#[cfg(test)]
mod tests;
