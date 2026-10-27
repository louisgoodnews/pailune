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
