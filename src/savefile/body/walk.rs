//! Decoding objects with the engine's exact layout. A decode is accepted
//! only when it consumes every byte of the object, so a misunderstanding of
//! the format shows up as a rejected object instead of wrong values.
//!
//! Object: `u16 mask_len, mask[mask_len], u32 reserved`, then the fields in
//! schema order. Absent fields write nothing, except arrays and lists
//! (kinds 3, 6, 7), which write one `0x01` byte. Present fields:
//! * 0, 2 — `size` bytes;
//! * 1 — `u32 count`, `count * size` bytes;
//! * 3 — `0x00`, `u32 count`, `count * size` bytes;
//! * 4, 5 — an embedded object: `u16 mask_len, mask, u16 type, u8, u32,
//!   u32, u32 offset`, then `u32 reserved`, its fields and `u32 size`
//!   (kind 5 may have up to 8 bytes before the header);
//! * 6, 7 — `0x00`, `u32 count`, 13 bytes, then `count` embedded objects.

use super::{Body, Cursor, TypeDef};
use crate::error::{PyluneError, Result};

/// Deepest nesting of embedded objects accepted.
const MAX_DEPTH: usize = 64;

/// A decoded field value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A fixed-size value and its offset in the body (for editing in place).
    Scalar {
        at: usize,
        bytes: Vec<u8>,
    },
    /// Counted items (kinds 1 and 3), raw.
    Items {
        count: u32,
        data: Vec<u8>,
    },
    Object(Box<Object>),
    List(Vec<Object>),
}

/// A decoded object: its type and present fields `(field index, value)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub class: u32,
    pub fields: Vec<(usize, Value)>,
}

fn bad(what: &str) -> PyluneError {
    PyluneError::Invalid(format!("save object: {what}"))
}

/// Whether bit `i` of the presence mask is set.
fn present(mask: &[u8], i: usize) -> bool {
    mask.get(i / 8).is_some_and(|b| b & (1 << (i % 8)) != 0)
}

/// Reads the fields of `class` at the cursor.
fn fields(
    body: &Body,
    c: &mut Cursor,
    class: u32,
    mask: &[u8],
    depth: usize,
) -> Result<Vec<(usize, Value)>> {
    let def: &TypeDef = body
        .types
        .get(class as usize)
        .ok_or_else(|| bad("unknown type"))?;
    let mut out = Vec::new();
    for (i, f) in def.fields.iter().enumerate() {
        if !present(mask, i) {
            if matches!(f.kind, 3 | 6 | 7) && c.take(1)? != [1] {
                return Err(bad("missing absence marker"));
            }
            continue;
        }
        let size = f.size as usize;
        let value = match f.kind {
            0 | 2 if size > 0 => Value::Scalar {
                at: c.pos,
                bytes: c.take(size)?.to_vec(),
            },
            1 if size > 0 => {
                let count = c.u32()?;
                Value::Items {
                    count,
                    data: c.take(count as usize * size)?.to_vec(),
                }
            }
            3 if size > 0 => {
                if c.take(1)? != [0] {
                    return Err(bad("array tag"));
                }
                let count = c.u32()?;
                Value::Items {
                    count,
                    data: c.take(count as usize * size)?.to_vec(),
                }
            }
            4 | 5 => Value::Object(Box::new(embedded(body, c, f.kind == 5, depth + 1)?)),
            6 | 7 => {
                if c.take(1)? != [0] {
                    return Err(bad("list tag"));
                }
                let count = c.u32()?;
                c.take(13)?;
                Value::List(
                    (0..count)
                        .map(|_| embedded(body, c, false, depth + 1))
                        .collect::<Result<_>>()?,
                )
            }
            _ => return Err(bad("unknown field kind")),
        };
        out.push((i, value));
    }
    Ok(out)
}

/// An embedded object (field of kind 4/5, or list element).
fn embedded(body: &Body, c: &mut Cursor, probe: bool, depth: usize) -> Result<Object> {
    // Real saves nest a few levels; a hostile file must not overflow the stack.
    if depth > MAX_DEPTH {
        return Err(bad("objects nested too deeply"));
    }
    if probe {
        // Kind 5: up to 8 bytes may precede the header; take the first
        // position with a plausible mask length.
        let skip = (0..=8)
            .find(|d| {
                let at = c.pos + d;
                c.data
                    .get(at..at + 2)
                    .is_some_and(|b| (1..=16).contains(&u16::from_le_bytes([b[0], b[1]])))
            })
            .ok_or_else(|| bad("no object header"))?;
        c.take(skip)?;
    }
    let mask_len = c.u16()? as usize;
    if !(1..=16).contains(&mask_len) {
        return Err(bad("mask length"));
    }
    let mask = c.take(mask_len)?.to_vec();
    let class = u32::from(c.u16()?);
    c.take(1 + 4 + 4 + 4)?;
    let start = c.pos;
    c.u32()?;
    let fields = fields(body, c, class, &mask, depth)?;
    let size = c.u32()? as usize;
    match size == c.pos - 4 - start {
        true => Ok(Object { class, fields }),
        false => Err(bad("embedded size")),
    }
}

/// Decodes the top-level object `index` of the table of contents.
pub fn object(body: &Body, data: &[u8], index: usize) -> Result<Object> {
    let entry = body.toc.get(index).ok_or_else(|| bad("no such entry"))?;
    let (start, end) = (
        entry.offset as usize,
        entry.offset as usize + entry.size as usize,
    );
    let block = data
        .get(..end)
        .ok_or_else(|| bad("entry outside the body"))?;
    let mut c = Cursor {
        data: block,
        pos: start,
    };
    let mask_len = c.u16()? as usize;
    let mask = c.take(mask_len)?.to_vec();
    c.u32()?;
    let fields = fields(body, &mut c, entry.class, &mask, 0)?;
    match c.pos == end {
        true => Ok(Object {
            class: entry.class,
            fields,
        }),
        false => Err(bad("bytes left over")),
    }
}
