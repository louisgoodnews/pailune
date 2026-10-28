//! Read-only Game Data Browser view: table list, record search, and field inspector.

use crate::steam::GameStatus;
use crate::tables::browser::InspectedRecord;
use crate::ui::intent::DataBrowserAction;
use crate::ui::theme::{accent, muted, success, warning};
use crate::ui::{Action, PailuneApp};
use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};

/// Renders the Data Browser page.
pub fn show(ui: &mut egui::Ui, app: &PailuneApp) -> Vec<Action> {
    let mut actions = Vec::new();
    if app.game != GameStatus::Ready {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(RichText::new("Game installation not found").size(18.0));
            ui.label(
                RichText::new(
                    "The Data Browser reads data tables from Crimson Desert's archive group 0008. \
                     Configure the game location in Settings › Game.",
                )
                .color(muted()),
            );
            ui.add_space(10.0);
            if ui.button("⚙ Open Settings").clicked() {
                actions.push(Action::SwitchTab(crate::ui::Tab::Settings));
            }
        });
        return actions;
    }

    if !app.data_browser.loaded {
        actions.push(Action::DataBrowser(DataBrowserAction::RefreshTables));
        ui.centered_and_justified(|ui| {
            ui.label("Loading archive tables…");
        });
        return actions;
    }

    if let Some(err) = &app.data_browser.error {
        ui.label(RichText::new(format!("⚠ {err}")).color(warning()));
    }

    let available_width = ui.available_width();
    let col_w = (available_width / 3.0).max(240.0);

    ui.horizontal(|ui| {
        // Column 1: Tables List
        ui.allocate_ui(egui::vec2(col_w, ui.available_height()), |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("Tables");
                    let count = app.data_browser.tables.len();
                    ui.label(RichText::new(format!("({count})")).color(muted()));
                    if ui.button("⟳").on_hover_text("Refresh table list").clicked() {
                        actions.push(Action::DataBrowser(DataBrowserAction::RefreshTables));
                    }
                });
                ui.add(
                    egui::TextEdit::singleline(&mut app.data_browser.table_filter.clone())
                        .hint_text("Filter tables…")
                        .desired_width(ui.available_width()),
                );
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("db_tables_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let filter = app.data_browser.table_filter.to_lowercase();
                        for t in &app.data_browser.tables {
                            if !filter.is_empty() && !t.stem.to_lowercase().contains(&filter) {
                                continue;
                            }
                            let is_selected =
                                app.data_browser.selected_table.as_deref() == Some(&t.stem);
                            ui.horizontal(|ui| {
                                if t.has_layout {
                                    crate::ui::theme::badge(ui, "LAYOUT", success());
                                } else {
                                    crate::ui::theme::badge(ui, "RAW", muted());
                                }
                                let label = RichText::new(&t.stem);
                                let label = if is_selected {
                                    label.strong().color(accent())
                                } else {
                                    label
                                };
                                if ui.selectable_label(is_selected, label).clicked() {
                                    actions.push(Action::DataBrowser(
                                        DataBrowserAction::SelectTable(t.stem.clone()),
                                    ));
                                }
                            });
                        }
                    });
            });
        });

        ui.separator();

        // Column 2: Records List
        ui.allocate_ui(egui::vec2(col_w, ui.available_height()), |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("Records");
                    let count = app.data_browser.records.len();
                    ui.label(RichText::new(format!("({count})")).color(muted()));
                });
                ui.add(
                    egui::TextEdit::singleline(&mut app.data_browser.record_filter.clone())
                        .hint_text("Search name or key…")
                        .desired_width(ui.available_width()),
                );
                ui.separator();
                if app.data_browser.records.is_empty() {
                    ui.label(RichText::new("Select a table to view its records.").color(muted()));
                } else {
                    let filter = app.data_browser.record_filter.to_lowercase();
                    egui::ScrollArea::vertical()
                        .id_salt("db_records_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut shown = 0;
                            for r in &app.data_browser.records {
                                if !filter.is_empty() {
                                    let match_name = r.name.to_lowercase().contains(&filter);
                                    let match_key = r.key.to_string().contains(&filter);
                                    if !match_name && !match_key {
                                        continue;
                                    }
                                }
                                shown += 1;
                                if shown > 500 {
                                    ui.label(
                                        RichText::new("… type more to narrow results …")
                                            .small()
                                            .color(muted()),
                                    );
                                    break;
                                }
                                let is_selected = app.data_browser.selected_record == Some(r.key);
                                let name_text = match r.name.is_empty() {
                                    true => format!("Record #{}", r.key),
                                    false => r.name.clone(),
                                };
                                let text = format!("{}: {}", r.key, name_text);
                                let label = if is_selected {
                                    RichText::new(text).strong().color(accent())
                                } else {
                                    RichText::new(text)
                                };
                                if ui.selectable_label(is_selected, label).clicked() {
                                    actions.push(Action::DataBrowser(
                                        DataBrowserAction::SelectRecord(r.key),
                                    ));
                                }
                            }
                        });
                }
            });
        });

        ui.separator();

        // Column 3: Record Details / Inspector
        ui.allocate_ui(egui::vec2(ui.available_width(), ui.available_height()), |ui| {
            ui.vertical(|ui| {
                ui.heading("Inspector");
                ui.separator();
                match &app.data_browser.inspected {
                    Some(rec) => render_inspector(ui, rec),
                    None => {
                        ui.label(
                            RichText::new("Select a record to inspect its values.").color(muted()),
                        );
                    }
                }
            });
        });
    });

    actions
}

fn render_inspector(ui: &mut egui::Ui, rec: &InspectedRecord) {
    ui.label(
        RichText::new(format!("Record: {}", rec.name))
            .size(16.0)
            .strong()
            .color(accent()),
    );
    ui.label(
        RichText::new(format!(
            "Key: {} | Size: {} bytes",
            rec.key, rec.payload_size
        ))
        .small()
        .color(muted()),
    );
    ui.add_space(8.0);

    if !rec.fields.is_empty() {
        ui.label(RichText::new("Verified Fields").strong());
        egui::ScrollArea::vertical()
            .id_salt("inspector_fields_scroll")
            .show(ui, |ui| {
                TableBuilder::new(ui)
                    .id_salt("fields_table")
                    .striped(true)
                    .column(Column::initial(130.0).resizable(true))
                    .column(Column::initial(80.0).resizable(true))
                    .column(Column::remainder())
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("Field");
                        });
                        header.col(|ui| {
                            ui.strong("Type");
                        });
                        header.col(|ui| {
                            ui.strong("Value");
                        });
                    })
                    .body(|mut body| {
                        for f in &rec.fields {
                            body.row(18.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(RichText::new(&f.name).strong());
                                });
                                row.col(|ui| {
                                    ui.label(RichText::new(&f.meaning).small().color(muted()));
                                });
                                row.col(|ui| {
                                    ui.label(RichText::new(&f.value).color(accent()));
                                });
                            });
                        }
                    });
            });
        ui.add_space(8.0);
    } else {
        ui.label(
            RichText::new("No verified layout for this table yet. Showing raw payload bytes:")
                .small()
                .color(muted()),
        );
    }

    egui::CollapsingHeader::new(format!("Raw Payload ({} bytes)", rec.payload_size))
        .id_salt("raw_payload_collapsing")
        .default_open(rec.fields.is_empty())
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("raw_hex_scroll")
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut rec.raw_hex.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY),
                    );
                });
        });
}

