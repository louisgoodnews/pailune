use super::format3::{Intent, resolve};
use super::*;

/// A tiny table in the shape of `wantedinfo` (bool, i64 price, bool).
fn wanted() -> (Vec<u8>, Vec<u8>) {
    let mut body = Vec::new();
    let mut header = vec![2, 0];
    for (key, price) in [(0x0601u16, 1500u64), (0x0602, 2000)] {
        header.extend_from_slice(&key.to_le_bytes());
        header.extend_from_slice(&(body.len() as u32).to_le_bytes());
        body.extend_from_slice(&key.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.push(0);
        body.extend_from_slice(&price.to_le_bytes());
        body.push(1);
    }
    (header, body)
}

fn set(key: u64, field: &str, new: serde_json::Value) -> Intent {
    Intent {
        entry: None,
        key: Some(key),
        field: Some(field.into()),
        op: "set".into(),
        new,
        old: None,
    }
}

#[test]
fn shipped_layouts_load_and_wanted_info_resolves() {
    assert!(layouts().tables.len() >= 20);
    let table = table("gamedata/wantedinfo.pabgb").expect("wantedinfo layout");
    let (header, body) = wanted();
    verify(table, &header, &body).unwrap();
    let (changes, failed) = resolve(
        "wantedinfo.staticinfobody",
        &header,
        &body,
        &[
            set(0x0602, "increase_price", serde_json::json!(9999)),
            set(0x0601, "_isBlocked", serde_json::json!(true)),
            set(0x0601, "_increasePrice", serde_json::json!(-1i64 << 63)),
            set(0x0601, "nope", serde_json::json!(1)),
            set(0x0999, "_isBlocked", serde_json::json!(true)),
        ],
    );
    assert_eq!(changes.len(), 3, "{failed:?}");
    assert_eq!(failed.len(), 2, "{failed:?}");
    let mut patched = body.clone();
    let patch = crate::patch::FilePatch {
        game_file: "wantedinfo.staticinfobody".into(),
        signature: None,
        changes,
        fields: Vec::new(),
    };
    let report = crate::patch::apply::apply(&mut patched, &patch, Some(&header), true, false);
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    let second = records(&header, &patched).unwrap()[1].clone();
    assert_eq!(
        u64::from_le_bytes(
            patched[second.payload + 1..second.payload + 9]
                .try_into()
                .unwrap()
        ),
        9999
    );
}

#[test]
fn unproven_fields_and_unknown_tables_are_refused() {
    let (header, body) = wanted();
    let (_, failed) = resolve(
        "iteminfo.pabgb",
        &header,
        &body,
        &[set(0x0601, "_maxStackCount", serde_json::json!(999))],
    );
    assert!(failed[0].contains("no verified layout"), "{failed:?}");
    // A body that does not match the layout disables the table.
    let (_, failed) = resolve(
        "wantedinfo.pabgb",
        &header,
        &body[..body.len() - 1],
        &[set(0x0601, "_isBlocked", serde_json::json!(1))],
    );
    assert!(failed[0].contains("does not match"), "{failed:?}");
}

#[test]
fn raw_replaces_work_on_tables_without_a_layout() {
    let (header, body) = wanted();
    let raw = Intent {
        entry: None,
        key: Some(0x0602),
        field: None,
        op: "set".into(),
        new: serde_json::json!("d0070000"),
        old: Some("d0070000".into()),
    };
    let swap = Intent {
        new: serde_json::json!("e8030000"),
        ..raw.clone()
    };
    let (changes, failed) = resolve(
        "iteminfo.pabgb",
        &header,
        &body,
        &[swap, set(0x0602, "_maxStackCount", serde_json::json!(1))],
    );
    assert_eq!(changes.len(), 1, "{failed:?}");
    assert!(failed[0].contains("raw old/new changes work"), "{failed:?}");
}
