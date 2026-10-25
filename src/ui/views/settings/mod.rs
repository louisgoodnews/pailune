//! Settings page: a short list of categories on the left, one category's
//! options on the right, and a Save/Discard bar while the draft differs from
//! `config.toml` (ux-principles R-09, R-28, R-29).

mod diagnostics;
mod prefs;
mod system;

use super::clicked;
use crate::ui::theme::{self, accent, muted, warning};
use crate::ui::{Action, PyluneApp, SettingsDraft, SettingsPage};
use eframe::egui::{self, RichText};

/// Categories in display order, with their label and one-line purpose.
const PAGES: [(SettingsPage, &str, &str); 8] = [
    (
        SettingsPage::Appearance,
        "🎨 Appearance",
        "Theme, size and motion",
    ),
    (
        SettingsPage::Behavior,
        "⚙ Behavior",
        "Confirmations and start-up",
    ),
    (SettingsPage::Mods, "📦 Mods", "Installing and deleting"),
    (
        SettingsPage::Game,
        "🎮 Game & launch",
        "Game folder and Play",
    ),
    (
        SettingsPage::Nexus,
        "🌐 Nexus Mods",
        "Downloads and updates",
    ),
    (SettingsPage::Backups, "🗄 Backups", "Pylune's own state"),
    (SettingsPage::Advanced, "🔧 Advanced", "Jobs and logs"),
    (
        SettingsPage::Diagnostics,
        "🔬 Diagnostics",
        "Archives, Proton, paths",
    ),
];

/// One setting: a title, the control, and a one-line explanation below.
pub(super) fn setting(ui: &mut egui::Ui, title: &str, help: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(title).strong());
        add(ui);
    });
    ui.label(RichText::new(help).color(muted()));
}

/// The category list on the left.
fn category_list(ui: &mut egui::Ui, draft: &mut SettingsDraft) {
    ui.vertical(|ui| {
        ui.set_width(170.0);
        PAGES.iter().for_each(|(page, label, purpose)| {
            ui.selectable_value(&mut draft.page, *page, *label)
                .on_hover_text(*purpose);
        });
    });
}

/// "Unsaved changes" bar with the primary Save and a quieter Discard.
fn save_bar(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    if !app.draft.is_changed(&app.config) {
        return Vec::new();
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new("⏺ Unsaved changes").color(warning()));
        let save = theme::filled_button(ui, "Save", accent()).clicked();
        if ui.button("Discard").clicked() {
            let page = app.draft.page;
            app.draft = SettingsDraft::from_config(&app.config);
            app.draft.page = page;
        }
        clicked([(save, Action::SaveSettings)])
    })
    .inner
}

/// Heading, options and reset button of the selected category.
fn page(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    let page = app.draft.page;
    let (_, title, purpose) = PAGES
        .iter()
        .find(|(p, _, _)| *p == page)
        .copied()
        .unwrap_or(PAGES[0]);
    ui.heading(title);
    ui.label(RichText::new(purpose).color(muted()));
    let actions = match page {
        SettingsPage::Appearance => prefs::appearance(ui, &mut app.draft),
        SettingsPage::Behavior => prefs::behavior(ui, &mut app.draft),
        SettingsPage::Mods => prefs::mods(ui, &mut app.draft),
        SettingsPage::Backups => prefs::backups(ui, app),
        SettingsPage::Game => system::game(ui, &mut app.draft),
        SettingsPage::Nexus => system::nexus(ui, &mut app.draft),
        SettingsPage::Advanced => system::advanced(ui, &mut app.draft),
        SettingsPage::Diagnostics => diagnostics::show(ui, app),
    };
    if page != SettingsPage::Diagnostics {
        ui.add_space(16.0);
        if ui
            .small_button("Reset this page to defaults")
            .on_hover_text("Takes effect when you press Save")
            .clicked()
        {
            app.draft.reset(page);
        }
    }
    actions
}

/// Renders the settings page and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    let mut actions = save_bar(ui, app);
    ui.horizontal_top(|ui| {
        category_list(ui, &mut app.draft);
        ui.separator();
        let inner = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| ui.vertical(|ui| page(ui, app)).inner)
            .inner;
        actions.extend(inner);
    });
    actions
}
