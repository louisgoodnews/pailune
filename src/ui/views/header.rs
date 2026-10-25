//! Top bar: title, indicators, page tabs and the main action buttons.

use super::clicked;
use crate::launch::GameProcess;
use crate::steam::{AsiLoader, GameStatus};
use crate::ui::guide::{GuideAction, GuidePage};
use crate::ui::theme::{self, accent, danger, muted, success, warning};
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
        (_, GameProcess::Running) => ("GAME RUNNING", accent()),
        (GameStatus::Ready, _) => ("GAME FOUND", success()),
        (GameStatus::Unrecognised, _) => ("NOT CRIMSON DESERT", warning()),
        (GameStatus::Missing, _) => ("GAME MISSING", danger()),
    }
}

fn loader_badge(loader: AsiLoader) -> (&'static str, Color32) {
    match loader {
        AsiLoader::Winmm => ("ASI LOADER: winmm.dll", success()),
        AsiLoader::Version => ("ASI LOADER: version.dll", success()),
        AsiLoader::Dinput8 => ("ASI LOADER: dinput8.dll", success()),
        AsiLoader::Absent => ("NO ASI LOADER", warning()),
    }
}

/// Renders the header and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    let mut actions = Vec::new();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Pylune").size(24.0).strong().color(accent()));
        ui.label(RichText::new("Crimson Desert mod manager").color(muted()));
        ui.add_space(12.0);
        let (text, color) = game_badge(app.game, app.process);
        theme::badge(ui, text, color);
        let (text, color) = loader_badge(app.loader);
        theme::badge(ui, text, color);
        if let Some(profile) = &app.profiles.book.active {
            theme::badge(ui, &format!("PROFILE: {profile}"), accent());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let ready = app.process == GameProcess::Stopped && app.game == GameStatus::Ready;
            let play = ui
                .add_enabled_ui(ready && app.launch_after.is_none(), |ui| {
                    theme::filled_button(ui, "▶  PLAY", success().linear_multiply(0.85))
                })
                .inner
                .on_hover_text("Launch with the active profile's options (applies pending changes first if enabled)");
            let guide = ui
                .button("❓ Guide")
                .on_hover_text("Getting started and how-to topics (F1)");
            actions.extend(clicked([
                (play.clicked(), Action::Deploy(DeployAction::Launch)),
                (
                    guide.clicked(),
                    Action::Guide(GuideAction::Open(GuidePage::GettingStarted)),
                ),
            ]));
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
            // Game-folder actions wait while the game runs or one is queued (A-22).
            let blocked = match (app.process, app.game_job_active()) {
                (GameProcess::Running, _) => Some("Close Crimson Desert first"),
                (_, true) => Some("A game-folder job is already running"),
                _ => None,
            };
            let free = blocked.is_none();
            let why = blocked.unwrap_or_default();
            let reset = ui
                .add_enabled_ui(free, |ui| {
                    theme::filled_button(ui, "⚠ CLEAN SLATE", danger())
                })
                .inner
                .on_hover_text(
                    "Remove all links, restore .bak originals and set foreign overlays aside",
                )
                .on_disabled_hover_text(why);
            let unlink = ui
                .add_enabled(free, egui::Button::new("Unlink all"))
                .on_disabled_hover_text(why);
            let fill = if app.dirty {
                accent()
            } else {
                accent().linear_multiply(0.45)
            };
            let apply = ui
                .add_enabled_ui(free, |ui| theme::filled_button(ui, "✔ Apply", fill))
                .inner
                .on_hover_text("Symlink all enabled mods into the game")
                .on_disabled_hover_text(why);
            actions.extend(clicked([
                (reset.clicked(), Action::Ask(Confirm::CleanSlate)),
                (unlink.clicked(), Action::Deploy(DeployAction::UnlinkAll)),
                (apply.clicked(), Action::Deploy(DeployAction::Apply)),
            ]));
        });
    });
    actions.extend(super::banner::show(ui, app));
    ui.add_space(6.0);
    actions
}
