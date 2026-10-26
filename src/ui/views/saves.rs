//! Saves tab: the player's slots and save backups on the left, the open
//! save (viewer and editor, B-07) on the right.

use crate::ui::{Action, PyluneApp};
use eframe::egui;

/// Renders the tab and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    let mut actions = Vec::new();
    egui::Panel::left("saves_list")
        .resizable(true)
        .default_size(400.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| actions.extend(super::save_backups::show(ui, app)));
        });
    actions.extend(super::save_view::panel(ui, app));
    actions
}
