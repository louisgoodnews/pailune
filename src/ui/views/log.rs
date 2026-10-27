//! Log page: the event journal stored in SQLite.

use super::clicked;
use crate::logging::{Event, Severity};
use crate::ui::theme::{danger, muted, warning};
use crate::ui::{Action, PailuneApp};
use eframe::egui::{self, RichText};

/// Colour of a journal severity.
fn severity_color(ui: &egui::Ui, severity: Severity) -> egui::Color32 {
    match severity {
        Severity::Info => ui.visuals().text_color(),
        Severity::Warn => warning(),
        Severity::Error => danger(),
    }
}

pub use crate::logging::timestamp;

/// One journal event.
fn row(ui: &mut egui::Ui, event: &Event) {
    ui.label(
        RichText::new(timestamp(event.ts))
            .small()
            .monospace()
            .color(muted()),
    );
    ui.label(RichText::new(event.kind.as_str()).small().strong());
    ui.label(RichText::new(event.mod_name.as_deref().unwrap_or("—")).small());
    ui.label(
        RichText::new(&event.message)
            .small()
            .color(severity_color(ui, event.severity)),
    );
    ui.end_row();
}

/// Renders the log page and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &mut PailuneApp) -> Vec<Action> {
    ui.heading("Log");
    let actions = ui
        .horizontal(|ui| {
            let selected = app
                .mods
                .selected
                .clone()
                .unwrap_or_else(|| "no mod selected".into());
            let filter = ui
                .checkbox(&mut app.log.only_selected, format!("Only {selected}"))
                .changed();
            clicked([
                (filter, Action::RefreshLog),
                (ui.button("⟳ Refresh").clicked(), Action::RefreshLog),
                (
                    ui.add_enabled(!app.log.events.is_empty(), egui::Button::new("📋 Copy log"))
                        .on_hover_text("Copy the entries shown here, e.g. for a bug report")
                        .clicked(),
                    Action::CopyLog,
                ),
                (
                    ui.button("📂 Open log folder").clicked(),
                    Action::OpenLogFolder,
                ),
            ])
        })
        .inner;
    ui.label(
        RichText::new(format!(
            "Times are UTC. Full logs: {} (daily files and logs/mods/<mod>.log)",
            app.paths.logs.display()
        ))
        .small()
        .color(muted()),
    );
    ui.separator();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("log_grid")
                .striped(true)
                .num_columns(4)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    app.log.events.iter().for_each(|e| row(ui, e));
                });
        });
    actions
}

#[cfg(test)]
mod tests {
    #[test]
    fn formats_timestamps() {
        assert_eq!(super::timestamp(0), "1970-01-01 00:00:00");
        assert_eq!(super::timestamp(1_792_113_598), "2026-10-16 01:19:58");
    }
}
