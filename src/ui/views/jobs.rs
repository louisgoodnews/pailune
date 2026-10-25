//! Bottom panel listing running and recently finished background jobs.

use crate::ui::theme::{DANGER, MUTED, SUCCESS};
use crate::ui::{JobState, JobView, PyluneApp};
use eframe::egui::{self, RichText};

fn row(ui: &mut egui::Ui, job: &JobView) {
    ui.horizontal(|ui| {
        match job.state {
            JobState::Queued => drop(ui.label(RichText::new("⏸").color(MUTED))),
            JobState::Running => drop(ui.spinner()),
            JobState::Succeeded => drop(ui.label(RichText::new("✔").color(SUCCESS))),
            JobState::Failed => drop(ui.label(RichText::new("✖").color(DANGER))),
        }
        ui.label(&job.title);
        if let (JobState::Running, Some((done, total))) = (job.state, job.progress) {
            let fraction = total.map_or(0.0, |t| done as f32 / t.max(1) as f32);
            let text = match total {
                Some(t) if t > 10_000 => format!("{:.1} / {:.1} MiB", mib(done), mib(t)),
                Some(t) => format!("{done} / {t}"),
                None => format!("{:.1} MiB", mib(done)),
            };
            ui.add(
                egui::ProgressBar::new(fraction)
                    .text(text)
                    .desired_width(220.0)
                    .animate(total.is_none()),
            );
        }
    });
}

fn mib(bytes: u64) -> f32 {
    bytes as f32 / 1_048_576.0
}

/// Renders the jobs list (only shown while jobs exist).
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) {
    let running = app
        .jobs
        .iter()
        .filter(|j| matches!(j.state, JobState::Running | JobState::Queued))
        .count();
    ui.label(
        RichText::new(format!("Background jobs — {running} active"))
            .small()
            .color(MUTED),
    );
    egui::ScrollArea::vertical()
        .max_height(110.0)
        .show(ui, |ui| {
            app.jobs.iter().rev().for_each(|job| row(ui, job));
        });
}
