//! Pylune — a native Linux mod manager for *Crimson Desert* (Steam / Proton).
//!
//! Pylune keeps every mod in a private staging folder and deploys it into the
//! game directory exclusively through symbolic links. Original files that a
//! mod would shadow are renamed to `.bak` and restored on revert.

mod backup;
mod config;
mod error;
mod mod_engine;
mod steam;
mod symlink;
#[cfg(test)]
mod tests;
mod ui;

/// Starts the eframe event loop with the Pylune application.
fn main() -> eframe::Result<()> {
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
