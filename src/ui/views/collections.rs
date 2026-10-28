//! Collections section of the Nexus page.

use super::clicked;
use crate::nexus::collection::Collection;
use crate::ui::theme::{self, accent, muted, success, warning};
use crate::ui::{Action, Confirm, NexusAction, PailuneApp};
use eframe::egui::{self, RichText};

/// Wraps a Nexus intent.
fn nexus(action: NexusAction) -> Action {
    Action::Nexus(action)
}

/// One installed collection with its controls and pending downloads.
fn installed(ui: &mut egui::Ui, c: &Collection) -> Vec<Action> {
    let wanted: Vec<_> = c.members.iter().filter(|m| m.selected).collect();
    let done = wanted.iter().filter(|m| m.mod_name.is_some()).count();
    theme::card_frame(ui.visuals())
        .show(ui, |ui| {
            let mut actions = ui
                .horizontal(|ui| {
                    ui.label(RichText::new(&c.name).strong().color(accent()));
                    ui.label(
                        RichText::new(format!(
                            "revision {} · {done}/{} installed",
                            c.revision,
                            wanted.len()
                        ))
                        .color(muted()),
                    );
                    let color = if done == wanted.len() {
                        success()
                    } else {
                        warning()
                    };
                    theme::badge(
                        ui,
                        if done == wanted.len() {
                            "COMPLETE"
                        } else {
                            "PENDING"
                        },
                        color,
                    );
                    clicked([
                        (
                            ui.button("Enable").clicked(),
                            nexus(NexusAction::CollectionEnabled(c.slug.clone(), true)),
                        ),
                        (
                            ui.button("Disable").clicked(),
                            nexus(NexusAction::CollectionEnabled(c.slug.clone(), false)),
                        ),
                        (
                            ui.button("⬆ Update").clicked(),
                            nexus(NexusAction::UpdateCollection(c.slug.clone())),
                        ),
                        (
                            ui.button("🗑").clicked(),
                            Action::Ask(Confirm::UninstallCollection(c.slug.clone())),
                        ),
                    ])
                })
                .inner;
            let pending: Vec<_> = wanted.iter().filter(|m| m.mod_name.is_none()).collect();
            if !pending.is_empty() {
                egui::CollapsingHeader::new(format!("Waiting for {} download(s)", pending.len()))
                    .id_salt(("pending", &c.slug))
                    .show(ui, |ui| {
                        pending.iter().for_each(|m| {
                            ui.horizontal(|ui| {
                                let open = ui
                                    .small_button("🌐 Open")
                                    .on_hover_text("Click \"Mod Manager Download\" there")
                                    .clicked();
                                ui.label(RichText::new(&m.title).small());
                                actions.extend(clicked([(
                                    open,
                                    nexus(NexusAction::OpenMemberPage(m.mod_id, m.file_id)),
                                )]));
                            });
                        });
                    });
            }
            actions
        })
        .inner
}

/// Renders the collections section and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &mut PailuneApp) -> Vec<Action> {
    ui.heading("Collections");
    let mut actions = ui
        .horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.nexus.collection_draft)
                    .hint_text("Collection slug or URL"),
            );
            clicked([(
                ui.button("🔍 Fetch").clicked(),
                nexus(NexusAction::FetchCollection),
            )])
        })
        .inner;
    match app.nexus.collections.is_empty() {
        true => drop(
            ui.label(
                RichText::new("No collections installed.")
                    .small()
                    .color(muted()),
            ),
        ),
        false => app
            .nexus
            .collections
            .clone()
            .iter()
            .for_each(|c| actions.extend(installed(ui, c))),
    }
    actions
}
