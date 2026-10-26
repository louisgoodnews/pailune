//! Deployment without symlinks (decision D-11, Windows fallback), forced on
//! Linux: copies and hard links are recognised by their stamp, foreign
//! changes are left alone, and interrupted runs are reverted exactly.

use crate::mod_engine::plan::{LinkOp, LinkPlan};
use crate::platform::LinkKind;
use crate::symlink::journal::NullJournal;
use crate::symlink::{Manifest, deploy, undeploy};
use std::fs;
use std::path::Path;

fn copy(source: &Path, target: &Path) -> std::io::Result<LinkKind> {
    fs::copy(source, target).map(|_| LinkKind::Copy)
}

fn hard(source: &Path, target: &Path) -> std::io::Result<LinkKind> {
    fs::hard_link(source, target).map(|()| LinkKind::HardLink)
}

/// A staged mod file and a game folder with one original it replaces.
fn setup() -> (tempfile::TempDir, LinkPlan) {
    let dir = tempfile::tempdir().unwrap();
    let (staging, game) = (dir.path().join("staging"), dir.path().join("game"));
    fs::create_dir_all(staging.join("bin64")).unwrap();
    fs::create_dir_all(game.join("bin64")).unwrap();
    fs::write(staging.join("bin64/a.dll"), "mod a").unwrap();
    fs::write(staging.join("bin64/new.asi"), "mod new").unwrap();
    fs::write(game.join("bin64/a.dll"), "original").unwrap();
    let op = |rel: &str| LinkOp {
        source: staging.join(rel),
        target: game.join(rel),
    };
    let plan = LinkPlan {
        ops: vec![op("bin64/a.dll"), op("bin64/new.asi")],
        ..LinkPlan::default()
    };
    (dir, plan)
}

#[test]
fn copies_and_hard_links_revert_exactly() {
    for linker in [copy as deploy::Linker, hard] {
        let (dir, plan) = setup();
        let game = dir.path().join("game");
        let mut manifest = Manifest::default();
        deploy::deploy_with(&plan, &[], &mut manifest, &mut NullJournal, linker).unwrap();
        assert_eq!(
            fs::read_to_string(game.join("bin64/a.dll")).unwrap(),
            "mod a"
        );
        assert!(manifest.links.iter().all(|r| r.stamp.is_some()));
        let report = undeploy::undeploy(&mut manifest, &mut NullJournal).unwrap();
        assert_eq!((report.linked, report.warnings.len()), (2, 0));
        assert_eq!(
            fs::read_to_string(game.join("bin64/a.dll")).unwrap(),
            "original"
        );
        assert!(!game.join("bin64/new.asi").exists());
    }
}

#[test]
fn changed_copies_are_left_alone() {
    let (dir, plan) = setup();
    let game = dir.path().join("game");
    let mut manifest = Manifest::default();
    deploy::deploy_with(&plan, &[], &mut manifest, &mut NullJournal, copy).unwrap();
    // The game (or the user) rewrote one placed file.
    fs::write(game.join("bin64/new.asi"), "edited by someone else").unwrap();
    let report = undeploy::undeploy(&mut manifest, &mut NullJournal).unwrap();
    assert_eq!(
        fs::read_to_string(game.join("bin64/new.asi")).unwrap(),
        "edited by someone else",
        "never delete what is not ours"
    );
    assert_eq!(
        fs::read_to_string(game.join("bin64/a.dll")).unwrap(),
        "original"
    );
    assert_eq!(report.linked, 1);
}

#[test]
fn interrupted_copies_are_recognised_by_content() {
    let (dir, plan) = setup();
    let game = dir.path().join("game");
    let mut manifest = Manifest::default();
    deploy::deploy_with(&plan, &[], &mut manifest, &mut NullJournal, copy).unwrap();
    // Crash after placing, before the stamp was journaled.
    manifest.links.iter_mut().for_each(|r| r.stamp = None);
    undeploy::undeploy(&mut manifest, &mut NullJournal).unwrap();
    assert_eq!(
        fs::read_to_string(game.join("bin64/a.dll")).unwrap(),
        "original"
    );
    assert!(!game.join("bin64/new.asi").exists());

    // Crash after journaling, before the original was moved to .bak: the
    // original must survive even if it equals the mod's file.
    let (dir, plan) = setup();
    let game = dir.path().join("game");
    fs::write(game.join("bin64/a.dll"), "mod a").unwrap();
    let mut manifest = Manifest::default();
    deploy::deploy_with(&plan, &[], &mut manifest, &mut NullJournal, copy).unwrap();
    let record = manifest.links[0].clone();
    fs::remove_file(&record.target).unwrap();
    fs::rename(record.backup.as_ref().unwrap(), &record.target).unwrap();
    let mut interrupted = Manifest {
        links: vec![crate::symlink::LinkRecord {
            stamp: None,
            ..record
        }],
        ..Manifest::default()
    };
    undeploy::undeploy(&mut interrupted, &mut NullJournal).unwrap();
    assert_eq!(
        fs::read_to_string(game.join("bin64/a.dll")).unwrap(),
        "mod a"
    );
}
