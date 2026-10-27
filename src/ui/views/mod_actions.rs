//! Action buttons of the details panel: enable, order (incl. move to a
//! load-order number), update, open folder and delete.

use super::clicked;
use crate::mod_engine::model::{ModEntry, MoveTo};
use crate::ui::theme::danger;
use crate::ui::{Action, Confirm, ModAction, PailuneApp};
use eframe::egui::{self, RichText};

/// Enable, order, update, open and delete buttons.
fn buttons(ui: &mut egui::Ui, entry: &ModEntry) -> Vec<Action> {
    let name = entry.name.clone();
    let mod_action = |a: ModAction| Action::Mod(a);
    let mut out = ui
        .horizontal_wrapped(|ui| {
            let toggle_text = if entry.enabled { "Disable" } else { "Enable" };
            clicked([
                (
                    ui.button(toggle_text).clicked(),
                    mod_action(ModAction::SetEnabled(name.clone(), !entry.enabled)),
                ),
                (
                    ui.button("⏫ Top").clicked(),
                    mod_action(ModAction::Move(name.clone(), MoveTo::Top)),
                ),
                (
                    ui.button("⏶").clicked(),
                    mod_action(ModAction::Move(name.clone(), MoveTo::Up)),
                ),
                (
                    ui.button("⏷").clicked(),
                    mod_action(ModAction::Move(name.clone(), MoveTo::Down)),
                ),
                (
                    ui.button("⏬ Bottom").clicked(),
                    mod_action(ModAction::Move(name.clone(), MoveTo::Bottom)),
                ),
            ])
        })
        .inner;
    out.extend(
        ui.horizontal_wrapped(|ui| {
            let update_hint = match entry.nexus {
                Some(_) => "Pick a newer file on Nexus",
                None => "Replace with another archive or file",
            };
            clicked([
                (
                    ui.button("⬆ Update…").on_hover_text(update_hint).clicked(),
                    mod_action(ModAction::Update(name.clone())),
                ),
                (
                    ui.button("📂 Open folder").clicked(),
                    mod_action(ModAction::OpenFolder(name.clone())),
                ),
                // Undoable, so a plain button, set apart from the others (R-04, R-06).
                (
                    {
                        ui.add_space(12.0);
                        ui.button(RichText::new("🗑 Delete").color(danger()))
                            .on_hover_text("Moves the mod to Recently deleted; Undo brings it back")
                            .clicked()
                    },
                    Action::Ask(Confirm::DeleteMod(name.clone())),
                ),
            ])
        })
        .inner,
    );
    out
}

/// "Move to number" field: type the load-order position and press Move
/// (B-15). The typed number lives in egui's memory per mod and starts at the
/// mod's current number.
fn position(ui: &mut egui::Ui, entry: &ModEntry, app: &PailuneApp) -> Vec<Action> {
    let count = app.registry.mods.len();
    let current = app
        .registry
        .mods
        .iter()
        .position(|m| m.name == entry.name)
        .map_or(1, |i| i + 1);
    let id = egui::Id::unique(("pailune_move_to", &entry.name, current));
    let mut number = ui.data(|d| d.get_temp(id)).unwrap_or(current);
    ui.horizontal(|ui| {
        ui.label("Move to number");
        let field = ui.add(egui::DragValue::new(&mut number).range(1..=count.max(1)));
        ui.data_mut(|d| d.insert_temp(id, number));
        let submit = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let moved = ui
            .add_enabled(number != current, egui::Button::new("Move"))
            .on_hover_text(format!("1 = first, {count} = last (wins conflicts)"))
            .clicked();
        clicked([(
            (moved || submit) && number != current,
            Action::Mod(ModAction::Move(
                entry.name.clone(),
                MoveTo::Position(number),
            )),
        )])
    })
    .inner
}

/// Renders the buttons and the position field.
pub fn show(ui: &mut egui::Ui, entry: &ModEntry, app: &PailuneApp) -> Vec<Action> {
    let mut actions = buttons(ui, entry);
    actions.extend(position(ui, entry, app));
    actions
}
