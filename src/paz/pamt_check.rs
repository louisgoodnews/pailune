//! Strict checks of a group index (`0.pamt`) against the invariants the
//! game's own indexes keep (`docs/paz-format.md` §5): folder hashes, sorted
//! folders and names, contiguous file ranges, archive checksums and sizes,
//! aligned offsets. Every vanilla group passes; Pailune checks its own
//! overlay with it before deploying.

use super::hash::pa_hash;
use super::pamt::full_name;
use super::reader::Reader;
use crate::error::Result;
use std::path::Path;

/// The problems found in `data`, the `0.pamt` of the group in `dir`
/// (empty when the index is sound).
pub fn problems(data: &[u8], dir: &Path) -> Result<Vec<String>> {
    let mut found = Vec::new();
    let mut r = Reader::new(data);
    let crc = r.u32()?;
    if data.len() < 12 || pa_hash(&data[12..]) != crc {
        found.push("index checksum".into());
    }
    let paz_count = r.u32()?;
    let _unknown = r.u32()?;
    let mut sizes = Vec::new();
    for _ in 0..paz_count {
        let (index, paz_crc, size) = (r.u32()?, r.u32()?, r.u32()?);
        let path = dir.join(format!("{index}.paz"));
        match std::fs::read(&path) {
            Ok(paz) if paz.len() as u64 != u64::from(size) => {
                found.push(format!("{index}.paz size {} ≠ {size}", paz.len()))
            }
            Ok(paz) if pa_hash(&paz) != paz_crc => found.push(format!("{index}.paz checksum")),
            Ok(_) => {}
            Err(err) => found.push(format!("{index}.paz: {err}")),
        }
        sizes.push(u64::from(size));
    }
    let dir_size = r.u32()? as usize;
    let dirs = r.bytes(dir_size)?.to_vec();
    let name_size = r.u32()? as usize;
    let names = r.bytes(name_size)?.to_vec();
    let folders = (0..r.u32()?)
        .map(|_| Ok((r.u32()?, r.u32()?, r.u32()? as usize, r.u32()? as usize)))
        .collect::<Result<Vec<_>>>()?;
    let files = (0..r.u32()?)
        .map(|_| {
            let name = r.u32()?;
            let (offset, stored, _orig) = (r.u32()?, r.u32()?, r.u32()?);
            let (paz, _flags) = (r.u16()?, r.u16()?);
            Ok((name, offset, stored, paz))
        })
        .collect::<Result<Vec<_>>>()?;
    if r.remaining() != 0 {
        found.push(format!("{} bytes after the file table", r.remaining()));
    }
    let mut next = 0;
    let mut last_folder: Option<String> = None;
    for (hash, name, first, count) in &folders {
        let folder = full_name(&dirs, *name, 0)?;
        if pa_hash(folder.as_bytes()) != *hash {
            found.push(format!("folder hash of \"{folder}\""));
        }
        if last_folder.as_ref().is_some_and(|l| *l >= folder) {
            found.push(format!("folder \"{folder}\" out of order"));
        }
        if *first != next {
            found.push(format!(
                "folder \"{folder}\" starts at file {first}, expected {next}"
            ));
        }
        next = first + count;
        let mut last_name: Option<String> = None;
        for (name, offset, stored, paz) in files.iter().skip(*first).take(*count) {
            let file = full_name(&names, *name, 0)?;
            if last_name.as_ref().is_some_and(|l| *l >= file) {
                found.push(format!("{folder}/{file} out of order"));
            }
            if offset % 16 != 0 {
                found.push(format!("{folder}/{file} offset not 16-aligned"));
            }
            let end = u64::from(*offset) + u64::from(*stored);
            if sizes.get(usize::from(*paz)).is_none_or(|size| end > *size) {
                found.push(format!("{folder}/{file} outside its archive"));
            }
            last_name = Some(file);
        }
        last_folder = Some(folder);
    }
    if next != files.len() {
        found.push(format!("folders cover {next} of {} files", files.len()));
    }
    Ok(found)
}
