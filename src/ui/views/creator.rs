//! Visual node-based mod creator canvas, toolbar and real-time diff preview.

use crate::ui::intent::{CreatorAction, DeployAction};
use crate::ui::theme::{self, accent, muted, success, warning};
use crate::ui::{Action, PailuneApp};
use crate::visual::compiler::compile;
use crate::visual::graph::{
    ItemMatchMode, MathOp, Node, NodeCategory, NodeType, SocketType,
};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, RichText, Stroke, Vec2};
use std::collections::HashMap;

/// Returns the color associated with a socket type.
fn socket_color(st: SocketType) -> Color32 {
    match st {
        SocketType::ItemRef => Color32::from_rgb(0x5a, 0x9c, 0xe0), // Blue
        SocketType::DropTableRef => Color32::from_rgb(0xb3, 0x7d, 0xde), // Purple
        SocketType::Number => Color32::from_rgb(0x5f, 0xb3, 0x6b),   // Green
        SocketType::Boolean => Color32::from_rgb(0xe0, 0xb3, 0x4a),  // Yellow
        SocketType::Text => Color32::from_rgb(0x9a, 0x9f, 0xa8),     // Grey
    }
}

/// Renders the complete Visual Creator page.
pub fn show(ui: &mut egui::Ui, app: &mut PailuneApp) -> Vec<Action> {
    let mut actions = Vec::new();

    // 1. Top Toolbar
    ui.horizontal(|ui| {
        ui.label(RichText::new("Visual Mod Creator").strong().size(18.0).color(accent()));
        ui.add_space(8.0);

        // Template selector
        egui::ComboBox::from_id_salt("template_select")
            .selected_text("Templates")
            .show_ui(ui, |ui| {
                if ui.selectable_label(false, "★ Mega Stacks (Arrows & Potions)").clicked() {
                    actions.push(Action::Creator(CreatorAction::LoadTemplate("mega_stacks".into())));
                }
                if ui.selectable_label(false, "★ Hunting & Animal Yields (100% Drops)").clicked() {
                    actions.push(Action::Creator(CreatorAction::LoadTemplate("loot_bounty".into())));
                }
                if ui.selectable_label(false, "★ Crate Coin Rewards (100 Coins)").clicked() {
                    actions.push(Action::Creator(CreatorAction::LoadTemplate("crate_coins".into())));
                }
            });

        // + Add Node Menu
        egui::ComboBox::from_id_salt("add_node_menu")
            .selected_text("+ Add Node")
            .show_ui(ui, |ui| {
                ui.label(RichText::new("SOURCES").small().strong().color(muted()));
                if ui.button("Item Query Node").clicked() {
                    let id = format!("item_{:x}", rand_id());
                    app.creator.graph.add_node(Node::new(
                        id,
                        "Item Query",
                        NodeType::ItemSource {
                            item_name: "Arrow".into(),
                            match_mode: ItemMatchMode::Contains,
                        },
                    ).with_pos(150.0 - app.creator.pan[0], 150.0 - app.creator.pan[1]));
                }
                if ui.button("Drop Set Query Node").clicked() {
                    let id = format!("drop_{:x}", rand_id());
                    app.creator.graph.add_node(Node::new(
                        id,
                        "Drop Set Query",
                        NodeType::DropSetSource {
                            table_name: "DropSet_Middle_Animal_Common".into(),
                            item_filter: None,
                        },
                    ).with_pos(150.0 - app.creator.pan[0], 150.0 - app.creator.pan[1]));
                }
                if ui.button("Number Constant").clicked() {
                    let id = format!("num_{:x}", rand_id());
                    app.creator.graph.add_node(Node::new(
                        id,
                        "Number Value",
                        NodeType::NumberConstant { value: 100.0 },
                    ).with_pos(150.0 - app.creator.pan[0], 150.0 - app.creator.pan[1]));
                }

                ui.separator();
                ui.label(RichText::new("OPERATORS").small().strong().color(muted()));
                if ui.button("Math Operation Node").clicked() {
                    let id = format!("math_{:x}", rand_id());
                    app.creator.graph.add_node(Node::new(
                        id,
                        "Math Multiplier",
                        NodeType::MathOperation {
                            op: MathOp::Multiply,
                            operand: 5.0,
                        },
                    ).with_pos(250.0 - app.creator.pan[0], 150.0 - app.creator.pan[1]));
                }

                ui.separator();
                ui.label(RichText::new("ACTIONS").small().strong().color(muted()));
                if ui.button("Set Stack Limit").clicked() {
                    let id = format!("act_stack_{:x}", rand_id());
                    app.creator.graph.add_node(Node::new(
                        id,
                        "Set Stack Limit",
                        NodeType::SetItemStackLimit { max_stack: 999 },
                    ).with_pos(450.0 - app.creator.pan[0], 150.0 - app.creator.pan[1]));
                }
                if ui.button("Set Drop Rate").clicked() {
                    let id = format!("act_rate_{:x}", rand_id());
                    app.creator.graph.add_node(Node::new(
                        id,
                        "Set Drop Rate",
                        NodeType::SetDropRate { rate_percent: 100.0 },
                    ).with_pos(450.0 - app.creator.pan[0], 150.0 - app.creator.pan[1]));
                }
                if ui.button("Set Loot Quantity").clicked() {
                    let id = format!("act_qty_{:x}", rand_id());
                    app.creator.graph.add_node(Node::new(
                        id,
                        "Set Loot Quantity",
                        NodeType::SetDropQuantity { min_amount: 10, max_amount: 30 },
                    ).with_pos(450.0 - app.creator.pan[0], 150.0 - app.creator.pan[1]));
                }
                if ui.button("Set Bounty Price").clicked() {
                    let id = format!("act_price_{:x}", rand_id());
                    app.creator.graph.add_node(Node::new(
                        id,
                        "Set Bounty Price",
                        NodeType::SetBountyPrice { price: 50000 },
                    ).with_pos(450.0 - app.creator.pan[0], 150.0 - app.creator.pan[1]));
                }

                ui.separator();
                ui.label(RichText::new("TARGET").small().strong().color(muted()));
                if ui.button("Mod Target Config").clicked() {
                    let id = format!("target_{:x}", rand_id());
                    app.creator.graph.add_node(Node::new(
                        id,
                        "Mod Configuration",
                        NodeType::ModTarget {
                            title: app.creator.graph.title.clone(),
                            author: "Pailune Creator".into(),
                            version: "1.0.0".into(),
                            description: "Custom visual mod".into(),
                        },
                    ).with_pos(700.0 - app.creator.pan[0], 150.0 - app.creator.pan[1]));
                }
            });

        ui.add_space(12.0);

        // Apply & Deploy button
        if theme::filled_button(ui, "⚡ Apply & Deploy", accent()).clicked() {
            actions.push(Action::Creator(CreatorAction::ApplyGraph));
        }

        // Test button
        if ui.button("▶ Test (Launch)").clicked() {
            actions.push(Action::Creator(CreatorAction::ApplyGraph));
            actions.push(Action::Deploy(DeployAction::Launch));
        }

        // Export ZIP button
        if ui.button("📦 Export to ZIP").clicked() {
            actions.push(Action::Creator(CreatorAction::ExportZip));
        }

        ui.checkbox(&mut app.creator.show_diff_preview, "Diff Preview");
        ui.label(RichText::new(format!("{:.0}%", app.creator.zoom * 100.0)).color(muted()));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🗑 Clear Canvas").clicked() {
                actions.push(Action::Creator(CreatorAction::ClearGraph));
            }
        });
    });

    if let Some((msg, level)) = &app.creator.status_feedback {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(msg).color(theme::level_color(*level)));
        });
    }

    ui.separator();

    // 2. Main Area: Left Canvas + Optional Right/Bottom Diff Preview
    let preview_height = if app.creator.show_diff_preview { 160.0 } else { 0.0 };
    let canvas_height = (ui.available_height() - preview_height - 10.0).max(200.0);

    // Canvas Container
    ui.allocate_ui(egui::vec2(ui.available_width(), canvas_height), |ui| {
        render_canvas(ui, app, &mut actions);
    });

    // 3. Diff Preview Panel
    if app.creator.show_diff_preview {
        ui.separator();
        render_diff_preview(ui, app);
    }

    actions
}

/// Renders the pannable, zoomable node graph canvas.
fn render_canvas(ui: &mut egui::Ui, app: &mut PailuneApp, actions: &mut Vec<Action>) {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    // Canvas background
    painter.rect_filled(rect, CornerRadius::ZERO, theme::card_frame(&ui.visuals()).fill);

    // Panning & Zooming
    let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
    if response.hovered() && scroll_delta.abs() > 0.0 {
        let factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
        app.creator.zoom = (app.creator.zoom * factor).clamp(0.5, 2.0);
    }
    if response.dragged_by(egui::PointerButton::Primary) && app.creator.dragging_node.is_none() {
        app.creator.pan[0] += response.drag_delta().x;
        app.creator.pan[1] += response.drag_delta().y;
    }
    if response.dragged_by(egui::PointerButton::Middle) {
        app.creator.pan[0] += response.drag_delta().x;
        app.creator.pan[1] += response.drag_delta().y;
    }

    let origin = rect.min + Vec2::new(app.creator.pan[0], app.creator.pan[1]);

    // Draw subtle grid lines
    let grid_size = 32.0;
    let stroke = Stroke::new(1.0, Color32::from_white_alpha(8));
    let x_start = (origin.x % grid_size).floor();
    let mut x = x_start;
    while x < rect.max.x {
        if x >= rect.min.x {
            painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], stroke);
        }
        x += grid_size;
    }
    let y_start = (origin.y % grid_size).floor();
    let mut y = y_start;
    while y < rect.max.y {
        if y >= rect.min.y {
            painter.line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)], stroke);
        }
        y += grid_size;
    }

    // Collect socket positions for wire rendering
    let mut input_socket_positions: HashMap<(String, String), Pos2> = HashMap::new();
    let mut output_socket_positions: HashMap<(String, String), Pos2> = HashMap::new();

    let node_width = 240.0;
    let mut nodes_to_delete: Vec<String> = Vec::new();
    let mut connections_to_add: Vec<(String, String, String, String)> = Vec::new();
    let mut clear_connecting_socket = false;
    let mut new_connecting_socket = None;
    let pointer_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);

    // Cancel wire dragging on right click
    if ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary)) {
        app.creator.connecting_socket = None;
    }

    // Render Nodes
    for node in &mut app.creator.graph.nodes {
        let node_min = origin + Vec2::new(node.pos[0], node.pos[1]);
        let estimated_height = 80.0 + (node.inputs.len().max(node.outputs.len()) as f32) * 26.0 + 30.0;
        let node_rect = Rect::from_min_size(node_min, egui::vec2(node_width, estimated_height));

        // Node card background
        let is_selected = app.creator.selected_node.as_deref() == Some(&node.id);
        let border_stroke = if is_selected {
            Stroke::new(2.0, accent())
        } else {
            Stroke::new(1.0, Color32::from_white_alpha(30))
        };

        painter.rect(
            node_rect,
            CornerRadius::same(6),
            Color32::from_rgb(0x1e, 0x22, 0x2a),
            border_stroke,
            egui::StrokeKind::Inside,
        );

        // Header bar
        let header_rect = Rect::from_min_size(node_rect.min, egui::vec2(node_width, 28.0));
        let header_bg = match node.category {
            NodeCategory::Source => Color32::from_rgb(0x28, 0x34, 0x44),
            NodeCategory::Operator => Color32::from_rgb(0x28, 0x40, 0x30),
            NodeCategory::Action => Color32::from_rgb(0x44, 0x34, 0x28),
            NodeCategory::Target => Color32::from_rgb(0x3e, 0x28, 0x44),
        };
        painter.rect_filled(header_rect, CornerRadius::same(6), header_bg);
        painter.text(
            header_rect.min + Vec2::new(8.0, 14.0),
            egui::Align2::LEFT_CENTER,
            &node.title,
            egui::FontId::proportional(13.0),
            Color32::WHITE,
        );

        // Header delete button
        let del_rect = Rect::from_min_size(header_rect.max - Vec2::new(22.0, 22.0), egui::vec2(18.0, 18.0));
        if ui.put(del_rect, egui::Button::new("🗙").small()).clicked() {
            nodes_to_delete.push(node.id.clone());
        }

        // Drag node header
        let header_resp = ui.allocate_rect(header_rect, egui::Sense::drag());
        if header_resp.dragged() {
            node.pos[0] += header_resp.drag_delta().x;
            node.pos[1] += header_resp.drag_delta().y;
            app.creator.selected_node = Some(node.id.clone());
            app.creator.dragging_node = Some(node.id.clone());
        }

        // Node Inner UI Controls
        let content_rect = Rect::from_min_max(
            node_rect.min + Vec2::new(14.0, 32.0),
            node_rect.max - Vec2::new(14.0, 8.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
            render_node_controls(ui, node);
        });

        // Input sockets (Left border)
        for (i, sock) in node.inputs.iter().enumerate() {
            let sock_pos = node_rect.left_top() + Vec2::new(0.0, 42.0 + (i as f32) * 24.0);
            input_socket_positions.insert((node.id.clone(), sock.id.clone()), sock_pos);

            let color = socket_color(sock.socket_type);
            painter.circle_filled(sock_pos, 5.0, color);
            painter.circle_stroke(sock_pos, 5.0, Stroke::new(1.0, Color32::BLACK));

            // Socket hit test
            let sock_hit = Rect::from_center_size(sock_pos, egui::vec2(14.0, 14.0));
            if sock_hit.contains(pointer_pos) {
                painter.circle_stroke(sock_pos, 8.0, Stroke::new(1.5, Color32::WHITE));
                if ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary)) {
                    if let Some((from_node, from_sock, is_output)) = &app.creator.connecting_socket {
                        if *is_output && from_node != &node.id {
                            connections_to_add.push((from_node.clone(), from_sock.clone(), node.id.clone(), sock.id.clone()));
                        }
                        clear_connecting_socket = true;
                    }
                }
            }
        }

        // Output sockets (Right border)
        for (i, sock) in node.outputs.iter().enumerate() {
            let sock_pos = node_rect.right_top() + Vec2::new(0.0, 42.0 + (i as f32) * 24.0);
            output_socket_positions.insert((node.id.clone(), sock.id.clone()), sock_pos);

            let color = socket_color(sock.socket_type);
            painter.circle_filled(sock_pos, 5.0, color);
            painter.circle_stroke(sock_pos, 5.0, Stroke::new(1.0, Color32::BLACK));

            // Socket hit test
            let sock_hit = Rect::from_center_size(sock_pos, egui::vec2(14.0, 14.0));
            if sock_hit.contains(pointer_pos) {
                painter.circle_stroke(sock_pos, 8.0, Stroke::new(1.5, Color32::WHITE));
                if ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary)) {
                    new_connecting_socket = Some((node.id.clone(), sock.id.clone(), true));
                }
            }
        }
    }

    for (from_node, from_sock, to_node, to_sock) in connections_to_add {
        let _ = app.creator.graph.connect(&from_node, &from_sock, &to_node, &to_sock);
    }
    if clear_connecting_socket {
        app.creator.connecting_socket = None;
    }
    if let Some(conn) = new_connecting_socket {
        app.creator.connecting_socket = Some(conn);
    }

    if response.drag_stopped() {
        app.creator.dragging_node = None;
    }

    // Delete requested nodes
    for id in nodes_to_delete {
        actions.push(Action::Creator(CreatorAction::DeleteNode(id)));
    }

    // Draw Wires (Cubic Bezier curves)
    for conn in &app.creator.graph.connections {
        let p1 = output_socket_positions.get(&(conn.from_node.clone(), conn.from_socket.clone()));
        let p2 = input_socket_positions.get(&(conn.to_node.clone(), conn.to_socket.clone()));

        if let (Some(&p1), Some(&p2)) = (p1, p2) {
            let dx = (p2.x - p1.x).abs();
            let offset_x = (dx * 0.5).max(40.0);
            let c1 = p1 + Vec2::new(offset_x, 0.0);
            let c2 = p2 - Vec2::new(offset_x, 0.0);

            let color = Color32::from_rgb(0xd0, 0x8a, 0x3c);
            let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                [p1, c1, c2, p2],
                false,
                Color32::TRANSPARENT,
                Stroke::new(2.5, color),
            );
            painter.add(shape);
        }
    }

    // Draw Active Connecting Wire
    if let Some((from_node, from_sock, is_out)) = &app.creator.connecting_socket {
        if *is_out {
            if let Some(&p1) = output_socket_positions.get(&(from_node.clone(), from_sock.clone())) {
                let p2 = pointer_pos;
                let dx = (p2.x - p1.x).abs();
                let offset_x = (dx * 0.5).max(40.0);
                let c1 = p1 + Vec2::new(offset_x, 0.0);
                let c2 = p2 - Vec2::new(offset_x, 0.0);

                let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                    [p1, c1, c2, p2],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(2.0, Color32::WHITE),
                );
                painter.add(shape);
            }
        }
    }
}

/// Renders specific parameter widgets inside a node frame.
fn render_node_controls(ui: &mut egui::Ui, node: &mut Node) {
    match &mut node.node_type {
        NodeType::ItemSource { item_name, match_mode } => {
            ui.label(RichText::new("Item Name:").small());
            ui.text_edit_singleline(item_name);
            egui::ComboBox::from_id_salt(format!("match_{}", node.id))
                .selected_text(match match_mode {
                    ItemMatchMode::Exact => "Exact Match",
                    ItemMatchMode::Contains => "Contains",
                    ItemMatchMode::All => "All Items",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(match_mode, ItemMatchMode::Exact, "Exact Match");
                    ui.selectable_value(match_mode, ItemMatchMode::Contains, "Contains");
                    ui.selectable_value(match_mode, ItemMatchMode::All, "All Items");
                });
        }
        NodeType::DropSetSource { table_name, .. } => {
            ui.label(RichText::new("Drop Set Table:").small());
            ui.text_edit_singleline(table_name);
        }
        NodeType::WantedPriceSource { record_name } => {
            ui.label(RichText::new("Wanted Entry:").small());
            ui.text_edit_singleline(record_name);
        }
        NodeType::NumberConstant { value } => {
            ui.label(RichText::new("Value:").small());
            ui.add(egui::DragValue::new(value).speed(1.0));
        }
        NodeType::MathOperation { op, operand } => {
            egui::ComboBox::from_id_salt(format!("op_{}", node.id))
                .selected_text(format!("{op:?}"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(op, MathOp::Multiply, "Multiply (x)");
                    ui.selectable_value(op, MathOp::Add, "Add (+)");
                    ui.selectable_value(op, MathOp::Subtract, "Subtract (-)");
                    ui.selectable_value(op, MathOp::ClampMin, "Clamp Minimum");
                    ui.selectable_value(op, MathOp::ClampMax, "Clamp Maximum");
                });
            ui.add(egui::DragValue::new(operand).speed(0.5));
        }
        NodeType::SetItemStackLimit { max_stack } => {
            ui.label(RichText::new("Max Stack Limit:").small());
            ui.add(egui::Slider::new(max_stack, 1..=99999).logarithmic(true));
        }
        NodeType::SetDropRate { rate_percent } => {
            ui.label(RichText::new("Drop Chance:").small());
            ui.add(egui::Slider::new(rate_percent, 0.0..=100.0).suffix("%"));
        }
        NodeType::SetDropQuantity { min_amount, max_amount } => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Min:").small());
                ui.add(egui::DragValue::new(min_amount).speed(1.0));
                ui.label(RichText::new("Max:").small());
                ui.add(egui::DragValue::new(max_amount).speed(1.0));
            });
        }
        NodeType::SetBountyPrice { price } => {
            ui.label(RichText::new("Reward Coins:").small());
            ui.add(egui::DragValue::new(price).speed(100.0));
        }
        NodeType::ModTarget { title, version, author, description } => {
            ui.label(RichText::new("Mod Title:").small());
            ui.text_edit_singleline(title);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Ver:").small());
                ui.text_edit_singleline(version);
                ui.label(RichText::new("By:").small());
                ui.text_edit_singleline(author);
            });
            ui.label(RichText::new("Description:").small());
            ui.text_edit_singleline(description);
        }
    }
}

/// Renders the real-time compiled diff preview panel.
fn render_diff_preview(ui: &mut egui::Ui, app: &PailuneApp) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("🔍 Live Mod Diff Preview").strong().color(accent()));
        ui.label(RichText::new("(Evaluated in real-time against verified table layouts)").small().color(muted()));
    });

    match compile(&app.creator.graph) {
        Ok(compiled) => {
            if compiled.field_patches.is_empty() {
                ui.label(RichText::new("No active rules generated yet. Connect a Source to an Action node.").color(muted()));
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for patch in &compiled.field_patches {
                        ui.horizontal(|ui| {
                            theme::badge(ui, "TARGET", success());
                            ui.label(RichText::new(&patch.target).strong());
                            ui.label(format!("({} changes)", patch.intents.len()));
                        });
                        for intent in &patch.intents {
                            ui.indent("intent_row", |ui| {
                                let entry_str = intent.entry.as_deref().unwrap_or("All / Filtered");
                                let field_str = intent.field.as_deref().unwrap_or("<raw>");
                                ui.label(format!("• {entry_str} -> {field_str} = {}", intent.new));
                            });
                        }
                    }
                });
            }
        }
        Err(err) => {
            ui.label(RichText::new(format!("⚠ Compilation Warning: {err}")).color(warning()));
        }
    }
}

fn rand_id() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    nanos as u32
}
