//! Mods shipping alternative JSON files (like *Fat Stacks*): offered as
//! options in the details panel, and only the chosen file is applied.

use super::fixture;
use super::ui::{app, settle};
use crate::mod_engine::plan::deployable_files;
use crate::ui::{Action, ModAction};
use std::fs;

fn stacks(factor: u64) -> String {
    format!(
        r#"{{"modinfo": {{"title": "Stacks {factor}X"}}, "format": 3, "target": "iteminfo.pabgb",
          "intents": [{{"entry": "Arrow", "key": 50001, "field": "max_stack_count", "new": {}}}]}}"#,
        100 * factor
    )
}

#[test]
fn alternative_json_files_are_options() {
    let fx = fixture();
    let mut app = app(&fx);
    let source = fx.sources.join("Stacks");
    fs::create_dir_all(&source).unwrap();
    [10, 2].iter().for_each(|f| {
        fs::write(source.join(format!("stacks_{f}x.field.json")), stacks(*f)).unwrap();
    });
    fs::write(source.join("readme.txt"), "pick one").unwrap();
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![source])));
    settle(&mut app);
    app.dispatch(Action::Mod(ModAction::Select("Stacks".into())));
    settle(&mut app);
    let details = app.mods.details.as_ref().unwrap().1.clone();
    assert_eq!(
        details.variants,
        ["stacks_2x.field.json", "stacks_10x.field.json"]
    );
    assert_eq!(details.variant_labels, ["Stacks 2X", "Stacks 10X"]);

    let root = app.paths.staging.join("Stacks");
    let deployed = |variant: Option<&str>| -> Vec<String> {
        deployable_files(&root, variant)
            .unwrap()
            .into_iter()
            .map(|f| f.rel.to_string_lossy().into_owned())
            .collect()
    };
    assert_eq!(deployed(None), ["stacks_2x.field.json"], "first by default");
    app.dispatch(Action::Mod(ModAction::SetVariant(
        "Stacks".into(),
        "stacks_10x.field.json".into(),
    )));
    let entry = app
        .registry
        .mods
        .iter()
        .find(|m| m.name == "Stacks")
        .unwrap();
    assert_eq!(
        deployed(entry.variant.as_deref()),
        ["stacks_10x.field.json"]
    );
}
