//! Dry run against the installed game: builds the overlay into a scratch
//! folder from the live (vanilla) metadata. Reads the game, writes nothing
//! there. `PYLUNE_TEST_MOD=<raw mod folder> PYLUNE_OUT=<dir> cargo test -- --ignored`.

use super::{OverlayInput, build};
use crate::paz::{extract, pamt, papgt};
use std::fs;
use std::path::PathBuf;

#[test]
#[ignore = "reads the local game installation"]
fn dry_run_against_the_installed_game() {
    let (Ok(module), Ok(out)) = (
        std::env::var("PYLUNE_TEST_MOD"),
        std::env::var("PYLUNE_OUT"),
    ) else {
        return;
    };
    let mut config = crate::config::Config::default();
    config.game.dir = crate::steam::library::find_game(config.game.app_id).expect("game installed");
    let info = crate::steam::locate::resolve(&config);
    let root = PathBuf::from(module);
    let files = crate::mod_engine::plan::deployable_files(&root, None).unwrap();
    let packs = files
        .into_iter()
        .filter_map(|f| crate::mod_engine::plan::virtual_path(&f.rel).map(|v| (v, f.source)))
        .collect();
    let input = OverlayInput {
        packs,
        ..OverlayInput::default()
    };
    let vanilla = fs::read(info.meta_dir.join("0.papgt")).unwrap();
    let out = PathBuf::from(out);
    let pathc = fs::read(info.meta_dir.join("0.pathc")).unwrap();
    let plan = build::build_from(&info, &vanilla, Some(&pathc), &input, &out).unwrap();
    println!("{plan:#?}");
    let table = papgt::parse(&fs::read(out.join("meta/0.papgt")).unwrap()).unwrap();
    assert_eq!(
        table.groups.len(),
        papgt::parse(&vanilla).unwrap().groups.len() + 1
    );
    // Resolve every packed path like the game: our group first, then vanilla.
    input.packs.iter().for_each(|(vpath, source)| {
        let dir = table
            .groups
            .iter()
            .map(|g| match out.join(&g.name).is_dir() {
                true => out.join(&g.name),
                false => info.game_dir.join(&g.name),
            });
        let (group, record) = dir
            .filter_map(|d| {
                let index = pamt::parse(&fs::read(d.join("0.pamt")).ok()?).ok()?;
                index.files.get(vpath).map(|r| (d.clone(), *r))
            })
            .next()
            .expect("resolvable");
        assert!(group.starts_with(&out), "{vpath} resolves to the overlay");
        assert_eq!(
            extract::extract(&group, vpath, &record).unwrap(),
            fs::read(source).unwrap()
        );
        println!(
            "{vpath}: flags {:#06x}, served from {}",
            record.flags,
            group.display()
        );
    });
}

/// JSON dry run: a one-byte patch on a real item record, built into
/// `PYLUNE_OUT`; the packed table must differ from vanilla in that byte only.
#[test]
#[ignore = "reads the local game installation"]
fn json_dry_run_against_the_installed_game() {
    use crate::patch::table;
    let Ok(out) = std::env::var("PYLUNE_OUT") else {
        return;
    };
    let out = PathBuf::from(out);
    let mut config = crate::config::Config::default();
    config.game.dir = crate::steam::library::find_game(config.game.app_id).expect("game installed");
    let info = crate::steam::locate::resolve(&config);
    let body_path = "gamedata/binarystaticinfo__/bin/iteminfo.staticinfobody";
    let group = info.game_dir.join("0008");
    let index = pamt::parse(&fs::read(group.join("0.pamt")).unwrap()).unwrap();
    let read = |vpath: &str| extract::extract(&group, vpath, &index.files[vpath]).unwrap();
    let body = read(body_path);
    let header = read("gamedata/binarystaticinfo__/bin/iteminfo.staticinfoheader");
    let anchors = table::parse_index(&header).unwrap().name_anchors(&body);
    let (entry, at) = anchors
        .iter()
        .min_by_key(|(name, _)| name.as_str())
        .map(|(n, a)| (n.clone(), *a))
        .unwrap();
    let (old, new) = (body[at + 1], body[at + 1] ^ 0x01);
    let json = format!(
        r#"{{"patches":[{{"game_file":"iteminfo.pabgb","changes":[{{"entry":"{entry}","rel_offset":1,"original":"{old:02x}","patched":"{new:02x}"}}]}}]}}"#
    );
    fs::create_dir_all(&out).unwrap();
    let json_file = out.join("dry.json");
    fs::write(&json_file, json).unwrap();
    let input = OverlayInput {
        patches: vec![("dry".into(), json_file)],
        ..OverlayInput::default()
    };
    let vanilla = fs::read(info.meta_dir.join("0.papgt")).unwrap();
    let plan = build::build_from(&info, &vanilla, None, &input, &out.join("overlay")).unwrap();
    println!("{:?}", plan.patch_notes);
    assert!(plan.patch_notes.iter().all(|n| n.ok));
    let ours = out.join("overlay").join(&plan.groups[0]);
    let our_index = pamt::parse(&fs::read(ours.join("0.pamt")).unwrap()).unwrap();
    let patched = extract::extract(&ours, body_path, &our_index.files[body_path]).unwrap();
    let diffs: Vec<usize> = (0..body.len())
        .filter(|i| body[*i] != patched[*i])
        .collect();
    assert_eq!(
        (patched.len(), diffs),
        (body.len(), vec![at + 1]),
        "only {entry}'s byte changed"
    );
    println!(
        "{entry}: byte {:#x} {old:#04x} -> {new:#04x}, flags {:#06x}",
        at + 1,
        our_index.files[body_path].flags
    );
}
