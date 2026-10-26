//! Encoding a format-3 `new` value for a fixed-size field, by its meaning.

use super::Field;
use serde_json::Value;

/// Bytes of `new` for a fixed field of `size` bytes.
pub fn encode(field: &Field, size: usize, new: &Value) -> std::result::Result<Vec<u8>, String> {
    let bad = |what: &str| Err(format!("{}: {what}", field.name));
    let int = |v: i128| -> std::result::Result<Vec<u8>, String> {
        let bits = size as u32 * 8;
        let (min, max) = (-(1i128 << (bits - 1)), (1i128 << bits) - 1);
        match (min..=max).contains(&v) {
            true => Ok(v.to_le_bytes()[..size].to_vec()),
            false => Err(format!(
                "{}: {v} does not fit in {size} byte(s)",
                field.name
            )),
        }
    };
    match (field.meaning.as_str(), new) {
        ("bool", Value::Bool(b)) => Ok(vec![u8::from(*b)]),
        ("bool", Value::Number(n)) if n.as_u64().is_some_and(|v| v <= 1) => {
            Ok(vec![n.as_u64().unwrap_or(0) as u8])
        }
        ("float", Value::Number(n)) if size == 4 => {
            Ok((n.as_f64().unwrap_or(0.0) as f32).to_le_bytes().to_vec())
        }
        ("float", Value::Number(n)) if size == 8 => {
            Ok(n.as_f64().unwrap_or(0.0).to_le_bytes().to_vec())
        }
        ("vec3", Value::Array(xs)) if xs.len() == 3 && size == 12 => Ok(xs
            .iter()
            .flat_map(|x| (x.as_f64().unwrap_or(0.0) as f32).to_le_bytes())
            .collect()),
        ("int" | "ref" | "hash", Value::Number(n)) => match n
            .as_i64()
            .map(i128::from)
            .or_else(|| n.as_u64().map(i128::from))
        {
            Some(v) => int(v),
            None => bad("expects a whole number"),
        },
        (_, Value::String(hex)) => match crate::patch::parse::hex_bytes(hex) {
            Ok(bytes) if bytes.len() == size => Ok(bytes),
            _ => bad(&format!("expects {size} byte(s) as hex")),
        },
        (meaning, _) => bad(&format!("unsupported value for a {meaning} field")),
    }
}
