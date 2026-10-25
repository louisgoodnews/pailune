//! Profiles page: save, load, overwrite and delete installation sets.

use crate::profiles::Profile;
use crate::ui::theme::{self, ACCENT, MUTED};
use crate::ui::{Action, Confirm, ProfileAction, PyluneApp};
use eframe::egui::{self, RichText};

/// One profile row.
fn row(ui: &mut egui::Ui, profile: &Profile, active: bool) -> Vec<Action> {
    let marker = if active { "★ " } else { "" };
    ui.label(
        RichText::new(format!("{marker}{}", profile.name))
            .strong()
            .color(if active {
                ACCENT
            } else {
                ui.visuals().text_color()
            }),
    );
    let enabled = profile.slots.iter().filter(|s| s.enabled).count();
    ui.label(RichText::new(format!("{enabled}/{} enabled", profile.slots.len())).color(MUTED));
    let load = ui
        .button("Load")
        .on_hover_text("Replace the current selection and order")
        .clicked();
    let overwrite = ui
        .button("Overwrite")
        .on_hover_text("Store the current selection in this profile")
        .clicked();
    let delete = ui.button("🗑").clicked();
    ui.end_row();
    [
        (
            load,
            Action::Profile(ProfileAction::Load(profile.name.clone())),
        ),
        (
            overwrite,
            Action::Profile(ProfileAction::Save(profile.name.clone())),
        ),
        (
            delete,
            Action::Ask(Confirm::DeleteProfile(profile.name.clone())),
        ),
    ]
    .into_iter()
    .filter(|(c, _)| *c)
    .map(|(_, a)| a)
    .collect()
}

/// Renders the profiles page and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    ui.heading("Profiles");
    ui.label(
        RichText::new("A profile remembers which mods are enabled and their load order. Loading one marks changes — press Apply to deploy.")
            .small()
            .color(MUTED),
    );
    ui.separator();
    let save = ui
        .horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.profiles.name_draft)
                    .hint_text("New profile name"),
            );
            let valid = !app.profiles.name_draft.trim().is_empty();
            ui.add_enabled(valid, egui::Button::new("💾 Save current as"))
                .clicked()
        })
        .inner;
    let mut actions: Vec<Action> = save
        .then(|| Action::Profile(ProfileAction::Save(app.profiles.name_draft.clone())))
        .into_iter()
        .collect();
    ui.add_space(8.0);
    let book = &app.profiles.book;
    if book.profiles.is_empty() {
        ui.label(RichText::new("No profiles yet.").color(MUTED));
        return actions;
    }
    if let Some(active) = &book.active {
        theme::badge(ui, &format!("ACTIVE: {active}"), ACCENT);
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("profile_grid")
                .striped(true)
                .num_columns(5)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    let rows = book
                        .profiles
                        .iter()
                        .flat_map(|p| row(ui, p, book.active.as_ref() == Some(&p.name)))
                        .collect::<Vec<_>>();
                    actions.extend(rows);
                });
        });
    actions
}
