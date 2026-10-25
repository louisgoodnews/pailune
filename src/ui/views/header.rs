//! Top bar: title, game/loader indicators and the main action buttons.

use crate::steam::{AsiLoader, GameStatus};
use crate::ui::theme::{self, ACCENT, DANGER, MUTED, SUCCESS, WARNING};
use crate::ui::{Action, Confirm, PyluneApp};
use eframe::egui::{self, RichText};

fn game_badge(status: GameStatus) -> (&'static str, egui::Color32) {
    match status {
        GameStatus::Ready => ("GAME FOUND", SUCCESS),
        GameStatus::Unrecognised => ("NOT CRIMSON DESERT", WARNING),
        GameStatus::Missing => ("GAME MISSING", DANGER),
    }
}

fn loader_badge(loader: AsiLoader) -> (&'static str, egui::Color32) {
    match loader {
        AsiLoader::Winmm => ("ASI LOADER: winmm.dll", SUCCESS),
        AsiLoader::Version => ("ASI LOADER: version.dll", SUCCESS),
        AsiLoader::Dinput8 => ("ASI LOADER: dinput8.dll", SUCCESS),
        AsiLoader::Absent => ("NO ASI LOADER", WARNING),
    }
}

/// Renders the header and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    let mut actions = Vec::new();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Pylune").size(24.0).strong().color(ACCENT));
        ui.label(RichText::new("Crimson Desert mod manager").color(MUTED));
        ui.add_space(12.0);
        let (text, color) = game_badge(app.game);
        theme::badge(ui, text, color);
        let (text, color) = loader_badge(app.loader);
        theme::badge(ui, text, color);
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let clicked = [
            (
                ui.button("📁 Import folder…").clicked(),
                Action::ImportFolder,
            ),
            (
                ui.button("📦 Import file / zip…").clicked(),
                Action::ImportFile,
            ),
            (ui.button("⟳ Rescan").clicked(), Action::Rescan),
        ];
        actions.extend(clicked.into_iter().filter(|(c, _)| *c).map(|(_, a)| a));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let reset = theme::filled_button(ui, "⚠ CLEAN SLATE", DANGER).on_hover_text(
                "Remove all links, restore .bak originals and set foreign overlays aside",
            );
            let unlink = ui.button("Unlink all");
            let apply_fill = match app.dirty {
                true => ACCENT,
                false => ACCENT.linear_multiply(0.45),
            };
            let apply = theme::filled_button(ui, "✔ Apply", apply_fill)
                .on_hover_text("Symlink all enabled mods into the game");
            let clicked = [
                (reset.clicked(), Action::Ask(Confirm::CleanSlate)),
                (unlink.clicked(), Action::UnlinkAll),
                (apply.clicked(), Action::Apply),
            ];
            actions.extend(clicked.into_iter().filter(|(c, _)| *c).map(|(_, a)| a));
        });
    });
    ui.add_space(6.0);
    actions
}
