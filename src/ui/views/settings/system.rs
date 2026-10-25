//! System pages: game and launch, Nexus Mods, and advanced options.

use super::setting;
use crate::ui::{Action, SettingsDraft, Tab};
use eframe::egui;

/// A single-line text field of fixed width.
fn text(ui: &mut egui::Ui, value: &mut String, hint: &str) {
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(320.0),
    );
}

/// Game folder, app id and launch behaviour.
pub fn game(ui: &mut egui::Ui, draft: &mut SettingsDraft) -> Vec<Action> {
    let mut browse = false;
    setting(
        ui,
        "Game folder",
        "The folder that contains bin64/ and meta/. Unlink all mods before changing it.",
        |ui| {
            text(
                ui,
                &mut draft.game_dir,
                "/…/steamapps/common/Crimson Desert",
            );
            browse = ui.button("Browse…").clicked();
        },
    );
    let config = &mut draft.config;
    setting(
        ui,
        "Steam app id",
        "Only change this for a different edition of the game.",
        |ui| drop(ui.add(egui::DragValue::new(&mut config.game.app_id))),
    );
    setting(
        ui,
        "Apply before Play",
        "Pending mod changes are applied automatically when you press Play.",
        |ui| drop(ui.checkbox(&mut config.launch.apply_before_launch, "")),
    );
    setting(
        ui,
        "Default launch options",
        "Steam-style options, e.g. WINEDLLOVERRIDES=\"version=n,b\" %command%. A profile's own options win.",
        |ui| text(ui, &mut config.launch.default_options, "%command%"),
    );
    match browse {
        true => vec![Action::BrowseGameDir],
        false => Vec::new(),
    }
}

/// Nexus game domain; the API key lives on the Nexus tab.
pub fn nexus(ui: &mut egui::Ui, draft: &mut SettingsDraft) -> Vec<Action> {
    setting(
        ui,
        "Game domain",
        "The part after nexusmods.com/ in the game's address.",
        |ui| text(ui, &mut draft.config.nexus.domain, "crimsondesert"),
    );
    ui.add_space(8.0);
    match ui.button("Set up the API key on the Nexus tab ›").clicked() {
        true => vec![Action::SwitchTab(Tab::Nexus)],
        false => Vec::new(),
    }
}

/// Background jobs and logging; both apply after a restart.
pub fn advanced(ui: &mut egui::Ui, draft: &mut SettingsDraft) -> Vec<Action> {
    let config = &mut draft.config;
    ui.label("These settings take effect after restarting Pylune.");
    setting(
        ui,
        "Parallel jobs",
        "Downloads and imports running at once. Game-folder changes always run one at a time.",
        |ui| drop(ui.add(egui::Slider::new(&mut config.jobs.max_parallel, 1..=16))),
    );
    setting(
        ui,
        "Log level",
        "info for normal use; debug or pylune=debug when reporting a problem.",
        |ui| text(ui, &mut config.logging.level, "info"),
    );
    setting(
        ui,
        "Days of logs kept",
        "One log file is written per day.",
        |ui| drop(ui.add(egui::DragValue::new(&mut config.logging.keep_days).range(1..=365))),
    );
    ui.add_space(8.0);
    match ui.button("Open log folder").clicked() {
        true => vec![Action::OpenLogFolder],
        false => Vec::new(),
    }
}
