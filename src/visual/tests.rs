use crate::visual::compiler::compile;
use crate::visual::graph::{Graph, ItemMatchMode, MathOp, Node, NodeType, Socket, SocketType};
use crate::visual::templates;
use tempfile::tempdir;

#[test]
fn graph_serialization_round_trip() {
    let mut graph = Graph::new("Test Mod");
    let item_node = Node::new(
        "n1",
        "Item Query",
        NodeType::ItemSource {
            item_name: "Arrow".into(),
            match_mode: ItemMatchMode::Exact,
        },
    );
    let action_node = Node::new(
        "n2",
        "Set Stacks",
        NodeType::SetItemStackLimit { max_stack: 500 },
    );

    graph.add_node(item_node);
    graph.add_node(action_node);
    graph.connect("n1", "item", "n2", "item").unwrap();

    let json_str = serde_json::to_string_pretty(&graph).expect("serializes to json");
    let deserialized: Graph = serde_json::from_str(&json_str).expect("deserializes from json");

    assert_eq!(graph, deserialized);
}

#[test]
fn topological_order_and_cycle_detection() {
    let mut graph = Graph::new("Acyclic Graph");
    let n1 = Node::new("n1", "Const", NodeType::NumberConstant { value: 10.0 });
    let n2 = Node::new(
        "n2",
        "Math",
        NodeType::MathOperation {
            op: MathOp::Multiply,
            operand: 2.0,
        },
    );
    let n3 = Node::new(
        "n3",
        "Set Stacks",
        NodeType::SetItemStackLimit { max_stack: 100 },
    );

    graph.add_node(n1);
    graph.add_node(n2);
    graph.add_node(n3);

    graph.connect("n1", "val", "n2", "in").unwrap();
    graph.connect("n2", "out", "n3", "override_val").unwrap();

    let order = graph.topological_order().expect("valid topological order");
    let order_ids: Vec<&str> = order.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(order_ids, vec!["n1", "n2", "n3"]);

    // Introduce a cycle: n2 -> n1
    graph.nodes[0]
        .inputs
        .push(Socket::new("cycle_in", "Cycle", SocketType::Number));
    graph
        .connections
        .push(crate::visual::graph::Connection::new(
            "n2", "out", "n1", "cycle_in",
        ));

    assert!(graph.topological_order().is_err());
}

#[test]
fn socket_type_safety_rejects_incompatible_connections() {
    let mut graph = Graph::new("Type Check");
    let num_node = Node::new("num", "Number", NodeType::NumberConstant { value: 10.0 });
    let item_action = Node::new(
        "action",
        "Set Stacks",
        NodeType::SetItemStackLimit { max_stack: 100 },
    );

    graph.add_node(num_node);
    graph.add_node(item_action);

    // Try connecting Number socket to ItemRef socket
    let res = graph.connect("num", "val", "action", "item");
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .to_string()
            .contains("socket type mismatch")
    );
}

#[test]
fn compiles_mega_stacks_template_to_format3() {
    let template = templates::mega_stacks(999);
    let compiled = compile(&template).expect("compiles cleanly");

    assert_eq!(compiled.title, "Mega Stacks");
    assert_eq!(compiled.field_patches.len(), 1);

    let patch = &compiled.field_patches[0];
    assert_eq!(patch.target, "iteminfo.pabgb");
    assert_eq!(patch.intents.len(), 2);

    // Arrow intent
    assert_eq!(patch.intents[0].field.as_deref(), Some("_maxStackCount"));
    assert_eq!(patch.intents[0].new, serde_json::json!(999));
    assert!(patch.intents[0].matches.is_some());

    // Potion intent
    assert_eq!(patch.intents[1].field.as_deref(), Some("_maxStackCount"));
    assert_eq!(patch.intents[1].new, serde_json::json!(999));
    assert!(patch.intents[1].matches.is_some());
}

#[test]
fn compiles_loot_multiplier_template_to_dropsetinfo() {
    let template = templates::loot_multiplier(100.0, 30, 30);
    let compiled = compile(&template).expect("compiles cleanly");

    assert_eq!(compiled.field_patches.len(), 1);
    let patch = &compiled.field_patches[0];
    assert_eq!(patch.target, "dropsetinfo.pabgb");

    // Should have rate intent and yield intents
    assert!(patch.intents.len() >= 3);

    let rate_intent = patch
        .intents
        .iter()
        .find(|i| i.field.as_deref() == Some("_dropInfoList[*]._dropRate"))
        .expect("rate intent found");
    assert_eq!(rate_intent.new, serde_json::json!(1_000_000)); // 100%

    let min_intent = patch
        .intents
        .iter()
        .find(|i| i.field.as_deref() == Some("_dropInfoList[*]._minDropCount"))
        .expect("min yield intent found");
    assert_eq!(min_intent.new, serde_json::json!(30));

    let max_intent = patch
        .intents
        .iter()
        .find(|i| i.field.as_deref() == Some("_dropInfoList[*]._maxDropCount"))
        .expect("max yield intent found");
    assert_eq!(max_intent.new, serde_json::json!(30));
}

#[test]
fn compiles_with_math_operator_node() {
    let mut graph = Graph::new("Dynamic Math Mod");
    let item = Node::new(
        "item_node",
        "Arrow",
        NodeType::ItemSource {
            item_name: "Pyeonjeon_Arrow".into(),
            match_mode: ItemMatchMode::Exact,
        },
    );
    let const_node = Node::new("c", "Base", NodeType::NumberConstant { value: 100.0 });
    let math_node = Node::new(
        "m",
        "10x Multiplier",
        NodeType::MathOperation {
            op: MathOp::Multiply,
            operand: 10.0,
        },
    );
    let action = Node::new(
        "act",
        "Set Stacks",
        NodeType::SetItemStackLimit { max_stack: 0 },
    );

    graph.add_node(item);
    graph.add_node(const_node);
    graph.add_node(math_node);
    graph.add_node(action);

    graph.connect("item_node", "item", "act", "item").unwrap();
    graph.connect("c", "val", "m", "in").unwrap();
    graph.connect("m", "out", "act", "override_val").unwrap();

    let compiled = compile(&graph).expect("compiles math graph");
    assert_eq!(compiled.field_patches.len(), 1);

    let intent = &compiled.field_patches[0].intents[0];
    assert_eq!(intent.entry.as_deref(), Some("Pyeonjeon_Arrow"));
    assert_eq!(intent.field.as_deref(), Some("_maxStackCount"));
    // 100.0 * 10.0 = 1000
    assert_eq!(intent.new, serde_json::json!(1000));
}

#[test]
fn save_to_staging_creates_valid_mod_files() {
    let template = templates::crate_coin_rewards(100);
    let compiled = compile(&template).expect("compiles");

    let tmp = tempdir().expect("tempdir");
    let mod_dir = compiled
        .save_to_staging(tmp.path())
        .expect("saved to staging");

    assert!(mod_dir.join("modinfo.json").is_file());
    assert!(mod_dir.join("graph.json").is_file());
    assert!(mod_dir.join("README.md").is_file());
    assert!(mod_dir.join("crate_coin_rewards.field.json").is_file());

    let modinfo_content = std::fs::read_to_string(mod_dir.join("modinfo.json")).unwrap();
    let modinfo: serde_json::Value = serde_json::from_str(&modinfo_content).unwrap();
    assert_eq!(modinfo["name"], "Crate Coin Rewards");
    assert_eq!(modinfo["type"], "json");
}
