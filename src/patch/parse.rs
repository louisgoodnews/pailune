//! Reading byte-patch JSON, tolerating the dialects of the community tools.

use super::{Anchor, Change, Edit, FilePatch, JsonMod};
use crate::error::{PyluneError, Result};
use serde_json::Value;

/// Shorthand for an invalid-input error.
fn invalid(what: String) -> PyluneError {
    PyluneError::Invalid(what)
}

/// Hex bytes, tolerating spaces and a `0x` prefix.
pub fn hex_bytes(text: &str) -> Result<Vec<u8>> {
    let clean: String = text
        .trim()
        .trim_start_matches("0x")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if !clean.len().is_multiple_of(2) || !clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(invalid(format!("not a hex byte string: {text:?}")));
    }
    Ok((0..clean.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).unwrap_or(0))
        .collect())
}

/// Offsets: integers, `0x…` hex, decimal strings, or bare hex with a–f.
pub fn offset(value: &Value) -> Result<u64> {
    let bad = || invalid(format!("not an offset: {value}"));
    match value {
        Value::Number(n) => n.as_u64().ok_or_else(bad),
        Value::String(s) => {
            let s = s.trim();
            match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                Some(hex) => u64::from_str_radix(hex, 16).map_err(|_| bad()),
                None if s.chars().all(|c| c.is_ascii_digit()) => s.parse().map_err(|_| bad()),
                None => u64::from_str_radix(s, 16).map_err(|_| bad()),
            }
        }
        _ => Err(bad()),
    }
}

/// First present key among `names`.
fn field<'a>(obj: &'a Value, names: &[&str]) -> Option<&'a Value> {
    names
        .iter()
        .find_map(|n| obj.get(n))
        .filter(|v| !v.is_null())
}

/// String value of the first present key among `names`.
fn text(obj: &Value, names: &[&str]) -> Option<String> {
    field(obj, names)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Parses one entry of a patch's `changes` list.
fn change(raw: &Value) -> Result<Change> {
    let label = text(raw, &["label", "name", "description"]).unwrap_or_default();
    let anchor = match (
        text(raw, &["entry"]),
        field(raw, &["rel_offset", "relative_offset"]),
    ) {
        (Some(name), Some(rel)) => Anchor::Entry {
            name,
            rel_offset: offset(rel)?,
        },
        _ => Anchor::Offset(offset(
            field(raw, &["offset"])
                .ok_or_else(|| invalid(format!("change without offset: {label}")))?,
        )?),
    };
    let kind = text(raw, &["type", "op"])
        .unwrap_or_else(|| "replace".into())
        .to_lowercase();
    let bytes = |names: &[&str]| text(raw, names).map(|t| hex_bytes(&t)).transpose();
    let edit = match kind.as_str() {
        "insert" => Edit::Insert {
            bytes: bytes(&["bytes", "patched", "new"])?
                .ok_or_else(|| invalid(format!("insert without bytes: {label}")))?,
        },
        "replace" | "patch" | "write" => Edit::Replace {
            original: bytes(&["original", "old"])?,
            patched: bytes(&["patched", "new", "bytes"])?
                .ok_or_else(|| invalid(format!("change without patched bytes: {label}")))?,
        },
        other => return Err(invalid(format!("unsupported change type {other:?}"))),
    };
    Ok(Change {
        anchor,
        edit,
        label,
    })
}

/// Parses a JSON mod file: byte patches (format 2), or field-name intents
/// (format 3), which are resolved against the table when applied.
pub fn parse(text_in: &str) -> Result<JsonMod> {
    let root: Value = serde_json::from_str(text_in.trim_start_matches('\u{feff}'))?;
    let format = root.get("format").and_then(Value::as_u64);
    let _ = format;
    if crate::tables::format3::is_format3(&root) {
        let info = root.get("modinfo").unwrap_or(&root);
        return Ok(JsonMod {
            title: text(info, &["title", "name"]).unwrap_or_default(),
            allow_partial: true,
            patches: crate::tables::format3::parse(&root)?
                .into_iter()
                .map(|t| FilePatch {
                    game_file: t.target,
                    signature: None,
                    changes: Vec::new(),
                    fields: t.intents,
                })
                .collect(),
        });
    }
    let info = root.get("modinfo").unwrap_or(&root);
    let title = text(info, &["title", "name"])
        .or_else(|| text(&root, &["name", "title"]))
        .unwrap_or_default();
    let allow_partial = info
        .get("allow_partial_apply")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let patches = root
        .get("patches")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("no \"patches\" list — not a byte-patch JSON mod".into()))?
        .iter()
        .map(|p| {
            Ok(FilePatch {
                game_file: text(p, &["game_file", "file"])
                    .ok_or_else(|| invalid("patch without game_file".into()))?,
                signature: text(p, &["signature"]).map(|s| hex_bytes(&s)).transpose()?,
                fields: Vec::new(),
                changes: p
                    .get("changes")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(change)
                    .collect::<Result<_>>()?,
            })
        })
        .collect::<Result<_>>()?;
    Ok(JsonMod {
        title,
        allow_partial,
        patches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_dialects() {
        let json = r#"{
          "format": 2, "name": "Stamina",
          "modinfo": { "title": "Stamina 50%", "allow_partial_apply": true },
          "patches": [ { "game_file": "gamedata/skill.pabgb", "changes": [
            { "entry": "Skill_CrowWing", "rel_offset": 116, "label": "[Flight] hover",
              "original": "f0d8ffff", "patched": "78ec ffff" },
            { "offset": "0x10", "type": "insert", "bytes": "deadbeef" },
            { "offset": "1A", "old": "00", "new": "01" },
            { "offset": "26", "original": "00", "patched": "02" } ] } ] }"#;
        let m = parse(json).unwrap();
        assert_eq!((m.title.as_str(), m.allow_partial), ("Stamina 50%", true));
        let c = &m.patches[0].changes;
        assert_eq!(
            c[0].anchor,
            Anchor::Entry {
                name: "Skill_CrowWing".into(),
                rel_offset: 116
            }
        );
        assert_eq!(
            c[0].edit,
            Edit::Replace {
                original: Some(vec![0xf0, 0xd8, 0xff, 0xff]),
                patched: vec![0x78, 0xec, 0xff, 0xff]
            }
        );
        assert_eq!(
            c[1].edit,
            Edit::Insert {
                bytes: vec![0xde, 0xad, 0xbe, 0xef]
            }
        );
        assert_eq!(
            (
                c[1].anchor.clone(),
                c[2].anchor.clone(),
                c[3].anchor.clone()
            ),
            (Anchor::Offset(16), Anchor::Offset(26), Anchor::Offset(26))
        );
        let fields = parse(r#"{"format":3,"targets":[{"file":"x.pabgb","intents":[{"entry":"A","field":"_n","new":5}]}]}"#).unwrap();
        assert_eq!(fields.patches[0].fields[0].field.as_deref(), Some("_n"));
        assert!(
            parse(r#"{"format":3,"intents":[]}"#)
                .unwrap_err()
                .to_string()
                .contains("without a file")
        );
    }
}
