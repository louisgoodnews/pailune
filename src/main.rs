//! Pailune — a native Linux mod manager for *Crimson Desert* (Steam / Proton).
//!
//! Pailune keeps every mod in a private staging folder and deploys it into the
//! game directory exclusively through symbolic links. Original files that a
//! mod would shadow are renamed to `.bak` and restored on revert.

// No console window behind the GUI on Windows; the command line re-attaches
// to the terminal it was started from (see `platform::attach_console`).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app_backup;
mod backup;
mod cli;
mod config;
mod db;
mod error;
mod fomod;
// Wired into Apply once the JMM/DMM analysis settles the game-side details.
#[allow(dead_code)]
mod inplace;
mod instance;
mod jobs;
mod launch;
mod logging;
mod mcp;
mod mod_engine;
mod modlist;
mod nexus;
mod overlay;
mod patch;
mod paz;
mod platform;
mod profiles;
mod save_backup;
mod savefile;
mod steam;
mod symlink;
mod tables;
#[cfg(test)]
mod tests;
mod ui;

/// Runs a command-line command, hands `nxm://` links to a running
/// instance, or starts the GUI.
fn main() -> std::process::ExitCode {
    let command = cli::parse();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths = match config::paths::data_paths() {
        Ok(paths) => paths,
        Err(err) => {
            eprintln!("pailune: {err}");
            std::process::exit(1);
        }
    };
    let (config, notice) = config::file::load_or_recover(&paths.config_file);
    let _log_guard =
        logging::init::init(&paths.logs, &config.logging.level, config.logging.keep_days);
    if let Some(command) = command {
        platform::attach_console();
        return cli::run(command, paths, config);
    }
    match gui(paths, config, notice, &args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("pailune: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Starts the window (or hands the request to the one already open).
fn gui(
    paths: config::paths::DataPaths,
    config: config::Config,
    notice: Option<String>,
    args: &[String],
) -> eframe::Result<()> {
    // The lock must live until the window closes.
    let _instance = match instance::bootstrap(&paths, args) {
        Ok(instance::Startup::HandedOff) => {
            eprintln!("pailune: already running — request handed to the open window");
            return Ok(());
        }
        Ok(instance::Startup::RunGui(lock)) => {
            tracing::info!("Pailune {} starting", env!("CARGO_PKG_VERSION"));
            lock
        }
        Err(err) => {
            tracing::error!(%err, "single-instance lock failed");
            eprintln!("pailune: cannot take the instance lock: {err}");
            std::process::exit(1);
        }
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Pailune — Crimson Desert Mod Manager")
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([860.0, 500.0])
            .with_app_id("pailune")
            .with_icon(ui::icon::icon()),
        ..Default::default()
    };
    // A broken database gets an explanatory window instead of a crash.
    let startup = db::open(&paths.database).map_err(|err| err.to_string());
    eframe::run_native(
        "Pailune",
        options,
        Box::new(move |cc| match startup {
            Ok(conn) => Ok(Box::new(ui::PailuneApp::new(
                cc, paths, config, conn, notice,
            ))),
            Err(err) => Ok(Box::new(ui::fatal::FatalApp::new(err, &paths))),
        }),
    )
}
