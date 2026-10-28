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

    // 4. Nested field intent reports error if root field doesn't exist
    let nested = Intent {
        entry: None,
        key: None,
        field: Some("data[*].nested_val".into()),
        op: "set".into(),
        new: serde_json::json!(1),
        old: None,
        matches: Some(serde_json::json!({"increase_price": 2000})),
    };
    let (_, failed) = resolve("wantedinfo.pabgb", &header, &body, &[nested]);
    assert_eq!(failed.len(), 1);
    assert!(failed[0].contains("no such field in WantedInfo"), "{failed:?}");
}

#[test]
fn nested_fields_and_table_index_rewriting() {
    // Construct a table with a list of structs:
    // Record layout:
    // _isBlocked: bool (fixed:1)
    // prefabs: list:struct
    //   subfield 0: name (fixed:4, e.g. id hash)
    //   subfield 1: tags (list:4)
    let tag_field = Field {
        name: "tags".into(),
        shape: "list:4".into(),
        meaning: "int".into(),
        offset: "proven".into(),
        name_certain: true,
        subfields: Vec::new(),
    };
    let id_field = Field {
        name: "id".into(),
        shape: "fixed:4".into(),
        meaning: "int".into(),
        offset: "proven".into(),
        name_certain: true,
        subfields: Vec::new(),
    };
    let prefabs_field = Field {
        name: "prefabs".into(),
        shape: "list:struct".into(),
        meaning: "list".into(),
        offset: "proven".into(),
        name_certain: true,
        subfields: vec![id_field, tag_field],
    };
    let blocked_field = Field {
        name: "_isBlocked".into(),
        shape: "fixed:1".into(),
        meaning: "bool".into(),
        offset: "proven".into(),
        name_certain: true,
        subfields: Vec::new(),
    };

    let table = Table {
        class: "CustomGearInfo".into(),
        key_size: 4,
        records: 2,
        partial: false,
        fields: vec![blocked_field, prefabs_field],
    };

    // Build Record 1 (key 10, name "Helm"):
    // _isBlocked: 0
    // prefabs count: 2
    //   elem 0: id=101, tags count=2 [1, 2]
    //   elem 1: id=102, tags count=1 [9]
    let mut r1_payload = Vec::new();
    r1_payload.push(0u8); // _isBlocked
    r1_payload.extend_from_slice(&2u32.to_le_bytes()); // prefabs count = 2
    // elem 0:
    r1_payload.extend_from_slice(&101u32.to_le_bytes()); // id
    r1_payload.extend_from_slice(&2u32.to_le_bytes()); // tags count = 2
    r1_payload.extend_from_slice(&1u32.to_le_bytes()); // tag 1
    r1_payload.extend_from_slice(&2u32.to_le_bytes()); // tag 2
    // elem 1:
    r1_payload.extend_from_slice(&102u32.to_le_bytes()); // id
    r1_payload.extend_from_slice(&1u32.to_le_bytes()); // tags count = 1
    r1_payload.extend_from_slice(&9u32.to_le_bytes()); // tag 9

    // Build Record 2 (key 20, name "Boots"):
    let mut r2_payload = Vec::new();
    r2_payload.push(1u8);
    r2_payload.extend_from_slice(&0u32.to_le_bytes()); // prefabs count = 0

    let mut body = Vec::new();
    let mut header = vec![2u16.to_le_bytes()[0], 2u16.to_le_bytes()[1]]; // count = 2

    // Record 1 in body
    header.extend_from_slice(&10u32.to_le_bytes());
    header.extend_from_slice(&(body.len() as u32).to_le_bytes());
    body.extend_from_slice(&10u32.to_le_bytes());
    body.extend_from_slice(&(4u32).to_le_bytes()); // name_len = 4
    body.extend_from_slice(b"Helm");
    body.extend_from_slice(&r1_payload);

    // Record 2 in body
    header.extend_from_slice(&20u32.to_le_bytes());
    let r2_start_in_body = body.len() as u32;
    header.extend_from_slice(&r2_start_in_body.to_le_bytes());
    body.extend_from_slice(&20u32.to_le_bytes());
    body.extend_from_slice(&(5u32).to_le_bytes()); // name_len = 5
    body.extend_from_slice(b"Boots");
    body.extend_from_slice(&r2_payload);

    // Verify spans
    let spans1 = spans(&table, &r1_payload).expect("r1 spans match");
    assert_eq!(spans1.len(), 2);

    // Test resolving nested intent: "prefabs[*].tags" -> new: []
    let intent = Intent {
        entry: Some("Helm".into()),
        key: None,
        field: Some("prefabs[*].tags".into()),
        op: "set".into(),
        new: serde_json::json!([]),
        old: None,
        matches: None,
    };

    let all_records = records(&header, &body).unwrap();
    let helm_record = &all_records[0];

    let layout_res: std::result::Result<&Table, String> = Ok(&table);
    let changes = format3::change(&layout_res, &body, helm_record, &intent).expect("changes produced");
    // Should produce 2 changes (one for elem 0, one for elem 1)
    assert_eq!(changes.len(), 2);
    // Elem 0 had tags len = 4 + 2*4 = 12 bytes; new is 4 bytes [0, 0, 0, 0] (delta = -8)
    // Elem 1 had tags len = 4 + 1*4 = 8 bytes; new is 4 bytes [0, 0, 0, 0] (delta = -4)

    // Now apply changes to body using apply_over
    let patch = crate::patch::FilePatch {
        game_file: "customgearinfo.staticinfobody".into(),
        signature: None,
        changes,
        fields: Vec::new(),
    };

    let mut patched_body = body.clone();
    let report = crate::patch::apply::apply_over(
        &mut patched_body,
        &patch,
        Some(&header),
        true,
        false,
        None,
    );

    assert_eq!(report.applied, 2);
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert!(report.rewritten_header.is_some());

    let new_header = report.rewritten_header.unwrap();
    let new_idx = crate::patch::table::parse_index(&new_header).unwrap();
    assert_eq!(new_idx.offsets[0], 0);
    // Total reduction = 8 + 4 = 12 bytes
    assert_eq!(new_idx.offsets[1], r2_start_in_body - 12);
    assert_eq!(patched_body.len(), body.len() - 12);
}
