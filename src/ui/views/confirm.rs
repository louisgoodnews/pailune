//! Modal confirmation for destructive actions.

use crate::ui::theme::{self, danger};
use crate::ui::{Action, Confirm, DeployAction, ModAction, ProfileAction};
use eframe::egui::{self, RichText};

/// Title, explanation and the action to run on confirmation.
fn describe(confirm: &Confirm) -> (String, String, Action) {
    match confirm {
        Confirm::CleanSlate => (
            "Revert to a clean slate?".into(),
            "All Pylune symlinks are removed, shadowed originals and meta/ files are restored \
             from .bak, foreign overlay folders above 0035 are renamed to .bak and every mod is \
             disabled. Staged mods are kept."
                .into(),
            Action::Deploy(DeployAction::CleanSlate),
        ),
        Confirm::DeleteMod(name) => (
            format!("Delete '{name}'?"),
            "The mod moves to \"Recently deleted\" and can be restored from there (or with Undo). \
             If it is deployed, the remaining mods are redeployed."
                .into(),
            Action::Mod(ModAction::Delete(name.clone())),
        ),
        Confirm::UninstallCollection(slug) => (
            format!("Uninstall collection {slug}?"),
            "Its mods are deleted from staging, except those another collection also uses. \
             Deployed mods are redeployed without them."
                .into(),
            Action::Nexus(crate::ui::NexusAction::UninstallCollection(slug.clone())),
        ),
        Confirm::RestoreBackup(path) => (
            "Restore this backup?".into(),
            format!(
                "The mod list, profiles and settings are replaced with the backup's (mod files too, \
                 if it contains them; the current ones are kept aside). A safety backup of the \
                 current state is taken first.\n\n{}",
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
            Action::Backup(crate::ui::BackupAction::Restore(path.clone())),
        ),
        Confirm::DeleteProfile(name) => (
            format!("Delete profile '{name}'?"),
            "Only the profile is deleted; staged mods stay untouched.".into(),
            Action::Profile(ProfileAction::Delete(name.clone())),
        ),
    }
}

/// Shows the dialog if a confirmation is pending; returns the user's choice.
pub fn show(ctx: &egui::Context, pending: Option<&Confirm>) -> Vec<Action> {
    let Some(confirm) = pending else {
        return Vec::new();
    };
    let (title, body, action) = describe(confirm);
    let response = egui::Modal::new(egui::Id::unique("pylune_confirm")).show(ctx, |ui| {
        ui.set_max_width(380.0);
        ui.heading(title);
        ui.add_space(4.0);
        ui.label(RichText::new(body));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let yes = theme::filled_button(ui, "Confirm", danger()).clicked();
            let no = ui.button("Cancel").clicked();
            match (yes, no) {
                (true, _) => Some(action.clone()),
                (_, true) => Some(Action::Dismiss),
                _ => None,
            }
        })
        .inner
    });
    let close = response.should_close();
    match (response.inner, close) {
        (Some(chosen), _) => vec![chosen],
        (None, true) => vec![Action::Dismiss],
        (None, false) => Vec::new(),
    }
}
