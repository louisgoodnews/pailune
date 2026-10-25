//! Crash safety: interrupting a deploy at any step must stay revertible.

use super::{enabled_registry, fixture, write};
use crate::db;
use crate::db::links::DbJournal;
use crate::error::{PyluneError, Result};
use crate::mod_engine::{install, plan};
use crate::symlink::journal::Journal;
use crate::symlink::{LinkRecord, Manifest, deploy, undeploy};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Delegates to the database, then "dies" after `budget` journal writes —
/// either before the next write or right after it (before the filesystem
/// change it announces).
struct Dying<'a> {
    inner: DbJournal<'a>,
    budget: usize,
    after_write: bool,
}

impl Dying<'_> {
    fn spend(&mut self) -> Result<()> {
        match self.budget {
            0 => Err(PyluneError::Invalid("simulated crash".into())),
            _ => {
                self.budget -= 1;
                Ok(())
            }
        }
    }
}

impl Journal for Dying<'_> {
    fn link_planned(&mut self, record: &LinkRecord) -> Result<()> {
        if self.after_write && self.budget == 0 {
            self.inner.link_planned(record)?;
        }
        self.spend()?;
        self.inner.link_planned(record)
    }
    fn link_reverted(&mut self, target: &Path) -> Result<()> {
        self.inner.link_reverted(target)
    }
    fn dir_planned(&mut self, dir: &Path) -> Result<()> {
        if self.after_write && self.budget == 0 {
            self.inner.dir_planned(dir)?;
        }
        self.spend()?;
        self.inner.dir_planned(dir)
    }
    fn dirs_cleared(&mut self) -> Result<()> {
        self.inner.dirs_cleared()
    }
}

/// Every entry below `root` with its content (or link target).
fn snapshot(root: &Path) -> BTreeMap<String, String> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| {
            let rel = e.path().strip_prefix(root).unwrap().display().to_string();
            let what = match (e.path_is_symlink(), e.file_type().is_file()) {
                (true, _) => format!("link:{}", fs::read_link(e.path()).unwrap().display()),
                (false, true) => fs::read_to_string(e.path()).unwrap_or_default(),
                (false, false) => "dir".into(),
            };
            (rel, what)
        })
        .collect()
}

#[test]
fn interrupted_deploys_are_fully_revertible() {
    let cases = (0..8).flat_map(|budget| [(budget, false), (budget, true)]);
    cases.for_each(|(budget, after_write)| {
        let fx = fixture();
        install::import(&fx.sources.join("Cool Mod"), &fx.staging).unwrap();
        install::import(&fx.sources.join("Overlay"), &fx.staging).unwrap();
        let game = fx.config.game.dir.clone();
        let before = snapshot(&game);
        let conn = db::open(&fx.staging.with_file_name("crash.sqlite3")).unwrap();
        let link_plan = plan::build(&fx.config, &fx.staging, &enabled_registry(&fx)).unwrap();
        let mut journal = Dying {
            inner: DbJournal { conn: &conn },
            budget,
            after_write,
        };
        let outcome = deploy::deploy(
            &link_plan,
            &[fx.staging.as_path()],
            &mut Manifest::default(),
            &mut journal,
        );
        // "Restart": only the persisted manifest survives.
        let mut manifest = db::links::load(&conn).unwrap();
        assert_eq!(
            outcome.is_err(),
            // 3 links + 1 created directory (bin64/plugins) are journaled.
            budget < 4,
            "budget {budget} after_write {after_write}"
        );
        let report = undeploy::undeploy(&mut manifest, &mut DbJournal { conn: &conn }).unwrap();
        assert!(
            report.warnings.is_empty(),
            "budget {budget} after_write {after_write}: {:?}",
            report.warnings
        );
        assert_eq!(
            snapshot(&game),
            before,
            "budget {budget} after_write {after_write}"
        );
        assert!(db::links::load(&conn).unwrap().links.is_empty());
    });
}

#[test]
fn orphan_links_from_older_runs_are_adopted() {
    let fx = fixture();
    install::import(&fx.sources.join("Overlay"), &fx.staging).unwrap();
    let papgt = fx.config.game.dir.join("bin64/winmm.dll");
    // An untracked earlier deploy: original in .bak, our link in place.
    fs::rename(&papgt, papgt.with_file_name("winmm.dll.bak")).unwrap();
    std::os::unix::fs::symlink(fx.staging.join("Overlay/bin64/winmm.dll"), &papgt).unwrap();
    write(&fx.sources.join("unused"), "");
    let link_plan = plan::build(&fx.config, &fx.staging, &enabled_registry(&fx)).unwrap();
    let mut manifest = Manifest::default();
    let mut journal = crate::symlink::journal::NullJournal;
    let report = deploy::deploy(
        &link_plan,
        &[fx.staging.as_path()],
        &mut manifest,
        &mut journal,
    )
    .unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    undeploy::undeploy(&mut manifest, &mut journal).unwrap();
    assert_eq!(fs::read_to_string(&papgt).unwrap(), "loader");
}
