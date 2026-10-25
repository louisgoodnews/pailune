//! Encoding file content for an archive entry: compress, then encrypt (§7).

use super::{Compression, Encryption, crypto};
use crate::error::{PyluneError, Result};

/// Turns original bytes into stored bytes according to `flags`.
pub fn encode(vpath: &str, data: &[u8], flags: u16) -> Result<Vec<u8>> {
    let mut stored = match Compression::from_flags(flags) {
        Compression::None => data.to_vec(),
        Compression::Lz4 => lz4_flex::block::compress(data),
        Compression::DdsPartial => {
            return Err(PyluneError::Archive(format!(
                "{vpath}: textures need PATHC support (not yet)"
            )));
        }
        Compression::Unknown(n) => {
            return Err(PyluneError::Archive(format!(
                "{vpath}: unknown compression {n}"
            )));
        }
    };
    match Encryption::from_flags(flags) {
        Encryption::None => {}
        Encryption::ChaCha20 => crypto::apply(vpath, &mut stored)?,
        Encryption::Unknown(n) => {
            return Err(PyluneError::Archive(format!(
                "{vpath}: unknown encryption {n}"
            )));
        }
    }
    Ok(stored)
}
