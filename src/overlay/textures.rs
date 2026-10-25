//! Packing DDS textures and registering them in `0.pathc` (§6.3, §7.4).

use crate::error::Result;
use crate::paz::pathc::Pathc;
use crate::paz::write::Packed;
use crate::paz::{Encryption, crypto, dds};

/// Compression type 1 ("partial DDS"), keeping the encryption bits.
fn texture_flags(vanilla_flags: u16) -> u16 {
    (vanilla_flags & 0xFFF0) | 0x0001
}

/// Encodes a texture with raw chunks and records its PATHC entry. The
/// format class comes from the texture it replaces, else from its format.
pub fn pack(vpath: &str, data: &[u8], vanilla_flags: u16, table: &mut Pathc) -> Result<Packed> {
    let info = dds::info(data)?;
    let class = table
        .lookup(vpath)
        .and_then(|(record, _)| record.get(124..128))
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .unwrap_or_else(|| dds::default_class(&info));
    let (mut stored, m) = dds::encode_raw(data, class)?;
    table.upsert(vpath, &dds::template(&stored[..info.header_len]), m)?;
    let flags = texture_flags(vanilla_flags);
    if Encryption::from_flags(flags) == Encryption::ChaCha20 {
        crypto::apply(vpath, &mut stored)?;
    }
    Ok(Packed {
        vpath: vpath.to_string(),
        stored,
        orig_size: data.len() as u32,
        flags,
    })
}
