//! The decompressed save body (B-07): a 14-byte prefix, a self-describing
//! schema (types with named, typed fields), a table of contents of
//! top-level objects, and the objects themselves.
//!
//! Layout as documented by the community library `crimson-rs` (MIT); this
//! is Pailune's own read-only implementation, accepting an object only when
//! the engine's layout accounts for every byte of it ([`walk`]).

pub mod walk;

use crate::error::{PailuneError, Result};

/// The body starts with these bytes.
pub const MAGIC: [u8; 4] = [0xFF, 0xFF, 0x04, 0x00];

/// One field of a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDef {
    pub name: String,
    pub type_name: String,
    /// Encoding: 0/2 scalar, 1 counted bytes, 3 tagged array, 4/5 object,
    /// 6/7 object list.
    pub kind: u16,
    /// Scalar or element size.
    pub size: u16,
    pub aux: u32,
}

/// One type of the schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

/// A top-level object: type index and its bytes in the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TocEntry {
    pub class: u32,
    pub offset: u32,
    pub size: u32,
}

/// Schema and table of contents of a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    pub types: Vec<TypeDef>,
    pub toc: Vec<TocEntry>,
    pub toc_offset: usize,
    pub stream_size: u32,
}

/// A little-endian reader over the body.
pub(crate) struct Cursor<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl Cursor<'_> {
    pub fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self.pos.checked_add(n).filter(|e| *e <= self.data.len());
        let end = end.ok_or_else(|| PailuneError::Invalid("save body is truncated".into()))?;
        let bytes = &self.data[self.pos..end];
        self.pos = end;
        Ok(bytes)
    }
    pub fn u16(&mut self) -> Result<u16> {
        self.take(2).map(|b| u16::from_le_bytes([b[0], b[1]]))
    }
    pub fn u32(&mut self) -> Result<u32> {
        self.take(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    pub fn text(&mut self) -> Result<String> {
        let len = self.u32()? as usize;
        self.take(len)
            .map(|b| String::from_utf8_lossy(b).into_owned())
    }
}

/// Reads the schema and the table of contents.
pub fn parse(body: &[u8]) -> Result<Body> {
    if !body.starts_with(&MAGIC) {
        return Err(PailuneError::Invalid("not a save body".into()));
    }
    let mut c = Cursor {
        data: body,
        pos: 0x0E,
    };
    let (_tag, _zero, count) = (c.u16()?, c.u16()?, c.u16()?);
    // Each type's name comes before its fields (the first one here).
    let mut name = c.text()?;
    let mut types = Vec::with_capacity(count as usize);
    for index in 0..count {
        let fields = (0..c.u16()?)
            .map(|_| {
                Ok(FieldDef {
                    name: c.text()?,
                    type_name: c.text()?,
                    kind: c.u16()?,
                    size: c.u16()?,
                    aux: c.u32()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        types.push(TypeDef {
            name: std::mem::take(&mut name),
            fields,
        });
        if index + 1 < count {
            name = c.text()?;
        }
    }
    let toc_offset = c.pos;
    let (_zero, entries, stream) = (c.u32()?, c.u32()?, c.u32()?);
    let toc = (0..entries)
        .map(|_| {
            let class = c.u32()?;
            let (_s1, _s2) = (c.u32()?, c.u32()?);
            Ok(TocEntry {
                class,
                offset: c.u32()?,
                size: c.u32()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Body {
        types,
        toc,
        toc_offset,
        stream_size: stream,
    })
}

#[cfg(test)]
mod tests;
