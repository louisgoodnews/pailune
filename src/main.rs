//! Pylune — a native Linux mod manager for *Crimson Desert* (Steam / Proton).
//!
//! Pylune keeps every mod in a private staging folder and deploys it into the
//! game directory exclusively through symbolic links. Original files that a
//! mod would shadow are renamed to `.bak` and restored on revert.

mod backup;
mod config;
mod error;
mod instance;
mod launch;
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
    let startup = config::store::data_paths().and_then(|paths| instance::bootstrap(&paths, &args));
    match startup {
        Ok(instance::Startup::HandedOff) => return Ok(()),
        Ok(instance::Startup::RunGui) => {}
        Err(err) => eprintln!("pylune: start-up coordination failed: {err}"),
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Pylune — Crimson Desert Mod Manager")
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([720.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Pylune",
        options,
        Box::new(|cc| Ok(Box::new(ui::PyluneApp::new(cc)))),
    )
}
