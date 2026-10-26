//! Read-only check of the overlay Pylune deployed into the local game
//! (`cargo test deployed_overlay -- --ignored --nocapture`): what it holds,
//! how it differs from the vanilla files, and where else the game keeps them.

use super::{extract, pamt, papgt};
use std::fs;

#[test]
#[ignore = "reads the local game installation"]
fn deployed_overlay_against_vanilla() {
    let game = crate::steam::library::find_game(crate::config::CRIMSON_DESERT_APP_ID).unwrap();
    let table = papgt::parse(&fs::read(game.join("meta/0.papgt")).unwrap()).unwrap();
    let order: Vec<_> = table.groups.iter().map(|g| g.name.clone()).collect();
    println!("papgt order: {order:?}");
    let overlay = game.join("0100");
    let index = pamt::parse(&fs::read(overlay.join("0.pamt")).unwrap()).unwrap();
    for (path, record) in &index.files {
        println!("overlay: {path} {record:?}");
        let ours = extract::extract(&overlay, path, record).unwrap();
        let name = path.rsplit('/').next().unwrap();
        // Every vanilla group that holds a file with this name.
        for group in order.iter().filter(|g| *g != "0100") {
            let dir = game.join(group);
            let Ok(bytes) = fs::read(dir.join("0.pamt")) else {
                continue;
            };
            let Ok(vanilla) = pamt::parse(&bytes) else {
                continue;
            };
            for (vpath, vrec) in vanilla.files.iter().filter(|(p, _)| p.ends_with(name)) {
                let theirs = extract::extract(&dir, vpath, vrec).unwrap();
                let diffs: Vec<usize> = (0..ours.len().min(theirs.len()))
                    .filter(|i| ours[*i] != theirs[*i])
                    .collect();
                println!(
                    "  vanilla {group}/{vpath} {vrec:?}: len {} vs {}, {} differing bytes, first {:?}",
                    theirs.len(),
                    ours.len(),
                    diffs.len(),
                    diffs.iter().take(8).collect::<Vec<_>>()
                );
            }
        }
    }
}

/// The arrow test against Fat Stacks 10x, as the conflict analysis sees
/// them on the installed game (`PYLUNE_TEST_MODS=<dir>`).
#[test]
#[ignore = "reads the local game installation"]
fn json_value_conflicts_on_the_real_tables() {
    let Ok(dir) = std::env::var("PYLUNE_TEST_MODS") else {
        return;
    };
    let dir = std::path::Path::new(&dir);
    let fat = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with("Fat Stacks"))
        .unwrap()
        .path();
    let patches = [
        (
            "Arrow Test".to_string(),
            dir.join("Pylune Test 1 - Arrow Stacks/arrow_stacks.json"),
        ),
        (
            "Fat Stacks".to_string(),
            fat.join("fat_stacks_10x.field.json"),
        ),
        (
            "Drops".to_string(),
            dir.join("Pylune Test 3 - Drop Rates/drop_rates.json"),
        ),
    ];
    let mut config = crate::config::Config::default();
    config.game.dir =
        crate::steam::library::find_game(crate::config::CRIMSON_DESERT_APP_ID).unwrap();
    let info = crate::steam::locate::resolve(&config);
    let started = std::time::Instant::now();
    // The first run scans the game's indexes; later runs reuse the scan.
    crate::overlay::value_conflicts::overlaps(&info, &patches).unwrap();
    println!("first run {:?}", started.elapsed());
    let started = std::time::Instant::now();
    let found = crate::overlay::value_conflicts::overlaps(&info, &patches).unwrap();
    println!("{:?} in {:?}", found, started.elapsed());
    assert_eq!(found.len(), 1);
    assert_eq!(
        (found[0].earlier.as_str(), found[0].later.as_str()),
        ("Arrow Test", "Fat Stacks")
    );
    let values: Vec<&String> = found[0].values.iter().collect();
    assert_eq!(values, ["Arrow", "Pyeonjeon_Arrow"]);
}

/// What Pylune would do with the mod in `PYLUNE_DRY_RUN_MOD=<dir>`: file
/// placements, options, and an overlay built into `PYLUNE_DRY_RUN_OUT`
/// from the installed game (read only; nothing is written to the game).
#[test]
#[ignore = "reads the local game installation"]
fn dry_run_a_mod_against_the_game() {
    let (Ok(dir), Ok(out)) = (
        std::env::var("PYLUNE_DRY_RUN_MOD"),
        std::env::var("PYLUNE_DRY_RUN_OUT"),
    ) else {
        return;
    };
    let root = std::path::PathBuf::from(dir);
    let mut counts = std::collections::BTreeMap::<String, (usize, Vec<String>)>::new();
    walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .for_each(|e| {
            let rel = e.path().strip_prefix(&root).unwrap();
            let entry = counts
                .entry(format!("{:?}", crate::mod_engine::plan::placement(rel)))
                .or_default();
            entry.0 += 1;
            if entry.1.len() < 3 {
                entry.1.push(rel.display().to_string());
            }
        });
    counts
        .iter()
        .for_each(|(p, (n, ex))| println!("{p}: {n} e.g. {ex:?}"));
    println!("options: {:?}", crate::mod_engine::variants::titled(&root));
    let name = root.file_name().unwrap().to_string_lossy().into_owned();
    let registry = crate::mod_engine::model::Registry {
        mods: vec![crate::mod_engine::model::ModEntry {
            enabled: true,
            ..crate::mod_engine::registry::discovered(&name, 0)
        }],
    };
    let input = crate::overlay::collect::collect(root.parent().unwrap(), &registry).unwrap();
    println!(
        "overlay input: {} packed, {} prebuilt, {} JSON, {} text patches",
        input.packs.len(),
        input.prebuilt.len(),
        input.patches.len(),
        input.text_patches.len()
    );
    let mut config = crate::config::Config::default();
    config.game.dir =
        crate::steam::library::find_game(crate::config::CRIMSON_DESERT_APP_ID).unwrap();
    let info = crate::steam::locate::resolve(&config);
    let papgt = crate::overlay::build::vanilla_papgt(&info).unwrap();
    let pathc = fs::read(info.meta_dir.join("0.pathc.bak")).ok();
    // Per extension: files, how many replace a game file, storage flags.
    let wanted = crate::paz::vanilla::Wanted {
        paths: input.packs.keys().cloned().collect(),
        names: Default::default(),
    };
    let base = crate::paz::vanilla::load_cached(&info.game_dir, &papgt, &wanted).unwrap();
    let mut by_ext =
        std::collections::BTreeMap::<String, (usize, usize, std::collections::BTreeSet<u16>)>::new(
        );
    input.packs.keys().for_each(|v| {
        let ext = v.rsplit('.').next().unwrap_or_default().to_string();
        let entry = by_ext.entry(ext).or_default();
        entry.0 += 1;
        entry.1 += usize::from(base.replaces(v));
        entry.2.insert(base.flags_for(v));
    });
    by_ext.iter().for_each(|(e, (n, r, f))| {
        println!("ext {e}: {n} files, {r} replace game files, flags {:x?}", f)
    });
    match crate::overlay::build::build_from(&info, &papgt, pathc.as_deref(), &input, out.as_ref()) {
        Ok(plan) => {
            println!(
                "overlay: {} packed, {} replace game files, groups {:?}",
                plan.packed, plan.replaced, plan.groups
            );
            plan.skipped
                .iter()
                .take(10)
                .for_each(|s| println!("skipped: {s}"));
            plan.patch_notes
                .iter()
                .for_each(|n| println!("patch: {n:?}"));
        }
        Err(err) => println!("overlay build failed: {err}"),
    }
}

/// How vanilla `.pac` meshes are stored: flags, stored vs. original size,
/// and the first bytes of a type-1 entry (research for new meshes).
#[test]
#[ignore = "reads the local game installation"]
fn vanilla_pac_storage() {
    let game = crate::steam::library::find_game(crate::config::CRIMSON_DESERT_APP_ID).unwrap();
    let table = papgt::parse(&fs::read(game.join("meta/0.papgt.bak")).unwrap()).unwrap();
    let mut stats = std::collections::BTreeMap::<u16, (usize, usize)>::new();
    let mut samples = 0;
    for group in table.groups.iter().map(|g| g.name.clone()) {
        let dir = game.join(&group);
        let Ok(index) = fs::read(dir.join("0.pamt"))
            .map_err(|_| ())
            .and_then(|b| pamt::parse(&b).map_err(|_| ()))
        else {
            continue;
        };
        for (vpath, record) in index.files.iter().filter(|(p, _)| p.ends_with(".pac")) {
            if record.flags == 0 {
                println!("plain: {group}/{vpath} {record:?}");
            }
            let entry = stats.entry(record.flags).or_default();
            entry.0 += 1;
            entry.1 += usize::from(record.stored_size == record.orig_size);
            if record.flags & 0x0F == 1 && samples < 4 {
                samples += 1;
                let stored = extract::read_stored(&dir, record).unwrap();
                println!(
                    "{group}/{vpath}: stored {} orig {} head {:02x?}",
                    record.stored_size,
                    record.orig_size,
                    &stored[..48.min(stored.len())]
                );
                match extract::decode(vpath, record, stored.clone()) {
                    Ok(d) => println!(
                        "  decodes to {} bytes, head {:02x?}",
                        d.len(),
                        &d[..32.min(d.len())]
                    ),
                    Err(e) => println!("  decode: {e}"),
                }
            }
        }
    }
    stats
        .iter()
        .for_each(|(f, (n, same))| println!("flags {f:#06x}: {n} .pac, {same} stored == orig"));
}
