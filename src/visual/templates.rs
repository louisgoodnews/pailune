//! Ready-made starter templates for the visual mod creator.
//!
//! Provides pre-wired graphs for common mod archetypes, allowing non-tech
//! creators to customize high-value game systems with a single click.

use super::graph::{Graph, ItemMatchMode, Node, NodeType};

/// Generates the "Mega Stacks" starter template graph.
pub fn mega_stacks(max_stack: u64) -> Graph {
    let mut graph = Graph::new("Mega Stacks");
    graph.version = "1.0.0".into();

    let arrows = Node::new(
        "arrows_source",
        "Ammunition Query",
        NodeType::ItemSource {
            item_name: "Arrow".into(),
            match_mode: ItemMatchMode::Contains,
        },
    )
    .with_pos(120.0, 160.0);

    let potions = Node::new(
        "potions_source",
        "Potions Query",
        NodeType::ItemSource {
            item_name: "Potion".into(),
            match_mode: ItemMatchMode::Contains,
        },
    )
    .with_pos(120.0, 360.0);

    let stack_arrows = Node::new(
        "stack_arrows",
        "Set Arrow Stacks",
        NodeType::SetItemStackLimit { max_stack },
    )
    .with_pos(450.0, 160.0);

    let stack_potions = Node::new(
        "stack_potions",
        "Set Potion Stacks",
        NodeType::SetItemStackLimit { max_stack },
    )
    .with_pos(450.0, 360.0);

    let target = Node::new(
        "mod_target",
        "Mod Configuration",
        NodeType::ModTarget {
            title: "Mega Stacks".into(),
            author: "Pailune Creator".into(),
            version: "1.0.0".into(),
            description: format!("Increases ammunition and potion stack sizes to {max_stack}."),
        },
    )
    .with_pos(780.0, 260.0);

    graph.add_node(arrows);
    graph.add_node(potions);
    graph.add_node(stack_arrows);
    graph.add_node(stack_potions);
    graph.add_node(target);

    let _ = graph.connect("arrows_source", "item", "stack_arrows", "item");
    let _ = graph.connect("potions_source", "item", "stack_potions", "item");
    let _ = graph.connect("stack_arrows", "action", "mod_target", "rules");

    graph
}

/// Generates the "Loot Bounty" starter template graph for hunting and animal drops.
pub fn loot_multiplier(rate_pct: f64, min_amount: u64, max_amount: u64) -> Graph {
    let mut graph = Graph::new("Hunting & Animal Yields");
    graph.version = "1.0.0".into();

    let middle_animals = Node::new(
        "middle_animals",
        "Medium Animals Drop Set",
        NodeType::DropSetSource {
            table_name: "DropSet_Middle_Animal_Common".into(),
            item_filter: None,
        },
    )
    .with_pos(120.0, 160.0);

    let small_animals = Node::new(
        "small_animals",
        "Small Animals Drop Set",
        NodeType::DropSetSource {
            table_name: "DropSet_Small_Animal_Common".into(),
            item_filter: None,
        },
    )
    .with_pos(120.0, 380.0);

    let set_rate = Node::new(
        "set_rate",
        "Set 100% Drop Rate",
        NodeType::SetDropRate {
            rate_percent: rate_pct,
        },
    )
    .with_pos(460.0, 160.0);

    let set_yield = Node::new(
        "set_yield",
        "Set Loot Amounts",
        NodeType::SetDropQuantity {
            min_amount,
            max_amount,
        },
    )
    .with_pos(460.0, 380.0);

    let target = Node::new(
        "mod_target",
        "Mod Configuration",
        NodeType::ModTarget {
            title: "Hunting & Animal Yields".into(),
            author: "Pailune Creator".into(),
            version: "1.0.0".into(),
            description: format!(
                "Guarantees hunting drops ({rate_pct:.0}%) and increases meat and leather yield to {min_amount}..{max_amount}."
            ),
        },
    )
    .with_pos(800.0, 260.0);

    graph.add_node(middle_animals);
    graph.add_node(small_animals);
    graph.add_node(set_rate);
    graph.add_node(set_yield);
    graph.add_node(target);

    let _ = graph.connect("middle_animals", "drop", "set_rate", "drop");
    let _ = graph.connect("small_animals", "drop", "set_yield", "drop");
    let _ = graph.connect("set_rate", "action", "mod_target", "rules");

    graph
}

/// Generates the "Crate Coin Rewards" starter template graph.
pub fn crate_coin_rewards(copper_amount: u64) -> Graph {
    let mut graph = Graph::new("Crate Coin Rewards");
    graph.version = "1.0.0".into();

    let barrels = Node::new(
        "barrels_source",
        "Barrels Drop Set",
        NodeType::DropSetSource {
            table_name: "DropSet_BoxBarrel_01".into(),
            item_filter: Some("Money_Copper".into()),
        },
    )
    .with_pos(120.0, 180.0);

    let pots = Node::new(
        "pots_source",
        "Pots Drop Set",
        NodeType::DropSetSource {
            table_name: "DropSet_Pot_01".into(),
            item_filter: Some("Money_Copper".into()),
        },
    )
    .with_pos(120.0, 380.0);

    let set_coins = Node::new(
        "set_coins",
        "Set 100 Copper Yield",
        NodeType::SetDropQuantity {
            min_amount: copper_amount,
            max_amount: copper_amount,
        },
    )
    .with_pos(480.0, 280.0);

    let target = Node::new(
        "mod_target",
        "Mod Configuration",
        NodeType::ModTarget {
            title: "Crate Coin Rewards".into(),
            author: "Pailune Creator".into(),
            version: "1.0.0".into(),
            description: format!(
                "Guarantees {copper_amount} copper coins from breaking crates, barrels, and pots."
            ),
        },
    )
    .with_pos(820.0, 280.0);

    graph.add_node(barrels);
    graph.add_node(pots);
    graph.add_node(set_coins);
    graph.add_node(target);

    let _ = graph.connect("barrels_source", "drop", "set_coins", "drop");
    let _ = graph.connect("set_coins", "action", "mod_target", "rules");

    graph
}
