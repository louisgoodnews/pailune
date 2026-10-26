//! Turning decoded save objects into readable values and trees (read-only).

use super::body::Body;
use super::body::walk::{Object, Value};

/// One line of the inspector tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub label: String,
    pub children: Vec<Node>,
    /// Set for values that can be changed in place.
    pub edit: Option<Editable>,
}

/// A fixed-size value of the body that can be changed without moving
/// anything else (`super::edit`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editable {
    /// Offset in the decompressed body.
    pub at: usize,
    pub field: String,
    pub type_name: String,
    /// The current bytes (checked again when saving).
    pub bytes: Vec<u8>,
}

/// A scalar by its declared type name and size.
pub fn scalar(type_name: &str, bytes: &[u8]) -> String {
    let int = |signed: bool| -> Option<String> {
        let mut le = [0u8; 8];
        le.get_mut(..bytes.len())?.copy_from_slice(bytes);
        let raw = u64::from_le_bytes(le);
        let bits = bytes.len() as u32 * 8;
        Some(match signed && bits < 64 && raw >> (bits - 1) & 1 == 1 {
            true => format!("{}", raw as i64 - (1i64 << bits)),
            false if signed => format!("{}", raw as i64),
            false => format!("{raw}"),
        })
    };
    let lower = type_name.to_lowercase();
    match (lower.as_str(), bytes.len()) {
        ("bool", 1) => (bytes[0] != 0).to_string(),
        ("float", 4) => f32::from_le_bytes(bytes.try_into().expect("4")).to_string(),
        ("double", 8) => f64::from_le_bytes(bytes.try_into().expect("8")).to_string(),
        (name, 1 | 2 | 4 | 8) if name.starts_with("int") => int(true).unwrap_or_default(),
        (_, 1 | 2 | 4 | 8) => int(false).unwrap_or_default(),
        _ => bytes.iter().map(|b| format!("{b:02x}")).collect(),
    }
}

/// Counted items: text when they look like a string, else a short summary.
fn items(type_name: &str, count: u32, data: &[u8]) -> String {
    let printable = !data.is_empty() && data.iter().all(|b| (0x20..0x7f).contains(b));
    match (printable, count as usize == data.len()) {
        (true, true) => format!("\"{}\"", String::from_utf8_lossy(data)),
        _ if count == 0 => "(empty)".into(),
        _ => format!("{count} × {type_name}"),
    }
}

/// The tree of an object.
pub fn tree(body: &Body, object: &Object) -> Vec<Node> {
    let def = &body.types[object.class as usize];
    object
        .fields
        .iter()
        .map(|(i, value)| {
            let field = &def.fields[*i];
            let leaf = |text: String| Node {
                label: format!("{} = {text}", field.name),
                children: Vec::new(),
                edit: None,
            };
            match value {
                Value::Scalar { at, bytes } => Node {
                    edit: Some(Editable {
                        at: *at,
                        field: field.name.clone(),
                        type_name: field.type_name.clone(),
                        bytes: bytes.clone(),
                    }),
                    ..leaf(scalar(&field.type_name, bytes))
                },
                Value::Items { count, data } => leaf(items(&field.type_name, *count, data)),
                Value::Object(child) => Node {
                    label: format!("{} ({})", field.name, body.types[child.class as usize].name),
                    children: tree(body, child),
                    edit: None,
                },
                Value::List(list) => Node {
                    label: format!("{} [{}]", field.name, list.len()),
                    children: list
                        .iter()
                        .enumerate()
                        .map(|(n, child)| Node {
                            label: format!("#{n} {}", body.types[child.class as usize].name),
                            children: tree(body, child),
                            edit: None,
                        })
                        .collect(),
                    edit: None,
                },
            }
        })
        .collect()
}

/// The scalar value of a field by name, searching nested objects too.
pub fn find(body: &Body, object: &Object, name: &str) -> Option<String> {
    let def = &body.types[object.class as usize];
    object.fields.iter().find_map(|(i, value)| match value {
        Value::Scalar { bytes, .. } if def.fields[*i].name == name => {
            Some(scalar(&def.fields[*i].type_name, bytes))
        }
        Value::Items { count, data } if def.fields[*i].name == name => {
            Some(items(&def.fields[*i].type_name, *count, data))
        }
        Value::Object(child) => find(body, child, name),
        _ => None,
    })
}

/// The stack count of the inventory item with `item_key` (1 = copper coins,
/// the pouch): the `_stackCount` next to a matching `_itemKey`.
pub fn item_count(nodes: &[Node], item_key: u64) -> Option<&Editable> {
    nodes.iter().find_map(|n| {
        let field = |name: &str| {
            n.children
                .iter()
                .find_map(|c| c.edit.as_ref().filter(|e| e.field == name))
        };
        let key = field("_itemKey").map(|e| scalar(&e.type_name, &e.bytes));
        match key == Some(item_key.to_string()) {
            true => field("_stackCount"),
            false => item_count(&n.children, item_key),
        }
    })
}
