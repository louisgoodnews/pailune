//! Read-only save commands: list slots, show one slot's contents.

use super::ops::Report;
use crate::config::Config;
use crate::error::{PailuneError, Result};
use crate::savefile::slots::{inspect, slots};
use crate::savefile::view::Node;
use serde_json::{Value, json};
use std::path::PathBuf;

/// The save folder of the configured game.
fn save_dir(config: &Config) -> Result<PathBuf> {
    let prefix = crate::steam::locate::resolve(config).prefix_dir;
    crate::save_backup::save_dir(prefix.as_deref())
        .filter(|d| d.is_dir())
        .ok_or_else(|| PailuneError::Invalid("no save folder found (start the game once)".into()))
}

/// Every slot with level and time.
pub fn list(config: &Config) -> Result<Report> {
    let all = slots(&save_dir(config)?);
    let text = all
        .iter()
        .map(|s| match &s.error {
            Some(err) => format!("{}: unreadable ({err})", s.name),
            None => format!(
                "{}: level {} · saved {}",
                s.name,
                s.level.as_deref().unwrap_or("?"),
                s.saved_at
                    .map(crate::logging::timestamp)
                    .unwrap_or_default()
            ),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let data = all
        .iter()
        .map(|s| json!({"slot": s.name, "level": s.level, "saved_at": s.saved_at, "mission_key": s.mission, "size": s.size, "error": s.error}))
        .collect();
    Ok(Report {
        text,
        data: Value::Array(data),
    })
}

/// The node and its descendants, keeping only branches that mention `find`.
fn filtered(node: &Node, find: &str) -> Option<Value> {
    let hit = node.label.to_lowercase().contains(find);
    let children: Vec<Value> = node
        .children
        .iter()
        .filter_map(|c| filtered(c, if hit { "" } else { find }))
        .collect();
    (hit || !children.is_empty()).then(|| match children.is_empty() {
        true => Value::String(node.label.clone()),
        false => json!({ node.label.clone(): children }),
    })
}

/// Indented text of a JSON tree.
fn text(value: &Value, depth: usize, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(format!("{}{s}", "  ".repeat(depth))),
        Value::Object(map) => map.iter().for_each(|(k, v)| {
            out.push(format!("{}{k}", "  ".repeat(depth)));
            v.as_array()
                .into_iter()
                .flatten()
                .for_each(|c| text(c, depth + 1, out));
        }),
        _ => {}
    }
}

/// One slot's contents.
pub fn show(config: &Config, slot: &str, find: Option<&str>) -> Result<Report> {
    let name = match slot.parse::<u32>() {
        Ok(n) => format!("slot{n}"),
        Err(_) => slot.to_string(),
    };
    let found = slots(&save_dir(config)?)
        .into_iter()
        .find(|s| s.name == name);
    let dir = found
        .ok_or_else(|| PailuneError::Invalid(format!("no save slot '{name}'")))?
        .dir;
    let needle = find.unwrap_or_default().to_lowercase();
    let tree: Vec<Value> = inspect(&dir)?
        .iter()
        .filter_map(|n| filtered(n, &needle))
        .collect();
    let mut lines = Vec::new();
    tree.iter().for_each(|v| text(v, 0, &mut lines));
    Ok(Report {
        text: lines.join("\n"),
        data: Value::Array(tree),
    })
}
