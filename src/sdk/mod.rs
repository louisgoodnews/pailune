//! Pailune Modding SDK (Phase 16 / B-18).
//!
//! Provides mod project scaffolding, library bundling, documentation generation,
//! and distribution export pipelines for ASI and JSON mods.

pub mod project;

#[cfg(test)]
mod tests;

pub use project::{ProjectConfig, ProjectType, ScaffoldReport, export_project, scaffold_project};
