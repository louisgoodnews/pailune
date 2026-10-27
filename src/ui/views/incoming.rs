//! Confirmation dialog for downloads requested by the browser (`nxm://`).

use super::clicked;
use crate::ui::theme::{self, accent, muted};
use crate::ui::{Action, Incoming, NexusAction};
use eframe::egui::{self, RichText};

/// Text for an incoming download, with names once they are known.
fn describe(incoming: &Incoming) -> (String, String) {
    let link = &incoming.link;
    match &incoming.summary {
        Some(s) => (
            s.mod_name.clone(),
            format!(
                "{} {} · {:.1} MB · mod {} file {}",
                s.file_name,
                s.version,
                s.size_kb as f32 / 1024.0,
                s.mod_id,
                s.file_id
            ),
        ),
        None => (
            format!("Nexus mod {}", link.mod_id),
            format!("file {} · looking up names…", link.file_id),
        ),
    }
}

/// Shows pending download requests; nothing is downloaded without a click.
pub fn show(ctx: &egui::Context, pending: &[Incoming]) -> Vec<Action> {
    if pending.is_empty() {
        return Vec::new();
    }
    egui::Modal::new(egui::Id::unique("pailune_incoming"))
        .show(ctx, |ui| {
            ui.set_max_width(520.0);
            ui.heading("Download from Nexus Mods?");
            ui.label(RichText::new("Your browser asked Pailune to download:").color(muted()));
            ui.add_space(6.0);
            pending
                .iter()
                .enumerate()
                .flat_map(|(index, incoming)| {
                    let (title, detail) = describe(incoming);
                    ui.label(RichText::new(title).strong().color(accent()));
                    ui.label(RichText::new(detail).small());
                    ui.horizontal(|ui| {
                        let accept = theme::filled_button(ui, "⬇ Download", accent()).clicked();
                        let reject = ui.button("Ignore").clicked();
                        ui.separator();
                        clicked([
                            (accept, Action::Nexus(NexusAction::AcceptIncoming(index))),
                            (reject, Action::Nexus(NexusAction::RejectIncoming(index))),
                        ])
                    })
                    .inner
                })
                .collect()
        })
        .inner
}
