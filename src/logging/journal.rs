//! Writing events to the log file, the database and per-mod logs.

use super::{Event, Severity};
use crate::db;
use crate::error::Result;
use rusqlite::Connection;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// File name used for a mod's log (path separators are neutralised).
fn mod_log_name(mod_name: &str) -> String {
    format!("{}.log", mod_name.replace(['/', '\\'], "_"))
}

/// Appends a line to the mod's own log file.
fn append_mod_log(mod_logs: &Path, event: &Event, mod_name: &str) {
    let line = format!(
        "{} [{}] {}: {}\n",
        event.ts,
        event.severity.as_str(),
        event.kind.as_str(),
        event.message
    );
    let written = OpenOptions::new()
        .create(true)
        .append(true)
        .open(mod_logs.join(mod_log_name(mod_name)))
        .and_then(|mut file| file.write_all(line.as_bytes()));
    if let Err(err) = written {
        tracing::warn!(mod_name, %err, "could not write mod log");
    }
}

/// Emits the event to `tracing`.
fn trace(event: &Event) {
    let (kind, mod_name, message) = (
        event.kind.as_str(),
        event.mod_name.as_deref().unwrap_or("-"),
        &event.message,
    );
    match event.severity {
        Severity::Info => tracing::info!(kind, mod_name, "{message}"),
        Severity::Warn => tracing::warn!(kind, mod_name, "{message}"),
        Severity::Error => tracing::error!(kind, mod_name, "{message}"),
    }
}

/// Records an event in every sink. Journal failures never abort the caller.
pub fn record(conn: &Connection, mod_logs: &Path, event: &Event) {
    trace(event);
    if let Some(name) = &event.mod_name {
        append_mod_log(mod_logs, event, name);
    }
    if let Err(err) = db::events::insert(conn, event) {
        tracing::error!(%err, "could not store event");
    }
}

/// Appends a multi-line block (e.g. a file list) to a mod's log only.
pub fn mod_detail(mod_logs: &Path, mod_name: &str, title: &str, lines: &[String]) -> Result<()> {
    let body: String = lines.iter().map(|l| format!("    {l}\n")).collect();
    let path = mod_logs.join(mod_log_name(mod_name));
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| file.write_all(format!("{title}\n{body}").as_bytes()))
        .map_err(|source| crate::error::PyluneError::Io { path, source })
}
