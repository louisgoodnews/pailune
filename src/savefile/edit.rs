//! Changing values of a save in place (B-07 step 3). Only fixed-size values
//! change, so nothing in the body moves: offsets and the table of contents
//! stay valid. Every change checks the bytes it replaces; the new file is
//! decoded again and compared before anyone writes it.

use super::{SaveFile, body, decode, encode};
use crate::error::{PyluneError, Result};
use sha2::{Digest, Sha256};

/// One change: the value at `at` in the body becomes `new`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveEdit {
    pub at: usize,
    /// What must be there now (from the viewer).
    pub old: Vec<u8>,
    pub new: Vec<u8>,
    /// For messages: `field = value`.
    pub label: String,
}

fn invalid(what: String) -> PyluneError {
    PyluneError::Invalid(what)
}

/// The bytes of `text` as a value of `type_name` with `size` bytes — the
/// inverse of [`super::view::scalar`]. Integers are range-checked.
pub fn parse_value(type_name: &str, size: usize, text: &str) -> Result<Vec<u8>> {
    let text = text.trim();
    let lower = type_name.to_lowercase();
    let bad = || invalid(format!("\"{text}\" is not a valid {type_name}"));
    match (lower.as_str(), size) {
        ("bool", 1) => match text.to_lowercase().as_str() {
            "true" | "1" => Ok(vec![1]),
            "false" | "0" => Ok(vec![0]),
            _ => Err(bad()),
        },
        ("float", 4) => text
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite())
            .map(|v| v.to_le_bytes().to_vec())
            .ok_or_else(bad),
        ("double", 8) => text
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(|v| v.to_le_bytes().to_vec())
            .ok_or_else(bad),
        (name, 1 | 2 | 4 | 8) => {
            let value: i128 = text.parse().map_err(|_| bad())?;
            let bits = size as u32 * 8;
            let (min, max) = match name.starts_with("int") {
                true => (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1),
                false => (0, (1i128 << bits) - 1),
            };
            if !(min..=max).contains(&value) {
                return Err(invalid(format!(
                    "{value} does not fit a {type_name} ({min} … {max})"
                )));
            }
            Ok(value.to_le_bytes()[..size].to_vec())
        }
        _ => crate::patch::parse::hex_bytes(text)
            .ok()
            .filter(|b| b.len() == size)
            .ok_or_else(|| invalid(format!("enter {size} bytes as hex"))),
    }
}

/// A fresh nonce for a rewritten save, derived from the old one and the
/// new content (no randomness source needed; never reused for other data).
fn nonce(old: &[u8; 16], body: &[u8]) -> [u8; 16] {
    let digest = Sha256::new()
        .chain_update(old)
        .chain_update(body)
        .finalize();
    digest[..16].try_into().expect("16 bytes")
}

/// The save file `bytes` with `edits` applied, ready to write. Fails, and
/// changes nothing, if a value is not what the viewer showed, a change has
/// another size, or the result does not decode to exactly the edited body.
pub fn apply(bytes: &[u8], edits: &[SaveEdit]) -> Result<Vec<u8>> {
    let SaveFile { header, mut body } = decode(bytes)?;
    for e in edits {
        let end =
            e.at.checked_add(e.old.len())
                .filter(|end| *end <= body.len());
        let current = end.map(|end| &body[e.at..end]);
        if e.new.len() != e.old.len() {
            return Err(invalid(format!("{}: the size must stay the same", e.label)));
        }
        if current != Some(e.old.as_slice()) {
            return Err(invalid(format!(
                "{}: the save changed since it was opened — reopen it",
                e.label
            )));
        }
        body[e.at..e.at + e.new.len()].copy_from_slice(&e.new);
    }
    // The structure must still read completely (only values changed).
    let parsed = body::parse(&body)?;
    (0..parsed.toc.len()).try_for_each(|i| body::walk::object(&parsed, &body, i).map(drop))?;
    let written = encode(&header, &body, nonce(&header.nonce, &body))?;
    match decode(&written)?.body == body {
        true => Ok(written),
        false => Err(invalid(
            "the rewritten save does not read back the same".into(),
        )),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::savefile::{HEADER, Header, slots, view};

    /// A save with one `Pouch { _money: uint64 = 250, _isNew: bool = true }`.
    pub(crate) fn pouch_save() -> Vec<u8> {
        let text = |s: &str| [&(s.len() as u32).to_le_bytes()[..], s.as_bytes()].concat();
        let mut body = super::super::body::MAGIC.to_vec();
        body.resize(0x0E, 0);
        body.extend([1u16, 0, 1].iter().flat_map(|v| v.to_le_bytes()));
        body.extend(text("Pouch"));
        body.extend(2u16.to_le_bytes());
        for (name, ty, size) in [("_money", "uint64", 8u16), ("_isNew", "bool", 1)] {
            body.extend(text(name));
            body.extend(text(ty));
            body.extend(0u16.to_le_bytes());
            body.extend(size.to_le_bytes());
            body.extend(0u32.to_le_bytes());
        }
        body.extend([0u32, 1, 0].iter().flat_map(|v| v.to_le_bytes()));
        let object = [&[1u8, 0, 0b11][..], &[0; 4], &250u64.to_le_bytes(), &[1]].concat();
        let offset = body.len() as u32 + 20;
        let toc = [0u32, 0, 0, offset, object.len() as u32];
        body.extend(toc.iter().flat_map(|v| v.to_le_bytes()));
        body.extend(object);
        let mut raw = vec![0u8; HEADER];
        raw[..4].copy_from_slice(b"SAVE");
        raw[4..6].copy_from_slice(&2u16.to_le_bytes());
        let template = Header {
            version: 2,
            uncompressed: 0,
            payload: 0,
            nonce: [3; 16],
            hmac: [0; 32],
            raw,
        };
        encode(&template, &body, [3; 16]).unwrap()
    }

    /// The editable values of a save, as the viewer lists them.
    fn editables(bytes: &[u8]) -> Vec<view::Editable> {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("save.save"), bytes).unwrap();
        let tree = slots::inspect(dir.path()).unwrap();
        tree[0]
            .children
            .iter()
            .filter_map(|n| n.edit.clone())
            .collect()
    }

    #[test]
    fn values_change_in_place_and_read_back() {
        let file = pouch_save();
        let values = editables(&file);
        let money = values.iter().find(|v| v.field == "_money").unwrap();
        assert_eq!(view::scalar(&money.type_name, &money.bytes), "250");
        let edit = SaveEdit {
            at: money.at,
            old: money.bytes.clone(),
            new: parse_value(&money.type_name, 8, "350").unwrap(),
            label: "_money = 350".into(),
        };
        let written = apply(&file, std::slice::from_ref(&edit)).unwrap();
        let after = editables(&written);
        let money = after.iter().find(|v| v.field == "_money").unwrap();
        assert_eq!(view::scalar(&money.type_name, &money.bytes), "350");
        assert_ne!(
            decode(&written).unwrap().header.nonce,
            [3; 16],
            "fresh nonce"
        );
        // Stale view: the old bytes are no longer there.
        assert!(
            apply(&written, &[edit])
                .unwrap_err()
                .to_string()
                .contains("reopen")
        );
    }

    #[test]
    fn typed_input_is_checked() {
        assert_eq!(parse_value("bool", 1, "false").unwrap(), [0]);
        assert_eq!(parse_value("int16", 2, "-2").unwrap(), [0xfe, 0xff]);
        assert_eq!(parse_value("uint8", 1, "255").unwrap(), [255]);
        assert!(parse_value("uint8", 1, "256").is_err());
        assert!(parse_value("uint32", 4, "-1").is_err());
        assert!(parse_value("float", 4, "NaN").is_err());
        assert_eq!(
            parse_value("float", 4, "1.5").unwrap(),
            1.5f32.to_le_bytes()
        );
        assert_eq!(parse_value("Guid", 3, "0a0b0c").unwrap(), [10, 11, 12]);
        assert!(parse_value("Guid", 3, "0a0b").is_err());
    }
}
