//! Read-only data table browser: listing available tables from the game's
//! archives, extracting table headers and bodies, and decoding record fields.

use super::{Record, records, spans, table as lookup_layout, value};
use crate::error::{PailuneError, Result};
use crate::paz::{extract, pamt};
use std::fs;
use std::path::Path;

/// High-level summary of one table available in the game archives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSummary {
    /// Stem name, e.g. "iteminfo" or "skill".
    pub stem: String,
    /// Whether Pailune has a verified layout definition for this table.
    pub has_layout: bool,
    /// Number of verified fields if layout exists.
    pub verified_fields: usize,
}

/// One decoded field of a record.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedField {
    pub name: String,
    pub shape: String,
    pub meaning: String,
    pub value: String,
    pub raw_hex: String,
    pub editable: bool,
}

/// One inspected record in a table.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectedRecord {
    pub key: u64,
    pub name: String,
    pub payload_size: usize,
    pub fields: Vec<DecodedField>,
    pub raw_hex: String,
}

/// Lists all table names available in the game's 0008 archive group.
pub fn list_tables(game_dir: &Path) -> Result<Vec<TableSummary>> {
    let group_dir = game_dir.join("0008");
    let pamt_path = group_dir.join("0.pamt");
    let pamt_bytes = fs::read(&pamt_path)
        .map_err(|e| PailuneError::Invalid(format!("could not read 0.pamt: {e}")))?;
    let index = pamt::parse(&pamt_bytes)?;

    let mut names = Vec::new();
    for path in index.files.keys() {
        if let Some(stem) = path
            .strip_prefix("gamedata/binarystaticinfo__/bin/")
            .and_then(|p| p.strip_suffix(".staticinfobody"))
        {
            names.push(stem.to_string());
        }
    }
    names.sort();
    let summaries = names
        .into_iter()
        .map(|stem| {
            let layout = lookup_layout(&stem);
            let has_layout = layout.is_some();
            let verified_fields = layout.map(|l| l.fields.len()).unwrap_or(0);
            TableSummary {
                stem,
                has_layout,
                verified_fields,
            }
        })
        .collect();
    Ok(summaries)
}

/// Reads the header and body for a table from 0008, and parses its records.
pub fn read_table(game_dir: &Path, stem: &str) -> Result<(Vec<Record>, Vec<u8>)> {
    let group_dir = game_dir.join("0008");
    let pamt_path = group_dir.join("0.pamt");
    let pamt_bytes = fs::read(&pamt_path)
        .map_err(|e| PailuneError::Invalid(format!("could not read 0.pamt: {e}")))?;
    let index = pamt::parse(&pamt_bytes)?;

    let body_path = format!("gamedata/binarystaticinfo__/bin/{stem}.staticinfobody");
    let header_path = format!("gamedata/binarystaticinfo__/bin/{stem}.staticinfoheader");

    let body_record = index
        .files
        .get(&body_path)
        .ok_or_else(|| PailuneError::Invalid(format!("table {stem} body not in 0.pamt")))?;
    let header_record = index
        .files
        .get(&header_path)
        .ok_or_else(|| PailuneError::Invalid(format!("table {stem} header not in 0.pamt")))?;

    let body_bytes = extract::extract(&group_dir, &body_path, body_record)?;
    let header_bytes = extract::extract(&group_dir, &header_path, header_record)?;

    let recs = records(&header_bytes, &body_bytes)?;
    Ok((recs, body_bytes))
}

/// Decodes fields of a single record using the table layout (if any).
pub fn inspect_record(stem: &str, record: &Record, body: &[u8]) -> InspectedRecord {
    let payload = match body.get(record.payload..record.end) {
        Some(p) => p,
        None => &[],
    };
    let raw_hex = crate::patch::document::hex(payload);
    let mut fields = Vec::new();

    if let Some(table) = lookup_layout(stem) {
        if let Some(spans) = spans(table, payload) {
            for (idx, field) in table.fields.iter().enumerate() {
                if let Some(&(start, end)) = spans.get(idx) {
                    if end <= payload.len() && start <= end {
                        let field_bytes = &payload[start..end];
                        let val_str = value::decode(field, field_bytes)
                            .map(|v| match v {
                                serde_json::Value::String(s) => s,
                                other => other.to_string(),
                            })
                            .unwrap_or_else(|_| crate::patch::document::hex(field_bytes));
                        fields.push(DecodedField {
                            name: field.name.clone(),
                            shape: field.shape.clone(),
                            meaning: field.meaning.clone(),
                            value: val_str,
                            raw_hex: crate::patch::document::hex(field_bytes),
                            editable: field.editable(),
                        });
                    }
                }
            }
        }
    }

    InspectedRecord {
        key: record.key,
        name: record.name.clone(),
        payload_size: payload.len(),
        fields,
        raw_hex,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspects_record_with_layout() {
        let mut body = Vec::new();
        let key = 0x0601u16;
        let price = 1500u64;
        body.extend_from_slice(&key.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        let payload_start = body.len();
        body.push(0); // _isBlocked (false)
        body.extend_from_slice(&price.to_le_bytes()); // _increasePrice
        body.push(1);
        let end = body.len();

        let rec = Record {
            start: 0,
            key: key as u64,
            name: "target_01".into(),
            payload: payload_start,
            end,
        };

        let inspected = inspect_record("wantedinfo", &rec, &body);
        assert_eq!(inspected.name, "target_01");
        assert_eq!(inspected.key, 0x0601);
        assert!(!inspected.fields.is_empty());
        assert_eq!(inspected.fields[0].name, "_isBlocked");
        assert_eq!(inspected.fields[0].value, "false");
        assert_eq!(inspected.fields[1].name, "_increasePrice");
        assert_eq!(inspected.fields[1].value, "1500");
    }

    #[test]
    fn inspects_record_without_layout_gives_raw_hex() {
        let body = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let rec = Record {
            start: 0,
            key: 42,
            name: "mystery".into(),
            payload: 0,
            end: 4,
        };

        let inspected = inspect_record("unknown_table_xyz", &rec, &body);
        assert_eq!(inspected.key, 42);
        assert!(inspected.fields.is_empty());
        assert_eq!(inspected.raw_hex, "deadbeef");
    }
}

