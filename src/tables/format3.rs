//! Field-name JSON mods ("format 3", `.field.json`): `set` by field name and
//! raw `old`/`new` within a record, resolved against the game's own table
//! and Pailune's derived layouts into ordinary verified byte changes.
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

/// Why Pailune cannot carry out an intent of a newer format revision yet.
fn unsupported(i: &Intent) -> Option<String> {
    let field = i.field.as_deref().unwrap_or_default();
    match field.contains(['[', '.']) {
        true => Some(format!(
            "{field}: nested fields (format 3.1) are not supported yet"
        )),
        false => None,
    }
}

fn value_matches(actual: &Value, expected: &Value) -> bool {
    if actual == expected {
        return true;
    }
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => {
            if let (Some(ai), Some(bi)) = (a.as_i64(), b.as_i64()) {
                ai == bi
            } else if let (Some(au), Some(bu)) = (a.as_u64(), b.as_u64()) {
                au == bu
            } else if let (Some(af), Some(bf)) = (a.as_f64(), b.as_f64()) {
                (af - bf).abs() < 1e-6
            } else {
                false
            }
        }
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (scalar, Value::Array(list)) => list.iter().any(|item| value_matches(scalar, item)),
        _ => false,
    }
}

fn filter_by_matches<'a>(
    layout: &std::result::Result<&Table, String>,
    body: &[u8],
    records: &[&'a super::Record],
    pattern: &Value,
) -> std::result::Result<Vec<&'a super::Record>, String> {
    let table = layout.as_ref().map_err(String::clone)?;
    let Value::Object(predicates) = pattern else {
        return Err("match predicate must be a JSON object".into());
    };
    let mut matched = Vec::new();
    'record: for &r in records {
        let payload = &body[r.payload..r.end];
        let spans = super::spans(table, payload).ok_or("record does not match layout")?;
        for (field_name, expected) in predicates {
            if same_name(field_name, "key") {
                if !value_matches(&Value::from(r.key), expected) {
                    continue 'record;
                }
                continue;
            }
            if same_name(field_name, "name") || same_name(field_name, "entry") {
                if !value_matches(&Value::String(r.name.clone()), expected) {
                    continue 'record;
                }
                continue;
            }
            let index = table
                .fields
                .iter()
                .position(|f| same_name(&f.name, field_name))
                .ok_or_else(|| format!("{field_name}: no such field in {}", table.class))?;
            let field = &table.fields[index];
            let (start, end) = spans[index];
            let actual = super::value::decode(field, &payload[start..end])?;
            if !value_matches(&actual, expected) {
                continue 'record;
            }
        }
        matched.push(r);
    }
    Ok(matched)
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
            let candidates: Vec<&super::Record> = match (i.key, i.entry.as_deref(), &i.matches) {
                (Some(key), _, _) => by_key.get(&key).into_iter().flatten().copied().collect(),
                (None, Some(name), _) => by_name.get(name).into_iter().flatten().copied().collect(),
                (None, None, Some(_)) => all.iter().collect(),
                (None, None, None) => Vec::new(),
            };
            let mut found: Vec<&super::Record> = candidates
                .into_iter()
                .filter(|r| i.entry.as_deref().is_none_or(|e| r.name == e))
                .collect();
            if let Some(pattern) = &i.matches {
                match filter_by_matches(&layout, body, &found, pattern) {
                    Ok(filtered) => found = filtered,
                    Err(why) => {
                        failed.push(why);
                        return (ok, failed);
                    }
                }
            }
            if found.is_empty() {
                if i.matches.is_some() {
                    failed.push(format!("no records matched the predicate {:?}", i.matches.as_ref().unwrap()));
                } else {
                    failed.push(format!("record {:?} / key {:?} not found", i.entry, i.key));
                }
                return (ok, failed);
            }
            if i.matches.is_some() {
                for r in found {
                    match change(&layout, body, r, i) {
                        Ok(c) => ok.push(c),
                        Err(why) => failed.push(why),
                    }
                }
            } else {
                match found.as_slice() {
                    [one] => match change(&layout, body, one, i) {
                        Ok(c) => ok.push(c),
                        Err(why) => failed.push(why),
                    },
                    _ => failed.push(format!(
                        "{:?}: {} records share this name; add its \"key\"",
                        i.entry.as_deref().unwrap_or_default(),
                        found.len()
                    )),
                }
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
