//! Visual node-based mod creation studio for Crimson Desert.
//!
//! Provides a no-code visual canvas for creating game mods, targeting
//! format-3 table patches, XML descriptors, and game assets.

pub mod compiler;
pub mod graph;
pub mod templates;
#[cfg(test)]
mod tests;

pub use compiler::{CompiledMod, compile};
pub use graph::{
    Connection, Graph, ItemMatchMode, MathOp, Node, NodeCategory, NodeType, Socket, SocketType,
};
