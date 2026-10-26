//! Field-name JSON mods ("format 3", `.field.json`): `set` by field name and
//! raw `old`/`new` within a record, resolved against the game's own table
//! and Pylune's derived layouts into ordinary verified byte changes.
//!
//! Only fields whose offset is proven and whose name is certain may be set;
//! other operations (`clone_record`, `new_record`, `delete_record`,
//! `array_append`, `scale`) are refused with a message.

use super::{Table, records, spans, verify};
use crate::patch::{Anchor, Change, Edit, FilePatch};
use serde_json::Value;
use std::collections::HashMap;

/// One intent of a format-3 file.
#[derive(Debug, Clone, PartialEq)]
pub struct Intent {
    /// Record name.
    pub entry: Option<String>,
    /// Record key.
    pub key: Option<u64>,
    pub field: Option<String>,
    pub op: String,
    pub new: Value,
    /// Raw replace: bytes to find in the record.
    pub old: Option<String>,
    /// Format 3.1: records chosen by field values (`"match"`), not yet
    /// supported.
    pub matches: Option<Value>,
}

/// The intents for one table file.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldPatch {
    pub target: String,
    pub intents: Vec<Intent>,
}

pub use super::format3_read::{is_format3, parse};

/// `_maxStackCount`, `maxStackCount` and `max_stack_count` compare equal.
fn same_name(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        s.chars()
            .filter(|c| *c != '_')
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    norm(a) == norm(b)
}

/// Turns one intent into a change, or explains why it cannot be applied.
/// Raw replaces need only record boundaries; `set` needs a verified
/// layout (`layout` explains why there is none).
fn change(
    layout: &std::result::Result<&Table, String>,
    body: &[u8],
    record: &super::Record,
    i: &Intent,
) -> std::result::Result<Change, String> {
    let payload = &body[record.payload..record.end];
    let at = |offset: usize, original: Vec<u8>, patched: Vec<u8>, label: String| Change {
        anchor: Anchor::Offset((record.payload + offset) as u64),
        edit: Edit::Replace {
            original: Some(original),
            patched,
        },
        label,
    };
    if let (Some(old), Value::String(new)) = (&i.old, &i.new) {
        let (old, new) = (
            crate::patch::parse::hex_bytes(old).map_err(|e| e.to_string())?,
            crate::patch::parse::hex_bytes(new).map_err(|e| e.to_string())?,
        );
        let hits: Vec<usize> = payload
            .windows(old.len().max(1))
            .enumerate()
            .filter(|(_, w)| *w == old.as_slice())
            .map(|(p, _)| p)
            .collect();
        return match (hits.as_slice(), old.len() == new.len()) {
            ([one], true) => Ok(at(*one, old, new, format!("{}: raw replace", record.name))),
            (_, false) => Err("raw replace must keep the length".into()),
            ([], _) => Err(format!("{}: old bytes not found", record.name)),
            _ => Err(format!(
                "{}: old bytes are not unique in the record",
                record.name
            )),
        };
    }
    if i.op != "set" {
        return Err(format!("operation \"{}\" is not supported yet", i.op));
    }
    let table = layout.as_ref().map_err(String::clone)?;
    let name = i.field.as_deref().ok_or("an intent without a field")?;
    let index = table
        .fields
        .iter()
        .position(|f| same_name(&f.name, name))
        .ok_or_else(|| format!("{name}: no such field in {}", table.class))?;
    let field = &table.fields[index];
    if !field.editable() {
        return Err(format!(
            "{}: the layout of this field is not proven yet, so it is not changed",
            field.name
        ));
    }
    let (start, end) = spans(table, payload).ok_or("record does not match the layout")?[index];
    let bytes = super::value::encode(field, end - start, &i.new)?;
    Ok(at(
        start,
        payload[start..end].to_vec(),
        bytes,
        format!("{}.{}", record.name, field.name),
    ))
}

/// Why Pylune cannot carry out an intent of a newer format revision yet.
fn unsupported(i: &Intent) -> Option<String> {
    let field = i.field.as_deref().unwrap_or_default();
    match (&i.matches, field.contains(['[', '.'])) {
        (Some(_), _) => Some(format!(
            "{field}: choosing records by \"match\" (format 3.1) is not supported yet"
        )),
        (None, true) => Some(format!(
            "{field}: nested fields (format 3.1) are not supported yet"
        )),
        (None, false) => None,
    }
}

/// Resolves intents for a table into changes; failures are explained.
pub fn resolve(
    table_file: &str,
    header: &[u8],
    body: &[u8],
    intents: &[Intent],
) -> (Vec<Change>, Vec<String>) {
    let layout = match super::table(table_file) {
        None => Err(format!(
            "{table_file}: no verified layout for this table yet (raw old/new changes work)"
        )),
        Some(table) => verify(table, header, body)
            .map(|()| table)
            .map_err(|e| e.to_string()),
    };
    let all = match records(header, body) {
        Ok(all) => all,
        Err(err) => return (Vec::new(), vec![err.to_string()]),
    };
    // Records by key and by name: mods like Fat Stacks carry thousands of
    // intents against thousands of records.
    let mut by_key: HashMap<u64, Vec<&super::Record>> = HashMap::new();
    let mut by_name: HashMap<&str, Vec<&super::Record>> = HashMap::new();
    all.iter().for_each(|r| {
        by_key.entry(r.key).or_default().push(r);
        by_name.entry(r.name.as_str()).or_default().push(r);
    });
    intents
        .iter()
        .fold((Vec::new(), Vec::new()), |(mut ok, mut failed), i| {
            if let Some(why) = unsupported(i) {
                failed.push(why);
                return (ok, failed);
            }
            let candidates = match (i.key, i.entry.as_deref()) {
                (Some(key), _) => by_key.get(&key),
                (None, Some(name)) => by_name.get(name),
                (None, None) => None,
            };
            let found: Vec<&super::Record> = candidates
                .into_iter()
                .flatten()
                .copied()
                .filter(|r| i.entry.as_deref().is_none_or(|e| r.name == e))
                .collect();
            match found.as_slice() {
                [one] => match change(&layout, body, one, i) {
                    Ok(c) => ok.push(c),
                    Err(why) => failed.push(why),
                },
                [] => failed.push(format!("record {:?} / key {:?} not found", i.entry, i.key)),
                _ => failed.push(format!(
                    "{:?}: {} records share this name; add its \"key\"",
                    i.entry.as_deref().unwrap_or_default(),
                    found.len()
                )),
            }
            (ok, failed)
        })
}

/// Resolves format-3 intents into byte changes against the current table.
pub fn with_fields(
    p: &FilePatch,
    vpath: &str,
    header: Option<&[u8]>,
    data: &[u8],
) -> (FilePatch, Vec<String>) {
    if p.fields.is_empty() {
        return (p.clone(), Vec::new());
    }
    let Some(header) = header else {
        return (p.clone(), vec!["field changes need a data table".into()]);
    };
    let (changes, failed) = resolve(vpath, header, data, &p.fields);
    let mut patch = p.clone();
    patch.changes.extend(changes);
    (patch, failed)
}
