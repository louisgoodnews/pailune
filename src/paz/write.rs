//! Writing a pack group: one `0.paz` plus its `0.pamt` (§5, §7.1).

use super::hash::pa_hash;
use std::collections::BTreeMap;

/// Header field copied by every known tool for new groups.
const PAMT_UNKNOWN: u32 = 0x610E_0232;
const NONE: u32 = 0xFFFF_FFFF;

/// One encoded file to place in the group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packed {
    /// Lowercase virtual path, e.g. `ui/xml/gamemain/play/view.html`.
    pub vpath: String,
    pub stored: Vec<u8>,
    pub orig_size: u32,
    pub flags: u16,
}

/// The finished group files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub paz: Vec<u8>,
    pub pamt: Vec<u8>,
    /// `pa_hash(pamt[12..])`, registered in `0.papgt`.
    pub pamt_crc: u32,
}

fn le(out: &mut Vec<u8>, v: u32) {
    out.extend(v.to_le_bytes());
}

fn split(vpath: &str) -> (&str, &str) {
    vpath.rsplit_once('/').unwrap_or(("", vpath))
}

/// Folder-name trie with one node per path segment; returns path → offset.
fn folder_trie(folders: &[&str]) -> (Vec<u8>, BTreeMap<String, u32>) {
    folders.iter().fold(
        (Vec::new(), BTreeMap::new()),
        |(mut block, mut offsets), folder| {
            let segments: Vec<&str> = folder.split('/').filter(|s| !s.is_empty()).collect();
            (0..segments.len()).for_each(|i| {
                let path = segments[..=i].join("/");
                if offsets.contains_key(&path) {
                    return;
                }
                let parent = match i {
                    0 => NONE,
                    _ => offsets[&segments[..i].join("/")],
                };
                let fragment = match i {
                    0 => segments[0].to_string(),
                    _ => format!("/{}", segments[i]),
                };
                offsets.insert(path, block.len() as u32);
                le(&mut block, parent);
                block.push(fragment.len() as u8);
                block.extend(fragment.as_bytes());
            });
            (block, offsets)
        },
    )
}

/// Builds the group from encoded files (sorted like vanilla: folders by
/// name, files by name, contiguous ranges; 16-byte aligned PAZ entries).
pub fn build(mut files: Vec<Packed>) -> Group {
    files.sort_by(|a, b| split(&a.vpath).cmp(&split(&b.vpath)));
    files.dedup_by(|a, b| a.vpath == b.vpath);
    let (paz, offsets) =
        files
            .iter()
            .fold((Vec::new(), Vec::new()), |(mut paz, mut offsets), f| {
                offsets.push(paz.len() as u32);
                paz.extend(&f.stored);
                paz.resize(paz.len().next_multiple_of(16), 0);
                (paz, offsets)
            });
    let mut folders: Vec<&str> = files.iter().map(|f| split(&f.vpath).0).collect();
    folders.dedup();
    let (dir_block, dir_offsets) = folder_trie(&folders);
    let (name_block, name_offsets) =
        files
            .iter()
            .fold((Vec::new(), Vec::new()), |(mut block, mut offs), f| {
                let name = split(&f.vpath).1;
                offs.push(block.len() as u32);
                le(&mut block, NONE);
                block.push(name.len() as u8);
                block.extend(name.as_bytes());
                (block, offs)
            });
    let mut pamt = Vec::new();
    [0, 1, PAMT_UNKNOWN, 0, pa_hash(&paz), paz.len() as u32]
        .into_iter()
        .for_each(|v| le(&mut pamt, v));
    le(&mut pamt, dir_block.len() as u32);
    pamt.extend(&dir_block);
    le(&mut pamt, name_block.len() as u32);
    pamt.extend(&name_block);
    le(&mut pamt, folders.len() as u32);
    folders.iter().for_each(|folder| {
        let first = files
            .iter()
            .position(|f| split(&f.vpath).0 == *folder)
            .unwrap_or(0);
        let count = files
            .iter()
            .filter(|f| split(&f.vpath).0 == *folder)
            .count();
        le(&mut pamt, pa_hash(folder.as_bytes()));
        le(&mut pamt, dir_offsets.get(*folder).copied().unwrap_or(NONE));
        le(&mut pamt, first as u32);
        le(&mut pamt, count as u32);
    });
    le(&mut pamt, files.len() as u32);
    files
        .iter()
        .zip(offsets)
        .zip(name_offsets)
        .for_each(|((f, offset), name)| {
            [name, offset, f.stored.len() as u32, f.orig_size]
                .into_iter()
                .for_each(|v| le(&mut pamt, v));
            pamt.extend(0u16.to_le_bytes());
            pamt.extend(f.flags.to_le_bytes());
        });
    let pamt_crc = pa_hash(&pamt[12..]);
    pamt[0..4].copy_from_slice(&pamt_crc.to_le_bytes());
    Group {
        paz,
        pamt,
        pamt_crc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paz::{encode::encode, extract::decode, pamt};

    #[test]
    fn written_groups_parse_and_extract() {
        let inputs = [
            (
                "ui/xml/gamemain/play/view.html",
                b"<html>radial</html>".repeat(30),
                0x32u16,
            ),
            (
                "ui/xml/gamemain/play/view.css",
                b"body{color:red}".to_vec(),
                0x02,
            ),
            ("ui/inputmap_common.xml", b"<map/>".to_vec(), 0x30),
            ("root.txt", b"top level".to_vec(), 0x00),
        ];
        let files = inputs
            .iter()
            .map(|(vpath, data, flags)| Packed {
                vpath: vpath.to_string(),
                stored: encode(vpath, data, *flags).unwrap(),
                orig_size: data.len() as u32,
                flags: *flags,
            })
            .collect();
        let group = build(files);
        assert_eq!(group.paz.len() % 16, 0);
        let index = pamt::parse(&group.pamt).unwrap();
        assert_eq!(index.crc, group.pamt_crc);
        assert_eq!(index.pazs[0].crc, pa_hash(&group.paz));
        inputs.iter().for_each(|(vpath, data, _)| {
            let rec = index.files[*vpath];
            assert_eq!(rec.offset % 16, 0);
            let stored =
                group.paz[rec.offset as usize..(rec.offset + rec.stored_size) as usize].to_vec();
            assert_eq!(&decode(vpath, &rec, stored).unwrap(), data, "{vpath}");
        });
    }
}
