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

#[derive(Debug, Clone, PartialEq)]
enum Selector {
    None,
    Wildcard,
    Index(usize),
}

#[derive(Debug, Clone, PartialEq)]
struct Segment {
    field_name: String,
    selector: Selector,
}

fn parse_segments(path: &str) -> Vec<Segment> {
    path.split('.')
        .filter(|p| !p.is_empty())
        .map(|part| {
            if let Some(open) = part.find('[')
                && let Some(close) = part.find(']')
                && close > open
            {
                let name = &part[..open];
                let idx_str = &part[open + 1..close];
                let selector = if idx_str == "*" {
                    Selector::Wildcard
                } else if let Ok(idx) = idx_str.parse::<usize>() {
                    Selector::Index(idx)
                } else {
                    Selector::None
                };
                Segment {
                    field_name: name.to_string(),
                    selector,
                }
            } else {
                Segment {
                    field_name: part.to_string(),
                    selector: Selector::None,
                }
            }
        })
        .collect()
}

struct TargetSpan<'a> {
    field: &'a super::Field,
    start: usize,
    end: usize,
    label: String,
}

fn resolve_target_spans<'a>(
    field: &'a super::Field,
    span: (usize, usize),
    payload: &[u8],
    segments: &[Segment],
    label: &str,
) -> std::result::Result<Vec<TargetSpan<'a>>, String> {
    if segments.is_empty() {
        return Ok(vec![TargetSpan {
            field,
            start: span.0,
            end: span.1,
            label: label.to_string(),
        }]);
    }

    let seg = &segments[0];
    match &seg.selector {
        Selector::None => {
            if segments.len() == 1 && same_name(&field.name, &seg.field_name) {
                return Ok(vec![TargetSpan {
                    field,
                    start: span.0,
                    end: span.1,
                    label: format!("{label}.{}", field.name),
                }]);
            }
            // Navigate down into subfields
            let mut pos = span.0;
            for sub in &field.subfields {
                let sub_end = super::skip_field(sub, payload, pos)
                    .ok_or_else(|| format!("{}: failed to skip subfield in struct", sub.name))?;
                if same_name(&sub.name, &seg.field_name) {
                    return resolve_target_spans(
                        sub,
                        (pos, sub_end),
                        payload,
                        &segments[1..],
                        &format!("{label}.{}", sub.name),
                    );
                }
                pos = sub_end;
            }
            Err(format!("{}: subfield not found in {}", seg.field_name, field.name))
        }
        Selector::Wildcard => {
            if !field.shape.starts_with("list") {
                return Err(format!("{}: wildcard [*] only valid on list fields", field.name));
            }
            let count = super::u32_at(payload, span.0).ok_or("truncated list")?;
            let mut pos = span.0 + 4;
            let mut results = Vec::new();

            for idx in 0..count {
                if field.shape == "list:struct" {
                    let elem_start = pos;
                    if segments.len() == 1 {
                        let mut elem_end = elem_start;
                        for sub in &field.subfields {
                            elem_end = super::skip_field(sub, payload, elem_end)
                                .ok_or("failed to skip struct subfield")?;
                        }
                        pos = elem_end;
                        results.push(TargetSpan {
                            field,
                            start: elem_start,
                            end: elem_end,
                            label: format!("{label}[{idx}]"),
                        });
                    } else {
                        let next_seg = &segments[1];
                        let mut sub_pos = elem_start;
                        let mut matched = false;
                        for sub in &field.subfields {
                            let sub_end = super::skip_field(sub, payload, sub_pos)
                                .ok_or("failed to skip struct subfield")?;
                            if same_name(&sub.name, &next_seg.field_name) {
                                let sub_targets = resolve_target_spans(
                                    sub,
                                    (sub_pos, sub_end),
                                    payload,
                                    &segments[1..],
                                    &format!("{label}[{idx}].{}", sub.name),
                                )?;
                                results.extend(sub_targets);
                                matched = true;
                            }
                            sub_pos = sub_end;
                        }
                        if !matched {
                            return Err(format!(
                                "{}: subfield not found in struct elements of {}",
                                next_seg.field_name, field.name
                            ));
                        }
                        pos = sub_pos;
                    }
                } else {
                    let elem_shape = field.shape.strip_prefix("list:").unwrap_or("4");
                    let elem_end = super::skip(elem_shape, payload, pos)
                        .ok_or("failed to skip list element")?;
                    if segments.len() == 1 {
                        results.push(TargetSpan {
                            field,
                            start: pos,
                            end: elem_end,
                            label: format!("{label}[{idx}]"),
                        });
                    }
                    pos = elem_end;
                }
            }
            Ok(results)
        }
        Selector::Index(wanted_idx) => {
            if !field.shape.starts_with("list") {
                return Err(format!("{}: indexing only valid on list fields", field.name));
            }
            let count = super::u32_at(payload, span.0).ok_or("truncated list")?;
            if *wanted_idx >= count {
                return Ok(Vec::new());
            }
            let mut pos = span.0 + 4;
            for idx in 0..count {
                if field.shape == "list:struct" {
                    let elem_start = pos;
                    let mut elem_end = elem_start;
                    for sub in &field.subfields {
                        elem_end = super::skip_field(sub, payload, elem_end)
                            .ok_or("failed to skip struct subfield")?;
                    }
                    pos = elem_end;
                    if idx == *wanted_idx {
                        if segments.len() == 1 {
                            return Ok(vec![TargetSpan {
                                field,
                                start: elem_start,
                                end: elem_end,
                                label: format!("{label}[{idx}]"),
                            }]);
                        } else {
                            let next_seg = &segments[1];
                            let mut sub_pos = elem_start;
                            for sub in &field.subfields {
                                let sub_end = super::skip_field(sub, payload, sub_pos)
                                    .ok_or("failed to skip struct subfield")?;
                                if same_name(&sub.name, &next_seg.field_name) {
                                    return resolve_target_spans(
                                        sub,
                                        (sub_pos, sub_end),
                                        payload,
                                        &segments[1..],
                                        &format!("{label}[{idx}].{}", sub.name),
                                    );
                                }
                                sub_pos = sub_end;
                            }
                            return Err(format!(
                                "{}: subfield not found in struct element of {}",
                                next_seg.field_name, field.name
                            ));
                        }
                    }
                } else {
                    let elem_shape = field.shape.strip_prefix("list:").unwrap_or("4");
                    let elem_end = super::skip(elem_shape, payload, pos)
                        .ok_or("failed to skip list element")?;
                    if idx == *wanted_idx && segments.len() == 1 {
                        return Ok(vec![TargetSpan {
                            field,
                            start: pos,
                            end: elem_end,
                            label: format!("{label}[{idx}]"),
                        }]);
                    }
                    pos = elem_end;
                }
            }
            Ok(Vec::new())
        }
    }
}

/// Turns one intent into changes, or explains why it cannot be applied.
/// Raw replaces need only record boundaries; `set` needs a verified
/// layout (`layout` explains why there is none).
pub(crate) fn change(
    layout: &std::result::Result<&Table, String>,
    body: &[u8],
    record: &super::Record,
    i: &Intent,
) -> std::result::Result<Vec<Change>, String> {
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
            ([one], true) => Ok(vec![at(*one, old, new, format!("{}: raw replace", record.name))]),
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
    let segments = parse_segments(name);
    if segments.is_empty() {
        return Err("an intent without a field".into());
    }
    let top_seg = &segments[0];
    let index = table
        .fields
        .iter()
        .position(|f| same_name(&f.name, &top_seg.field_name))
        .ok_or_else(|| format!("{name}: no such field in {}", table.class))?;
    let top_field = &table.fields[index];
    let spans = spans(table, payload).ok_or("record does not match the layout")?;
    let top_span = spans[index];

    let targets = resolve_target_spans(
        top_field,
        top_span,
        payload,
        &segments,
        &record.name,
    )?;

    if targets.is_empty() {
        return Err(format!("{}: field matched no targets in record {}", name, record.name));
    }

    let mut changes = Vec::with_capacity(targets.len());
    for t in targets {
        if !t.field.editable() {
            return Err(format!(
                "{}: the layout of this field is not proven yet, so it is not changed",
                t.field.name
            ));
        }
        let bytes = super::value::encode(t.field, t.end - t.start, &i.new)?;
        changes.push(at(
            t.start,
            payload[t.start..t.end].to_vec(),
            bytes,
            t.label,
        ));
    }
    Ok(changes)
}

/// Why Pailune cannot carry out an intent of a newer format revision yet.
fn unsupported(_i: &Intent) -> Option<String> {
    None
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
            let maybe_idx = table
                .fields
                .iter()
                .position(|f| same_name(&f.name, field_name));
            if let Some(index) = maybe_idx {
                let field = &table.fields[index];
                let (start, end) = spans[index];
                let actual = super::value::decode(field, &payload[start..end])?;
                if !value_matches(&actual, expected) {
                    continue 'record;
                }
            } else if table.partial {
                // Heuristic search for partial tables when filtering by unmapped fields:
                // Check if any expected integer values exist in the payload
                let mut found_value = false;
                match expected {
                    Value::Number(n) => {
                        if let Some(val) = n.as_u64() {
                            let b4 = (val as u32).to_le_bytes();
                            let b8 = val.to_le_bytes();
                            if payload.windows(4).any(|w| w == b4) || payload.windows(8).any(|w| w == b8) {
                                found_value = true;
                            }
                        }
                    }
                    Value::Array(items) => {
                        for item in items {
                            if let Value::Number(n) = item
                                && let Some(val) = n.as_u64()
                            {
                                let b4 = (val as u32).to_le_bytes();
                                let b8 = val.to_le_bytes();
                                if payload.windows(4).any(|w| w == b4) || payload.windows(8).any(|w| w == b8) {
                                    found_value = true;
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                }
                if !found_value {
                    continue 'record;
                }
            } else {
                return Err(format!("{field_name}: no such field in {}", table.class));
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
                        Ok(cs) => ok.extend(cs),
                        Err(why) => failed.push(why),
                    }
                }
            } else {
                match found.as_slice() {
                    [one] => match change(&layout, body, one, i) {
                        Ok(cs) => ok.extend(cs),
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
