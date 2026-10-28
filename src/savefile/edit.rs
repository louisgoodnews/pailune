//! Changing values of a save in place (B-07 step 3). Only fixed-size values
//! change, so nothing in the body moves: offsets and the table of contents
//! stay valid. Every change checks the bytes it replaces; the new file is
//! decoded again and compared before anyone writes it.

use super::{SaveFile, body, decode, encode};
use crate::error::{PailuneError, Result};
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

fn invalid(what: String) -> PailuneError {
    PailuneError::Invalid(what)
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

/// The save file `bytes` with `edits` applied, ready to write. Supports both
/// fixed-size and variable-length modifications (B-07 step 4). When values change
/// size or list items are added/removed, all enclosing embedded object size trailers,
/// the table of contents (TOC offsets & sizes), TOC stream size, container unpacked
/// size, and container HMAC-SHA256 are recalculated.
pub fn apply(bytes: &[u8], edits: &[SaveEdit]) -> Result<Vec<u8>> {
    let SaveFile { header, mut body } = decode(bytes)?;
    if edits.is_empty() {
        return Ok(bytes.to_vec());
    }

    // Verify all edits match current content
    for e in edits {
        let end =
            e.at.checked_add(e.old.len())
                .filter(|end| *end <= body.len());
        let current = end.map(|end| &body[e.at..end]);
        if current != Some(e.old.as_slice()) {
            return Err(invalid(format!(
                "{}: the save changed since it was opened — reopen it",
                e.label
            )));
        }
    }

    let all_fixed = edits.iter().all(|e| e.new.len() == e.old.len());
    if all_fixed {
        for e in edits {
            body[e.at..e.at + e.new.len()].copy_from_slice(&e.new);
        }
    } else {
        // Variable-length edits: sort in descending order of `at` so that
        // earlier offsets are preserved as higher offsets shift.
        let mut sorted = edits.to_vec();
        sorted.sort_by(|a, b| b.at.cmp(&a.at));

        for e in sorted {
            let delta = e.new.len() as i64 - e.old.len() as i64;
            if delta == 0 {
                body[e.at..e.at + e.new.len()].copy_from_slice(&e.new);
                continue;
            }

            // Parse current body to locate TOC and enclosing objects
            let parsed = body::parse(&body)?;
            let toc_idx = parsed
                .toc
                .iter()
                .position(|entry| {
                    let start = entry.offset as usize;
                    let end = start + entry.size as usize;
                    start <= e.at && e.at <= end
                })
                .ok_or_else(|| invalid(format!("{}: offset outside all TOC objects", e.label)))?;

            let obj = body::walk::object(&parsed, &body, toc_idx)?;
            let trailers = body::walk::enclosing_size_trailers(&obj, e.at);

            // 1. Splice body bytes
            body.splice(e.at..e.at + e.old.len(), e.new.iter().copied());

            // 2. Update enclosing embedded objects' size trailers
            for size_pos in trailers {
                let shifted_pos = (size_pos as i64 + delta) as usize;
                let old_size = u32::from_le_bytes([
                    body[shifted_pos],
                    body[shifted_pos + 1],
                    body[shifted_pos + 2],
                    body[shifted_pos + 3],
                ]);
                let new_size = (old_size as i64 + delta) as u32;
                body[shifted_pos..shifted_pos + 4].copy_from_slice(&new_size.to_le_bytes());
            }

            // 3. Update top-level TOC entry size in body
            let toc_entry_pos = parsed.toc_offset + 12 + toc_idx * 20;
            let old_size = u32::from_le_bytes([
                body[toc_entry_pos + 16],
                body[toc_entry_pos + 17],
                body[toc_entry_pos + 18],
                body[toc_entry_pos + 19],
            ]);
            let new_size = (old_size as i64 + delta) as u32;
            body[toc_entry_pos + 16..toc_entry_pos + 20].copy_from_slice(&new_size.to_le_bytes());

            // 4. Shift subsequent TOC entries' offsets in body
            for (j, entry) in parsed.toc.iter().enumerate() {
                if entry.offset > e.at as u32 {
                    let j_pos = parsed.toc_offset + 12 + j * 20;
                    let old_offset = u32::from_le_bytes([
                        body[j_pos + 12],
                        body[j_pos + 13],
                        body[j_pos + 14],
                        body[j_pos + 15],
                    ]);
                    let new_offset = (old_offset as i64 + delta) as u32;
                    body[j_pos + 12..j_pos + 16].copy_from_slice(&new_offset.to_le_bytes());
                }
            }

            // 5. Update TOC stream_size if set
            let stream_pos = parsed.toc_offset + 8;
            let old_stream = u32::from_le_bytes([
                body[stream_pos],
                body[stream_pos + 1],
                body[stream_pos + 2],
                body[stream_pos + 3],
            ]);
            if old_stream > 0 {
                let new_stream = (old_stream as i64 + delta) as u32;
                body[stream_pos..stream_pos + 4].copy_from_slice(&new_stream.to_le_bytes());
            }
        }
    }

    // Verify rewritten structure parses completely and every object in TOC walks without error
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

/// Finds a `Value::List` in an `Object` hierarchy by its `list_at` offset.
fn find_list_in_object<'a>(
    obj: &'a body::walk::Object,
    list_at: usize,
) -> Option<&'a body::walk::Value> {
    for (_, val) in &obj.fields {
        match val {
            val @ body::walk::Value::List { at, .. } if *at == list_at => return Some(val),
            body::walk::Value::Object(child) => {
                if let Some(found) = find_list_in_object(child, list_at) {
                    return Some(found);
                }
            }
            body::walk::Value::List { items, .. } => {
                for item in items {
                    if let Some(found) = find_list_in_object(item, list_at) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Generates the edits to delete item `item_index` from the list at `list_at`.
pub fn delete_list_item_edits(
    body: &[u8],
    list_at: usize,
    item_index: usize,
) -> Result<Vec<SaveEdit>> {
    let parsed = body::parse(body)?;
    let toc_idx = parsed
        .toc
        .iter()
        .position(|e| (e.offset as usize) <= list_at && list_at <= (e.offset + e.size) as usize)
        .ok_or_else(|| invalid("list offset outside TOC objects".into()))?;
    let obj = body::walk::object(&parsed, body, toc_idx)?;
    let list_val = find_list_in_object(&obj, list_at)
        .ok_or_else(|| invalid(format!("no list at offset {list_at}")))?;
    let body::walk::Value::List {
        count_at, items, ..
    } = list_val
    else {
        return Err(invalid("expected a list".into()));
    };
    if item_index >= items.len() {
        return Err(invalid(format!(
            "item index {item_index} out of bounds (count {})",
            items.len()
        )));
    }
    let count = items.len() as u32;
    let item = &items[item_index];
    let item_bytes = body[item.span.clone()].to_vec();
    let count_edit = SaveEdit {
        at: *count_at,
        old: count.to_le_bytes().to_vec(),
        new: (count - 1).to_le_bytes().to_vec(),
        label: format!("list count = {}", count - 1),
    };
    let item_edit = SaveEdit {
        at: item.span.start,
        old: item_bytes,
        new: Vec::new(),
        label: format!("delete item #{item_index}"),
    };
    Ok(vec![count_edit, item_edit])
}

/// Generates the edits to duplicate item `item_index` in the list at `list_at`.
pub fn duplicate_list_item_edits(
    body: &[u8],
    list_at: usize,
    item_index: usize,
) -> Result<Vec<SaveEdit>> {
    let parsed = body::parse(body)?;
    let toc_idx = parsed
        .toc
        .iter()
        .position(|e| (e.offset as usize) <= list_at && list_at <= (e.offset + e.size) as usize)
        .ok_or_else(|| invalid("list offset outside TOC objects".into()))?;
    let obj = body::walk::object(&parsed, body, toc_idx)?;
    let list_val = find_list_in_object(&obj, list_at)
        .ok_or_else(|| invalid(format!("no list at offset {list_at}")))?;
    let body::walk::Value::List {
        count_at, items, ..
    } = list_val
    else {
        return Err(invalid("expected a list".into()));
    };
    if item_index >= items.len() {
        return Err(invalid(format!(
            "item index {item_index} out of bounds (count {})",
            items.len()
        )));
    }
    let count = items.len() as u32;
    let item = &items[item_index];
    let item_bytes = body[item.span.clone()].to_vec();
    let count_edit = SaveEdit {
        at: *count_at,
        old: count.to_le_bytes().to_vec(),
        new: (count + 1).to_le_bytes().to_vec(),
        label: format!("list count = {}", count + 1),
    };
    let item_edit = SaveEdit {
        at: item.span.end,
        old: Vec::new(),
        new: item_bytes,
        label: format!("duplicate item #{item_index}"),
    };
    Ok(vec![count_edit, item_edit])
}

/// Generates the edits to append an item with `item_bytes` to the list at `list_at`.
pub fn add_list_item_edits(
    body: &[u8],
    list_at: usize,
    item_bytes: &[u8],
) -> Result<Vec<SaveEdit>> {
    let parsed = body::parse(body)?;
    let toc_idx = parsed
        .toc
        .iter()
        .position(|e| (e.offset as usize) <= list_at && list_at <= (e.offset + e.size) as usize)
        .ok_or_else(|| invalid("list offset outside TOC objects".into()))?;
    let obj = body::walk::object(&parsed, body, toc_idx)?;
    let list_val = find_list_in_object(&obj, list_at)
        .ok_or_else(|| invalid(format!("no list at offset {list_at}")))?;
    let body::walk::Value::List {
        count_at, items, ..
    } = list_val
    else {
        return Err(invalid("expected a list".into()));
    };
    let count = items.len() as u32;
    let insert_pos = match items.last() {
        Some(last) => last.span.end,
        None => count_at + 4 + 13,
    };
    let count_edit = SaveEdit {
        at: *count_at,
        old: count.to_le_bytes().to_vec(),
        new: (count + 1).to_le_bytes().to_vec(),
        label: format!("list count = {}", count + 1),
    };
    let item_edit = SaveEdit {
        at: insert_pos,
        old: Vec::new(),
        new: item_bytes.to_vec(),
        label: format!("add item #{count}"),
    };
    Ok(vec![count_edit, item_edit])
}

/// Deletes item `item_index` from the list at `list_at`, returning the rewritten save file bytes.
pub fn delete_list_item(bytes: &[u8], list_at: usize, item_index: usize) -> Result<Vec<u8>> {
    let save = decode(bytes)?;
    let edits = delete_list_item_edits(&save.body, list_at, item_index)?;
    apply(bytes, &edits)
}

/// Duplicates item `item_index` in the list at `list_at`, returning the rewritten save file bytes.
pub fn duplicate_list_item(bytes: &[u8], list_at: usize, item_index: usize) -> Result<Vec<u8>> {
    let save = decode(bytes)?;
    let edits = duplicate_list_item_edits(&save.body, list_at, item_index)?;
    apply(bytes, &edits)
}

/// Appends `item_bytes` to the list at `list_at`, returning the rewritten save file bytes.
pub fn add_list_item(bytes: &[u8], list_at: usize, item_bytes: &[u8]) -> Result<Vec<u8>> {
    let save = decode(bytes)?;
    let edits = add_list_item_edits(&save.body, list_at, item_bytes)?;
    apply(bytes, &edits)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::savefile::{HEADER, Header, slots, view};

    /// A save with two `Pouch` objects in the TOC to verify multi-object offset shifts.
    pub(crate) fn two_object_save() -> Vec<u8> {
        let text = |s: &str| [&(s.len() as u32).to_le_bytes()[..], s.as_bytes()].concat();
        let mut body = super::super::body::MAGIC.to_vec();
        body.resize(0x0E, 0);
        body.extend([1u16, 0, 1].iter().flat_map(|v| v.to_le_bytes()));
        body.extend(text("Pouch"));
        body.extend(3u16.to_le_bytes());
        // field 0: _money (kind 0, size 8)
        body.extend(text("_money"));
        body.extend(text("uint64"));
        body.extend(0u16.to_le_bytes());
        body.extend(8u16.to_le_bytes());
        body.extend(0u32.to_le_bytes());
        // field 1: _note (kind 1, size 1)
        body.extend(text("_note"));
        body.extend(text("string"));
        body.extend(1u16.to_le_bytes());
        body.extend(1u16.to_le_bytes());
        body.extend(0u32.to_le_bytes());
        // field 2: _isNew (kind 0, size 1)
        body.extend(text("_isNew"));
        body.extend(text("bool"));
        body.extend(0u16.to_le_bytes());
        body.extend(1u16.to_le_bytes());
        body.extend(0u32.to_le_bytes());

        let make_pouch = |money: u64, note: &[u8], is_new: u8| {
            let mut out = Vec::new();
            out.extend(&[1u8, 0, 0b111]); // mask_len = 1, mask = [0b111]
            out.extend(0u32.to_le_bytes()); // reserved
            out.extend(money.to_le_bytes());
            out.extend((note.len() as u32).to_le_bytes());
            out.extend(note);
            out.push(is_new);
            out
        };

        let obj1 = make_pouch(100, b"Gold", 1);
        let obj2 = make_pouch(200, b"Silver", 0);
        let stream_size = (obj1.len() + obj2.len()) as u32;
        body.extend([0u32, 2, stream_size].iter().flat_map(|v| v.to_le_bytes()));
        let offset1 = body.len() as u32 + 40;
        let offset2 = offset1 + obj1.len() as u32;
        body.extend(
            [0u32, 0, 0, offset1, obj1.len() as u32]
                .iter()
                .flat_map(|v| v.to_le_bytes()),
        );
        body.extend(
            [0u32, 0, 0, offset2, obj2.len() as u32]
                .iter()
                .flat_map(|v| v.to_le_bytes()),
        );
        body.extend(obj1);
        body.extend(obj2);
        let mut raw = vec![0u8; HEADER];
        raw[..4].copy_from_slice(b"SAVE");
        raw[4..6].copy_from_slice(&2u16.to_le_bytes());
        let template = Header {
            version: 2,
            uncompressed: 0,
            payload: 0,
            nonce: [5; 16],
            hmac: [0; 32],
            raw,
        };
        encode(&template, &body, [5; 16]).unwrap()
    }

    /// A save with `ItemSaveData` and an `InventorySaveData` holding a dynamic list of items.
    pub(crate) fn inventory_save() -> Vec<u8> {
        let text = |s: &str| [&(s.len() as u32).to_le_bytes()[..], s.as_bytes()].concat();
        let mut body = super::super::body::MAGIC.to_vec();
        body.resize(0x0E, 0);
        // 2 types in schema
        body.extend([1u16, 0, 2].iter().flat_map(|v| v.to_le_bytes()));
        // Type 0: ItemSaveData
        body.extend(text("ItemSaveData"));
        body.extend(2u16.to_le_bytes());
        for (name, ty, size) in [
            ("_itemKey", "uint64", 8u16),
            ("_stackCount", "uint32", 4u16),
        ] {
            body.extend(text(name));
            body.extend(text(ty));
            body.extend(0u16.to_le_bytes());
            body.extend(size.to_le_bytes());
            body.extend(0u32.to_le_bytes());
        }
        // Type 1: InventorySaveData
        body.extend(text("InventorySaveData"));
        body.extend(2u16.to_le_bytes());
        // field 0: _name (kind 1, counted bytes, size 1)
        body.extend(text("_name"));
        body.extend(text("string"));
        body.extend(1u16.to_le_bytes());
        body.extend(1u16.to_le_bytes());
        body.extend(0u32.to_le_bytes());
        // field 1: _items (kind 6, object list)
        body.extend(text("_items"));
        body.extend(text("ItemSaveData"));
        body.extend(6u16.to_le_bytes());
        body.extend(0u16.to_le_bytes());
        body.extend(0u32.to_le_bytes());

        // Construct items (embedded ItemSaveData objects)
        let make_item = |key: u64, stack: u32| {
            let mut item = Vec::new();
            item.extend(&[1u8, 0, 0b11]); // mask_len = 1, mask = [0b11]
            item.extend(0u16.to_le_bytes()); // class 0 = ItemSaveData
            item.extend(&[0u8; 13]); // 13 header bytes
            let start = item.len();
            item.extend(0u32.to_le_bytes()); // reserved
            item.extend(key.to_le_bytes());
            item.extend(stack.to_le_bytes());
            let size = (item.len() - start) as u32;
            item.extend(size.to_le_bytes());
            item
        };

        let item1 = make_item(1, 50); // coins
        let item2 = make_item(50001, 100); // arrows

        // Construct InventorySaveData
        let mut inv = Vec::new();
        inv.extend(&[1u8, 0, 0b11]); // mask_len = 1, mask = [0b11]
        inv.extend(0u32.to_le_bytes()); // reserved
        // field 0: _name (kind 1): count = 4, data = "Main"
        inv.extend(4u32.to_le_bytes());
        inv.extend(b"Main");
        // field 1: _items (kind 6): 0x00, count = 2, 13 bytes, item1, item2
        inv.push(0x00);
        inv.extend(2u32.to_le_bytes());
        inv.extend(&[0u8; 13]);
        inv.extend(item1);
        inv.extend(item2);

        let stream_size = inv.len() as u32;
        body.extend([0u32, 1, stream_size].iter().flat_map(|v| v.to_le_bytes()));
        let offset = body.len() as u32 + 20;
        body.extend(
            [1u32, 0, 0, offset, inv.len() as u32]
                .iter()
                .flat_map(|v| v.to_le_bytes()),
        );
        body.extend(inv);

        let mut raw = vec![0u8; HEADER];
        raw[..4].copy_from_slice(b"SAVE");
        raw[4..6].copy_from_slice(&2u16.to_le_bytes());
        let template = Header {
            version: 2,
            uncompressed: 0,
            payload: 0,
            nonce: [7; 16],
            hmac: [0; 32],
            raw,
        };
        encode(&template, &body, [7; 16]).unwrap()
    }

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

    #[test]
    fn variable_length_edit_shifts_subsequent_objects_and_toc() {
        let file = two_object_save();
        let decoded = decode(&file).unwrap();
        let parsed = body::parse(&decoded.body).unwrap();
        assert_eq!(parsed.toc.len(), 2);
        let orig_offset2 = parsed.toc[1].offset;

        // In obj1: reserved (4 bytes), _money (8 bytes), then _note (4 bytes count + 4 bytes data = 8 bytes)
        // Offset of _note is offset1 + 7 (mask_len=1, mask 1B, reserved 4B) + 8 = offset1 + 15
        let at = (parsed.toc[0].offset + 15) as usize;
        let old = decoded.body[at..at + 8].to_vec();
        // Replace with count = 15, b"PreciousGoldBag" (19 bytes total, +11 bytes)
        let note_bytes = b"PreciousGoldBag";
        let mut new = (note_bytes.len() as u32).to_le_bytes().to_vec();
        new.extend(note_bytes);

        let delta = (new.len() - old.len()) as i64;

        let edit = SaveEdit {
            at,
            old,
            new,
            label: "expanded_note".into(),
        };
        let rewritten = apply(&file, &[edit]).unwrap();
        let after_decoded = decode(&rewritten).unwrap();
        let after_parsed = body::parse(&after_decoded.body).unwrap();

        assert_eq!(
            after_parsed.toc[0].size,
            (parsed.toc[0].size as i64 + delta) as u32
        );
        assert_eq!(
            after_parsed.toc[1].offset,
            (orig_offset2 as i64 + delta) as u32
        );
        assert_eq!(after_parsed.toc[1].size, parsed.toc[1].size);
        assert_eq!(
            after_parsed.stream_size,
            (parsed.stream_size as i64 + delta) as u32
        );

        // Object 1 still decodes cleanly!
        let obj1 = body::walk::object(&after_parsed, &after_decoded.body, 0).unwrap();
        assert_eq!(obj1.fields.len(), 3);
        match &obj1.fields[1].1 {
            body::walk::Value::Items { data, .. } => {
                assert_eq!(data, b"PreciousGoldBag");
            }
            _ => panic!("expected items"),
        }

        // Object 2 still decodes cleanly!
        let obj2 = body::walk::object(&after_parsed, &after_decoded.body, 1).unwrap();
        assert_eq!(obj2.fields.len(), 3);
    }

    #[test]
    fn list_item_operations_delete_duplicate_add() {
        let file = inventory_save();
        let decoded = decode(&file).unwrap();
        let parsed = body::parse(&decoded.body).unwrap();
        let obj = body::walk::object(&parsed, &decoded.body, 0).unwrap();
        let list_at = match &obj.fields[1].1 {
            body::walk::Value::List { at, items, .. } => {
                assert_eq!(items.len(), 2);
                *at
            }
            _ => panic!("expected list"),
        };

        // 1. Duplicate item 0 (coins)
        let dup_file = duplicate_list_item(&file, list_at, 0).unwrap();
        let dup_decoded = decode(&dup_file).unwrap();
        let dup_parsed = body::parse(&dup_decoded.body).unwrap();
        let dup_obj = body::walk::object(&dup_parsed, &dup_decoded.body, 0).unwrap();
        match &dup_obj.fields[1].1 {
            body::walk::Value::List { items, .. } => {
                assert_eq!(items.len(), 3, "item duplicated, count now 3");
            }
            _ => panic!("expected list"),
        }

        // 2. Delete item 1 from the 3-item save
        let del_file = delete_list_item(&dup_file, list_at, 1).unwrap();
        let del_decoded = decode(&del_file).unwrap();
        let del_parsed = body::parse(&del_decoded.body).unwrap();
        let del_obj = body::walk::object(&del_parsed, &del_decoded.body, 0).unwrap();
        match &del_obj.fields[1].1 {
            body::walk::Value::List { items, .. } => {
                assert_eq!(items.len(), 2, "item deleted, count now 2");
            }
            _ => panic!("expected list"),
        }

        // 3. Inspect tree through slots/view to verify Node labels and ListItem/ListInfo
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("save.save"), &dup_file).unwrap();
        let tree = slots::inspect(dir.path()).unwrap();
        assert_eq!(tree[0].label, "InventorySaveData");
        let list_node = &tree[0].children[1];
        assert_eq!(list_node.label, "_items [3]");
        assert!(list_node.list.is_some());
        assert_eq!(list_node.children.len(), 3);
        assert_eq!(list_node.children[0].list_item.as_ref().unwrap().index, 0);
        assert_eq!(list_node.children[1].list_item.as_ref().unwrap().index, 1);
        assert_eq!(list_node.children[2].list_item.as_ref().unwrap().index, 2);

        // 4. Add new item to the list
        let new_item_bytes = {
            let mut item = Vec::new();
            item.extend(&[1u8, 0, 0b11]);
            item.extend(0u16.to_le_bytes());
            item.extend(&[0u8; 13]);
            let start = item.len();
            item.extend(0u32.to_le_bytes());
            item.extend(9999u64.to_le_bytes());
            item.extend(1u32.to_le_bytes());
            let size = (item.len() - start) as u32;
            item.extend(size.to_le_bytes());
            item
        };
        let added_file = add_list_item(&del_file, list_at, &new_item_bytes).unwrap();
        let added_decoded = decode(&added_file).unwrap();
        let added_parsed = body::parse(&added_decoded.body).unwrap();
        let added_obj = body::walk::object(&added_parsed, &added_decoded.body, 0).unwrap();
        match &added_obj.fields[1].1 {
            body::walk::Value::List { items, .. } => {
                assert_eq!(items.len(), 3, "item added, count now 3");
            }
            _ => panic!("expected list"),
        }
    }
}
