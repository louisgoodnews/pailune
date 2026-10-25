//! `meta/0.pathc`: the global texture header collection (§6).
//!
//! Maps `pa_hash("/" + vpath)` to a DDS header template and the four mip
//! chunk sizes. It has no checksum.

use super::hash::pa_hash;
use super::reader::Reader;
use crate::error::{PyluneError, Result};

/// One map entry: selector plus chunk sizes m1..m4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapEntry {
    pub selector: u32,
    pub m: [u32; 4],
}

/// One entry of a hash collision group, disambiguated by full path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Collision {
    pub path_offset: u32,
    pub record: u32,
    pub m: [u32; 4],
}

/// The whole table; unknown header words are preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pathc {
    pub unknown: [u32; 2],
    pub record_size: u32,
    pub records: Vec<Vec<u8>>,
    /// Sorted ascending, parallel to `map`.
    pub hashes: Vec<u32>,
    pub map: Vec<MapEntry>,
    pub collisions: Vec<Collision>,
    pub blob: Vec<u8>,
}

/// Direct selectors carry `0xFFFF` in the high half.
const DIRECT: u32 = 0xFFFF_0000;

/// Reads the four chunk sizes m1..m4.
fn sizes(r: &mut Reader) -> Result<[u32; 4]> {
    Ok([r.u32()?, r.u32()?, r.u32()?, r.u32()?])
}

/// Parses the table.
pub fn parse(data: &[u8]) -> Result<Pathc> {
    let mut r = Reader::new(data);
    let unknown = [r.u32()?, r.u32()?];
    let (record_size, records, hashes, collisions, blob) =
        (r.u32()?, r.u32()?, r.u32()?, r.u32()?, r.u32()?);
    let records = (0..records)
        .map(|_| Ok(r.bytes(record_size as usize)?.to_vec()))
        .collect::<Result<_>>()?;
    let hashes: Vec<u32> = (0..hashes).map(|_| r.u32()).collect::<Result<_>>()?;
    let map = (0..hashes.len())
        .map(|_| {
            Ok(MapEntry {
                selector: r.u32()?,
                m: sizes(&mut r)?,
            })
        })
        .collect::<Result<_>>()?;
    let collisions = (0..collisions)
        .map(|_| {
            Ok(Collision {
                path_offset: r.u32()?,
                record: r.u32()?,
                m: sizes(&mut r)?,
            })
        })
        .collect::<Result<_>>()?;
    let blob = r.bytes(blob as usize)?.to_vec();
    Ok(Pathc {
        unknown,
        record_size,
        records,
        hashes,
        map,
        collisions,
        blob,
    })
}

/// Serialises the table (no checksum to fix).
pub fn serialize(p: &Pathc) -> Vec<u8> {
    let words = [
        p.unknown[0],
        p.unknown[1],
        p.record_size,
        p.records.len() as u32,
        p.hashes.len() as u32,
    ];
    let tail = [p.collisions.len() as u32, p.blob.len() as u32];
    let le = |v: &u32| v.to_le_bytes();
    words
        .iter()
        .chain(&tail)
        .flat_map(le)
        .chain(p.records.iter().flatten().copied())
        .chain(p.hashes.iter().flat_map(le))
        .chain(p.map.iter().flat_map(|e| {
            std::iter::once(e.selector)
                .chain(e.m)
                .flat_map(|v| v.to_le_bytes())
        }))
        .chain(p.collisions.iter().flat_map(|c| {
            [c.path_offset, c.record]
                .into_iter()
                .chain(c.m)
                .flat_map(|v| v.to_le_bytes())
        }))
        .chain(p.blob.iter().copied())
        .collect()
}

/// Key of a virtual path.
pub fn key(vpath: &str) -> u32 {
    pa_hash(format!("/{}", vpath.to_lowercase()).as_bytes())
}

impl Pathc {
    /// A table with no entries (used when no textures are packed).
    pub fn empty() -> Self {
        Self {
            unknown: [0, 0],
            record_size: super::dds::TEMPLATE_SIZE as u32,
            records: Vec::new(),
            hashes: Vec::new(),
            map: Vec::new(),
            collisions: Vec::new(),
            blob: Vec::new(),
        }
    }

    /// The full path stored at `offset` in the collision blob.
    fn blob_path(&self, offset: u32) -> &[u8] {
        let rest = self.blob.get(offset as usize..).unwrap_or_default();
        &rest[..rest.iter().position(|b| *b == 0).unwrap_or(rest.len())]
    }

    /// Template record and chunk sizes currently registered for `vpath`.
    pub fn lookup(&self, vpath: &str) -> Option<(&[u8], [u32; 4])> {
        let i = self.hashes.binary_search(&key(vpath)).ok()?;
        let entry = self.map[i];
        let (record, m) = match entry.selector >> 16 {
            0xFFFF => (entry.selector & 0xFFFF, entry.m),
            _ => {
                let path = format!("/{}", vpath.to_lowercase());
                let (start, end) = ((entry.selector >> 16) & 0xFF, entry.selector >> 24);
                let c = self
                    .collisions
                    .get(start as usize..end as usize)?
                    .iter()
                    .find(|c| self.blob_path(c.path_offset) == path.as_bytes())?;
                (c.record, c.m)
            }
        };
        self.records.get(record as usize).map(|r| (r.as_slice(), m))
    }

    /// Adds or replaces the entry for `vpath` (§6.3).
    pub fn upsert(&mut self, vpath: &str, template: &[u8], m: [u32; 4]) -> Result<()> {
        if template.len() != self.record_size as usize {
            return Err(PyluneError::Archive(
                "texture template has the wrong size".into(),
            ));
        }
        let record = match self.records.iter().position(|r| r == template) {
            Some(i) => i,
            None => {
                self.records.push(template.to_vec());
                self.records.len() - 1
            }
        } as u32;
        if record > 0xFFFF {
            return Err(PyluneError::Archive("too many texture templates".into()));
        }
        match self.hashes.binary_search(&key(vpath)) {
            Ok(i) if self.map[i].selector >> 16 == 0xFFFF => {
                self.map[i] = MapEntry {
                    selector: DIRECT | record,
                    m,
                }
            }
            Ok(i) => {
                let path = format!("/{}", vpath.to_lowercase());
                let (start, end) = (
                    (self.map[i].selector >> 16) & 0xFF,
                    self.map[i].selector >> 24,
                );
                let found = (start..end).find(|j| {
                    self.blob_path(self.collisions[*j as usize].path_offset) == path.as_bytes()
                });
                let j = found.ok_or_else(|| {
                    PyluneError::Archive(format!("{vpath}: hash collides with another texture"))
                })?;
                let c = &mut self.collisions[j as usize];
                (c.record, c.m) = (record, m);
            }
            Err(i) => {
                self.hashes.insert(i, key(vpath));
                self.map.insert(
                    i,
                    MapEntry {
                        selector: DIRECT | record,
                        m,
                    },
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upserts_keep_the_table_sorted_and_round_trip() {
        let mut table = Pathc {
            unknown: [0, 0],
            record_size: 4,
            records: vec![b"AAAA".to_vec()],
            hashes: vec![key("ui/a.dds")],
            map: vec![MapEntry {
                selector: DIRECT,
                m: [1, 1, 0, 0],
            }],
            collisions: vec![],
            blob: vec![],
        };
        table.upsert("ui/b.dds", b"BBBB", [5, 6, 0, 0]).unwrap();
        table.upsert("ui/a.dds", b"BBBB", [7, 7, 0, 0]).unwrap();
        assert_eq!(table.records.len(), 2, "identical templates are shared");
        assert!(table.hashes.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(table.lookup("UI/A.dds"), Some((&b"BBBB"[..], [7, 7, 0, 0])));
        let bytes = serialize(&table);
        assert_eq!(parse(&bytes).unwrap(), table);
        assert!(table.upsert("ui/c.dds", b"toolong", [0; 4]).is_err());
    }
}
