//! Two JSON mods changing the same value: reported as a conflict with the
//! record involved, and on Apply the later mod wins.

use super::json_patches::{BODY, HEADER, item_table, latin1};
use super::overlay::{game_lookup_bytes, write_group};
use super::ui::{app, settle};
use super::{fixture, write};
use crate::paz::papgt::{self, GroupEntry, Papgt};
use crate::ui::{Action, DeployAction, ModAction};
use std::fs;

#[test]
fn json_mods_changing_the_same_value_conflict_and_the_later_wins() {
    let fx = fixture();
    let game = fx.config.game.dir.clone();
    let (body, header) = item_table();
    let crc = write_group(
        &game.join("0008"),
        &[
            (BODY, &latin1(&body), 0x32),
            (HEADER, &latin1(&header), 0x30),
        ],
    );
    let table = Papgt {
        unknown0: 1,
        unknown1: 2,
        groups: vec![GroupEntry {
            is_optional: false,
            lang_mask: 0x7FFF,
            name: "0008".into(),
            pamt_crc: crc,
        }],
    };
    fs::write(game.join("meta/0.papgt"), papgt::serialize(&table).unwrap()).unwrap();
    // Arrow's stack is bytes 33.. (record "Arrow", payload +1); Potion's is 15.
    // Both mods write the whole 4-byte value, as real mods write whole fields.
    write(
        &fx.sources.join("Arrow Test/arrows.json"),
        r#"{"patches": [{"game_file": "iteminfo.pabgb", "changes": [
            {"entry": "Arrow", "rel_offset": 1, "original": "10000000", "patched": "e7030000"}]}]}"#,
    );
    write(
        &fx.sources.join("Stacks/stacks.json"),
        r#"{"patches": [{"game_file": "gamedata/binarystaticinfo__/bin/iteminfo.pabgb", "changes": [
            {"offset": 33, "original": "10000000", "patched": "20000000"},
            {"offset": 15, "original": "05", "patched": "0a"}]}]}"#,
    );
    let mut app = app(&fx);
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Arrow Test"),
    ])));
    settle(&mut app);
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Stacks"),
    ])));
    settle(&mut app);
    let order: Vec<String> = app.registry.mods.iter().map(|m| m.name.clone()).collect();
    assert_eq!(order, ["Arrow Test", "Stacks"]);
    app.dispatch(Action::Mod(ModAction::SetAll(true)));
    settle(&mut app);

    let lost = &app.mods.conflicts["Arrow Test"].loses;
    assert_eq!(lost.len(), 1, "{lost:?}");
    assert_eq!(
        (lost[0].other.as_str(), lost[0].values.as_slice()),
        ("Stacks", &["Arrow".to_string()][..])
    );
    assert!(lost[0].file.ends_with(BODY));
    assert_eq!(app.mods.conflicts["Stacks"].wins[0].other, "Arrow Test");

    // The later mod overrides the earlier one's value instead of failing.
    app.dispatch(Action::Deploy(DeployAction::Apply));
    settle(&mut app);
    let patched = game_lookup_bytes(&game, BODY).expect("patched table");
    assert_eq!(
        (&patched[33..37], patched[15]),
        (&[0x20, 0, 0, 0][..], 0x0a),
        "{:?}",
        app.status
    );
}
