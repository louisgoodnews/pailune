//! Pylune — a native Linux mod manager for *Crimson Desert* (Steam / Proton).
//!
//! Pylune keeps every mod in a private staging folder and deploys it into the
//! game directory exclusively through symbolic links. Original files that a
//! mod would shadow are renamed to `.bak` and restored on revert.

mod backup;
mod config;
mod db;
mod error;
mod instance;
mod jobs;
mod launch;
mod logging;
mod mod_engine;
mod nexus;
mod profiles;
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
    let config = config::file::load_or_create(&paths.config_file).unwrap_or_else(|err| {
        eprintln!("pylune: {err}; using defaults");
        config::Config::default()
    });
    let _log_guard = logging::init::init(&paths.logs, &config.logging.level);
    match instance::bootstrap(&paths, &args) {
        Ok(instance::Startup::HandedOff) => return Ok(()),
        Ok(instance::Startup::RunGui) => {
            tracing::info!("Pylune {} starting", env!("CARGO_PKG_VERSION"))
        }
        Err(err) => tracing::error!(%err, "start-up coordination failed"),
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Pylune — Crimson Desert Mod Manager")
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([860.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Pylune",
        options,
        Box::new(move |cc| Ok(Box::new(ui::PyluneApp::new(cc, paths, config)))),
    )
}
