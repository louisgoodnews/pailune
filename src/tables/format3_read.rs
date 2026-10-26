//! Reading format-3 documents: one target (`target` + `intents`) or
//! several (`targets`), each intent naming a record, field and new value.

use super::format3::{FieldPatch, Intent};
use crate::error::{PyluneError, Result};
use serde_json::Value;

/// Whether a parsed JSON document is a format-3 mod.
pub fn is_format3(root: &Value) -> bool {
    root.get("format").and_then(Value::as_u64) == Some(3)
        || root.get("intents").is_some()
        || root.get("targets").is_some()
}

fn intent(v: &Value) -> Intent {
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    Intent {
        entry: text("entry"),
        key: v.get("key").and_then(|k| {
            k.as_u64()
                .or_else(|| k.as_str().and_then(|s| s.parse().ok()))
        }),
        field: text("field"),
        op: text("op").unwrap_or_else(|| "set".into()),
        new: v.get("new").cloned().unwrap_or(Value::Null),
        old: text("old"),
        matches: v.get("match").cloned(),
    }
}

/// Reads the targets of a format-3 document (singular or plural form).
pub fn parse(root: &Value) -> Result<Vec<FieldPatch>> {
    let one = |file: Option<&Value>, intents: Option<&Value>| -> Result<FieldPatch> {
        Ok(FieldPatch {
            target: file
                .and_then(Value::as_str)
                .ok_or_else(|| PyluneError::Invalid("format-3 target without a file".into()))?
                .to_string(),
            intents: intents
                .and_then(Value::as_array)
                .map(|a| a.iter().map(intent).collect())
                .unwrap_or_default(),
        })
    };
    match root.get("targets").and_then(Value::as_array) {
        Some(list) => list
            .iter()
            .map(|t| one(t.get("file").or(t.get("target")), t.get("intents")))
            .collect(),
        None => Ok(vec![one(root.get("target"), root.get("intents"))?]),
    }
}
