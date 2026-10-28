//! Pailune Modding SDK (Phase 16 / B-18).
//!
//! Provides mod project scaffolding, library bundling, documentation generation,
//! and distribution export pipelines for ASI and JSON mods.

pub mod build;
pub mod log;
pub mod project;

#[cfg(test)]
mod tests;

pub use build::{BuildReport, build_project, detect_project_type, stage_built_project};
pub use log::read_plugin_log;
pub use project::{ProjectConfig, ProjectType, ScaffoldReport, export_project, scaffold_project};
