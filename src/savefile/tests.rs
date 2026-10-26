use super::*;

/// A synthetic save (the real ones are the player's and stay private).
fn sample() -> (Header, Vec<u8>) {
    let mut raw = vec![0u8; HEADER];
    raw[..4].copy_from_slice(b"SAVE");
    raw[4..6].copy_from_slice(&2u16.to_le_bytes());
    raw[6..8].copy_from_slice(&0x80u16.to_le_bytes());
    let template = Header {
        version: 2,
        uncompressed: 0,
        payload: 0,
        nonce: [0; 16],
        hmac: [0; 32],
        raw,
    };
    let body: Vec<u8> = (0..5000u32).flat_map(|i| (i % 97).to_le_bytes()).collect();
    (template, body)
}

#[test]
fn encodes_decodes_and_detects_tampering() {
    let (template, body) = sample();
    let file = encode(&template, &body, [7; 16]).unwrap();
    let save = decode(&file).unwrap();
    assert_eq!(save.body, body);
    assert_eq!(save.header.nonce, [7; 16]);
    // Re-encrypting the unchanged compressed payload gives the same file.
    let (header, data) = compressed(&file).unwrap();
    assert_eq!(reencrypt(&header, &data).unwrap(), file);
    // Any changed byte fails the checksum.
    let mut broken = file.clone();
    let last = broken.len() - 1;
    broken[last] ^= 1;
    assert!(decode(&broken).is_err());
    assert!(decode(b"SAVE").is_err());
}

/// Real saves of this machine (read-only): `PYLUNE_SAVES=<save folder>`.
#[test]
#[ignore = "reads the local save files"]
fn real_saves_verify_and_round_trip() {
    let Ok(dir) = std::env::var("PYLUNE_SAVES") else {
        return;
    };
    let files: Vec<_> = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "save"))
        // Settings files next to the slots are plain data, not SAVE containers.
        .filter(|e| std::fs::read(e.path()).is_ok_and(|b| b.starts_with(b"SAVE")))
        .collect();
    let dump = std::env::var("PYLUNE_DUMP").ok();
    assert!(!files.is_empty());
    files.iter().for_each(|e| {
        let bytes = std::fs::read(e.path()).unwrap();
        let (header, data) =
            compressed(&bytes).unwrap_or_else(|err| panic!("{}: {err}", e.path().display()));
        assert_eq!(reencrypt(&header, &data).unwrap(), bytes, "byte-identical");
        let save = decode(&bytes).unwrap();
        let again = decode(&encode(&header, &save.body, header.nonce).unwrap()).unwrap();
        assert_eq!(again.body, save.body);
        println!(
            "{}: v{} {} -> {} bytes",
            e.path().display(),
            header.version,
            data.len(),
            save.body.len()
        );
        if let Some(out) = &dump {
            let name = e
                .path()
                .components()
                .rev()
                .take(2)
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>();
            std::fs::write(
                std::path::Path::new(out).join(format!("{}_{}.body", name[1], name[0])),
                &save.body,
            )
            .unwrap();
        }
    });
}

/// Editing real saves in memory (nothing is written): one number in each
/// save changes and the rewritten file reads back with exactly that change.
#[test]
#[ignore = "reads the local save files"]
fn real_saves_edit_in_memory() {
    let Ok(dir) = std::env::var("PYLUNE_SAVES") else {
        return;
    };
    let slots = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "save.save");
    slots.for_each(|e| {
        let bytes = std::fs::read(e.path()).unwrap();
        let tree = slots::inspect(e.path().parent().unwrap()).unwrap();
        fn first_int(nodes: &[view::Node]) -> Option<view::Editable> {
            nodes.iter().find_map(|n| {
                n.edit
                    .clone()
                    .filter(|v| {
                        v.type_name.to_lowercase().starts_with("uint") && v.bytes.len() == 4
                    })
                    .or_else(|| first_int(&n.children))
            })
        }
        let coins = view::item_count(&tree, 1).map(|c| view::scalar(&c.type_name, &c.bytes));
        println!("{}: copper coins {coins:?}", e.path().display());
        let value = first_int(&tree).expect("an integer");
        let mut new = value.bytes.clone();
        new[0] ^= 1;
        let change = edit::SaveEdit {
            at: value.at,
            old: value.bytes.clone(),
            new: new.clone(),
            label: value.field.clone(),
        };
        let written = edit::apply(&bytes, &[change]).unwrap();
        let (before, after) = (decode(&bytes).unwrap().body, decode(&written).unwrap().body);
        let diff: Vec<usize> = (0..before.len())
            .filter(|i| before[*i] != after[*i])
            .collect();
        println!(
            "{}: {} changed at {:?} ({} -> {} bytes)",
            e.path().display(),
            value.field,
            diff,
            bytes.len(),
            written.len()
        );
        assert_eq!(diff, [value.at]);
    });
}
