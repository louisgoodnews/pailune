//! Core data structures for Pailune's visual node-based mod creator.
//!
//! Visual mod graphs represent logic and parameter mutations as nodes and
//! connections. The graph serializes to `graph.json` inside a staged mod
//! folder and compiles to verified format-3 table patches and descriptors.

use crate::error::{PailuneError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Supported socket data types for connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocketType {
    /// References an item name, key, or category in `iteminfo`.
    ItemRef,
    /// References a loot drop set in `dropsetinfo`.
    DropTableRef,
    /// Numeric scalar or multiplier (int/float).
    Number,
    /// Boolean condition or toggle flag.
    Boolean,
    /// Arbitrary text or identifier.
    Text,
}

/// An input or output port on a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Socket {
    pub id: String,
    pub label: String,
    pub socket_type: SocketType,
}

impl Socket {
    pub fn new(id: impl Into<String>, label: impl Into<String>, socket_type: SocketType) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            socket_type,
        }
    }
}

/// Broad category of a node's function in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeCategory {
    /// Data sources: items, drop sets, game tables, constants.
    Source,
    /// Transforms: multipliers, math, filters, conditions.
    Operator,
    /// Mutations: set stack limits, modify drop rates, alter prices.
    Action,
    /// Output targets: metadata, package output.
    Target,
}

/// Matching rule for item queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemMatchMode {
    /// Exact match on the item entry name.
    Exact,
    /// Matches any item whose name contains the substring (case-insensitive).
    Contains,
    /// Matches all items in the table.
    All,
}

/// Mathematical operations supported by operator nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MathOp {
    Multiply,
    Add,
    Subtract,
    ClampMin,
    ClampMax,
}

/// Specific behavior and parameters of a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeType {
    /// Queries items from `iteminfo`.
    ItemSource {
        item_name: String,
        match_mode: ItemMatchMode,
    },
    /// Queries drop sets from `dropsetinfo`.
    DropSetSource {
        table_name: String,
        item_filter: Option<String>,
    },
    /// Queries bounty prices from `wantedinfo`.
    WantedPriceSource { record_name: String },
    /// Constant numeric value.
    NumberConstant { value: f64 },
    /// Mathematical transformation on input.
    MathOperation { op: MathOp, operand: f64 },
    /// Modifies `_maxStackCount` in `iteminfo`.
    SetItemStackLimit { max_stack: u64 },
    /// Modifies `_dropRate` in `DropSetInfo` (0.0 to 100.0 percent).
    SetDropRate { rate_percent: f64 },
    /// Modifies `_minDropCount` and `_maxDropCount` in `DropSetInfo`.
    SetDropQuantity { min_amount: u64, max_amount: u64 },
    /// Modifies `_increasePrice` in `wantedinfo`.
    SetBountyPrice { price: i64 },
    /// Mod target and metadata configuration.
    ModTarget {
        title: String,
        author: String,
        version: String,
        description: String,
    },
}

/// A node placed on the visual canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub title: String,
    pub category: NodeCategory,
    pub node_type: NodeType,
    /// Canvas position `[x, y]` for GUI persistence.
    pub pos: [f32; 2],
    pub inputs: Vec<Socket>,
    pub outputs: Vec<Socket>,
}

impl Node {
    pub fn new(id: impl Into<String>, title: impl Into<String>, node_type: NodeType) -> Self {
        let id = id.into();
        let title = title.into();
        let (category, inputs, outputs) = Self::default_sockets(&node_type);
        Self {
            id,
            title,
            category,
            node_type,
            pos: [100.0, 100.0],
            inputs,
            outputs,
        }
    }

    /// Positions the node on the visual canvas.
    pub fn with_pos(mut self, x: f32, y: f32) -> Self {
        self.pos = [x, y];
        self
    }

    /// Default input and output socket configurations for a node type.
    fn default_sockets(node_type: &NodeType) -> (NodeCategory, Vec<Socket>, Vec<Socket>) {
        match node_type {
            NodeType::ItemSource { .. } => (
                NodeCategory::Source,
                Vec::new(),
                vec![Socket::new("item", "Item Ref", SocketType::ItemRef)],
            ),
            NodeType::DropSetSource { .. } => (
                NodeCategory::Source,
                Vec::new(),
                vec![Socket::new("drop", "Drop Table", SocketType::DropTableRef)],
            ),
            NodeType::WantedPriceSource { .. } => (
                NodeCategory::Source,
                Vec::new(),
                vec![Socket::new("record", "Wanted Record", SocketType::Text)],
            ),
            NodeType::NumberConstant { .. } => (
                NodeCategory::Source,
                Vec::new(),
                vec![Socket::new("val", "Value", SocketType::Number)],
            ),
            NodeType::MathOperation { .. } => (
                NodeCategory::Operator,
                vec![Socket::new("in", "Input", SocketType::Number)],
                vec![Socket::new("out", "Result", SocketType::Number)],
            ),
            NodeType::SetItemStackLimit { .. } => (
                NodeCategory::Action,
                vec![
                    Socket::new("item", "Item Ref", SocketType::ItemRef),
                    Socket::new("override_val", "Stack Size", SocketType::Number),
                ],
                vec![Socket::new("action", "Rule", SocketType::Text)],
            ),
            NodeType::SetDropRate { .. } => (
                NodeCategory::Action,
                vec![
                    Socket::new("drop", "Drop Table", SocketType::DropTableRef),
                    Socket::new("rate", "Rate %", SocketType::Number),
                ],
                vec![Socket::new("action", "Rule", SocketType::Text)],
            ),
            NodeType::SetDropQuantity { .. } => (
                NodeCategory::Action,
                vec![
                    Socket::new("drop", "Drop Table", SocketType::DropTableRef),
                    Socket::new("min", "Min Count", SocketType::Number),
                    Socket::new("max", "Max Count", SocketType::Number),
                ],
                vec![Socket::new("action", "Rule", SocketType::Text)],
            ),
            NodeType::SetBountyPrice { .. } => (
                NodeCategory::Action,
                vec![
                    Socket::new("record", "Wanted Record", SocketType::Text),
                    Socket::new("price", "Reward Price", SocketType::Number),
                ],
                vec![Socket::new("action", "Rule", SocketType::Text)],
            ),
            NodeType::ModTarget { .. } => (
                NodeCategory::Target,
                vec![Socket::new("rules", "Rules", SocketType::Text)],
                Vec::new(),
            ),
        }
    }
}

/// A wire connection between an output socket and an input socket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    pub from_node: String,
    pub from_socket: String,
    pub to_node: String,
    pub to_socket: String,
}

impl Connection {
    pub fn new(
        from_node: impl Into<String>,
        from_socket: impl Into<String>,
        to_node: impl Into<String>,
        to_socket: impl Into<String>,
    ) -> Self {
        Self {
            from_node: from_node.into(),
            from_socket: from_socket.into(),
            to_node: to_node.into(),
            to_socket: to_socket.into(),
        }
    }
}

/// Complete node graph representing a visual mod project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub id: String,
    pub title: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
}

fn default_version() -> String {
    "1.0.0".into()
}

impl Graph {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: uuid_simple(),
            title: title.into(),
            version: default_version(),
            nodes: Vec::new(),
            connections: Vec::new(),
        }
    }

    /// Adds a node to the graph and returns its ID.
    pub fn add_node(&mut self, node: Node) -> String {
        let id = node.id.clone();
        self.nodes.push(node);
        id
    }

    /// Removes a node and any connections attached to it.
    pub fn remove_node(&mut self, node_id: &str) {
        self.nodes.retain(|n| n.id != node_id);
        self.connections
            .retain(|c| c.from_node != node_id && c.to_node != node_id);
    }

    /// Adds a connection between two sockets with type safety check.
    pub fn connect(
        &mut self,
        from_node: &str,
        from_socket: &str,
        to_node: &str,
        to_socket: &str,
    ) -> Result<()> {
        let src_node = self
            .find_node(from_node)
            .ok_or_else(|| PailuneError::Invalid(format!("source node '{from_node}' not found")))?;
        let src_sock = src_node
            .outputs
            .iter()
            .find(|s| s.id == from_socket)
            .ok_or_else(|| {
                PailuneError::Invalid(format!("output socket '{from_socket}' not found"))
            })?;

        let dst_node = self
            .find_node(to_node)
            .ok_or_else(|| PailuneError::Invalid(format!("dest node '{to_node}' not found")))?;
        let dst_sock = dst_node
            .inputs
            .iter()
            .find(|s| s.id == to_socket)
            .ok_or_else(|| {
                PailuneError::Invalid(format!("input socket '{to_socket}' not found"))
            })?;

        if src_sock.socket_type != dst_sock.socket_type {
            return Err(PailuneError::Invalid(format!(
                "socket type mismatch: cannot connect {:?} to {:?}",
                src_sock.socket_type, dst_sock.socket_type
            )));
        }

        // Remove existing connection to destination socket if present (single-input)
        self.connections
            .retain(|c| !(c.to_node == to_node && c.to_socket == to_socket));

        self.connections
            .push(Connection::new(from_node, from_socket, to_node, to_socket));
        Ok(())
    }

    pub fn disconnect(
        &mut self,
        from_node: &str,
        from_socket: &str,
        to_node: &str,
        to_socket: &str,
    ) {
        self.connections.retain(|c| {
            !(c.from_node == from_node
                && c.from_socket == from_socket
                && c.to_node == to_node
                && c.to_socket == to_socket)
        });
    }

    pub fn find_node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn find_node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Evaluates topological order of nodes for execution.
    pub fn topological_order(&self) -> Result<Vec<&Node>> {
        let mut in_degrees: HashMap<&str, usize> =
            self.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for c in &self.connections {
            if in_degrees.contains_key(c.to_node.as_str())
                && in_degrees.contains_key(c.from_node.as_str())
            {
                *in_degrees.get_mut(c.to_node.as_str()).unwrap() += 1;
                adj.entry(c.from_node.as_str())
                    .or_default()
                    .push(c.to_node.as_str());
            }
        }

        let mut queue: VecDeque<&str> = in_degrees
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(curr_id) = queue.pop_front() {
            let node = self.find_node(curr_id).unwrap();
            order.push(node);

            if let Some(neighbors) = adj.get(curr_id) {
                for &neighbor in neighbors {
                    let deg = in_degrees.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        if order.len() != self.nodes.len() {
            return Err(PailuneError::Invalid("graph contains cycles".into()));
        }

        Ok(order)
    }

    /// Validates the graph and returns any errors or warnings.
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let node_ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();

        for c in &self.connections {
            if !node_ids.contains(c.from_node.as_str()) {
                warnings.push(format!(
                    "connection references unknown source '{}'",
                    c.from_node
                ));
            }
            if !node_ids.contains(c.to_node.as_str()) {
                warnings.push(format!(
                    "connection references unknown target '{}'",
                    c.to_node
                ));
            }
        }

        let has_target = self
            .nodes
            .iter()
            .any(|n| n.category == NodeCategory::Target);
        if !has_target {
            warnings.push("graph has no ModTarget node".into());
        }

        warnings
    }
}

/// Simple pseudorandom ID generator without external UUID dependencies.
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("graph_{nanos:x}")
}
