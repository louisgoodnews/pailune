use super::store::*;

fn game() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let write = |rel: &str, text: &str| {
        let path = dir.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write("meta/0.papgt", "papgt vanilla");
    write("0008/0.pamt", "index vanilla");
    dir
}

#[test]
fn base_and_versions_restore_exactly_and_the_live_state_is_known() {
    let game = game();
    let data = tempfile::tempdir().unwrap();
    let store = Store {
        root: data.path().join("game-backup"),
    };
    let read = |rel: &str| std::fs::read_to_string(game.path().join(rel)).unwrap();
    let files = |papgt: &str, index: &str| {
        vec![
            ("meta/0.papgt".to_string(), papgt.as_bytes().to_vec()),
            ("0008/0.pamt".to_string(), index.as_bytes().to_vec()),
        ]
    };
    let vanilla = files("papgt vanilla", "index vanilla");
    let snap = |label: &str, files: Vec<(String, Vec<u8>)>, links: Vec<Stored>| Snapshot {
        label: label.into(),
        game_build: "0203".into(),
        files,
        archives: Vec::new(),
        links,
    };
    store
        .save(true, snap("base", vanilla.clone(), Vec::new()), 1)
        .unwrap();
    let again = store.save(true, snap("again", vanilla, Vec::new()), 2);
    assert!(again.is_err(), "one base per build");
    assert_eq!(store.live_state(game.path()).unwrap(), Some(0));

    let hash = store.put_archive(b"pailune entries").unwrap();
    let link = Stored {
        rel: "0008/1.paz".into(),
        sha256: hash.clone(),
    };
    for (n, text) in [(1, "v1"), (2, "v2")] {
        let written = files(&format!("papgt {text}"), &format!("index {text}"));
        let v = store
            .save(false, snap(text, written.clone(), vec![link.clone()]), n)
            .unwrap();
        assert_eq!(v.number, n as u32);
        written
            .iter()
            .for_each(|(rel, b)| write_atomic(&game.path().join(rel), b).unwrap());
    }
    assert_eq!(store.live_state(game.path()).unwrap(), Some(2));
    assert_eq!(
        std::fs::read(store.archive_path(&hash)).unwrap(),
        b"pailune entries"
    );

    store.restore(1, game.path()).unwrap();
    assert_eq!(
        (read("meta/0.papgt"), read("0008/0.pamt")),
        ("papgt v1".into(), "index v1".into())
    );
    assert_eq!(store.live_state(game.path()).unwrap(), Some(1));
    store.restore(0, game.path()).unwrap();
    assert_eq!(read("0008/0.pamt"), "index vanilla");

    // Something else changed the game (an update, another manager).
    std::fs::write(game.path().join("0008/0.pamt"), "foreign").unwrap();
    assert_eq!(store.live_state(game.path()).unwrap(), None);
    // A damaged backup copy is refused before anything is written.
    std::fs::write(store.root.join("versions/2/files/0008/0.pamt"), "rot").unwrap();
    assert!(store.restore(2, game.path()).is_err());
    assert_eq!(read("meta/0.papgt"), "papgt vanilla", "nothing written");
}

#[test]
fn inplace_build_packs_archives_redirects_index_and_reverts_to_vanilla() {
    use crate::paz::papgt::{GroupEntry, Papgt};
    use crate::paz::vanilla::{Wanted, load};
    use crate::paz::write::{Packed, build as build_paz};
    use std::collections::{BTreeMap, BTreeSet};

    let dir = tempfile::tempdir().unwrap();
    let game = dir.path();
    let data_dir = tempfile::tempdir().unwrap();
    let store = Store {
        root: data_dir.path().join("game-backup"),
    };

    let vanilla_group = build_paz(vec![
        Packed {
            vpath: "sound/dialogue/en/voice.wem".into(),
            stored: b"original voice 12345".to_vec(),
            orig_size: 20,
            flags: 0,
        },
        Packed {
            vpath: "gamedata/item/socket/gem.xml".into(),
            stored: b"<vanilla_socket/>".to_vec(),
            orig_size: 17,
            flags: 0,
        },
    ]);

    let g_dir = game.join("0008");
    std::fs::create_dir_all(&g_dir).unwrap();
    std::fs::write(g_dir.join("0.paz"), &vanilla_group.paz).unwrap();
    std::fs::write(g_dir.join("0.pamt"), &vanilla_group.pamt).unwrap();

    let meta = game.join("meta");
    std::fs::create_dir_all(&meta).unwrap();
    std::fs::write(meta.join("0.paver"), b"v2.3").unwrap();

    let papgt = Papgt {
        unknown0: 1,
        unknown1: 0,
        groups: vec![GroupEntry {
            is_optional: false,
            lang_mask: 0x7FFF,
            name: "0008".into(),
            pamt_crc: vanilla_group.pamt_crc,
        }],
    };
    let papgt_bytes = crate::paz::papgt::serialize(&papgt).unwrap();
    std::fs::write(meta.join("0.papgt"), &papgt_bytes).unwrap();

    let wanted = Wanted {
        paths: BTreeSet::from([
            "sound/dialogue/en/voice.wem".into(),
            "gamedata/item/socket/gem.xml".into(),
        ]),
        names: BTreeSet::new(),
    };
    let vanilla = load(game, &papgt_bytes, &wanted).unwrap();

    // Staged mod files with modded audio and modded socket xml
    let mod_dir = tempfile::tempdir().unwrap();
    let mod_audio = mod_dir.path().join("new_voice.wem");
    let mod_xml = mod_dir.path().join("modded_socket.xml");
    std::fs::write(&mod_audio, b"MODIFIED DIALOGUE AUDIO").unwrap();
    std::fs::write(&mod_xml, b"<modded_socket_extended/>").unwrap();

    let files = BTreeMap::from([
        ("sound/dialogue/en/voice.wem".into(), mod_audio),
        ("gamedata/item/socket/gem.xml".into(), mod_xml),
    ]);

    // 1. In-place build
    let plan = super::build(game, &store, &meta, &vanilla, &files, 100).unwrap();
    assert_eq!(plan.file_count, 2);
    assert_eq!(plan.groups, vec!["0008".to_string()]);
    assert_eq!(plan.links.len(), 1);
    assert_eq!(plan.links[0].target, g_dir.join("1.paz"));

    // Base and version backups exist
    assert!(store.has_base());
    let versions = store.versions();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].number, 1);
    assert_eq!(store.live_state(game).unwrap(), Some(1));

    // Vanilla 0.paz must be completely untouched
    assert_eq!(
        std::fs::read(g_dir.join("0.paz")).unwrap(),
        vanilla_group.paz
    );

    // Group index 0.pamt was updated
    let new_pamt_bytes = std::fs::read(g_dir.join("0.pamt")).unwrap();
    assert_ne!(new_pamt_bytes, vanilla_group.pamt);
    let parsed_pamt = crate::paz::pamt::parse(&new_pamt_bytes).unwrap();
    let voice_rec = &parsed_pamt.files["sound/dialogue/en/voice.wem"];
    assert_eq!(voice_rec.paz_index, 1);
    let xml_rec = &parsed_pamt.files["gamedata/item/socket/gem.xml"];
    assert_eq!(xml_rec.paz_index, 1);

    // Content in the added archive matches
    let added_paz_bytes = std::fs::read(&plan.links[0].source).unwrap();
    let stored_voice = &added_paz_bytes
        [voice_rec.offset as usize..(voice_rec.offset + voice_rec.stored_size) as usize];
    assert_eq!(stored_voice, b"MODIFIED DIALOGUE AUDIO");

    // 0.papgt has updated pamt_crc
    let new_papgt_bytes = std::fs::read(meta.join("0.papgt")).unwrap();
    let parsed_papgt = crate::paz::papgt::parse(&new_papgt_bytes).unwrap();
    assert_eq!(parsed_papgt.groups[0].pamt_crc, parsed_pamt.crc);

    // 2. In-place revert
    super::revert(game, &store).unwrap();
    assert_eq!(store.live_state(game).unwrap(), Some(0));
    assert_eq!(
        std::fs::read(g_dir.join("0.pamt")).unwrap(),
        vanilla_group.pamt
    );
    assert_eq!(std::fs::read(meta.join("0.papgt")).unwrap(), papgt_bytes);
}
