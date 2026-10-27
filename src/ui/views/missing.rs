//! Window listing the mods of an imported mod list that are not installed,
//! with ways to get them (B-12). Not modal: users switch to the browser.

use super::clicked;
use crate::modlist::ListedMod;
use crate::ui::theme::muted;
use crate::ui::{Action, ModListAction, PailuneApp};
use eframe::egui::{self, RichText};

/// One missing mod: name, where it came from, and how to get it.
fn row(ui: &mut egui::Ui, index: usize, listed: &ListedMod) -> Vec<Action> {
    ui.label(RichText::new(&listed.name).strong());
    let open = match &listed.nexus {
        Some(n) => {
            ui.label(
                RichText::new(format!("Nexus mod {} · file {}", n.mod_id, n.file_id))
                    .color(muted()),
            );
            ui.button("🌐 Open on Nexus").clicked()
        }
        None => {
            let source = match listed.source.is_empty() {
                true => "unknown source".to_string(),
                false => listed.source.clone(),
            };
            ui.label(RichText::new(format!("Install manually: {source}")).color(muted()));
            false
        }
    };
    ui.end_row();
    clicked([(open, Action::ModList(ModListAction::OpenMissing(index)))])
}

/// Renders the window while an imported list has missing mods.
pub fn show(ctx: &egui::Context, app: &PailuneApp) -> Vec<Action> {
    let Some(import) = &app.mods.list_import else {
        return Vec::new();
    };
    let premium = app.nexus.user.as_ref().is_some_and(|u| u.is_premium);
    let from_nexus = import.missing.iter().filter(|m| m.nexus.is_some()).count();
    let mut open = true;
    let inner = egui::Window::new(format!("{} mod(s) from the list are missing", import.missing.len()))
        .open(&mut open)
        .default_width(520.0)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(
                "They are added in the list's order as soon as they are installed — from Nexus or by importing them yourself.",
            );
            ui.add_space(6.0);
            let mut actions = egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    egui::Grid::new("missing_mods")
                        .num_columns(3)
                        .striped(true)
                        .show(ui, |ui| {
                            import
                                .missing
                                .iter()
                                .enumerate()
                                .flat_map(|(i, m)| row(ui, i, m))
                                .collect::<Vec<_>>()
                        })
                        .inner
                })
                .inner;
            ui.add_space(6.0);
            let all = ui
                .add_enabled(
                    premium && from_nexus > 0,
                    egui::Button::new(format!("⬇ Download all {from_nexus} from Nexus")),
                )
                .on_disabled_hover_text("Needs a Premium account; otherwise open each page and use \"Mod manager download\"")
                .clicked();
            actions.extend(clicked([(all, Action::ModList(ModListAction::DownloadMissing))]));
            actions
        })
        .and_then(|r| r.inner)
        .unwrap_or_default();
    let mut actions = inner;
    actions.extend(clicked([(
        !open,
        Action::ModList(ModListAction::CloseMissing),
    )]));
    actions
}
