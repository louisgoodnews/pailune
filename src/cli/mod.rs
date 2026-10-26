//! Command-line interface (board item B-14): `pylune <command>` manages mods
//! without the window; `pylune mcp` serves the same operations to AI agents
//! over the Model Context Protocol. Without a command the window opens.

pub mod engine;
pub mod ops;
pub mod read;

use crate::config::Config;
use crate::config::paths::DataPaths;
use crate::error::{PyluneError, Result};
use clap::{Parser, Subcommand};
use ops::{Op, Report};
use std::path::PathBuf;
use std::process::ExitCode;

/// Pylune — Crimson Desert mod manager.
#[derive(Debug, Parser)]
#[command(name = "pylune", version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Print machine-readable JSON.
    #[arg(long, global = true)]
    json: bool,
    /// Confirm an operation that cannot be undone (clean-slate, restore).
    #[arg(long, global = true)]
    yes: bool,
}

/// Commands; each maps to one [`Op`].
#[derive(Debug, Subcommand)]
enum Command {
    /// Game, deployment and mod counts.
    Status,
    /// Mods in load order.
    List,
    /// Saved profiles.
    Profiles,
    /// Recent events (installs, deployments, errors).
    Log {
        /// Only events of this mod.
        #[arg(long = "mod")]
        mod_name: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Pylune and save-game backups.
    Backups,
    /// Enable mods (nothing reaches the game until `apply`).
    Enable {
        names: Vec<String>,
        #[arg(long)]
        all: bool,
    },
    /// Disable mods.
    Disable {
        names: Vec<String>,
        #[arg(long)]
        all: bool,
    },
    /// Move a mod to a load-order number (1 = first; last wins conflicts).
    Move { name: String, position: usize },
    /// Choose the option folder of a mod with variants.
    Variant { name: String, variant: String },
    /// Install mods from archives, folders or loose files.
    Install { paths: Vec<PathBuf> },
    /// Delete a mod (it stays restorable in the window's "Recently deleted").
    Delete { name: String },
    /// Link the enabled mods into the game.
    Apply,
    /// Remove every link and restore the original files.
    Unlink,
    /// Put the game back to its original state and disable every mod (needs --yes).
    CleanSlate,
    /// Save or load a profile.
    Profile {
        #[command(subcommand)]
        action: ProfileCommand,
    },
    /// Write the mod list to a file.
    Export { path: PathBuf },
    /// Apply a mod list file; missing mods are reported.
    Import { path: PathBuf },
    /// Back up settings and the mod list (--full: with mod files).
    Backup {
        #[arg(long)]
        full: bool,
    },
    /// Restore a Pylune backup (needs --yes).
    Restore { path: PathBuf },
    /// Back up the game's save files.
    BackupSaves,
    /// Check Nexus Mods for updates.
    CheckUpdates,
    /// Start the game (applies pending changes first if configured).
    Launch,
    /// Serve these operations over the Model Context Protocol (stdio).
    Mcp,
}

/// Profile subcommands.
#[derive(Debug, Subcommand)]
enum ProfileCommand {
    Save { name: String },
    Load { name: String },
}

/// The operation a command stands for.
fn op(command: Command) -> Option<Op> {
    Some(match command {
        Command::Status => Op::Status,
        Command::List => Op::List,
        Command::Profiles => Op::Profiles,
        Command::Log { mod_name, limit } => Op::Log { mod_name, limit },
        Command::Backups => Op::Backups,
        Command::Enable { all: true, .. } => Op::SetAll(true),
        Command::Disable { all: true, .. } => Op::SetAll(false),
        Command::Enable { names, .. } => Op::SetEnabled {
            names,
            enabled: true,
        },
        Command::Disable { names, .. } => Op::SetEnabled {
            names,
            enabled: false,
        },
        Command::Move { name, position } => Op::Move { name, position },
        Command::Variant { name, variant } => Op::SetVariant { name, variant },
        Command::Install { paths } => Op::Install { paths },
        Command::Delete { name } => Op::Delete { name },
        Command::Apply => Op::Apply,
        Command::Unlink => Op::Unlink,
        Command::CleanSlate => Op::CleanSlate,
        Command::Profile {
            action: ProfileCommand::Save { name },
        } => Op::ProfileSave { name },
        Command::Profile {
            action: ProfileCommand::Load { name },
        } => Op::ProfileLoad { name },
        Command::Export { path } => Op::ExportList { path },
        Command::Import { path } => Op::ImportList { path },
        Command::Backup { full } => Op::Backup { full },
        Command::Restore { path } => Op::RestoreBackup { path },
        Command::BackupSaves => Op::BackupSaves,
        Command::CheckUpdates => Op::CheckUpdates,
        Command::Launch => Op::Launch,
        Command::Mcp => return None,
    })
}

/// Runs one operation: read-only ones from the database, others headlessly.
pub fn execute(op: &Op, paths: &DataPaths, config: &Config) -> Result<Report> {
    match op.read_only() {
        true => read::run(op, paths, config),
        false => engine::Engine::open(paths.clone(), config.clone())?.run(op),
    }
}

/// Parses the arguments; `None` means "open the window": no arguments, or
/// only `nxm://` links from the browser (checked before clap sees them).
pub fn parse() -> Option<Cli> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().all(|a| a.starts_with("nxm://")) {
        true => None,
        false => Some(Cli::parse()),
    }
}

/// Runs a parsed command line and returns the process exit code.
pub fn run(cli: Cli, paths: DataPaths, config: Config) -> ExitCode {
    let (json, yes) = (cli.json, cli.yes);
    let Some(command) = cli.command else {
        // Options without a command, e.g. `pylune --json`.
        eprintln!("pylune: a command is needed — see `pylune --help`");
        return ExitCode::from(2);
    };
    let Some(op) = op(command) else {
        return crate::mcp::serve(paths, config);
    };
    let outcome = match op.destructive() && !yes {
        true => Err(PyluneError::Invalid(
            "this cannot be undone from Pylune — repeat with --yes to confirm".into(),
        )),
        false => execute(&op, &paths, &config),
    };
    match (outcome, json) {
        (Ok(report), true) => {
            println!(
                "{}",
                serde_json::json!({"ok": true, "message": report.text, "data": report.data})
            );
            ExitCode::SUCCESS
        }
        (Ok(report), false) => {
            println!("{}", report.text);
            ExitCode::SUCCESS
        }
        (Err(err), true) => {
            println!(
                "{}",
                serde_json::json!({"ok": false, "message": err.to_string()})
            );
            ExitCode::FAILURE
        }
        (Err(err), false) => {
            eprintln!("pylune: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_flags_work_before_and_after_the_command() {
        let parse = |args: &[&str]| Cli::try_parse_from(args).unwrap();
        let cli = parse(&["pylune", "--json", "status"]);
        assert!(cli.json && matches!(cli.command, Some(Command::Status)));
        let cli = parse(&["pylune", "clean-slate", "--yes"]);
        assert!(cli.yes && matches!(cli.command, Some(Command::CleanSlate)));
        let cli = parse(&["pylune", "move", "My Mod", "3"]);
        assert!(matches!(
            op(cli.command.unwrap()),
            Some(Op::Move { position: 3, .. })
        ));
        assert!(Cli::try_parse_from(["pylune", "nxm://crimsondesert/mods/1"]).is_err());
    }
}
