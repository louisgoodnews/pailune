//! Top bar: title, indicators, page tabs and the main action buttons.

use super::clicked;
use crate::launch::GameProcess;
use crate::steam::{AsiLoader, GameStatus};
use crate::ui::theme::{self, ACCENT, DANGER, MUTED, SUCCESS, WARNING};
use crate::ui::{Action, Confirm, DeployAction, PyluneApp, Tab};
use eframe::egui::{self, Color32, RichText};

const TABS: [(Tab, &str); 5] = [
    (Tab::Mods, "Mods"),
    (Tab::Profiles, "Profiles"),
    (Tab::Nexus, "Nexus"),
    (Tab::Log, "Log"),
    (Tab::Settings, "Settings"),
];

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
        if let Some(profile) = &app.profiles.book.active {
            theme::badge(ui, &format!("PROFILE: {profile}"), ACCENT);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let ready = app.process == GameProcess::Stopped && app.game == GameStatus::Ready;
            let play = ui
                .add_enabled_ui(ready && app.launch_after.is_none(), |ui| {
                    theme::filled_button(ui, "▶  PLAY", SUCCESS.linear_multiply(0.85))
                })
                .inner
                .on_hover_text("Launch with the active profile's options (applies pending changes first if enabled)");
            actions.extend(clicked([(play.clicked(), Action::Deploy(DeployAction::Launch))]));
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let tabs = TABS.map(|(tab, label)| {
            let text = RichText::new(label).strong().size(15.0);
            (
                ui.selectable_label(app.tab == tab, text).clicked(),
                Action::SwitchTab(tab),
            )
        });
        actions.extend(clicked(tabs));
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
            actions.extend(clicked([
                (reset.clicked(), Action::Ask(Confirm::CleanSlate)),
                (unlink.clicked(), Action::Deploy(DeployAction::UnlinkAll)),
                (apply.clicked(), Action::Deploy(DeployAction::Apply)),
            ]));
        });
    });
    ui.add_space(6.0);
    actions
}
