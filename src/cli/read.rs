//! Read-only operations straight from the database, so they also work
//! while the window is open.

use super::ops::{Op, Report};
use crate::config::Config;
use crate::config::paths::DataPaths;
use crate::db;
use crate::error::Result;
use crate::steam::{GameStatus, locate};
use serde_json::{Value, json};

/// Runs a read-only operation.
pub fn run(op: &Op, paths: &DataPaths, config: &Config) -> Result<Report> {
    let conn = db::open(&paths.database)?;
    match op {
        Op::Status => status(&conn, config),
        Op::List => list(&conn),
        Op::Profiles => profiles(&conn),
        Op::Log { mod_name, limit } => log(&conn, mod_name.as_deref(), *limit),
        _ => Ok(backups(paths)),
    }
}

fn status(conn: &rusqlite::Connection, config: &Config) -> Result<Report> {
    let info = locate::resolve(config);
    let registry = db::mods::load(conn)?;
    let links = db::links::load(conn)?.links.len();
    let pending = db::mods::pending(conn, &registry, links)?;
    let enabled = registry.mods.iter().filter(|m| m.enabled).count();
    let game = match locate::game_status(&info) {
        GameStatus::Ready => "found",
        GameStatus::Unrecognised => "not Crimson Desert",
        GameStatus::Missing => "missing",
    };
    let active = db::profiles::load(conn)?.active;
    let place = match info.game_dir.as_os_str().is_empty() {
        true => "no game folder set".to_string(),
        false => format!("game {game} at {}", info.game_dir.display()),
    };
    let text = format!(
        "{place}\n{enabled} of {} mod(s) enabled, {links} link(s) deployed{}{}",
        registry.mods.len(),
        if pending { ", changes not applied" } else { "" },
        active
            .as_ref()
            .map(|p| format!(", profile '{p}'"))
            .unwrap_or_default(),
    );
    Ok(Report {
        text,
        data: json!({
            "game": game, "game_dir": info.game_dir, "mods": registry.mods.len(),
            "enabled": enabled, "links": links, "pending": pending, "profile": active,
            "version": env!("CARGO_PKG_VERSION"),
        }),
    })
}

fn list(conn: &rusqlite::Connection) -> Result<Report> {
    let registry = db::mods::load(conn)?;
    let rows: Vec<Value> = registry
        .mods
        .iter()
        .enumerate()
        .map(|(i, m)| {
            json!({
                "position": i + 1, "name": m.name, "enabled": m.enabled,
                "kind": db::mods::kind_to_str(m.kind), "files": m.file_count,
                "variant": m.variant,
                "nexus": m.nexus.as_ref().map(|n| json!({"mod_id": n.mod_id, "file_id": n.file_id, "version": n.version})),
            })
        })
        .collect();
    let text = registry
        .mods
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let mark = if m.enabled { "x" } else { " " };
            format!(
                "{:>3} [{mark}] {} ({})",
                i + 1,
                m.name,
                db::mods::kind_to_str(m.kind)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Report {
        text: if text.is_empty() {
            "No mods installed.".into()
        } else {
            text
        },
        data: Value::Array(rows),
    })
}

fn profiles(conn: &rusqlite::Connection) -> Result<Report> {
    let book = db::profiles::load(conn)?;
    let text = book
        .profiles
        .iter()
        .map(|p| {
            let active = if book.active.as_deref() == Some(&p.name) {
                " (active)"
            } else {
                ""
            };
            format!("{}{active}: {} mod(s)", p.name, p.slots.len())
        })
        .collect::<Vec<_>>()
        .join("\n");
    let data = book
        .profiles
        .iter()
        .map(|p| json!({"name": p.name, "mods": p.slots.len(), "launch_options": p.launch_options}))
        .collect();
    Ok(Report {
        text: if text.is_empty() {
            "No profiles.".into()
        } else {
            text
        },
        data: json!({"active": book.active, "profiles": Value::Array(data)}),
    })
}

fn log(conn: &rusqlite::Connection, mod_name: Option<&str>, limit: usize) -> Result<Report> {
    let events = db::events::recent(conn, limit, mod_name)?;
    let text = events
        .iter()
        .map(|e| {
            format!(
                "{} {:<8} {:<8} {} {}",
                crate::logging::timestamp(e.ts),
                e.severity.as_str(),
                e.kind.as_str(),
                e.mod_name.as_deref().unwrap_or("-"),
                e.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let data = events
        .iter()
        .map(|e| json!({"ts": e.ts, "severity": e.severity.as_str(), "kind": e.kind.as_str(), "mod": e.mod_name, "message": e.message}))
        .collect();
    Ok(Report {
        text,
        data: Value::Array(data),
    })
}

fn backups(paths: &DataPaths) -> Report {
    let own = crate::app_backup::create::list(&paths.backups);
    let saves = crate::save_backup::list(&paths.save_backups);
    let lines = own
        .iter()
        .map(|b| {
            format!(
                "{} pylune {} {} {}",
                crate::logging::timestamp(b.created_at),
                b.kind.as_str(),
                b.label,
                b.path.display()
            )
        })
        .chain(saves.iter().map(|b| {
            format!(
                "{} saves {} {}",
                crate::logging::timestamp(b.created_at),
                b.label,
                b.path.display()
            )
        }));
    Report {
        text: lines.collect::<Vec<_>>().join("\n"),
        data: json!({
            "pylune": own.iter().map(|b| json!({"path": b.path, "created_at": b.created_at, "kind": b.kind.as_str(), "label": b.label, "mods": b.mods})).collect::<Vec<_>>(),
            "saves": saves.iter().map(|b| json!({"path": b.path, "created_at": b.created_at, "label": b.label, "files": b.files})).collect::<Vec<_>>(),
        }),
    }
}
