//! Single-instance coordination and the `nxm://` hand-off spool.
//!
//! The primary instance holds an exclusive `flock` on `pailune.lock` for its
//! whole lifetime; the kernel releases it on exit or crash, so there is no
//! stale state. A secondary process (started by the browser for an `nxm://`
//! link, or by the user) writes one spool file per request — atomically, via
//! rename — and exits. The primary drains the spool once per second.

use crate::config::paths::DataPaths;
use crate::error::{IoContext, Result};
use std::fs::{self, File, OpenOptions, TryLockError};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Spool entry asking the running window to come to the front.
pub const FOCUS_REQUEST: &str = "pailune:focus";

/// Proof of being the primary instance; keep it alive until exit.
#[derive(Debug)]
pub struct InstanceLock {
    _file: File,
}

/// What `main` should do after start-up coordination.
#[derive(Debug)]
pub enum Startup {
    /// This is the only instance: open the window.
    RunGui(InstanceLock),
    /// Another instance runs and received the request: exit quietly.
    HandedOff,
}

/// Writes one request into the spool atomically (temp file + rename).
fn spool(dir: &Path, request: &str) -> Result<()> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let stem = format!("{nanos:024}-{}", std::process::id());
    let tmp = dir.join(format!("{stem}.tmp"));
    fs::write(&tmp, request).at(&tmp)?;
    let done = dir.join(format!("{stem}.req"));
    fs::rename(&tmp, &done).at(&done)
}

/// Takes the instance lock without starting a window (command line, MCP);
/// `None` when another instance holds it.
pub fn try_lock(paths: &DataPaths) -> Result<Option<InstanceLock>> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&paths.lock)
        .at(&paths.lock)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(InstanceLock { _file: file })),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(err)) => Err(err).at(&paths.lock),
    }
}

/// [`try_lock`], retrying for up to `wait`. A lock that was just released
/// can look taken for a moment: a process started at that instant holds an
/// inherited copy of the lock file until it executes its program.
pub fn try_lock_for(paths: &DataPaths, wait: std::time::Duration) -> Result<Option<InstanceLock>> {
    let started = std::time::Instant::now();
    loop {
        match try_lock(paths)? {
            Some(lock) => return Ok(Some(lock)),
            None if started.elapsed() >= wait => return Ok(None),
            None => std::thread::sleep(std::time::Duration::from_millis(10)),
        }
    }
}

/// Spools `nxm://` arguments and decides whether to show the GUI.
pub fn bootstrap(paths: &DataPaths, args: &[String]) -> Result<Startup> {
    fs::create_dir_all(&paths.spool).at(&paths.spool)?;
    let links: Vec<&String> = args.iter().filter(|a| a.starts_with("nxm://")).collect();
    links
        .iter()
        .try_for_each(|link| spool(&paths.spool, link))?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&paths.lock)
        .at(&paths.lock)?;
    match file.try_lock() {
        Ok(()) => Ok(Startup::RunGui(InstanceLock { _file: file })),
        Err(TryLockError::WouldBlock) => {
            if links.is_empty() {
                spool(&paths.spool, FOCUS_REQUEST)?;
            }
            Ok(Startup::HandedOff)
        }
        Err(TryLockError::Error(err)) => Err(err).at(&paths.lock),
    }
}

/// Takes every complete request from the spool, oldest first.
pub fn drain(dir: &Path) -> Vec<String> {
    let mut files: Vec<_> = fs::read_dir(dir)
        .map(|entries| entries.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    files.retain(|p| p.extension().is_some_and(|e| e == "req"));
    files.sort();
    files
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok();
            let _ = fs::remove_file(&path);
            text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_instance_hands_off_and_spool_keeps_order() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = DataPaths::under(tmp.path(), tmp.path());
        let primary = bootstrap(&paths, &["nxm://a/mods/1/files/1".into()]).unwrap();
        assert!(matches!(primary, Startup::RunGui(_)));
        let secondary = bootstrap(&paths, &["nxm://a/mods/2/files/2".into()]).unwrap();
        assert!(matches!(secondary, Startup::HandedOff));
        assert!(matches!(
            bootstrap(&paths, &[]).unwrap(),
            Startup::HandedOff
        ));
        assert_eq!(
            drain(&paths.spool),
            [
                "nxm://a/mods/1/files/1",
                "nxm://a/mods/2/files/2",
                FOCUS_REQUEST
            ]
        );
        assert!(drain(&paths.spool).is_empty());
        drop(primary);
        assert!(
            matches!(bootstrap(&paths, &[]).unwrap(), Startup::RunGui(_)),
            "lock released on drop"
        );
    }
}
