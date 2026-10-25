//! Top bar: title, indicators, page tabs and the main action buttons.

use crate::launch::GameProcess;
use crate::steam::{AsiLoader, GameStatus};
use crate::ui::theme::{self, ACCENT, DANGER, MUTED, SUCCESS, WARNING};
use crate::ui::{Action, Confirm, PyluneApp, Tab};
use eframe::egui::{self, Color32, RichText};

fn game_badge(status: GameStatus, process: GameProcess) -> (&'static str, Color32) {
    match (status, process) {
        (_, GameProcess::Running) => ("GAME RUNNING", ACCENT),
        (GameStatus::Ready, _) => ("GAME FOUND", SUCCESS),
        (GameStatus::Unrecognised, _) => ("NOT CRIMSON DESERT", WARNING),
        (GameStatus::Missing, _) => ("GAME MISSING", DANGER),
    }
}

fn loader_badge(loader: AsiLoader) -> (&'static str, Color32) {
    match loader {
        AsiLoader::Winmm => ("ASI LOADER: winmm.dll", SUCCESS),
        AsiLoader::Version => ("ASI LOADER: version.dll", SUCCESS),
        AsiLoader::Dinput8 => ("ASI LOADER: dinput8.dll", SUCCESS),
        AsiLoader::Absent => ("NO ASI LOADER", WARNING),
    }
}

/// Keeps only the actions whose button was clicked.
fn clicked(pairs: impl IntoIterator<Item = (bool, Action)>) -> impl Iterator<Item = Action> {
    pairs.into_iter().filter(|(c, _)| *c).map(|(_, a)| a)
}

fn tabs(ui: &mut egui::Ui, current: Tab) -> Vec<Action> {
    let tab = |ui: &mut egui::Ui, tab: Tab, label: &str| {
        let text = RichText::new(label).strong().size(15.0);
        (
            ui.selectable_label(current == tab, text).clicked(),
            Action::SwitchTab(tab),
        )
    };
    let pairs = [
        tab(ui, Tab::Mods, "Mods"),
        tab(ui, Tab::Profiles, "Profiles"),
        tab(ui, Tab::Nexus, "Nexus"),
    ];
    clicked(pairs).collect()
}

/// Renders the header and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    let mut actions = Vec::new();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Pylune").size(24.0).strong().color(ACCENT));
        ui.label(RichText::new("Crimson Desert mod manager").color(MUTED));
        ui.add_space(12.0);
        let (text, color) = game_badge(app.game, app.process);
        theme::badge(ui, text, color);
        let (text, color) = loader_badge(app.loader);
        theme::badge(ui, text, color);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let running = app.process == GameProcess::Running;
            let play = ui
                .add_enabled_ui(!running && app.game == GameStatus::Ready, |ui| {
                    theme::filled_button(ui, "▶  PLAY", SUCCESS.linear_multiply(0.85))
                })
                .inner
                .on_hover_text("Launch through Steam (applies pending changes first if enabled)");
            actions.extend(clicked([(play.clicked(), Action::Launch)]));
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        actions.extend(tabs(ui, app.tab));
        ui.separator();
        let pairs = [
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
        actions.extend(clicked(pairs));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let reset = theme::filled_button(ui, "⚠ CLEAN SLATE", DANGER).on_hover_text(
                "Remove all links, restore .bak originals and set foreign overlays aside",
            );
            let unlink = ui.button("Unlink all");
            let fill = if app.dirty {
                ACCENT
            } else {
                ACCENT.linear_multiply(0.45)
            };
            let apply = theme::filled_button(ui, "✔ Apply", fill)
                .on_hover_text("Symlink all enabled mods into the game");
            let pairs = [
                (reset.clicked(), Action::Ask(Confirm::CleanSlate)),
                (unlink.clicked(), Action::UnlinkAll),
                (apply.clicked(), Action::Apply),
            ];
            actions.extend(clicked(pairs));
        });
    });
    ui.add_space(6.0);
    actions
}
