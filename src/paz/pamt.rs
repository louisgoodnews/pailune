//! `NNNN/0.pamt`: a pack group's index (`docs/paz-format.md` §5).

use super::hash::pa_hash;
use super::reader::Reader;
use crate::error::{PyluneError, Result};
use std::collections::HashMap;

/// "No node" marker in name tries and folder records.
const NONE: u32 = 0xFFFF_FFFF;

/// One `.paz` data file of the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PazInfo {
    pub index: u32,
    pub crc: u32,
    pub size: u32,
}

/// Where one file lives and how it is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileRecord {
    pub paz_index: u16,
    pub offset: u32,
    pub stored_size: u32,
    pub orig_size: u32,
    pub flags: u16,
}

/// A parsed index: virtual path (lowercase, `/`-separated) → record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pamt {
    pub crc: u32,
    pub unknown: u32,
    pub pazs: Vec<PazInfo>,
    pub files: HashMap<String, FileRecord>,
}

/// Resolves a node of a name trie by concatenating its ancestors.
fn full_name(block: &[u8], offset: u32, depth: usize) -> Result<String> {
    if offset == NONE {
        return Ok(String::new());
    }
    if depth > 512 {
        return Err(PyluneError::Archive(
            "pamt name trie too deep (cycle?)".into(),
        ));
    }
    let mut r = Reader::at(block, offset as usize);
    let parent = r.u32()?;
    let len = r.u8()? as usize;
    let fragment = String::from_utf8_lossy(r.bytes(len)?).into_owned();
    Ok(full_name(block, parent, depth + 1)? + &fragment)
}

/// Parses an index and verifies its checksum.
pub fn parse(data: &[u8]) -> Result<Pamt> {
    let mut r = Reader::new(data);
    let crc = r.u32()?;
    if data.len() < 12 || pa_hash(&data[12..]) != crc {
        return Err(PyluneError::Archive("0.pamt checksum mismatch".into()));
    }
    let paz_count = r.u32()?;
    let unknown = r.u32()?;
    let pazs = (0..paz_count)
        .map(|_| {
            Ok(PazInfo {
                index: r.u32()?,
                crc: r.u32()?,
                size: r.u32()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let dir_size = r.u32()? as usize;
    let dirs = r.bytes(dir_size)?;
    let name_size = r.u32()? as usize;
    let names = r.bytes(name_size)?;
    let folders = (0..r.u32()?)
        .map(|_| {
            let (_hash, name, first, count) = (r.u32()?, r.u32()?, r.u32()?, r.u32()?);
            Ok((full_name(dirs, name, 0)?, first as usize, count as usize))
        })
        .collect::<Result<Vec<_>>>()?;
    let records = (0..r.u32()?)
        .map(|_| {
            let name = r.u32()?;
            let (offset, stored_size, orig_size) = (r.u32()?, r.u32()?, r.u32()?);
            let (paz_index, flags) = (r.u16()?, r.u16()?);
            Ok((
                name,
                FileRecord {
                    paz_index,
                    offset,
                    stored_size,
                    orig_size,
                    flags,
                },
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let files = folders
        .iter()
        .flat_map(|(folder, first, count)| (*first..first + count).map(move |i| (folder, i)))
        .map(|(folder, i)| {
            let (name, record) = records
                .get(i)
                .ok_or_else(|| PyluneError::Archive("pamt file range out of bounds".into()))?;
            let base = full_name(names, *name, 0)?;
            let path = match folder.is_empty() {
                true => base,
                false => format!("{folder}/{base}"),
            };
            Ok((path.to_lowercase(), *record))
        })
        .collect::<Result<_>>()?;
    Ok(Pamt {
        crc,
        unknown,
        pazs,
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `0013/0.pamt`, transcribed from `docs/paz-format.md` §5.4.
    fn sample() -> Vec<u8> {
        let hex = concat!(
            "f3d9ba0e0100000032020e61000000008245715e5024100018000000ffffffff",
            "08616973637269707400000000062f62696e5f5f26000000ffffffff0b616963",
            "686172742e706169ffffffff117061746866696e647461626c652e7061690200",
            "0000dc45ade9000000000000000000000000c4826ad30d000000000000000200",
            "000002000000000000000000000081f60f00bd5c3600000002001000000090f6",
            "0f00b62d00008723030000000200",
        );
        let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        (0..clean.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn parses_the_documented_example() {
        let pamt = parse(&sample()).unwrap();
        assert_eq!(
            pamt.pazs,
            vec![PazInfo {
                index: 0,
                crc: 0x5E71_4582,
                size: 0x10_2450
            }]
        );
        let rec = pamt.files["aiscript/bin__/pathfindtable.pai"];
        assert_eq!(
            (rec.offset, rec.stored_size, rec.orig_size, rec.flags),
            (0x0F_F690, 11_702, 205_703, 2)
        );
        assert_eq!(pamt.files.len(), 2);
    }
}
