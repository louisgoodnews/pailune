//! Floating window showing a mod's readme.

use crate::ui::{Action, ModAction};
use eframe::egui::{self, RichText};
use std::path::PathBuf;

/// Shows the readme window if one is open; closing emits `CloseReadme`.
pub fn show(ctx: &egui::Context, readme: Option<&(PathBuf, String)>) -> Vec<Action> {
    let Some((path, text)) = readme else {
        return Vec::new();
    };
    let mut open = true;
    egui::Window::new(format!("📄 {}", path.display()))
        .open(&mut open)
        .default_size([640.0, 480.0])
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.label(RichText::new(text).monospace());
                });
        });
    match open {
        true => Vec::new(),
        false => vec![Action::Mod(ModAction::CloseReadme)],
    }
}
