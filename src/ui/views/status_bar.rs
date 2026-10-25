//! Bottom bar: last message and deployment summary.

use crate::ui::theme::{MUTED, level_color};
use crate::ui::{Action, PyluneApp};
use eframe::egui::{self, RichText};

/// Renders the status bar. It never emits actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    ui.horizontal(|ui| {
        ui.label(RichText::new("●").color(level_color(app.status.level)));
        ui.label(&app.status.text);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let enabled = app.registry.mods.iter().filter(|m| m.enabled).count();
            let pending = if app.dirty {
                " · changes not applied"
            } else {
                ""
            };
            ui.label(
                RichText::new(format!(
                    "{} link(s) deployed · {enabled}/{} mod(s) enabled{pending}",
                    app.deployed_links,
                    app.registry.mods.len()
                ))
                .color(MUTED),
            );
        });
    });
    Vec::new()
}
