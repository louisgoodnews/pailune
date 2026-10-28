//! Compiles visual node graphs into game-ready mod artifacts.
//!
//! Evaluates the graph connections, resolves sources to targets, applies
//! math operations, and outputs format-3 table patches (`.field.json`),
//! metadata (`modinfo.json`), and generated documentation (`README.md`).

use super::graph::{Graph, ItemMatchMode, MathOp, Node, NodeCategory, NodeType};
use crate::error::{PailuneError, Result};
use crate::tables::format3::{FieldPatch, Intent};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A fully compiled mod generated from a visual graph.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledMod {
    pub mod_name: String,
    pub title: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub field_patches: Vec<FieldPatch>,
    pub graph_json: String,
    pub readme: String,
}

impl CompiledMod {
    /// Saves the compiled mod files directly into a staging folder.
    pub fn save_to_staging(&self, staging_root: &Path) -> Result<PathBuf> {
        let mod_dir = staging_root.join(&self.mod_name);
        fs::create_dir_all(&mod_dir).map_err(|e| {
            PailuneError::Invalid(format!("could not create mod directory {}: {e}", mod_dir.display()))
        })?;

        // 1. Write modinfo.json
        let modinfo = json!({
            "name": self.mod_name,
            "title": self.title,
            "version": self.version,
            "author": self.author,
            "description": self.description,
            "type": "json"
        });
        let modinfo_path = mod_dir.join("modinfo.json");
        fs::write(&modinfo_path, serde_json::to_string_pretty(&modinfo)?)
            .map_err(|e| PailuneError::Invalid(format!("failed to write modinfo.json: {e}")))?;

        // 2. Write graph.json for round-trip editing in the visual creator
        let graph_path = mod_dir.join("graph.json");
        fs::write(&graph_path, &self.graph_json)
            .map_err(|e| PailuneError::Invalid(format!("failed to write graph.json: {e}")))?;

        // 3. Write README.md
        let readme_path = mod_dir.join("README.md");
        fs::write(&readme_path, &self.readme)
            .map_err(|e| PailuneError::Invalid(format!("failed to write README.md: {e}")))?;

        // 4. Write format-3 patch file: <mod_name>.field.json
        if !self.field_patches.is_empty() {
            let targets_json: Vec<Value> = self
                .field_patches
                .iter()
                .map(|p| {
                    let intents_json: Vec<Value> = p
                        .intents
                        .iter()
                        .map(|i| {
                            let mut obj = serde_json::Map::new();
                            if let Some(entry) = &i.entry {
                                obj.insert("entry".into(), json!(entry));
                            }
                            if let Some(key) = i.key {
                                obj.insert("key".into(), json!(key));
                            }
                            if let Some(field) = &i.field {
                                obj.insert("field".into(), json!(field));
                            }
                            obj.insert("op".into(), json!(i.op));
                            obj.insert("new".into(), i.new.clone());
                            if let Some(old) = &i.old {
                                obj.insert("old".into(), json!(old));
                            }
                            if let Some(matches) = &i.matches {
                                obj.insert("match".into(), matches.clone());
                            }
                            Value::Object(obj)
                        })
                        .collect();

                    json!({
                        "target": p.target,
                        "intents": intents_json
                    })
                })
                .collect();

            let doc = json!({
                "format": 3,
                "targets": targets_json
            });

            let patch_filename = format!("{}.field.json", self.mod_name.to_lowercase().replace(' ', "_"));
            let patch_path = mod_dir.join(patch_filename);
            fs::write(&patch_path, serde_json::to_string_pretty(&doc)?)
                .map_err(|e| PailuneError::Invalid(format!("failed to write patch file: {e}")))?;
        }

        Ok(mod_dir)
    }
}

/// Compiles a `Graph` into ready-to-deploy mod artifacts.
pub fn compile(graph: &Graph) -> Result<CompiledMod> {
    // 1. Extract metadata from ModTarget node or fall back to graph title
    let target_node = graph
        .nodes
        .iter()
        .find(|n| n.category == NodeCategory::Target);

    let (title, author, version, description) = match target_node.map(|n| &n.node_type) {
        Some(NodeType::ModTarget {
            title,
            author,
            version,
            description,
        }) => (
            title.clone(),
            author.clone(),
            version.clone(),
            description.clone(),
        ),
        _ => (
            graph.title.clone(),
            "Pailune Visual Creator".into(),
            graph.version.clone(),
            format!("Created with Pailune Visual Mod Creator: {}", graph.title),
        ),
    };

    let mod_name = if title.trim().is_empty() {
        "Custom_Mod".into()
    } else {
        title.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
    };

    // 2. Map outgoing connections for each socket
    let mut target_groups: HashMap<String, Vec<Intent>> = HashMap::new();
    let mut notes: Vec<String> = Vec::new();

    // 3. Process each Action node
    for node in &graph.nodes {
        match &node.node_type {
            NodeType::SetItemStackLimit { max_stack } => {
                let limit = resolve_numeric_input(graph, node, "override_val").unwrap_or(*max_stack as f64) as u64;
                let sources = incoming_nodes(graph, &node.id, "item");
                if sources.is_empty() {
                    // Default to Arrow if not connected
                    let intent = Intent {
                        entry: Some("Arrow".into()),
                        key: None,
                        field: Some("_maxStackCount".into()),
                        op: "set".into(),
                        new: json!(limit),
                        old: None,
                        matches: None,
                    };
                    target_groups
                        .entry("iteminfo.pabgb".into())
                        .or_default()
                        .push(intent);
                    notes.push(format!("Arrow: set _maxStackCount = {limit}"));
                } else {
                    for src in sources {
                        if let NodeType::ItemSource {
                            item_name,
                            match_mode,
                        } = &src.node_type
                        {
                            match match_mode {
                                ItemMatchMode::Exact => {
                                    let intent = Intent {
                                        entry: Some(item_name.clone()),
                                        key: None,
                                        field: Some("_maxStackCount".into()),
                                        op: "set".into(),
                                        new: json!(limit),
                                        old: None,
                                        matches: None,
                                    };
                                    target_groups
                                        .entry("iteminfo.pabgb".into())
                                        .or_default()
                                        .push(intent);
                                    notes.push(format!("{item_name}: set _maxStackCount = {limit}"));
                                }
                                ItemMatchMode::Contains => {
                                    let intent = Intent {
                                        entry: None,
                                        key: None,
                                        field: Some("_maxStackCount".into()),
                                        op: "set".into(),
                                        new: json!(limit),
                                        old: None,
                                        matches: Some(json!({ "name_contains": item_name })),
                                    };
                                    target_groups
                                        .entry("iteminfo.pabgb".into())
                                        .or_default()
                                        .push(intent);
                                    notes.push(format!("Items containing \"{item_name}\": set _maxStackCount = {limit}"));
                                }
                                ItemMatchMode::All => {
                                    let intent = Intent {
                                        entry: None,
                                        key: None,
                                        field: Some("_maxStackCount".into()),
                                        op: "set".into(),
                                        new: json!(limit),
                                        old: None,
                                        matches: Some(json!({ "all": true })),
                                    };
                                    target_groups
                                        .entry("iteminfo.pabgb".into())
                                        .or_default()
                                        .push(intent);
                                    notes.push(format!("All Items: set _maxStackCount = {limit}"));
                                }
                            }
                        }
                    }
                }
            }
            NodeType::SetDropRate { rate_percent } => {
                let pct = resolve_numeric_input(graph, node, "rate").unwrap_or(*rate_percent);
                let rate_val = ((pct.clamp(0.0, 100.0) / 100.0) * 1_000_000.0) as u32;
                let sources = incoming_nodes(graph, &node.id, "drop");

                for src in if sources.is_empty() {
                    vec![]
                } else {
                    sources
                } {
                    if let NodeType::DropSetSource { table_name, .. } = &src.node_type {
                        let intent = Intent {
                            entry: Some(table_name.clone()),
                            key: None,
                            field: Some("_dropInfoList[*]._dropRate".into()),
                            op: "set".into(),
                            new: json!(rate_val),
                            old: None,
                            matches: None,
                        };
                        target_groups
                            .entry("dropsetinfo.pabgb".into())
                            .or_default()
                            .push(intent);
                        notes.push(format!("{table_name}: set drop rate = {pct:.1}% ({rate_val})"));
                    }
                }
            }
            NodeType::SetDropQuantity {
                min_amount,
                max_amount,
            } => {
                let min_val = resolve_numeric_input(graph, node, "min").unwrap_or(*min_amount as f64) as u64;
                let max_val = resolve_numeric_input(graph, node, "max").unwrap_or(*max_amount as f64) as u64;
                let sources = incoming_nodes(graph, &node.id, "drop");

                for src in if sources.is_empty() {
                    vec![]
                } else {
                    sources
                } {
                    if let NodeType::DropSetSource { table_name, .. } = &src.node_type {
                        let min_intent = Intent {
                            entry: Some(table_name.clone()),
                            key: None,
                            field: Some("_dropInfoList[*]._minDropCount".into()),
                            op: "set".into(),
                            new: json!(min_val),
                            old: None,
                            matches: None,
                        };
                        let max_intent = Intent {
                            entry: Some(table_name.clone()),
                            key: None,
                            field: Some("_dropInfoList[*]._maxDropCount".into()),
                            op: "set".into(),
                            new: json!(max_val),
                            old: None,
                            matches: None,
                        };
                        target_groups
                            .entry("dropsetinfo.pabgb".into())
                            .or_default()
                            .extend([min_intent, max_intent]);
                        notes.push(format!("{table_name}: set yield = {min_val}..{max_val}"));
                    }
                }
            }
            NodeType::SetBountyPrice { price } => {
                let p = resolve_numeric_input(graph, node, "price").unwrap_or(*price as f64) as i64;
                let sources = incoming_nodes(graph, &node.id, "record");

                for src in if sources.is_empty() {
                    vec![]
                } else {
                    sources
                } {
                    if let NodeType::WantedPriceSource { record_name } = &src.node_type {
                        let intent = Intent {
                            entry: Some(record_name.clone()),
                            key: None,
                            field: Some("_increasePrice".into()),
                            op: "set".into(),
                            new: json!(p),
                            old: None,
                            matches: None,
                        };
                        target_groups
                            .entry("wantedinfo.pabgb".into())
                            .or_default()
                            .push(intent);
                        notes.push(format!("{record_name}: set bounty price = {p}"));
                    }
                }
            }
            _ => {}
        }
    }

    let field_patches = target_groups
        .into_iter()
        .map(|(target, intents)| FieldPatch { target, intents })
        .collect();

    let graph_json = serde_json::to_string_pretty(graph)?;
    let readme = generate_readme(&title, &author, &version, &description, &notes);

    Ok(CompiledMod {
        mod_name,
        title,
        version,
        author,
        description,
        field_patches,
        graph_json,
        readme,
    })
}

/// Traces connected nodes feeding into a socket.
fn incoming_nodes<'a>(graph: &'a Graph, to_node: &str, to_socket: &str) -> Vec<&'a Node> {
    graph
        .connections
        .iter()
        .filter(|c| c.to_node == to_node && c.to_socket == to_socket)
        .filter_map(|c| graph.find_node(&c.from_node))
        .collect()
}

/// Evaluates numeric value flowing into an input socket, traversing MathOperations.
fn resolve_numeric_input(graph: &Graph, node: &Node, socket_id: &str) -> Option<f64> {
    let conn = graph
        .connections
        .iter()
        .find(|c| c.to_node == node.id && c.to_socket == socket_id)?;

    let src = graph.find_node(&conn.from_node)?;
    match &src.node_type {
        NodeType::NumberConstant { value } => Some(*value),
        NodeType::MathOperation { op, operand } => {
            let base = resolve_numeric_input(graph, src, "in").unwrap_or(1.0);
            Some(match op {
                MathOp::Multiply => base * operand,
                MathOp::Add => base + operand,
                MathOp::Subtract => base - operand,
                MathOp::ClampMin => base.max(*operand),
                MathOp::ClampMax => base.min(*operand),
            })
        }
        _ => None,
    }
}

fn generate_readme(
    title: &str,
    author: &str,
    version: &str,
    description: &str,
    notes: &[String],
) -> String {
    let mut text = format!(
        "# {title}\n\n**Version:** {version}  \n**Author:** {author}  \n**Type:** Pailune Visual Mod\n\n## Description\n\n{description}\n\n## Applied Rules\n\n"
    );
    if notes.is_empty() {
        text.push_str("No active rules defined.\n");
    } else {
        for n in notes {
            text.push_str(&format!("- {n}\n"));
        }
    }
    text.push_str(
        "\n---\n*Created with [Pailune](https://github.com/louisgoodnews/pailune) Visual Mod Creator for Crimson Desert.*\n"
    );
    text
}
