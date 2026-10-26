//! Pylune — a native Linux mod manager for *Crimson Desert* (Steam / Proton).
//!
//! Pylune keeps every mod in a private staging folder and deploys it into the
//! game directory exclusively through symbolic links. Original files that a
//! mod would shadow are renamed to `.bak` and restored on revert.

mod app_backup;
mod backup;
mod config;
mod db;
mod error;
mod instance;
mod jobs;
mod launch;
mod logging;
mod mod_engine;
mod modlist;
mod nexus;
mod overlay;
mod patch;
mod paz;
mod platform;
mod profiles;
mod save_backup;
mod steam;
mod symlink;
#[cfg(test)]
mod tests;
mod ui;

/// Hands `nxm://` links to a running instance, or starts the GUI.
fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths = match config::paths::data_paths() {
        Ok(paths) => paths,
        Err(err) => {
            eprintln!("pylune: {err}");
            std::process::exit(1);
        }
    };
    let (config, notice) = config::file::load_or_recover(&paths.config_file);
    let _log_guard =
        logging::init::init(&paths.logs, &config.logging.level, config.logging.keep_days);
    // The lock must live until the window closes.
    let _instance = match instance::bootstrap(&paths, &args) {
        Ok(instance::Startup::HandedOff) => {
            eprintln!("pylune: already running — request handed to the open window");
            return Ok(());
        }
        Ok(instance::Startup::RunGui(lock)) => {
            tracing::info!("Pylune {} starting", env!("CARGO_PKG_VERSION"));
            lock
        }
        Err(err) => {
            tracing::error!(%err, "single-instance lock failed");
            eprintln!("pylune: cannot take the instance lock: {err}");
            std::process::exit(1);
        }
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Pylune — Crimson Desert Mod Manager")
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([860.0, 500.0])
            .with_app_id("pylune")
            .with_icon(ui::icon::icon()),
        ..Default::default()
    };
    // A broken database gets an explanatory window instead of a crash.
    let startup = db::open(&paths.database).map_err(|err| err.to_string());
    eframe::run_native(
        "Pylune",
        options,
        Box::new(move |cc| match startup {
            Ok(conn) => Ok(Box::new(ui::PyluneApp::new(
                cc, paths, config, conn, notice,
            ))),
            Err(err) => Ok(Box::new(ui::fatal::FatalApp::new(err, &paths))),
        }),
    )
}
