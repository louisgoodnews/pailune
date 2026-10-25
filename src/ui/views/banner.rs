//! Banners below the header: guided recovery after a game update, and the
//! pending-changes notice (ux-principles R-01).

use super::clicked;
use crate::backup::MetaBackup;
use crate::launch::GameProcess;
use crate::ui::theme::{self, accent, muted, warning};
use crate::ui::{Action, DeployAction, PyluneApp};
use eframe::egui::{self, RichText};

/// Renders the banner that applies: game update first, else pending changes.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    match (app.backup.meta, app.dirty) {
        (MetaBackup::Outdated, _) => updated(ui, app),
        (_, true) => pending(ui, app),
        _ => Vec::new(),
    }
}

/// The mod list differs from what is linked into the game; Apply is offered
/// right where the user looks.
fn pending(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    let free = app.process == GameProcess::Stopped && !app.game_job_active();
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, accent()))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("⏺ Changes not applied")
                        .strong()
                        .color(accent()),
                );
                ui.label(RichText::new("The game still has your previous setup.").color(muted()));
                let apply = ui
                    .add_enabled(free, egui::Button::new("✔ Apply now"))
                    .on_disabled_hover_text("Wait until the game or the running job has finished")
                    .clicked();
                clicked([(apply, Action::Deploy(DeployAction::Apply))])
            })
            .inner
        })
        .inner
}

/// Recovery steps after the game changed under Pylune's backup.
fn updated(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    egui::Frame::new()
        .fill(warning().linear_multiply(0.12))
        .stroke(egui::Stroke::new(1.0, warning()))
        .corner_radius(6)
        .inner_margin(8)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Crimson Desert was updated since Pylune's backup")
                    .strong()
                    .color(warning()),
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
                    warning(),
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
