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
        matches: None,
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
        "skill.pabgb",
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
        matches: None,
    };
    let swap = Intent {
        new: serde_json::json!("e8030000"),
        ..raw.clone()
    };
    let (changes, failed) = resolve(
        "skill.pabgb",
        &header,
        &body,
        &[swap, set(0x0602, "_maxStackCount", serde_json::json!(1))],
    );
    assert_eq!(changes.len(), 1, "{failed:?}");
    assert!(failed[0].contains("raw old/new changes work"), "{failed:?}");
}

#[test]
fn ambiguous_record_names_are_refused() {
    let (header, body) = wanted();
    let by_name = Intent {
        entry: Some(String::new()),
        ..set(0, "_isBlocked", serde_json::json!(true))
    };
    let by_name = Intent {
        key: None,
        ..by_name
    };
    let (changes, failed) = resolve("wantedinfo.pabgb", &header, &body, &[by_name]);
    assert!(
        changes.is_empty() && failed[0].contains("share this name"),
        "{failed:?}"
    );
}

/// An item table: u32 keys, `_isBlocked`, `_maxStackCount`, then fields no
/// layout describes yet (different lengths per record).
fn items() -> (Vec<u8>, Vec<u8>) {
    let mut body = Vec::new();
    let mut header = vec![2, 0];
    for (key, name, stack, rest) in [(50001u32, "Arrow", 100u64, 7usize), (2200, "Bow", 1, 30)] {
        header.extend_from_slice(&key.to_le_bytes());
        header.extend_from_slice(&(body.len() as u32).to_le_bytes());
        body.extend_from_slice(&key.to_le_bytes());
        body.extend_from_slice(&(name.len() as u32).to_le_bytes());
        body.extend_from_slice(name.as_bytes());
        body.push(0);
        body.extend_from_slice(&stack.to_le_bytes());
        body.extend(std::iter::repeat_n(0xAB, rest));
    }
    (header, body)
}

#[test]
fn partial_layouts_change_leading_fields_only() {
    let (header, body) = items();
    let table = table("iteminfo.pabgb").expect("partial iteminfo layout");
    assert!(table.partial);
    verify(table, &header, &body).unwrap();
    // As Fat Stacks writes it: entry, key and the snake_case field name.
    let arrow = Intent {
        entry: Some("Arrow".into()),
        ..set(50001, "max_stack_count", serde_json::json!(200))
    };
    let (changes, failed) = resolve(
        "iteminfo.pabgb",
        &header,
        &body,
        &[arrow, set(2200, "_itemName", serde_json::json!(1))],
    );
    assert_eq!(changes.len(), 1, "{failed:?}");
    assert!(failed[0].contains("no such field"), "{failed:?}");
    assert_eq!(
        changes[0].edit,
        crate::patch::Edit::Replace {
            original: Some(100u64.to_le_bytes().to_vec()),
            patched: 200u64.to_le_bytes().to_vec()
        }
    );
    // A record too short for the leading fields disables the table.
    let cut = body.len() - 30 - 8;
    let (_, failed) = resolve(
        "iteminfo.pabgb",
        &header,
        &body[..cut],
        &[set(50001, "_maxStackCount", serde_json::json!(5))],
    );
    assert!(failed[0].contains("does not match"), "{failed:?}");
}

#[test]
fn records_chosen_by_match_predicate() {
    let (header, body) = wanted();
    let table = table("wantedinfo.pabgb").expect("wantedinfo layout");
    verify(table, &header, &body).unwrap();

    // 1. Single record matched by scalar field value
    let match_single = Intent {
        entry: None,
        key: None,
        field: Some("increase_price".into()),
        op: "set".into(),
        new: serde_json::json!(8888),
        old: None,
        matches: Some(serde_json::json!({
            "increase_price": 2000
        })),
    };

    let (changes, failed) = resolve("wantedinfo.pabgb", &header, &body, &[match_single]);
    assert_eq!(changes.len(), 1, "{failed:?}");
    assert!(failed.is_empty(), "{failed:?}");
    assert_eq!(
        changes[0].edit,
        crate::patch::Edit::Replace {
            original: Some(2000u64.to_le_bytes().to_vec()),
            patched: 8888u64.to_le_bytes().to_vec()
        }
    );

    // 2. Multiple records matched by IN list predicate
    let match_multi = Intent {
        entry: None,
        key: None,
        field: Some("_isBlocked".into()),
        op: "set".into(),
        new: serde_json::json!(true),
        old: None,
        matches: Some(serde_json::json!({
            "increase_price": [1500, 2000]
        })),
    };

    let (changes, failed) = resolve("wantedinfo.pabgb", &header, &body, &[match_multi]);
    assert_eq!(changes.len(), 2, "{failed:?}");
    assert!(failed.is_empty(), "{failed:?}");

    // 3. No records matching predicate
    let match_none = Intent {
        entry: None,
        key: None,
        field: Some("increase_price".into()),
        op: "set".into(),
        new: serde_json::json!(5000),
        old: None,
        matches: Some(serde_json::json!({
            "increase_price": 99999
        })),
    };

    let (changes, failed) = resolve("wantedinfo.pabgb", &header, &body, &[match_none]);
    assert!(changes.is_empty());
    assert_eq!(failed.len(), 1);
    assert!(failed[0].contains("no records matched"), "{failed:?}");

    // 4. Nested field intent returns descriptive unsupported message
    let nested = Intent {
        entry: None,
        key: None,
        field: Some("data[*].nested_val".into()),
        op: "set".into(),
        new: serde_json::json!(1),
        old: None,
        matches: Some(serde_json::json!({"x": 1})),
    };
    let (_, failed) = resolve("wantedinfo.pabgb", &header, &body, &[nested]);
    assert_eq!(failed.len(), 1);
    assert!(failed[0].contains("nested fields (format 3.1) are not supported yet"), "{failed:?}");
}
