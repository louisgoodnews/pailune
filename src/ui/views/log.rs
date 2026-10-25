//! Log page: the event journal stored in SQLite.

use super::clicked;
use crate::logging::{Event, Severity};
use crate::ui::theme::{danger, muted, warning};
use crate::ui::{Action, PyluneApp};
use eframe::egui::{self, RichText};

fn severity_color(ui: &egui::Ui, severity: Severity) -> egui::Color32 {
    match severity {
        Severity::Info => ui.visuals().text_color(),
        Severity::Warn => warning(),
        Severity::Error => danger(),
    }
}

/// Formats a Unix timestamp as `YYYY-MM-DD HH:MM:SS` (UTC).
pub fn timestamp(ts: i64) -> String {
    let (days, secs) = (ts.div_euclid(86_400), ts.rem_euclid(86_400));
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

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
pub fn show(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
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
