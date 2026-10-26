//! Editing JSON mod files in place (the JSON editor in the details panel):
//! checking a text the way Apply reads it, changing one value of one change,
//! and showing byte strings as the little-endian numbers they usually are.

use super::JsonMod;
use serde_json::Value;

/// Which value of the document an edit replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// The new bytes of change `change` in byte patch `patch`.
    Patched { patch: usize, change: usize },
    /// The `new` value of intent `intent` in format-3 target `target`.
    Intent { target: usize, intent: usize },
}

/// Keys that may hold a change's new bytes, in the order the parser reads them.
const PATCHED_KEYS: [&str; 3] = ["patched", "new", "bytes"];

/// Parses `text` exactly as Apply does; the error names line and column for
/// JSON syntax errors.
pub fn check(text: &str) -> Result<JsonMod, String> {
    super::parse::parse(text).map_err(|err| err.to_string())
}

/// The document pretty-printed (two spaces, key order kept, final newline).
pub fn pretty(root: &Value) -> String {
    serde_json::to_string_pretty(root).unwrap_or_default() + "\n"
}

/// `text` with the value at `slot` replaced by `value`, pretty-printed.
pub fn set(text: &str, slot: Slot, value: Value) -> Result<String, String> {
    let mut root: Value =
        serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|err| err.to_string())?;
    let missing = || "the change is no longer in the file".to_string();
    match slot {
        Slot::Patched { patch, change } => {
            let change = root
                .get_mut("patches")
                .and_then(|p| p.get_mut(patch))
                .and_then(|p| p.get_mut("changes"))
                .and_then(|c| c.get_mut(change))
                .and_then(Value::as_object_mut)
                .ok_or_else(missing)?;
            let key = PATCHED_KEYS
                .iter()
                .find(|k| change.get(**k).is_some_and(|v| !v.is_null()))
                .unwrap_or(&PATCHED_KEYS[0]);
            change.insert((*key).to_string(), value);
        }
        Slot::Intent { target, intent } => {
            let intents = match root.get("targets").is_some() {
                true => root
                    .get_mut("targets")
                    .and_then(|t| t.get_mut(target))
                    .and_then(|t| t.get_mut("intents")),
                false => root.get_mut("intents"),
            };
            intents
                .and_then(|i| i.get_mut(intent))
                .and_then(Value::as_object_mut)
                .ok_or_else(missing)?
                .insert("new".into(), value);
        }
    }
    Ok(pretty(&root))
}

/// Bytes as lower-case hex without separators.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A 1, 2, 4 or 8-byte string as a little-endian integer: signed when the
/// top bit is set and the value is small enough to be meant as negative.
pub fn number(bytes: &[u8]) -> Option<i128> {
    if !matches!(bytes.len(), 1 | 2 | 4 | 8) {
        return None;
    }
    let mut le = [0u8; 8];
    le[..bytes.len()].copy_from_slice(bytes);
    let raw = u64::from_le_bytes(le);
    let bits = bytes.len() as u32 * 8;
    let negative = bits > 8 && raw >> (bits - 1) == 1;
    Some(match negative {
        true => raw as i128 - (1i128 << bits),
        false => raw as i128,
    })
}

/// `value` as `len` little-endian bytes, if it fits (signed or unsigned).
pub fn number_bytes(value: i128, len: usize) -> Option<Vec<u8>> {
    let bits = u32::try_from(len * 8)
        .ok()
        .filter(|b| matches!(b, 8 | 16 | 32 | 64))?;
    let fits = value >= -(1i128 << (bits - 1)) && value < (1i128 << bits);
    let raw = value.rem_euclid(1i128 << bits) as u64;
    fits.then(|| raw.to_le_bytes()[..len].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOD: &str = r#"{
  "modinfo": { "title": "Stacks" },
  "patches": [ { "game_file": "gamedata/iteminfo.pabgb", "changes": [
    { "entry": "Arrow", "rel_offset": 1, "original": "64000000", "patched": "e7030000" },
    { "offset": 16, "old": "00", "new": "01" } ] } ]
}"#;

    #[test]
    fn edits_keep_the_document_valid_and_its_key_order() {
        let edited = set(
            MOD,
            Slot::Patched {
                patch: 0,
                change: 1,
            },
            Value::String("02".into()),
        )
        .unwrap();
        let parsed = check(&edited).unwrap();
        assert_eq!(
            parsed.patches[0].changes[1].edit,
            super::super::Edit::Replace {
                original: Some(vec![0]),
                patched: vec![2]
            }
        );
        assert!(edited.find("modinfo") < edited.find("patches"), "{edited}");
        let missing = Slot::Patched {
            patch: 0,
            change: 9,
        };
        assert!(set(MOD, missing, Value::Null).is_err());
        assert!(check("{ \"patches\": [ }").unwrap_err().contains("line 1"));

        let format3 = r#"{"format": 3, "targets": [{"file": "wantedinfo.pabgb",
            "intents": [{"key": 1, "field": "_isBlocked", "new": false}]}]}"#;
        let slot = Slot::Intent {
            target: 0,
            intent: 0,
        };
        let edited = set(format3, slot, Value::Bool(true)).unwrap();
        assert_eq!(
            check(&edited).unwrap().patches[0].fields[0].new,
            Value::Bool(true)
        );
    }

    #[test]
    fn byte_strings_read_and_write_as_numbers() {
        assert_eq!(number(&[0xe7, 0x03, 0, 0]), Some(999));
        assert_eq!(number(&[0xff, 0xff, 0xff, 0xff]), Some(-1));
        assert_eq!(number(&[0xff]), Some(255));
        assert_eq!(number(&[1, 2, 3]), None);
        assert_eq!(number_bytes(999, 4), Some(vec![0xe7, 0x03, 0, 0]));
        assert_eq!(number_bytes(-1, 2), Some(vec![0xff, 0xff]));
        assert_eq!(number_bytes(256, 1), None);
        assert_eq!(number_bytes(1, 3), None);
        assert_eq!(hex(&[0xe7, 0x03]), "e703");
    }
}
