//! Guided recovery shown after a game update.

use super::clicked;
use crate::backup::MetaBackup;
use crate::ui::theme::{self, WARNING};
use crate::ui::{Action, DeployAction, PyluneApp};
use eframe::egui::{self, RichText};

/// Renders the update banner when the metadata backup is outdated.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    if app.backup.meta != MetaBackup::Outdated {
        return Vec::new();
    }
    egui::Frame::new()
        .fill(WARNING.linear_multiply(0.12))
        .stroke(egui::Stroke::new(1.0, WARNING))
        .corner_radius(6)
        .inner_margin(8)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Crimson Desert was updated since Pylune's backup")
                    .strong()
                    .color(WARNING),
            );
            ui.label(
                "Mods built for the old version may break the game, and the old .bak files must \
                 not be restored. Recover in three steps, then apply your mods again:",
            );
            ui.horizontal_wrapped(|ui| {
                let unlinked = app.deployed_links == 0;
                let unlink = ui
                    .add_enabled(!unlinked, egui::Button::new("1. Unlink all"))
                    .clicked();
                let verify = ui.button("2. Verify files in Steam").clicked();
                let refresh = ui
                    .add_enabled(unlinked, egui::Button::new("3. Refresh backup"))
                    .on_disabled_hover_text("Unlink all first")
                    .clicked();
                theme::badge(
                    ui,
                    if unlinked {
                        "LINKS REMOVED"
                    } else {
                        "LINKS DEPLOYED"
                    },
                    WARNING,
                );
                clicked([
                    (unlink, Action::Deploy(DeployAction::UnlinkAll)),
                    (verify, Action::Deploy(DeployAction::VerifyGame)),
                    (refresh, Action::Deploy(DeployAction::RefreshBackup)),
                ])
            })
            .inner
        })
        .inner
}
