//! Single-instance coordination and the `nxm://` hand-off queue.
//!
//! When the browser opens an `nxm://` link, a second Pylune process starts.
//! It appends the link to a queue file and exits if a primary instance is
//! running; the primary polls the queue and downloads the file.

use crate::config::paths::DataPaths;
use crate::error::{IoContext, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// What `main` should do after start-up coordination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Startup {
    /// No other instance runs: open the window.
    RunGui,
    /// Links were handed to the running instance: exit quietly.
    HandedOff,
}

/// Whether the PID in `pid_file` belongs to a live process of this binary.
fn primary_alive(pid_file: &Path) -> bool {
    let me = std::env::current_exe().ok();
    fs::read_to_string(pid_file)
        .ok()
        .and_then(|pid| pid.trim().parse::<u32>().ok())
        .filter(|pid| *pid != std::process::id())
        .and_then(|pid| fs::read_link(format!("/proc/{pid}/exe")).ok())
        .is_some_and(|exe| Some(exe) == me)
}

/// Appends links to the queue file.
fn enqueue(queue: &Path, links: &[String]) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(queue)
        .at(queue)?;
    links
        .iter()
        .try_for_each(|link| writeln!(file, "{link}").at(queue))
}

/// Queues any `nxm://` arguments and decides whether to show the GUI.
pub fn bootstrap(paths: &DataPaths, args: &[String]) -> Result<Startup> {
    let links: Vec<String> = args
        .iter()
        .filter(|a| a.starts_with("nxm://"))
        .cloned()
        .collect();
    if !links.is_empty() {
        enqueue(&paths.nxm_queue, &links)?;
    }
    match primary_alive(&paths.pid) {
        true => Ok(Startup::HandedOff),
        false => fs::write(&paths.pid, std::process::id().to_string())
            .at(&paths.pid)
            .map(|()| Startup::RunGui),
    }
}

/// Takes every queued link, leaving the queue empty.
pub fn drain(queue: &Path) -> Vec<String> {
    let taken = queue.with_extension("processing");
    match fs::rename(queue, &taken) {
        Err(_) => Vec::new(),
        Ok(()) => {
            let links = fs::read_to_string(&taken)
                .map(|text| {
                    text.lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            let _ = fs::remove_file(&taken);
            links
        }
    }
}
