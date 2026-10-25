//! Review dialog for installing or updating a collection.

use super::clicked;
use crate::nexus::collection::Collection;
use crate::ui::theme::{self, accent, muted};
use crate::ui::{Action, NexusAction};
use eframe::egui::{self, RichText};

/// Shows the fetched collection with its members; optional ones can be
/// toggled. Nothing is downloaded before "Install".
pub fn show(
    ctx: &egui::Context,
    preview: Option<&Collection>,
    installed: Option<&Collection>,
) -> Vec<Action> {
    let Some(c) = preview else {
        return Vec::new();
    };
    let chosen: u64 = c
        .members
        .iter()
        .filter(|m| m.selected)
        .map(|m| m.size)
        .sum();
    egui::Modal::new(egui::Id::unique("pylune_collection"))
        .show(ctx, |ui| {
            ui.set_max_width(600.0);
            ui.heading(RichText::new(&c.name).color(accent()));
            let verb = match installed {
                Some(old) if old.revision == c.revision => {
                    format!("installed (revision {})", c.revision)
                }
                Some(old) => format!("update revision {} to {}", old.revision, c.revision),
                None => format!("revision {}", c.revision),
            };
            ui.label(
                RichText::new(format!(
                    "{verb} · {} mod(s) · {:.1} MB selected",
                    c.members.len(),
                    chosen as f64 / 1e6
                ))
                .color(muted()),
            );
            ui.label(
                RichText::new(
                    "Mods are installed in the curator's load order. Optional mods can be ticked.",
                )
                .small(),
            );
            ui.separator();
            let mut actions: Vec<Action> = egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    c.members
                        .iter()
                        .flat_map(|m| {
                            ui.horizontal(|ui| {
                                let mut selected = m.selected;
                                let toggled = ui
                                    .add_enabled(m.optional, egui::Checkbox::new(&mut selected, ""))
                                    .changed();
                                ui.label(
                                    RichText::new(format!("{:>2}. {}", m.position + 1, m.title))
                                        .small(),
                                );
                                if m.optional {
                                    theme::badge(ui, "OPTIONAL", muted());
                                }
                                clicked([(
                                    toggled,
                                    Action::Nexus(NexusAction::ToggleMember(m.position)),
                                )])
                            })
                            .inner
                        })
                        .collect()
                })
                .inner;
            ui.separator();
            ui.horizontal(|ui| {
                let label = if installed.is_some() {
                    "⬆ Update"
                } else {
                    "⬇ Install"
                };
                actions.extend(clicked([
                    (
                        theme::filled_button(ui, label, accent()).clicked(),
                        Action::Nexus(NexusAction::InstallCollection),
                    ),
                    (
                        ui.button("Close").clicked(),
                        Action::Nexus(NexusAction::ClosePreview),
                    ),
                ]));
            });
            actions
        })
        .inner
}
