//! Nexus page: account, nxm handler, mod lookup and downloads.

use crate::nexus::{NexusFile, NexusMod};
use crate::ui::theme::{self, ACCENT, MUTED, SUCCESS, WARNING};
use crate::ui::{Action, NexusAction, PyluneApp};
use eframe::egui::{self, RichText};

const KEY_URL: &str = "https://www.nexusmods.com/users/myaccount?tab=api";

fn pick(pairs: impl IntoIterator<Item = (bool, NexusAction)>) -> Vec<Action> {
    pairs
        .into_iter()
        .filter(|(c, _)| *c)
        .map(|(_, a)| Action::Nexus(a))
        .collect()
}

fn account(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    let (text, color) = match &app.nexus.user {
        Some(user) if user.is_premium => (format!("{} · PREMIUM", user.name), SUCCESS),
        Some(user) => (
            format!("{} · FREE (use nxm:// downloads)", user.name),
            WARNING,
        ),
        None => ("NOT SIGNED IN".to_string(), MUTED),
    };
    ui.horizontal(|ui| {
        theme::badge(ui, &text, color);
        let limits = crate::nexus::limits::current();
        if let (Some(hourly), Some(daily)) = (limits.hourly, limits.daily) {
            ui.label(
                RichText::new(format!("API budget left: {hourly}/hour, {daily}/day"))
                    .small()
                    .color(MUTED),
            );
        }
    });
    let (save, validate) = ui
        .horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.nexus.key_draft)
                    .password(true)
                    .hint_text("Personal API key"),
            );
            let save = ui.button("Save key").clicked();
            let validate = ui.button("Validate").clicked();
            ui.hyperlink_to("Get key", KEY_URL);
            (save, validate)
        })
        .inner;
    match app.nexus.handler_registered {
        true => drop(ui.label(RichText::new("✔ Pylune handles nxm:// links").color(SUCCESS))),
        false => {
            drop(ui.label(RichText::new("nxm:// links currently go to another app").color(MUTED)))
        }
    }
    let register = !app.nexus.handler_registered
        && ui
            .button("Make Pylune the nxm:// handler")
            .on_hover_text(
                "Lets the website's \"Mod Manager Download\" button send files to Pylune",
            )
            .clicked();
    pick([
        (save, NexusAction::SaveKey),
        (validate, NexusAction::Validate),
        (register, NexusAction::RegisterHandler),
    ])
}

fn lookup(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.nexus.mod_id_draft)
                .hint_text("Mod id or nexusmods.com URL"),
        );
        let fetch = ui.button("🔍 Fetch").clicked();
        let open = ui.button("🌐 Open on Nexus").clicked();
        let updates = ui.button("⬆ Check updates").clicked();
        pick([
            (fetch, NexusAction::Fetch),
            (open, NexusAction::OpenPage),
            (updates, NexusAction::CheckUpdates),
        ])
    })
    .inner
}

fn file_row(ui: &mut egui::Ui, file: &NexusFile, can_download: bool) -> Option<Action> {
    ui.label(RichText::new(&file.name).strong());
    ui.label(&file.version);
    ui.label(RichText::new(file.category_name.as_deref().unwrap_or("-")).color(MUTED));
    ui.label(RichText::new(format!("{:.1} MB", file.size_kb as f32 / 1024.0)).color(MUTED));
    let clicked = ui
        .add_enabled(can_download, egui::Button::new("⬇ Download"))
        .on_disabled_hover_text("Free accounts: use \"Mod Manager Download\" on the website")
        .clicked();
    ui.end_row();
    clicked.then_some(Action::Nexus(NexusAction::Download(file.file_id)))
}

fn details(
    ui: &mut egui::Ui,
    info: &NexusMod,
    files: &[NexusFile],
    app: &PyluneApp,
) -> Vec<Action> {
    ui.label(RichText::new(&info.name).size(18.0).strong().color(ACCENT));
    ui.label(
        RichText::new(format!(
            "by {} · version {} · id {}",
            info.author, info.version, info.mod_id
        ))
        .color(MUTED),
    );
    ui.label(&info.summary);
    if let Some(target) = &app.nexus.replace_target {
        ui.label(RichText::new(format!("The next download replaces '{target}'")).color(WARNING));
    }
    let can_download = app.nexus.user.as_ref().is_none_or(|u| u.is_premium);
    egui::Grid::new("nexus_files")
        .striped(true)
        .num_columns(5)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            files
                .iter()
                .filter_map(|f| file_row(ui, f, can_download))
                .collect()
        })
        .inner
}

/// Renders the Nexus page and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    ui.heading("Nexus Mods");
    ui.label(
        RichText::new(format!("Game: nexusmods.com/{}", app.config.nexus.domain))
            .small()
            .color(MUTED),
    );
    ui.separator();
    let mut actions = account(ui, app);
    ui.separator();
    actions.extend(super::collections::show(ui, app));
    ui.separator();
    actions.extend(lookup(ui, app));
    ui.add_space(6.0);
    let browsing = app.nexus.browsing.clone();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| match &browsing {
            Some((info, files)) => actions.extend(details(ui, info, files, app)),
            None => drop(ui.label(RichText::new("Fetch a mod to see its files.").color(MUTED))),
        });
    actions
}
