//! Modal confirmation for destructive actions.

use crate::mod_engine::model::Registry;
use crate::ui::theme::{self, DANGER};
use crate::ui::{Action, Confirm};
use eframe::egui::{self, RichText};

/// Title, explanation and the action to run on confirmation.
fn describe(confirm: Confirm, registry: &Registry) -> (String, String, Action) {
    match confirm {
        Confirm::CleanSlate => (
            "Revert to a clean slate?".into(),
            "All Pylune symlinks are removed, shadowed originals and meta/ files are restored \
             from .bak, foreign overlay folders above 0035 are renamed to .bak and every mod is \
             disabled. Staged mods are kept."
                .into(),
            Action::CleanSlate,
        ),
        Confirm::Remove(i) => {
            let name = registry.mods.get(i).map_or("?", |m| m.name.as_str());
            (
                format!("Delete '{name}'?"),
                "The mod's folder is deleted from the staging area. Deployed links are rebuilt."
                    .into(),
                Action::Remove(i),
            )
        }
    }
}

/// Shows the dialog if a confirmation is pending; returns the user's choice.
pub fn show(ctx: &egui::Context, pending: Option<Confirm>, registry: &Registry) -> Vec<Action> {
    let Some(confirm) = pending else {
        return Vec::new();
    };
    let (title, body, action) = describe(confirm, registry);
    let response = egui::Modal::new(egui::Id::unique("pylune_confirm")).show(ctx, |ui| {
        ui.set_max_width(380.0);
        ui.heading(title);
        ui.add_space(4.0);
        ui.label(RichText::new(body));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let yes = theme::filled_button(ui, "Confirm", DANGER).clicked();
            let no = ui.button("Cancel").clicked();
            match (yes, no) {
                (true, _) => Some(action.clone()),
                (_, true) => Some(Action::Dismiss),
                _ => None,
            }
        })
        .inner
    });
    let close = response.should_close();
    match (response.inner, close) {
        (Some(chosen), _) => vec![chosen],
        (None, true) => vec![Action::Dismiss],
        (None, false) => Vec::new(),
    }
}
