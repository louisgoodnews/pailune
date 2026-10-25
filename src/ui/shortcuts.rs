//! Keyboard shortcuts and drag-and-drop import.

use super::guide::{GuideAction, GuidePage};
use super::{Action, Confirm, DeployAction, ModAction, PyluneApp, Tab};
use eframe::egui::{self, Key, Modifiers};

/// Id of the mod-list search box (focused by Ctrl+F).
pub fn search_id() -> egui::Id {
    egui::Id::unique("pylune_mod_search")
}

/// Reads shortcuts and dropped files for this frame.
pub fn collect(ctx: &egui::Context, app: &PyluneApp) -> Vec<Action> {
    let typing = ctx.memory(|m| m.focused().is_some());
    let (search, rescan, apply, import, delete, help) = ctx.input_mut(|i| {
        (
            i.consume_key(Modifiers::COMMAND, Key::F),
            i.consume_key(Modifiers::NONE, Key::F5),
            i.consume_key(Modifiers::COMMAND, Key::Enter),
            i.consume_key(Modifiers::COMMAND, Key::I),
            !typing && i.consume_key(Modifiers::NONE, Key::Delete),
            i.consume_key(Modifiers::NONE, Key::F1),
        )
    });
    if search {
        ctx.memory_mut(|m| m.request_focus(search_id()));
    }
    let dropped: Vec<_> = ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .map(|f| f.path().to_path_buf())
            .filter(|p| !p.as_os_str().is_empty())
            .collect()
    });
    let selected = app.mods.selected.clone().filter(|_| app.tab == Tab::Mods);
    [
        (search && app.tab != Tab::Mods, Action::SwitchTab(Tab::Mods)),
        (rescan, Action::Mod(ModAction::Rescan)),
        (apply, Action::Deploy(DeployAction::Apply)),
        (import, Action::Mod(ModAction::ImportFiles)),
        (
            help,
            Action::Guide(GuideAction::Open(GuidePage::GettingStarted)),
        ),
        (
            !dropped.is_empty(),
            Action::Mod(ModAction::ImportPaths(dropped)),
        ),
    ]
    .into_iter()
    .filter(|(hit, _)| *hit)
    .map(|(_, action)| action)
    .chain(
        selected
            .filter(|_| delete)
            .map(|name| Action::Ask(Confirm::DeleteMod(name))),
    )
    .collect()
}

/// Hint painted over the window while files are dragged over it.
pub fn drop_hint(ctx: &egui::Context) {
    if ctx.input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    egui::Area::new(egui::Id::unique("pylune_drop_hint"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.heading("Drop to import mods");
            });
        });
}
