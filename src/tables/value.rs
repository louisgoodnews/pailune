//! Encoding a format-3 `new` value for a fixed-size field, by its meaning.

use super::Field;
use serde_json::Value;

/// Bytes of `new` for a fixed or list field of `size` bytes.
pub fn encode(field: &Field, size: usize, new: &Value) -> std::result::Result<Vec<u8>, String> {
    let bad = |what: &str| Err(format!("{}: {what}", field.name));

    if field.shape.starts_with("list") {
        match new {
            Value::Array(items) if items.is_empty() => return Ok(vec![0u8; 4]),
            Value::Null => return Ok(vec![0u8; 4]),
            Value::Array(items) => {
                let elem_shape = field.shape.strip_prefix("list:").unwrap_or("4");
                let elem_size: usize = match elem_shape {
                    "1" => 1,
                    "2" => 2,
                    "4" => 4,
                    "8" => 8,
                    _ => return bad(&format!("unsupported list element shape {elem_shape}")),
                };
                let mut out = Vec::with_capacity(4 + items.len() * elem_size);
                out.extend_from_slice(&(items.len() as u32).to_le_bytes());
                for item in items {
                    let dummy_field = Field {
                        name: field.name.clone(),
                        shape: format!("fixed:{elem_size}"),
                        meaning: if field.meaning == "list" {
                            "int".into()
                        } else {
                            field.meaning.clone()
                        },
                        offset: "proven".into(),
                        name_certain: true,
                        subfields: Vec::new(),
                    };
                    let encoded = encode(&dummy_field, elem_size, item)?;
                    out.extend_from_slice(&encoded);
                }
                return Ok(out);
            }
            _ => return bad("expects a JSON array for list field"),
        }
    }

    // Shipped layouts only have 1-8 byte scalars (and 12-byte vec3); anything else is refused.
    if (!((1..=8).contains(&size) || (field.meaning == "vec3" && size == 12)))
        || (field.meaning == "bool" && size != 1)
    {
        return bad(&format!("unsupported field size {size}"));
    }
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

/// Decodes field bytes into a JSON value based on the field's meaning and shape.
pub fn decode(field: &Field, bytes: &[u8]) -> std::result::Result<Value, String> {
    let size = bytes.len();
    match field.meaning.as_str() {
        "bool" if size == 1 => Ok(Value::Bool(bytes[0] != 0)),
        "float" if size == 4 => {
            let arr: [u8; 4] = bytes
                .try_into()
                .map_err(|_| format!("{}: expects 4 bytes for float", field.name))?;
            Ok(serde_json::json!(f32::from_le_bytes(arr)))
        }
        "float" if size == 8 => {
            let arr: [u8; 8] = bytes
                .try_into()
                .map_err(|_| format!("{}: expects 8 bytes for float", field.name))?;
            Ok(serde_json::json!(f64::from_le_bytes(arr)))
        }
        "vec3" if size == 12 => {
            let x = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
            let y = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
            let z = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
            Ok(serde_json::json!([x, y, z]))
        }
        "int" => match size {
            1 => Ok(serde_json::json!(bytes[0] as i8)),
            2 => {
                let arr: [u8; 2] = bytes.try_into().unwrap();
                Ok(serde_json::json!(i16::from_le_bytes(arr)))
            }
            4 => {
                let arr: [u8; 4] = bytes.try_into().unwrap();
                Ok(serde_json::json!(i32::from_le_bytes(arr)))
            }
            8 => {
                let arr: [u8; 8] = bytes.try_into().unwrap();
                Ok(serde_json::json!(i64::from_le_bytes(arr)))
            }
            _ => Ok(serde_json::json!(crate::patch::document::hex(bytes))),
        },
        "ref" | "hash" => match size {
            1 => Ok(serde_json::json!(bytes[0])),
            2 => {
                let arr: [u8; 2] = bytes.try_into().unwrap();
                Ok(serde_json::json!(u16::from_le_bytes(arr)))
            }
            4 => {
                let arr: [u8; 4] = bytes.try_into().unwrap();
                Ok(serde_json::json!(u32::from_le_bytes(arr)))
            }
            8 => {
                let arr: [u8; 8] = bytes.try_into().unwrap();
                Ok(serde_json::json!(u64::from_le_bytes(arr)))
            }
            _ => Ok(serde_json::json!(crate::patch::document::hex(bytes))),
        },
        _ => {
            if field.shape == "str" && size >= 4 {
                let len = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
                if 4 + len <= size {
                    let s = String::from_utf8_lossy(&bytes[4..4 + len]);
                    return Ok(Value::String(s.to_string()));
                }
            }
            Ok(serde_json::json!(crate::patch::document::hex(bytes)))
        }
    }
}
