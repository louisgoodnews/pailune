//! First-run dialog: choose the Crimson Desert folder.

use super::clicked;
use crate::ui::theme::{self, accent, muted};
use crate::ui::{Action, PyluneApp, SetupAction};
use eframe::egui::{self, RichText};

/// Renders the setup dialog and returns triggered actions.
pub fn show(ctx: &egui::Context, app: &mut PyluneApp) -> Vec<Action> {
    egui::Modal::new(egui::Id::unique("pylune_setup"))
        .show(ctx, |ui| {
            ui.set_max_width(520.0);
            ui.heading(RichText::new("Welcome to Pylune").color(accent()));
            ui.label(
                "Pylune could not find Crimson Desert in your Steam libraries. Choose the \
                 game folder — the one containing bin64/ and meta/.",
            );
            ui.add_space(8.0);
            let browse = ui
                .horizontal(|ui| {
                    let browse = ui.button("📁 Browse…").clicked();
                    ui.add(
                        egui::TextEdit::singleline(&mut app.draft.game_dir)
                            .hint_text(".../steamapps/common/Crimson Desert")
                            .desired_width(f32::INFINITY),
                    );
                    browse
                })
                .inner;
            ui.label(RichText::new(&app.status.text).small().color(muted()));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let confirm = theme::filled_button(ui, "Use this folder", accent()).clicked();
                let detect = ui.button("🔍 Search Steam again").clicked();
                let later = ui.button("Later").clicked();
                clicked([
                    (browse, Action::BrowseGameDir),
                    (confirm, Action::Setup(SetupAction::Confirm)),
                    (detect, Action::Setup(SetupAction::Detect)),
                    (later, Action::Setup(SetupAction::Later)),
                ])
            })
            .inner
        })
        .inner
}
