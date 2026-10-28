//! Layouts of the game's data tables (`*.staticinfobody`), derived from the
//! game itself (`docs/research/table-layouts.md`), and an exact decoder.
//!
//! A layout is trusted only after it decodes **every record** of the
//! installed table with no byte left over ([`verify`]); otherwise its table
//! is treated as unknown for this game build. A *partial* layout lists only
//! a table's leading fields; it is trusted when those decode on every record. Each field records whether its
//! offset is proven (every exact fit agrees) and whether its name is certain
//! (no other field of the class has the same shape, so names cannot swap).

pub mod browser;
pub mod format3;
mod format3_read;
pub mod value;

use crate::error::{PailuneError, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The derived layouts shipped with Pailune.
const DATA: &str = include_str!("../../data/table-layouts.json");

/// The layout file.
#[derive(Debug, Deserialize)]
pub struct Layouts {
    pub game_version: String,
    pub tables: BTreeMap<String, Table>,
}

/// One table's layout.
#[derive(Debug, Clone, Deserialize)]
pub struct Table {
    pub class: String,
    pub key_size: usize,
    pub records: usize,
    /// Only the leading fields are known; the rest of each record is left
    /// as it is.
    #[serde(default)]
    pub partial: bool,
    pub fields: Vec<Field>,
}

/// One field of a record payload (the key and name come before it).
#[derive(Debug, Clone, Deserialize)]
pub struct Field {
    pub name: String,
    /// `fixed:N`, `str`, `loc`, `list:N`, `list:str`, `list:loc`, `struct`, `list:struct`.
    pub shape: String,
    /// `bool`, `int`, `float`, `ref`, `hash`, `vec3` or `raw`.
    pub meaning: String,
    /// `proven` or `inferred`.
    pub offset: String,
    pub name_certain: bool,
    #[serde(default)]
    pub subfields: Vec<Field>,
}

impl Field {
    /// Whether mods may change this field by name.
    pub fn editable(&self) -> bool {
        (self.offset == "proven" && self.name_certain && self.shape.starts_with("fixed:"))
            || (self.shape.starts_with("list")
                && (self.offset == "proven" || self.offset == "inferred"))
            || self.shape == "struct"
    }
}

/// The shipped layouts.
pub fn layouts() -> &'static Layouts {
    static LAYOUTS: OnceLock<Layouts> = OnceLock::new();
    LAYOUTS.get_or_init(|| serde_json::from_str(DATA).expect("table-layouts.json is valid"))
}

/// The layout of a table by file name (`iteminfo.staticinfobody`,
/// `iteminfo.pabgb` or `iteminfo`).
pub fn table(name: &str) -> Option<&'static Table> {
    let base = name.rsplit('/').next().unwrap_or(name).to_lowercase();
    let stem = base.split('.').next().unwrap_or(&base);
    layouts().tables.get(stem)
}

pub fn u32_at(data: &[u8], at: usize) -> Option<usize> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

/// End of one value of `shape` starting at `at`.
pub fn skip(shape: &str, data: &[u8], at: usize) -> Option<usize> {
    let string = |at: usize| {
        u32_at(data, at)
            .and_then(|n| at.checked_add(4 + n))
            .filter(|e| *e <= data.len())
    };
    let end = match shape.split_once(':') {
        Some(("fixed", n)) => at + n.parse::<usize>().ok()?,
        None if shape == "str" => string(at)?,
        None if shape == "loc" => string(at + 9)?,
        Some(("list", elem)) => {
            let count = u32_at(data, at)?;
            let mut pos = at + 4;
            match elem.parse::<usize>() {
                Ok(size) => pos = pos.checked_add(count.checked_mul(size)?)?,
                Err(_) => {
                    for _ in 0..count {
                        pos = skip(elem, data, pos)?;
                    }
                }
            }
            pos
        }
        _ => return None,
    };
    (end <= data.len()).then_some(end)
}

/// End of one field value, taking nested subfields into account.
pub fn skip_field(field: &Field, data: &[u8], at: usize) -> Option<usize> {
    if field.shape == "struct" {
        let mut pos = at;
        for sub in &field.subfields {
            pos = skip_field(sub, data, pos)?;
        }
        (pos <= data.len()).then_some(pos)
    } else if field.shape == "list:struct" {
        let count = u32_at(data, at)?;
        let mut pos = at + 4;
        for _ in 0..count {
            for sub in &field.subfields {
                pos = skip_field(sub, data, pos)?;
            }
        }
        (pos <= data.len()).then_some(pos)
    } else {
        skip(&field.shape, data, at)
    }
}

/// Field spans `(start, end)` within one record payload; `None` if the
/// layout does not account for every byte (for partial layouts: if the
/// leading fields do not fit).
pub fn spans(table: &Table, payload: &[u8]) -> Option<Vec<(usize, usize)>> {
    let mut at = 0;
    let spans = table
        .fields
        .iter()
        .map(|f| {
            let end = skip_field(f, payload, at)?;
            let span = (at, end);
            at = end;
            Some(span)
        })
        .collect::<Option<Vec<_>>>()?;
    (table.partial || at == payload.len()).then_some(spans)
}

/// A record of a table body: where it starts, its key, name and payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub start: usize,
    pub key: u64,
    pub name: String,
    /// Offset of the payload in the body.
    pub payload: usize,
    pub end: usize,
}

/// The records of a table, in body order.
pub fn records(header: &[u8], body: &[u8]) -> Result<Vec<Record>> {
    let index = crate::patch::table::parse_index(header)
        .ok_or_else(|| PailuneError::Invalid("unreadable table index".into()))?;
    let mut starts: Vec<usize> = index.offsets.iter().map(|o| *o as usize).collect();
    starts.sort_unstable();
    let ends = starts
        .iter()
        .skip(1)
        .copied()
        .chain(std::iter::once(body.len()));
    starts
        .iter()
        .zip(ends.collect::<Vec<_>>())
        .map(|(&start, end)| {
            let ks = index.key_size;
            let key_bytes = body
                .get(start..start + ks)
                .ok_or_else(|| PailuneError::Invalid("record outside the table".into()))?;
            let mut key = [0u8; 8];
            key[..ks.min(8)].copy_from_slice(&key_bytes[..ks.min(8)]);
            let name_len = u32_at(body, start + ks)
                .ok_or_else(|| PailuneError::Invalid("record name".into()))?;
            let payload = start + ks + 4 + name_len;
            let name = body
                .get(start + ks + 4..payload)
                .map(|n| String::from_utf8_lossy(n).into_owned())
                .unwrap_or_default();
            match payload <= end {
                true => Ok(Record {
                    start,
                    key: u64::from_le_bytes(key),
                    name,
                    payload,
                    end,
                }),
                false => Err(PailuneError::Invalid(
                    "record name runs past the record".into(),
                )),
            }
        })
        .collect()
}

/// Whether `table` decodes every record of this body exactly.
pub fn verify(table: &Table, header: &[u8], body: &[u8]) -> Result<()> {
    let key_size = crate::patch::table::parse_index(header).map(|i| i.key_size);
    if key_size != Some(table.key_size) {
        return Err(PailuneError::Invalid(format!(
            "the index of {} changed in this game build; format-3 changes to it are disabled",
            table.class
        )));
    }
    let records = records(header, body)?;
    let bad = records
        .iter()
        .filter(|r| spans(table, &body[r.payload..r.end]).is_none())
        .count();
    match bad {
        0 => Ok(()),
        n => Err(PailuneError::Invalid(format!(
            "the layout of {} (derived from game {}, {} records) does not match this game \
             build ({n} of {} records); format-3 changes to it are disabled",
            table.class,
            layouts().game_version,
            table.records,
            records.len()
        ))),
    }
}

#[cfg(test)]
mod tests;
