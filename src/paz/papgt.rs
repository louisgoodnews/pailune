//! `meta/0.papgt`: the pack group table (`docs/paz-format.md` §4).

use super::hash::pa_hash;
use super::reader::Reader;
use crate::error::{PyluneError, Result};

/// One registered pack group (directory such as `0000`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupEntry {
    /// May be absent on disk (optional voice/language packs).
    pub is_optional: bool,
    pub lang_mask: u16,
    pub name: String,
    /// Must equal the group's `0.pamt` header checksum.
    pub pamt_crc: u32,
}

/// The whole table. Unknown header fields are preserved verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Papgt {
    pub unknown0: u32,
    pub unknown1: u16,
    pub groups: Vec<GroupEntry>,
}

/// NUL-terminated string at `offset` in the string table.
fn name_at(table: &[u8], offset: u32) -> Result<String> {
    let start = offset as usize;
    let rest = table
        .get(start..)
        .ok_or_else(|| PyluneError::Archive("papgt name offset out of range".into()))?;
    let end = rest.iter().position(|b| *b == 0).unwrap_or(rest.len());
    Ok(String::from_utf8_lossy(&rest[..end]).into_owned())
}

/// Parses and verifies the file checksum.
pub fn parse(data: &[u8]) -> Result<Papgt> {
    let mut r = Reader::new(data);
    let unknown0 = r.u32()?;
    let crc = r.u32()?;
    let count = r.u8()? as usize;
    let unknown1 = r.u16()?;
    r.u8()?;
    if pa_hash(&data[12..]) != crc {
        return Err(PyluneError::Archive("0.papgt checksum mismatch".into()));
    }
    let raw: Vec<(bool, u16, u32, u32)> = (0..count)
        .map(|_| {
            let optional = r.u8()? == 1;
            let lang = r.u16()?;
            r.u8()?;
            Ok((optional, lang, r.u32()?, r.u32()?))
        })
        .collect::<Result<_>>()?;
    let table_size = r.u32()? as usize;
    let table = r.bytes(table_size)?;
    let groups = raw
        .into_iter()
        .map(|(is_optional, lang_mask, offset, pamt_crc)| {
            Ok(GroupEntry {
                is_optional,
                lang_mask,
                name: name_at(table, offset)?,
                pamt_crc,
            })
        })
        .collect::<Result<_>>()?;
    Ok(Papgt {
        unknown0,
        unknown1,
        groups,
    })
}

/// Serialises the table, rebuilding the string table and checksum.
pub fn serialize(papgt: &Papgt) -> Result<Vec<u8>> {
    let count = u8::try_from(papgt.groups.len())
        .map_err(|_| PyluneError::Archive("more than 255 pack groups".into()))?;
    let (entries, table) =
        papgt
            .groups
            .iter()
            .fold((Vec::new(), Vec::new()), |(mut entries, mut table), g| {
                entries.push(g.is_optional as u8);
                entries.extend(g.lang_mask.to_le_bytes());
                entries.push(0);
                entries.extend((table.len() as u32).to_le_bytes());
                entries.extend(g.pamt_crc.to_le_bytes());
                table.extend(g.name.as_bytes());
                table.push(0);
                (entries, table)
            });
    let mut out = Vec::with_capacity(16 + entries.len() + table.len());
    out.extend(papgt.unknown0.to_le_bytes());
    out.extend([0u8; 4]);
    out.push(count);
    out.extend(papgt.unknown1.to_le_bytes());
    out.push(0);
    out.extend(entries);
    out.extend((table.len() as u32).to_le_bytes());
    out.extend(table);
    let crc = pa_hash(&out[12..]);
    out[4..8].copy_from_slice(&crc.to_le_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_detects_corruption() {
        let table = Papgt {
            unknown0: 0x12FA,
            unknown1: 2,
            groups: vec![
                GroupEntry {
                    is_optional: false,
                    lang_mask: 0x7FFF,
                    name: "0000".into(),
                    pamt_crc: 0x728E_5337,
                },
                GroupEntry {
                    is_optional: true,
                    lang_mask: 0x0080,
                    name: "dmmsa".into(),
                    pamt_crc: 7,
                },
            ],
        };
        let bytes = serialize(&table).unwrap();
        assert_eq!(parse(&bytes).unwrap(), table);
        assert_eq!(
            serialize(&parse(&bytes).unwrap()).unwrap(),
            bytes,
            "byte-identical"
        );
        let mut broken = bytes.clone();
        *broken.last_mut().unwrap() ^= 1;
        assert!(parse(&broken).is_err());
    }
}
