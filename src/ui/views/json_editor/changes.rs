//! The table of a JSON mod's changes: one row per change, its new bytes
//! editable as hex or as the number they encode; format-3 rows edit the
//! field's new value. Every edit rewrites the draft text at once.

use crate::patch::document::{self, Slot};
use crate::patch::{Anchor, Change, Edit, JsonMod};
use crate::tables::format3::Intent;
use crate::ui::theme::{danger, muted};
use eframe::egui::{self, RichText, TextStyle};
use serde_json::Value;

/// A text cell that keeps what is typed while it has focus, so half-typed
/// values (`-`, an odd hex digit) are not replaced by the stored value.
/// Returns the text when it changed.
fn cell(ui: &mut egui::Ui, id: egui::Id, current: &str, width: f32) -> Option<String> {
    let mut text: String = ui
        .data(|d| d.get_temp(id))
        .unwrap_or_else(|| current.to_string());
    let response = ui.add(
        egui::TextEdit::singleline(&mut text)
            .font(TextStyle::Monospace)
            .desired_width(width),
    );
    match response.has_focus() {
        true => ui.data_mut(|d| {
            d.insert_temp(id, text.clone());
        }),
        false => ui.data_mut(|d| d.remove::<String>(id)),
    }
    response.changed().then_some(text)
}

/// Where a change applies, as the mod author wrote it.
fn anchor(anchor: &Anchor) -> String {
    match anchor {
        Anchor::Entry { name, rel_offset } => format!("{name} +{rel_offset}"),
        Anchor::Offset(offset) => format!("@0x{offset:x}"),
    }
}

/// Applies `value` at `slot` to the draft, or shows why it cannot.
fn write(ui: &mut egui::Ui, text: &mut String, slot: Slot, value: Value) {
    match document::set(text, slot, value) {
        Ok(next) => *text = next,
        Err(err) => drop(ui.label(RichText::new(err).color(danger()))),
    }
}

/// A name with a small, muted second line under it, `width` wide (longer
/// text wraps).
fn two_lines(ui: &mut egui::Ui, main: &str, small: &str, width: f32) -> egui::Response {
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.label(main);
        ui.label(RichText::new(small).small().monospace().color(muted()));
    })
    .response
}

/// Column widths: what and where, original, new.
const WIDTHS: [f32; 3] = [170.0, 110.0, 120.0];

/// Writes a typed number as bytes of the same length, or says it can't.
fn write_number(ui: &mut egui::Ui, text: &mut String, slot: Slot, typed: &str, len: usize) {
    let bytes = typed
        .trim()
        .parse::<i128>()
        .ok()
        .and_then(|v| document::number_bytes(v, len));
    match bytes {
        Some(bytes) => write(ui, text, slot, Value::String(document::hex(&bytes))),
        None => drop(ui.label(RichText::new(format!("not a {len}-byte number")).color(danger()))),
    }
}

/// One byte-patch row: what and where, the original value, the new value
/// as a number (when the bytes are one) and as hex.
fn change_row(ui: &mut egui::Ui, id: egui::Id, text: &mut String, slot: Slot, change: &Change) {
    let (original, patched) = match &change.edit {
        Edit::Replace { original, patched } => (original.as_deref(), patched),
        Edit::Insert { bytes } => (None, bytes),
    };
    let label = match change.label.is_empty() {
        true => "(no label)",
        false => &change.label,
    };
    two_lines(ui, label, &anchor(&change.anchor), WIDTHS[0]);
    match original {
        Some(o) => drop(
            two_lines(
                ui,
                &document::number(o).map_or("—".into(), |n| n.to_string()),
                &format!("{} bytes", o.len()),
                WIDTHS[1],
            )
            .on_hover_text(document::hex(o)),
        ),
        None => drop(two_lines(ui, "—", "inserted", WIDTHS[1])),
    }
    ui.vertical(|ui| {
        if let Some(n) = document::number(patched)
            && let Some(typed) = cell(ui, id.with("num"), &n.to_string(), WIDTHS[2])
        {
            write_number(ui, text, slot, &typed, patched.len());
        }
        if let Some(hex) = cell(ui, id.with("hex"), &document::hex(patched), WIDTHS[2]) {
            write(ui, text, slot, Value::String(hex));
        }
    });
    ui.end_row();
}

/// One format-3 row: the new value is JSON (number, `true`, "text").
fn intent_row(ui: &mut egui::Ui, id: egui::Id, text: &mut String, slot: Slot, intent: &Intent) {
    let record = intent
        .entry
        .clone()
        .or(intent.key.map(|k| format!("key {k}")))
        .unwrap_or_default();
    two_lines(
        ui,
        &record,
        intent.field.as_deref().unwrap_or("raw bytes"),
        WIDTHS[0],
    );
    two_lines(
        ui,
        intent.old.as_deref().unwrap_or("—"),
        &intent.op,
        WIDTHS[1],
    );
    if let Some(typed) = cell(ui, id.with("new"), &intent.new.to_string(), WIDTHS[2]) {
        let value = serde_json::from_str(&typed).unwrap_or(Value::String(typed));
        write(ui, text, slot, value);
    }
    ui.end_row();
}

/// The table for every game file the mod patches.
pub fn show(ui: &mut egui::Ui, id: egui::Id, text: &mut String, json: &JsonMod) {
    for (p, patch) in json.patches.iter().enumerate() {
        ui.label(RichText::new(&patch.game_file).monospace().strong());
        egui::Grid::new(id.with(("grid", p)))
            .striped(true)
            .num_columns(3)
            .show(ui, |ui| {
                ["Change", "Original", "New"]
                    .into_iter()
                    .for_each(|h| drop(ui.label(RichText::new(h).color(muted()))));
                ui.end_row();
                patch.changes.iter().enumerate().for_each(|(c, change)| {
                    let slot = Slot::Patched {
                        patch: p,
                        change: c,
                    };
                    change_row(ui, id.with((p, c)), text, slot, change);
                });
                patch.fields.iter().enumerate().for_each(|(i, intent)| {
                    let slot = Slot::Intent {
                        target: p,
                        intent: i,
                    };
                    intent_row(ui, id.with((p, "f", i)), text, slot, intent);
                });
            });
        ui.add_space(6.0);
    }
}
