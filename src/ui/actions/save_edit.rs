//! Editing a save from the viewer (B-07): typed values become checked
//! changes, written by an exclusive job after confirmation.

use crate::error::{PailuneError, Result};
use crate::jobs::JobKind;
use crate::savefile::edit::{SaveEdit, parse_value};
use crate::savefile::view::{Editable, Node};
use crate::ui::state::SaveView;
use crate::ui::{BackupAction, Confirm, Level, PailuneApp, StatusLine};
use std::path::{Path, PathBuf};

/// Every editable value of a tree, by body offset.
fn editables(nodes: &[Node], out: &mut Vec<Editable>) {
    nodes.iter().for_each(|n| {
        out.extend(n.edit.clone());
        editables(&n.children, out);
    });
}

/// The typed values of an open save as changes; values typed back to what
/// the save holds are left out. The first invalid value is the error.
pub fn changes(view: &SaveView) -> Result<Vec<SaveEdit>> {
    let mut values = Vec::new();
    editables(&view.tree, &mut values);
    view.edits
        .iter()
        .filter_map(|(at, text)| values.iter().find(|v| v.at == *at).map(|v| (v, text)))
        .map(|(value, text)| {
            let new = parse_value(&value.type_name, value.bytes.len(), text)
                .map_err(|e| PailuneError::Invalid(format!("{}: {e}", value.field)))?;
            Ok(SaveEdit {
                at: value.at,
                old: value.bytes.clone(),
                new,
                label: format!("{} = {}", value.field, text.trim()),
            })
        })
        .filter(|edit| edit.as_ref().map_or(true, |e| e.new != e.old))
        .collect()
}

impl PailuneApp {
    /// Opens a slot in the viewer (replacing what was open).
    pub(super) fn open_save(&mut self, dir: PathBuf) -> Result<Option<StatusLine>> {
        let tree = crate::savefile::slots::inspect(&dir)?;
        let slot = dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        self.save_view = Some(SaveView {
            slot,
            dir,
            tree,
            ..SaveView::default()
        });
        self.tab = crate::ui::Tab::Saves;
        Ok(None)
    }

    /// Save, confirm and discard in the viewer.
    pub(super) fn save_edit_action(&mut self, action: BackupAction) -> Result<Option<StatusLine>> {
        let Some(view) = self.save_view.as_mut() else {
            return Ok(None);
        };
        match action {
            BackupAction::SaveEdits => {
                let count = changes(view)?.len();
                match count {
                    0 => Ok(Some(StatusLine::new(Level::Info, "Nothing changed"))),
                    n => {
                        self.confirm = Some(Confirm::EditSave(n));
                        Ok(None)
                    }
                }
            }
            BackupAction::WriteSaveEdits => {
                self.confirm = None;
                if self.process == crate::launch::GameProcess::Running {
                    return Err(PailuneError::Invalid(
                        "close Crimson Desert before editing a save".into(),
                    ));
                }
                let edits = changes(view)?;
                let slot = view.dir.clone();
                self.submit(JobKind::EditSave { slot, edits });
                Ok(Some(StatusLine::new(Level::Info, "Writing the save…")))
            }
            _ => {
                view.edits.clear();
                Ok(None)
            }
        }
    }

    /// After a save was written: show it as it is now, edits cleared.
    pub(super) fn after_save_edit(&mut self, slot: &Path) {
        let open = self.save_view.as_ref().is_some_and(|v| v.dir == slot);
        if open && let Err(err) = self.open_save(slot.to_path_buf()) {
            self.status = StatusLine::new(Level::Error, err.to_string());
        }
    }
}
