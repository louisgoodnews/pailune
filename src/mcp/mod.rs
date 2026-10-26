//! Model Context Protocol server (B-14): `pylune mcp` speaks JSON-RPC 2.0,
//! one message per line on stdin/stdout, and offers Pylune's operations as
//! tools. Logs go to the log file, never to stdout.
//!
//! Changing tools take the single-instance lock on first use and keep it for
//! the session, so they fail while the window is open; read-only tools
//! always work.

pub mod tools;

#[cfg(test)]
mod tests;

use crate::cli::engine::Engine;
use crate::cli::ops::Report;
use crate::cli::read;
use crate::config::Config;
use crate::config::paths::DataPaths;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::process::ExitCode;

/// Protocol revision this server implements.
pub const PROTOCOL: &str = "2025-06-18";

/// Session state: where the data lives and the settings. Each changing tool
/// opens the engine (and the instance lock) only for its own run, so the
/// window can be used between calls.
pub struct Server {
    paths: DataPaths,
    config: Config,
}

impl Server {
    /// A server for these paths and settings.
    pub fn new(paths: DataPaths, config: Config) -> Self {
        Self { paths, config }
    }

    /// Answers one message; `None` for notifications.
    pub fn handle(&mut self, message: &Value) -> Option<Value> {
        let id = message.get("id").cloned()?;
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let result = match method {
            "initialize" => Ok(initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(tools::list()),
            "tools/call" => Ok(self.call(&params)),
            other => Err((-32601, format!("method not found: {other}"))),
        };
        Some(match result {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err((code, message)) => {
                json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
            }
        })
    }

    /// Runs a tool; failures are tool results with `isError`, as MCP asks.
    fn call(&mut self, params: &Value) -> Value {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let args = params.get("arguments").cloned().unwrap_or(json!({}));
        let outcome = tools::op(name, &args).and_then(|op| {
            let run = match op.read_only() {
                true => read::run(&op, &self.paths, &self.config),
                false => Engine::open(self.paths.clone(), self.config.clone())
                    .and_then(|mut engine| engine.run(&op)),
            };
            run.map_err(|e| e.to_string())
        });
        match outcome {
            Ok(Report { text, data }) => json!({
                "content": [{"type": "text", "text": text}],
                "structuredContent": {"message": text, "data": data},
                "isError": false,
            }),
            Err(message) => {
                json!({"content": [{"type": "text", "text": message}], "isError": true})
            }
        }
    }
}

/// The `initialize` result: protocol, capabilities, server info.
fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    json!({
        "protocolVersion": asked.filter(|v| *v == PROTOCOL).unwrap_or(PROTOCOL),
        "capabilities": {"tools": {"listChanged": false}},
        "serverInfo": {"name": "pylune", "version": env!("CARGO_PKG_VERSION")},
        "instructions": "Pylune manages Crimson Desert mods. Mods load in list order; later ones win \
            conflicts. Changes reach the game only after `apply`. Deletes are restorable; \
            clean_slate and restore_backup need confirm: true. Changing tools fail while the \
            Pylune window is open.",
    })
}

/// Serves stdin/stdout until the client closes the connection.
pub fn serve(paths: DataPaths, config: Config) -> ExitCode {
    let mut server = Server::new(paths, config);
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines().map_while(Result::ok) {
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(message) => server.handle(&message),
            Err(err) => Some(
                json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": err.to_string()}}),
            ),
        };
        if let Some(reply) = reply {
            let written = writeln!(stdout, "{reply}").and_then(|()| stdout.flush());
            if written.is_err() {
                break;
            }
        }
    }
    ExitCode::SUCCESS
}
