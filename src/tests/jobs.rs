//! Job pool and Nexus connectivity.

use crate::config::Config;
use std::fs;

#[test]
#[ignore = "talks to api.nexusmods.com"]
fn nexus_rejects_bad_key() {
    let client = crate::nexus::api::Client::new("not-a-real-key", "crimsondesert");
    let err = client.validate().unwrap_err().to_string();
    assert!(err.contains("rejected"), "{err}");
}

#[test]
fn job_pool_runs_shared_jobs_in_parallel_and_exclusive_ones_alone() {
    use crate::config::paths::DataPaths;
    use crate::jobs::pool::JobPool;
    use crate::jobs::{JobKind, JobOutput, JobUpdate};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    let tmp = tempfile::tempdir().unwrap();
    let running = Arc::new(AtomicUsize::new(0));
    let peaks = Arc::new(Mutex::new(Vec::<(bool, usize)>::new()));
    let (r, p) = (Arc::clone(&running), Arc::clone(&peaks));
    let runner = Arc::new(move |_: &crate::jobs::context::JobContext, kind: JobKind| {
        let now = r.fetch_add(1, Ordering::SeqCst) + 1;
        std::thread::sleep(Duration::from_millis(150));
        p.lock().unwrap().push((kind == JobKind::Apply, now));
        r.fetch_sub(1, Ordering::SeqCst);
        Ok(JobOutput::Done(String::new()))
    });
    let pool = JobPool::spawn(
        4,
        DataPaths::under(tmp.path(), tmp.path()),
        runner,
        Arc::new(|| {}),
    );
    let config = Config::default();
    let kinds = [
        JobKind::CheckUpdates,
        JobKind::CheckUpdates,
        JobKind::CheckUpdates,
        JobKind::Apply,
        JobKind::Apply,
    ];
    let ids: Vec<_> = kinds
        .iter()
        .map(|k| pool.submit(k.clone(), &config))
        .collect();
    let start = Instant::now();
    let mut finished = 0;
    while finished < ids.len() && start.elapsed() < Duration::from_secs(5) {
        finished += pool
            .poll()
            .iter()
            .filter(|e| matches!(e.update, JobUpdate::Finished(Ok(_))))
            .count();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(finished, ids.len());
    let peaks = peaks.lock().unwrap();
    assert!(
        peaks.iter().any(|(_, n)| *n >= 3),
        "shared jobs overlapped: {peaks:?}"
    );
    // Two exclusive jobs never overlap each other.
    assert!(start.elapsed() >= Duration::from_millis(300));
}

/// Waits until every listed job finished, returning their results in order.
fn wait_all(
    pool: &crate::jobs::pool::JobPool,
    ids: &[u64],
) -> Vec<Result<crate::jobs::JobOutput, String>> {
    use crate::jobs::JobUpdate;
    use std::collections::BTreeMap;
    use std::time::{Duration, Instant};
    let start = Instant::now();
    let mut done = BTreeMap::new();
    while done.len() < ids.len() && start.elapsed() < Duration::from_secs(20) {
        pool.poll().into_iter().for_each(|e| {
            if let JobUpdate::Finished(result) = e.update {
                done.insert(e.id, result);
            }
        });
        std::thread::sleep(Duration::from_millis(10));
    }
    ids.iter()
        .map(|id| done.remove(id).expect("job finished"))
        .collect()
}

#[test]
fn real_jobs_import_apply_remove_and_reset() {
    use crate::config::paths::DataPaths;
    use crate::db;
    use crate::jobs::pool::JobPool;
    use crate::jobs::{JobKind, tasks};
    use crate::mod_engine::registry;
    use std::sync::Arc;

    let fx = super::fixture();
    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    let pool = JobPool::spawn(3, paths.clone(), Arc::new(tasks::run), Arc::new(|| {}));
    let import = |name: &str| JobKind::Import {
        source: fx.sources.join(name),
        replace: None,
    };
    let ids = [
        pool.submit(import("Cool Mod"), &fx.config),
        pool.submit(import("Overlay"), &fx.config),
    ];
    let results = wait_all(&pool, &ids);
    assert!(results.iter().all(Result::is_ok), "{results:?}");

    let mut conn = db::open(&paths.database).unwrap();
    assert_eq!(
        db::mods::load(&conn).unwrap().mods.len(),
        2,
        "parallel imports both registered"
    );
    db::mods::update(&mut conn, |r| Ok(registry::with_all(r, true))).unwrap();
    let apply = pool.submit(JobKind::Apply, &fx.config);
    let analyze = pool.submit(JobKind::AnalyzeConflicts(1), &fx.config);
    assert!(wait_all(&pool, &[apply, analyze]).iter().all(Result::is_ok));
    assert_eq!(db::links::load(&conn).unwrap().links.len(), 4);
    assert!(
        paths.mod_logs.join("Cool Mod.log").is_file(),
        "per-mod log written"
    );

    let remove = pool.submit(JobKind::Remove("Cool Mod".into()), &fx.config);
    assert!(wait_all(&pool, &[remove])[0].is_ok());
    assert!(
        !fx.config.game.dir.join("bin64/CoolMod.asi").exists(),
        "redeployed without the mod"
    );
    assert_eq!(db::links::load(&conn).unwrap().links.len(), 2);

    let reset = pool.submit(JobKind::CleanSlate, &fx.config);
    assert!(wait_all(&pool, &[reset])[0].is_ok());
    assert!(db::links::load(&conn).unwrap().links.is_empty());
    assert_eq!(
        fs::read_to_string(fx.config.game.dir.join("meta/0.papgt")).unwrap(),
        "vanilla-papgt"
    );
    assert!(
        db::mods::load(&conn)
            .unwrap()
            .mods
            .iter()
            .all(|m| !m.enabled)
    );
    assert!(!db::events::recent(&conn, 50, None).unwrap().is_empty());
}

#[test]
fn pending_state_and_deleting_a_disabled_but_deployed_mod() {
    use crate::config::paths::DataPaths;
    use crate::db;
    use crate::jobs::pool::JobPool;
    use crate::jobs::{JobKind, tasks};
    use crate::mod_engine::registry;
    use std::sync::Arc;

    let fx = super::fixture();
    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    let pool = JobPool::spawn(2, paths.clone(), Arc::new(tasks::run), Arc::new(|| {}));
    let source = fx.sources.join("Cool Mod");
    let import = pool.submit(
        JobKind::Import {
            source,
            replace: None,
        },
        &fx.config,
    );
    assert!(wait_all(&pool, &[import])[0].is_ok());

    let mut conn = db::open(&paths.database).unwrap();
    let pending = |conn: &rusqlite::Connection| {
        let registry = db::mods::load(conn).unwrap();
        let links = db::links::load(conn).unwrap().links.len();
        db::mods::pending(conn, &registry, links).unwrap()
    };
    assert!(!pending(&conn), "nothing enabled, nothing linked");
    db::mods::update(&mut conn, |r| Ok(registry::with_all(r, true))).unwrap();
    assert!(pending(&conn), "enabled but not applied");
    let apply = pool.submit(JobKind::Apply, &fx.config);
    assert!(wait_all(&pool, &[apply])[0].is_ok());
    assert!(!pending(&conn), "applied");

    // Disable without applying, then delete: its links must still go.
    db::mods::update(&mut conn, |r| Ok(registry::with_all(r, false))).unwrap();
    assert!(pending(&conn));
    let remove = pool.submit(JobKind::Remove("Cool Mod".into()), &fx.config);
    assert!(wait_all(&pool, &[remove])[0].is_ok());
    assert!(db::links::load(&conn).unwrap().links.is_empty());
    assert!(!fx.config.game.dir.join("bin64/CoolMod.asi").exists());
    assert!(!pending(&conn), "redeployed and clean");

    let unlink = pool.submit(JobKind::UnlinkAll, &fx.config);
    assert!(wait_all(&pool, &[unlink])[0].is_ok());
    assert!(!pending(&conn), "unlinked with nothing enabled");
}
