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
