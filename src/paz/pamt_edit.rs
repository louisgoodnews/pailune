//! Pointing a vanilla group index (`0.pamt`) at patched entries in an
//! additional archive of the same group (D-14, `docs/design/in-place.md`).
//! The index stays as the game wrote it except for three things: one more
//! archive in the archive table, the records of the redirected files, and
//! the header checksum. Names, folders and every other record keep their
//! bytes.

use super::hash::pa_hash;
use super::pamt::full_name;
use super::reader::Reader;
use crate::error::{PailuneError, Result};
use std::collections::BTreeMap;

/// Where a redirected file's entry now lives in the added archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub offset: u32,
    pub stored_size: u32,
    pub orig_size: u32,
    pub flags: u16,
}

/// The added archive: its checksum and size, for the archive table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddedPaz {
    pub crc: u32,
    pub size: u32,
}

fn bad(what: String) -> PailuneError {
    PailuneError::Archive(what)
}

/// Byte position of every file record, by lowercase virtual path, and the
/// position where the archive table ends.
fn layout(pamt: &[u8]) -> Result<(u32, usize, BTreeMap<String, usize>)> {
    let mut r = Reader::new(pamt);
    r.u32()?;
    let paz_count = r.u32()?;
    r.u32()?;
    r.bytes(paz_count as usize * 12)?;
    let table_end = 12 + paz_count as usize * 12;
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
    let count = r.u32()? as usize;
    let first_record = pamt.len() - r.remaining();
    if r.remaining() != count * 20 {
        return Err(bad("unexpected index layout after the file table".into()));
    }
    let mut records = BTreeMap::new();
    for (folder, first, n) in folders {
        for i in first..first + n {
            let at = first_record + i * 20;
            let name_offset = u32::from_le_bytes(
                pamt.get(at..at + 4)
                    .ok_or_else(|| bad("file range out of bounds".into()))?
                    .try_into()
                    .expect("4 bytes"),
            );
            let base = full_name(names, name_offset, 0)?;
            let path = match folder.is_empty() {
                true => base,
                false => format!("{folder}/{base}"),
            };
            records.insert(path.to_lowercase(), at);
        }
    }
    Ok((paz_count, table_end, records))
}

/// The index with `added` registered as the group's next archive and the
/// records of `redirects` pointing into it. Returns the new index and the
/// added archive's number (its file is `<number>.paz`).
pub fn redirect(
    pamt: &[u8],
    added: AddedPaz,
    redirects: &BTreeMap<String, Entry>,
) -> Result<(Vec<u8>, u32)> {
    let (paz_count, table_end, records) = layout(pamt)?;
    let index = paz_count;
    let paz_index = u16::try_from(index).map_err(|_| bad("too many archives".into()))?;
    let mut out = pamt.to_vec();
    for (vpath, entry) in redirects {
        let at = *records
            .get(vpath)
            .ok_or_else(|| bad(format!("{vpath} is not in this group")))?;
        out[at + 4..at + 8].copy_from_slice(&entry.offset.to_le_bytes());
        out[at + 8..at + 12].copy_from_slice(&entry.stored_size.to_le_bytes());
        out[at + 12..at + 16].copy_from_slice(&entry.orig_size.to_le_bytes());
        out[at + 16..at + 18].copy_from_slice(&paz_index.to_le_bytes());
        out[at + 18..at + 20].copy_from_slice(&entry.flags.to_le_bytes());
    }
    let row: Vec<u8> = [index, added.crc, added.size]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    out.splice(table_end..table_end, row);
    out[4..8].copy_from_slice(&(paz_count + 1).to_le_bytes());
    let crc = pa_hash(&out[12..]);
    out[..4].copy_from_slice(&crc.to_le_bytes());
    Ok((out, index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paz::pamt;
    use crate::paz::write::{Packed, build};

    #[test]
    fn redirected_records_point_into_the_added_archive() {
        let file = |vpath: &str, data: &[u8]| Packed {
            vpath: vpath.into(),
            stored: data.to_vec(),
            orig_size: data.len() as u32,
            flags: 0,
        };
        let vanilla = build(vec![
            file("gamedata/bin/a.body", b"aaaa"),
            file("gamedata/bin/b.body", b"bbbb"),
            file("ui/c.css", b"cc"),
        ]);
        let entry = Entry {
            offset: 32,
            stored_size: 7,
            orig_size: 9,
            flags: 2,
        };
        let added = AddedPaz {
            crc: 0xABCD,
            size: 64,
        };
        let redirects = BTreeMap::from([("gamedata/bin/b.body".to_string(), entry)]);
        let (index, number) = redirect(&vanilla.pamt, added, &redirects).unwrap();
        assert_eq!(number, 1);
        let parsed = pamt::parse(&index).unwrap();
        let b = parsed.files["gamedata/bin/b.body"];
        assert_eq!(
            (b.paz_index, b.offset, b.stored_size, b.orig_size, b.flags),
            (1, 32, 7, 9, 2)
        );
        let before = pamt::parse(&vanilla.pamt).unwrap();
        assert_eq!(
            parsed.files["gamedata/bin/a.body"],
            before.files["gamedata/bin/a.body"]
        );
        assert_eq!(parsed.files["ui/c.css"], before.files["ui/c.css"]);
        assert_eq!(parsed.pazs.len(), 2);
        assert_eq!(
            (
                parsed.pazs[1].index,
                parsed.pazs[1].crc,
                parsed.pazs[1].size
            ),
            (1, 0xABCD, 64)
        );
        // Only the archive table, the record and the checksum differ.
        assert_eq!(index.len(), vanilla.pamt.len() + 12);
        let missing = BTreeMap::from([("nope/x".to_string(), entry)]);
        assert!(redirect(&vanilla.pamt, added, &missing).is_err());
    }
}
