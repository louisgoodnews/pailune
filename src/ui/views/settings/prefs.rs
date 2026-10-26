//! Preference pages: appearance, behaviour, mods and backups.

use super::setting;
use crate::config::prefs::{Density, Theme};
use crate::ui::{Action, PyluneApp, SettingsDraft};
use eframe::egui;

/// Scale presets offered (a list instead of a slider, so the control does
/// not move under the pointer while the interface resizes).
const SCALES: [f32; 8] = [0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];

/// A scale factor as a percentage.
fn percent(scale: f32) -> String {
    format!("{:.0} %", scale * 100.0)
}

/// Theme, scale, density and motion; previewed live.
pub fn appearance(ui: &mut egui::Ui, draft: &mut SettingsDraft) -> Vec<Action> {
    let prefs = &mut draft.config.ui;
    ui.label("Changes are previewed right away; press Save to keep them.");
    setting(
        ui,
        "Theme",
        "System follows your desktop's light or dark mode.",
        |ui| {
            ui.selectable_value(&mut prefs.theme, Theme::System, "System");
            ui.selectable_value(&mut prefs.theme, Theme::Dark, "Dark");
            ui.selectable_value(&mut prefs.theme, Theme::Light, "Light");
        },
    );
    setting(ui, "Size", "Scales all text and controls.", |ui| {
        egui::ComboBox::from_id_salt("ui_scale")
            .selected_text(percent(prefs.scale))
            .show_ui(ui, |ui| {
                SCALES.iter().for_each(|s| {
                    ui.selectable_value(&mut prefs.scale, *s, percent(*s));
                });
            });
    });
    setting(ui, "Density", "Compact fits more mods on screen.", |ui| {
        ui.selectable_value(&mut prefs.density, Density::Compact, "Compact");
        ui.selectable_value(&mut prefs.density, Density::Comfortable, "Comfortable");
    });
    setting(
        ui,
        "Reduce motion",
        "Turns off animations such as sliding panels.",
        |ui| drop(ui.checkbox(&mut prefs.reduced_motion, "")),
    );
    Vec::new()
}

/// Confirmations, start-up checks and the guide.
pub fn behavior(ui: &mut egui::Ui, draft: &mut SettingsDraft) -> Vec<Action> {
    let prefs = &mut draft.config.behavior;
    setting(
        ui,
        "Ask before deleting a mod",
        "Off by default: deleted mods can be restored with Undo or from \"Recently deleted\".",
        |ui| drop(ui.checkbox(&mut prefs.confirm_deletes, "")),
    );
    setting(
        ui,
        "Check for mod updates at start",
        "Needs a Nexus API key; uses a few API requests per installed Nexus mod.",
        |ui| drop(ui.checkbox(&mut prefs.check_updates_on_start, "")),
    );
    setting(
        ui,
        "Show the getting-started guide",
        "Opens the guide on the next start. It is always available from the ❓ button.",
        |ui| drop(ui.checkbox(&mut prefs.show_guide, "")),
    );
    setting(
        ui,
        "Show pop-up notifications",
        "Briefly shows when mods are installed, applied or something failed. The 🔔 list in the header keeps them either way.",
        |ui| drop(ui.checkbox(&mut prefs.toasts, "")),
    );
    Vec::new()
}

/// Installing and deleting mods.
pub fn mods(ui: &mut egui::Ui, draft: &mut SettingsDraft) -> Vec<Action> {
    let prefs = &mut draft.config.mods;
    setting(
        ui,
        "Enable new mods",
        "Newly installed mods start enabled. Nothing reaches the game until you press Apply.",
        |ui| drop(ui.checkbox(&mut prefs.enable_new, "")),
    );
    setting(
        ui,
        "Keep deleted mods",
        "How many deleted mods stay restorable; older ones are removed for good.",
        |ui| drop(ui.add(egui::DragValue::new(&mut prefs.trash_keep).range(1..=200))),
    );
    Vec::new()
}

/// Automatic backups, then the list of existing backups.
pub fn backups(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    let prefs = &mut app.draft.config.backup;
    setting(
        ui,
        "Daily automatic backup",
        "Saves settings, profiles and the mod list once a day when Pylune starts.",
        |ui| drop(ui.checkbox(&mut prefs.auto_daily, "")),
    );
    setting(
        ui,
        "Automatic backups kept",
        "Older automatic backups are deleted; manual ones are always kept.",
        |ui| drop(ui.add(egui::DragValue::new(&mut prefs.keep_auto).range(1..=100))),
    );
    setting(
        ui,
        "Back up save games before Play",
        "Copies the game's save files each time you press Play (they are small).",
        |ui| drop(ui.checkbox(&mut prefs.saves_before_play, "")),
    );
    setting(
        ui,
        "Save backups kept",
        "Older automatic save backups are deleted; manual ones are always kept.",
        |ui| drop(ui.add(egui::DragValue::new(&mut prefs.keep_saves).range(1..=100))),
    );
    ui.add_space(12.0);
    let actions = crate::ui::views::backups::show(ui, app);
    ui.add_space(12.0);
    ui.label(
        egui::RichText::new("Save games, their backups and the save editor are on the Saves tab.")
            .color(crate::ui::theme::muted()),
    );
    actions
}
