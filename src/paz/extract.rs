//! Reading one archived file: read → decrypt → decompress (§7).

use super::pamt::FileRecord;
use super::{Compression, Encryption, crypto};
use crate::error::{IoContext, PyluneError, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// The stored bytes of `record` inside `group_dir/<paz_index>.paz`.
pub fn read_stored(group_dir: &Path, record: &FileRecord) -> Result<Vec<u8>> {
    let path = group_dir.join(format!("{}.paz", record.paz_index));
    let mut file = File::open(&path).at(&path)?;
    file.seek(SeekFrom::Start(u64::from(record.offset)))
        .at(&path)?;
    let mut buf = vec![0u8; record.stored_size as usize];
    file.read_exact(&mut buf).at(&path)?;
    Ok(buf)
}

/// Decodes stored bytes into the original file content.
pub fn decode(vpath: &str, record: &FileRecord, mut stored: Vec<u8>) -> Result<Vec<u8>> {
    match Encryption::from_flags(record.flags) {
        Encryption::None => {}
        Encryption::ChaCha20 => crypto::apply(vpath, &mut stored)?,
        Encryption::Unknown(n) => {
            return Err(PyluneError::Archive(format!(
                "{vpath}: unknown encryption {n}"
            )));
        }
    }
    match Compression::from_flags(record.flags) {
        Compression::None => Ok(stored),
        Compression::Lz4 => lz4_flex::block::decompress(&stored, record.orig_size as usize)
            .map_err(|e| PyluneError::Archive(format!("{vpath}: LZ4: {e}"))),
        Compression::DdsPartial => super::dds::decode(&stored, record.orig_size as usize)
            .map_err(|e| PyluneError::Archive(format!("{vpath}: {e}"))),
        Compression::Unknown(n) => Err(PyluneError::Archive(format!(
            "{vpath}: unknown compression {n}"
        ))),
    }
}

/// Extracts the file `vpath` described by `record` from its group folder.
pub fn extract(group_dir: &Path, vpath: &str, record: &FileRecord) -> Result<Vec<u8>> {
    let data = decode(vpath, record, read_stored(group_dir, record)?)?;
    match data.len() == record.orig_size as usize {
        true => Ok(data),
        false => Err(PyluneError::Archive(format!(
            "{vpath}: decoded {} bytes, index says {}",
            data.len(),
            record.orig_size
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_encrypted_lz4_entries() {
        let original = b"<root><item stack=\"999\"/></root>".repeat(20);
        let mut stored = lz4_flex::block::compress(&original);
        crypto::apply("ui/test.xml", &mut stored).unwrap();
        let record = FileRecord {
            paz_index: 0,
            offset: 0,
            stored_size: stored.len() as u32,
            orig_size: original.len() as u32,
            flags: 0x32,
        };
        assert_eq!(decode("ui/test.xml", &record, stored).unwrap(), original);
    }
}
