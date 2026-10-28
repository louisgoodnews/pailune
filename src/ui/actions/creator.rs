//! Handling of visual mod creator user actions.

use crate::error::{IoContext, Result};
use crate::mod_engine::package::{PackageMetadata, package_mod};
use crate::mod_engine::registry;
use crate::ui::intent::{CreatorAction, DeployAction};
use crate::ui::{Level, PailuneApp, StatusLine};
use crate::visual::{compiler, templates};

impl PailuneApp {
    /// Dispatches visual creator actions.
    pub fn creator_action(&mut self, action: CreatorAction) -> Result<Option<StatusLine>> {
        match action {
            CreatorAction::ApplyGraph => {
                let compiled = compiler::compile(&self.creator.graph)?;
                let _mod_dir = compiled.save_to_staging(&self.paths.staging)?;
                let now = crate::db::now();
                let scan = registry::scan(&self.paths.staging)?;
                let mod_name = compiled.mod_name.clone();
                self.update_registry(|r| {
                    let mut reg = registry::merge(r, &scan, now);
                    if let Some(entry) = reg.mods.iter_mut().find(|m| m.name == mod_name) {
                        entry.enabled = true;
                    }
                    Ok(reg)
                })?;

                self.deploy_action(DeployAction::Apply)?;
                let msg = format!(
                    "Applied visual mod '{}' ({}) to game",
                    compiled.title, compiled.mod_name
                );
                self.creator.status_feedback = Some((msg.clone(), Level::Success));
                Ok(Some(StatusLine::new(Level::Success, msg)))
            }
            CreatorAction::ExportZip => {
                let compiled = compiler::compile(&self.creator.graph)?;
                let _mod_dir = compiled.save_to_staging(&self.paths.staging)?;
                let export_meta = PackageMetadata {
                    name: compiled.mod_name.clone(),
                    version: compiled.version.clone(),
                    author: compiled.author.clone(),
                    description: compiled.description.clone(),
                    bump_version: false,
                };
                let export_dir = self.paths.exports.clone();
                std::fs::create_dir_all(&export_dir).at(&export_dir)?;
                let res = package_mod(
                    &self.paths.staging,
                    &compiled.mod_name,
                    &export_dir,
                    &export_meta,
                )?;
                let msg = format!("Exported visual mod to {}", res.archive_path.display());
                self.creator.status_feedback = Some((msg.clone(), Level::Success));
                Ok(Some(StatusLine::new(Level::Success, msg)))
            }
            CreatorAction::LoadTemplate(name) => {
                let new_graph = match name.as_str() {
                    "mega_stacks" => templates::mega_stacks(999),
                    "loot_bounty" => templates::loot_multiplier(100.0, 30, 30),
                    "crate_coins" => templates::crate_coin_rewards(100),
                    _ => templates::mega_stacks(999),
                };
                self.creator.graph = new_graph;
                self.creator.selected_node = None;
                self.creator.dragging_node = None;
                self.creator.connecting_socket = None;
                let msg = format!("Loaded starter template: {}", self.creator.graph.title);
                self.creator.status_feedback = Some((msg.clone(), Level::Info));
                Ok(Some(StatusLine::new(Level::Info, msg)))
            }
            CreatorAction::ClearGraph => {
                self.creator.graph = crate::visual::Graph::new("New Mod");
                self.creator.selected_node = None;
                self.creator.dragging_node = None;
                self.creator.connecting_socket = None;
                Ok(Some(StatusLine::new(Level::Info, "Cleared visual canvas")))
            }
            CreatorAction::DeleteNode(id) => {
                self.creator.graph.remove_node(&id);
                if self.creator.selected_node.as_deref() == Some(&id) {
                    self.creator.selected_node = None;
                }
                Ok(None)
            }
        }
    }
}
