//! MCP tool definitions and their mapping to [`Op`]s.

use crate::cli::ops::Op;
use serde_json::{Value, json};
use std::path::PathBuf;

mod table;

pub use table::TOOLS;

/// The tool list as `tools/list` returns it.
pub fn list() -> Value {
    let tools: Vec<Value> = TOOLS
        .iter()
        .map(|t| {
            let properties: serde_json::Map<String, Value> = t
                .params
                .iter()
                .map(|(name, kind, description)| {
                    let schema = match *kind {
                        "array" => json!({"type": "array", "items": {"type": "string"}, "description": description}),
                        kind => json!({"type": kind, "description": description}),
                    };
                    (name.to_string(), schema)
                })
                .collect();
            let required: Vec<&str> = t
                .params
                .iter()
                .filter(|(_, _, d)| !d.starts_with("Optional"))
                .map(|(n, _, _)| *n)
                .collect();
            json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": {"type": "object", "properties": properties, "required": required},
                "annotations": {"readOnlyHint": t.read_only, "destructiveHint": t.destructive, "openWorldHint": false},
            })
        })
        .collect();
    json!({ "tools": tools })
}

/// Builds the operation for a tool call; `Err` explains bad arguments.
pub fn op(name: &str, args: &Value) -> Result<Op, String> {
    let text = |key: &str| {
        args.get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or(format!("missing string argument '{key}'"))
    };
    let list = |key: &str| {
        args.get(key)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .ok_or(format!("missing array argument '{key}'"))
    };
    let flag = |key: &str| args.get(key).and_then(Value::as_bool).unwrap_or(false);
    let number = |key: &str| args.get(key).and_then(Value::as_u64).map(|n| n as usize);
    let confirmed = |op: Op| match flag("confirm") {
        true => Ok(op),
        false => Err("this cannot be undone: call again with confirm: true".to_string()),
    };
    match name {
        "status" => Ok(Op::Status),
        "list_mods" => Ok(Op::List),
        "list_profiles" => Ok(Op::Profiles),
        "read_log" => Ok(Op::Log {
            mod_name: text("mod_name").ok(),
            limit: number("limit").unwrap_or(50),
        }),
        "list_backups" => Ok(Op::Backups),
        "list_saves" => Ok(Op::Saves),
        "read_save" => Ok(Op::SaveShow {
            slot: text("slot")?,
            find: text("find").ok(),
        }),
        "enable_mods" => Ok(Op::SetEnabled {
            names: list("names")?,
            enabled: true,
        }),
        "disable_mods" => Ok(Op::SetEnabled {
            names: list("names")?,
            enabled: false,
        }),
        "set_all_mods" => Ok(Op::SetAll(flag("enabled"))),
        "move_mod" => Ok(Op::Move {
            name: text("name")?,
            position: number("position").ok_or("missing integer argument 'position'")?,
        }),
        "set_variant" => Ok(Op::SetVariant {
            name: text("name")?,
            variant: text("variant")?,
        }),
        "install_mods" => Ok(Op::Install {
            paths: list("paths")?.into_iter().map(PathBuf::from).collect(),
        }),
        "delete_mod" => Ok(Op::Delete {
            name: text("name")?,
        }),
        "apply" => Ok(Op::Apply),
        "unlink_all" => Ok(Op::Unlink),
        "clean_slate" => confirmed(Op::CleanSlate),
        "save_profile" => Ok(Op::ProfileSave {
            name: text("name")?,
        }),
        "load_profile" => Ok(Op::ProfileLoad {
            name: text("name")?,
        }),
        "export_mod_list" => Ok(Op::ExportList {
            path: text("path")?.into(),
        }),
        "import_mod_list" => Ok(Op::ImportList {
            path: text("path")?.into(),
        }),
        "create_backup" => Ok(Op::Backup { full: flag("full") }),
        "restore_backup" => confirmed(Op::RestoreBackup {
            path: text("path")?.into(),
        }),
        "backup_saves" => Ok(Op::BackupSaves),
        "check_updates" => Ok(Op::CheckUpdates),
        "launch_game" => Ok(Op::Launch),
        other => Err(format!("unknown tool '{other}'")),
    }
}
