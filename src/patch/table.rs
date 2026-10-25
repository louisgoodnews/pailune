//! Game data tables: a `.pabgh`/`.staticinfoheader` index over a
//! `.pabgb`/`.staticinfobody` body (§10.1).
//!
//! Index: `count` (u16, u32 or u8) then `count × (key[key_size] + u32
//! offset)`. Body record: `key | u32 name_len | name | payload`.

use std::collections::HashMap;

/// A parsed table index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableIndex {
    pub key_size: usize,
    /// Record start offsets in the body.
    pub offsets: Vec<u32>,
}

/// Little-endian u32 at `off`, if in bounds.
fn u32_at(data: &[u8], off: usize) -> Option<u32> {
    data.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Reads an index, deriving the count and key sizes by tiling: the file
/// must be exactly `count_size + count × (key_size + 4)` bytes and the
/// offsets must start at 0 (in sorted order).
pub fn parse_index(header: &[u8]) -> Option<TableIndex> {
    let count = |size: usize| -> Option<usize> {
        let b = header.get(..size)?;
        Some(match size {
            1 => b[0] as usize,
            2 => u16::from_le_bytes([b[0], b[1]]) as usize,
            _ => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize,
        })
    };
    [2usize, 4, 1]
        .iter()
        .flat_map(|cs| [4usize, 2, 8, 12].iter().map(move |ks| (*cs, *ks)))
        .find_map(|(cs, ks)| {
            let n = count(cs)?;
            (n > 0 && header.len() == cs + n * (ks + 4)).then_some(())?;
            let mut offsets: Vec<u32> = (0..n)
                .filter_map(|i| u32_at(header, cs + i * (ks + 4) + ks))
                .collect();
            offsets.sort_unstable();
            (offsets.first() == Some(&0)).then_some(TableIndex {
                key_size: ks,
                offsets,
            })
        })
}

impl TableIndex {
    /// Record name → offset right after the name characters (the v2 anchor).
    pub fn name_anchors(&self, body: &[u8]) -> HashMap<String, usize> {
        self.offsets
            .iter()
            .rev()
            .fold(HashMap::new(), |mut map, start| {
                let start = *start as usize;
                let name_at = start + self.key_size;
                let name = u32_at(body, name_at)
                    .map(|l| l as usize)
                    .filter(|l| *l < 1024)
                    .and_then(|len| body.get(name_at + 4..name_at + 4 + len));
                // Iterating backwards lets the first record win for duplicates.
                if let Some(name) = name {
                    map.insert(
                        String::from_utf8_lossy(name).into_owned(),
                        name_at + 4 + name.len(),
                    );
                }
                map
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_the_index_and_anchors_names() {
        // Two records with 4-byte keys: "Sword" and "Shield".
        let record = |key: u32, name: &str, payload: &[u8]| {
            [
                &key.to_le_bytes()[..],
                &(name.len() as u32).to_le_bytes(),
                name.as_bytes(),
                payload,
            ]
            .concat()
        };
        let a = record(7, "Sword", &[0, 1, 2, 3]);
        let b = record(9, "Shield", &[0, 9, 9, 9]);
        let body = [a.clone(), b].concat();
        let header = [
            &2u16.to_le_bytes()[..],
            &7u32.to_le_bytes(),
            &0u32.to_le_bytes(),
            &9u32.to_le_bytes(),
            &(a.len() as u32).to_le_bytes(),
        ]
        .concat();
        let index = parse_index(&header).unwrap();
        assert_eq!(index.key_size, 4);
        let anchors = index.name_anchors(&body);
        assert_eq!(anchors["Sword"], 4 + 4 + 5);
        assert_eq!(body[anchors["Shield"] + 1], 9);
        assert_eq!(parse_index(&[1, 2, 3]), None);
    }
}
