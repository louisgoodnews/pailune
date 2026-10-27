//! Profiles page: installation sets with their own launch options.

use super::clicked;
use crate::profiles::Profile;
use crate::ui::theme::{self, accent, muted};
use crate::ui::{Action, Confirm, ModListAction, PailuneApp, ProfileAction};
use eframe::egui::{self, RichText};

/// One profile card.
fn card(ui: &mut egui::Ui, profile: &Profile, active: bool, draft: &mut String) -> Vec<Action> {
    let name = profile.name.clone();
    egui::Frame::group(ui.style())
        .show(ui, |ui| {
            let mut actions = ui
                .horizontal(|ui| {
                    let color = if active {
                        accent()
                    } else {
                        ui.visuals().text_color()
                    };
                    let marker = if active { "★ " } else { "" };
                    ui.label(
                        RichText::new(format!("{marker}{name}"))
                            .strong()
                            .size(15.0)
                            .color(color),
                    );
                    let enabled = profile.slots.iter().filter(|s| s.enabled).count();
                    ui.label(
                        RichText::new(format!("{enabled}/{} enabled", profile.slots.len()))
                            .color(muted()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        clicked([
                            (
                                ui.button("🗑").clicked(),
                                Action::Ask(Confirm::DeleteProfile(name.clone())),
                            ),
                            (
                                ui.button("Export…")
                                    .on_hover_text("Save as a .json file to share or back up")
                                    .clicked(),
                                Action::Profile(ProfileAction::Export(name.clone())),
                            ),
                            (
                                ui.button("Overwrite")
                                    .on_hover_text("Store the current selection in this profile")
                                    .clicked(),
                                Action::Profile(ProfileAction::Save(name.clone())),
                            ),
                            (
                                ui.button("Load")
                                    .on_hover_text("Replace the current selection and order")
                                    .clicked(),
                                Action::Profile(ProfileAction::Load(name.clone())),
                            ),
                        ])
                    })
                    .inner
                })
                .inner;
            let save = ui
                .horizontal(|ui| {
                    ui.label(RichText::new("Launch options").small().color(muted()));
                    ui.add(
                        egui::TextEdit::singleline(draft)
                            .hint_text("e.g. WINEDLLOVERRIDES=\"winmm=n,b\" %command% -dx12")
                            .desired_width(f32::INFINITY - 60.0),
                    );
                    let changed = draft.trim() != profile.launch_options.trim();
                    ui.add_enabled(changed, egui::Button::new("Save")).clicked()
                })
                .inner;
            actions.extend(clicked([(
                save,
                Action::Profile(ProfileAction::SaveOptions(name.clone())),
            )]));
            actions
        })
        .inner
}

/// Renders the profiles page and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &mut PailuneApp) -> Vec<Action> {
    ui.heading("Profiles");
    ui.label(
        RichText::new(
            "A profile remembers which mods are active, their load order and its own launch options. \
             Options with environment variables (KEY=value) start the game through Proton directly.",
        )
        .small()
        .color(muted()),
    );
    ui.separator();
    let save = ui
        .horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.profiles.name_draft)
                    .hint_text("New profile name"),
            );
            let valid = !app.profiles.name_draft.trim().is_empty();
            let save = ui
                .add_enabled(valid, egui::Button::new("💾 Save current as"))
                .clicked();
            let import = ui
                .button("📥 Import…")
                .on_hover_text("Load a profile from a .json file")
                .clicked();
            ui.separator();
            let list = clicked([
                (
                    ui.button("📤 Export mod list…")
                        .on_hover_text(
                            "Order, options and Nexus origins of every mod, to share or keep",
                        )
                        .clicked(),
                    Action::ModList(ModListAction::Export),
                ),
                (
                    ui.button("📥 Import mod list…")
                        .on_hover_text(
                            "Rebuild a setup from a mod list; missing mods can be downloaded",
                        )
                        .clicked(),
                    Action::ModList(ModListAction::Import),
                ),
            ]);
            (save, import, list)
        })
        .inner;
    let (save, import, list) = save;
    let mut actions = clicked([(import, Action::Profile(ProfileAction::Import))]);
    actions.extend(list);
    actions.extend(clicked([(
        save,
        Action::Profile(ProfileAction::Save(app.profiles.name_draft.clone())),
    )]));
    ui.add_space(8.0);
    if app.profiles.book.profiles.is_empty() {
        // Teach the next step (R-19).
        ui.label(RichText::new("No profiles yet").strong());
        ui.label(
            RichText::new(
                "A profile remembers which mods are enabled and their order. Type a name above \
                 and press \"Save current as\" to keep your current setup; load it later to switch \
                 back.",
            )
            .color(muted()),
        );
        return actions;
    }
    if let Some(active) = &app.profiles.book.active {
        theme::badge(ui, &format!("ACTIVE: {active}"), accent());
    }
    let book = app.profiles.book.clone();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            book.profiles.iter().for_each(|profile| {
                let draft = app
                    .profiles
                    .option_drafts
                    .entry(profile.name.clone())
                    .or_insert_with(|| profile.launch_options.clone());
                let active = book.active.as_ref() == Some(&profile.name);
                actions.extend(card(ui, profile, active, draft));
                ui.add_space(4.0);
            });
        });
    actions
}
