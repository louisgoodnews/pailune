//! Light and dark palettes, appearance preferences and small reusable widgets.

use super::Level;
use crate::config::prefs::{Density, SCALE_RANGE, Theme, UiConfig};
use crate::mod_engine::model::ModKind;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};
use std::sync::atomic::{AtomicBool, Ordering};

/// The light palette is in use (set every frame by [`sync`]).
static LIGHT: AtomicBool = AtomicBool::new(false);

/// Picks the variant for the active palette. Both variants reach at least
/// 4.5:1 contrast on their panel colour (WCAG 2.2, R-24).
fn pick(dark: Color32, light: Color32) -> Color32 {
    match LIGHT.load(Ordering::Relaxed) {
        true => light,
        false => dark,
    }
}

/// Highlight colour for primary actions and links.
pub fn accent() -> Color32 {
    pick(rgb(0xd0, 0x8a, 0x3c), rgb(0x9a, 0x55, 0x10))
}

/// Destructive actions and errors.
pub fn danger() -> Color32 {
    pick(rgb(0xd9, 0x55, 0x55), rgb(0xb0, 0x26, 0x26))
}

/// Successful outcomes.
pub fn success() -> Color32 {
    pick(rgb(0x5f, 0xb3, 0x6b), rgb(0x2a, 0x77, 0x36))
}

/// Warnings and pending state.
pub fn warning() -> Color32 {
    pick(rgb(0xe0, 0xb3, 0x4a), rgb(0x87, 0x5c, 0x00))
}

/// Secondary text.
pub fn muted() -> Color32 {
    pick(rgb(0x9a, 0x9f, 0xa8), rgb(0x55, 0x5a, 0x62))
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn dark_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = rgb(0x17, 0x19, 0x1d);
    visuals.window_fill = rgb(0x1e, 0x21, 0x26);
    visuals.extreme_bg_color = rgb(0x10, 0x11, 0x14);
    visuals.selection.bg_fill = rgb(0xd0, 0x8a, 0x3c).linear_multiply(0.6);
    visuals.hyperlink_color = rgb(0xd0, 0x8a, 0x3c);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals
}

fn light_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = rgb(0xf6, 0xf5, 0xf2);
    visuals.window_fill = rgb(0xff, 0xff, 0xff);
    visuals.extreme_bg_color = rgb(0xff, 0xff, 0xff);
    visuals.selection.bg_fill = rgb(0xf0, 0xc8, 0x96);
    visuals.hyperlink_color = rgb(0x9a, 0x55, 0x10);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals
}

/// Installs both palettes and the preferred one, scale, spacing and motion.
pub fn apply(ctx: &egui::Context, prefs: &UiConfig) {
    ctx.set_visuals_of(egui::Theme::Dark, dark_visuals());
    ctx.set_visuals_of(egui::Theme::Light, light_visuals());
    ctx.set_theme(match prefs.theme {
        Theme::System => egui::ThemePreference::System,
        Theme::Dark => egui::ThemePreference::Dark,
        Theme::Light => egui::ThemePreference::Light,
    });
    ctx.set_zoom_factor(prefs.scale.clamp(*SCALE_RANGE.start(), *SCALE_RANGE.end()));
    let (gap, padding) = match prefs.density {
        Density::Compact => (egui::vec2(6.0, 3.0), egui::vec2(6.0, 2.0)),
        Density::Comfortable => (egui::vec2(8.0, 6.0), egui::vec2(10.0, 5.0)),
    };
    let animation = match prefs.reduced_motion {
        true => 0.0,
        false => 1.0 / 12.0,
    };
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = gap;
        style.spacing.button_padding = padding;
        style.animation_time = animation;
    });
    sync(ctx);
}

/// Matches the palette functions to the theme egui resolved (relevant for
/// [`Theme::System`]); call once per frame.
pub fn sync(ctx: &egui::Context) {
    LIGHT.store(!ctx.global_style().visuals.dark_mode, Ordering::Relaxed);
}

/// Colour associated with a status level.
pub fn level_color(level: Level) -> Color32 {
    match level {
        Level::Info => muted(),
        Level::Success => success(),
        Level::Warning => warning(),
        Level::Error => danger(),
    }
}

/// Label and colour for a mod category.
pub fn kind_style(kind: ModKind) -> (&'static str, Color32) {
    match kind {
        ModKind::Json => ("JSON", Color32::from_rgb(0x4a, 0x90, 0xd9)),
        ModKind::Raw => ("RAW", Color32::from_rgb(0x3f, 0xa8, 0x9a)),
        ModKind::Asi => ("ASI", Color32::from_rgb(0xa8, 0x6b, 0xd6)),
        ModKind::Overlay => ("OVERLAY", Color32::from_rgb(0x3f, 0xa8, 0x9a)),
        ModKind::Mixed => ("MIXED", accent()),
        ModKind::Empty => ("EMPTY", muted()),
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
