//! Dark theme and small reusable widgets.

use super::Level;
use crate::mod_engine::model::ModKind;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};

pub const ACCENT: Color32 = Color32::from_rgb(0xd0, 0x8a, 0x3c);
pub const DANGER: Color32 = Color32::from_rgb(0xc4, 0x3d, 0x3d);
pub const SUCCESS: Color32 = Color32::from_rgb(0x5f, 0xb3, 0x6b);
pub const WARNING: Color32 = Color32::from_rgb(0xe0, 0xb3, 0x4a);
pub const MUTED: Color32 = Color32::from_rgb(0x8a, 0x8f, 0x98);
const PANEL: Color32 = Color32::from_rgb(0x17, 0x19, 0x1d);
const WINDOW: Color32 = Color32::from_rgb(0x1e, 0x21, 0x26);

/// Installs Pylune's dark visuals on the context.
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = PANEL;
    visuals.window_fill = WINDOW;
    visuals.extreme_bg_color = Color32::from_rgb(0x10, 0x11, 0x14);
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.6);
    visuals.hyperlink_color = ACCENT;
    visuals.window_corner_radius = CornerRadius::same(8);
    ctx.set_visuals(visuals);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
    });
}

/// Colour associated with a status level.
pub fn level_color(level: Level) -> Color32 {
    match level {
        Level::Info => MUTED,
        Level::Success => SUCCESS,
        Level::Warning => WARNING,
        Level::Error => DANGER,
    }
}

/// Label and colour for a mod category.
pub fn kind_style(kind: ModKind) -> (&'static str, Color32) {
    match kind {
        ModKind::Json => ("JSON · NEEDS PAZ", WARNING),
        ModKind::Raw => ("RAW · NEEDS PAZ", WARNING),
        ModKind::Asi => ("ASI", Color32::from_rgb(0xa8, 0x6b, 0xd6)),
        ModKind::Overlay => ("OVERLAY", Color32::from_rgb(0x3f, 0xa8, 0x9a)),
        ModKind::Mixed => ("MIXED", ACCENT),
        ModKind::Empty => ("EMPTY", MUTED),
    }
}

/// A small pill with coloured outline and text.
pub fn badge(ui: &mut egui::Ui, text: &str, color: Color32) -> egui::Response {
    egui::Frame::new()
        .stroke(Stroke::new(1.0, color))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(egui::Margin::symmetric(7, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(text).small().strong().color(color))
        })
        .response
}

/// A filled, prominent button.
pub fn filled_button(ui: &mut egui::Ui, text: &str, fill: Color32) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).strong().color(Color32::WHITE))
            .fill(fill)
            .min_size(egui::vec2(0.0, 30.0)),
    )
}
