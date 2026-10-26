//! Toasts (top right, below the header) and the notification center.

use crate::logging::timestamp;
use crate::ui::notify::{NoticeAction, Notification, TOAST_TIME};
use crate::ui::theme::{self, muted};
use crate::ui::{Action, PyluneApp};
use eframe::egui::{self, RichText};
use std::time::Instant;

/// Width of a toast and of the center's entries.
const WIDTH: f32 = 320.0;

/// Title (in its level's colour), time (in the center) and text.
fn content(ui: &mut egui::Ui, n: &Notification, with_time: bool) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&n.title)
                .strong()
                .color(theme::level_color(n.level)),
        );
        if with_time {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} UTC", &timestamp(n.at)[11..16]))
                        .small()
                        .color(muted()),
                );
            });
        }
    });
    if !n.text.is_empty() {
        ui.label(RichText::new(&n.text).small());
    }
}

/// The current toasts; clicking one closes it.
pub fn toasts(ctx: &egui::Context, app: &PyluneApp) -> Vec<Action> {
    if !app.config.behavior.toasts || app.notices.open {
        return Vec::new();
    }
    let now = Instant::now();
    let shown = app.notices.toasts(now);
    if shown.is_empty() {
        return Vec::new();
    }
    // Wake up when the oldest shown toast should go, even if nothing moves.
    ctx.request_repaint_after(TOAST_TIME);
    egui::Area::new(egui::Id::unique("pylune_toasts"))
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 100.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            shown
                .iter()
                .filter_map(|n| {
                    let frame = egui::Frame::popup(ui.style())
                        .stroke(egui::Stroke::new(1.5, theme::level_color(n.level)))
                        .show(ui, |ui| {
                            ui.set_width(WIDTH);
                            content(ui, n, false);
                        });
                    let click = frame.response.interact(egui::Sense::click());
                    ui.add_space(6.0);
                    click
                        .on_hover_text("Click to close")
                        .clicked()
                        .then_some(Action::Notice(NoticeAction::Dismiss(n.id)))
                })
                .collect()
        })
        .inner
}

/// The notification center, while open.
pub fn center(ctx: &egui::Context, app: &PyluneApp) -> Vec<Action> {
    if !app.notices.open {
        return Vec::new();
    }
    let mut open = true;
    let mut actions = Vec::new();
    egui::Window::new("🔔 Notifications")
        .open(&mut open)
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 100.0])
        .default_height(420.0)
        .collapsible(false)
        .resizable(true)
        .show(ctx, |ui| {
            let items = &app.notices.items;
            if ui
                .add_enabled(!items.is_empty(), egui::Button::new("Clear all"))
                .clicked()
            {
                actions.push(Action::Notice(NoticeAction::Clear));
            }
            ui.separator();
            if items.is_empty() {
                ui.label(
                    RichText::new("Nothing yet. Installs, applies and errors show up here.")
                        .color(muted()),
                );
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                items.iter().rev().for_each(|n| {
                    ui.vertical(|ui| {
                        ui.set_width(WIDTH);
                        content(ui, n, true);
                    });
                    ui.separator();
                });
            });
        });
    if !open {
        actions.push(Action::Notice(NoticeAction::ToggleCenter));
    }
    actions
}
