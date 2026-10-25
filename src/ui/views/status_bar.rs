//! Bottom bar: last message and deployment summary.

use crate::ui::theme::{level_color, muted};
use crate::ui::{Action, ModAction, PyluneApp};
use eframe::egui::{self, RichText};

/// Renders the status bar. It never emits actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    ui.horizontal(|ui| {
        ui.label(RichText::new("⏺").color(level_color(app.status.level)));
        ui.label(&app.status.text);
        // Error recovery: the last delete can be undone right here.
        let undo = app
            .mods
            .undo
            .as_ref()
            .filter(|_| ui.small_button("↩ Undo").clicked());
        let action = undo.map(|id| Action::Mod(ModAction::RestoreDeleted(id.clone())));
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
                .color(muted()),
            );
        });
        action
    })
    .inner
    .into_iter()
    .collect()
}
