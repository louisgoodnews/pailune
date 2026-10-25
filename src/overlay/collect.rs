//! Gathering packable files and prebuilt groups from the enabled mods.

use super::OverlayInput;
use crate::error::Result;
use crate::mod_engine::model::Registry;
use crate::mod_engine::plan::{Placement, deployable_files, virtual_path};
use std::path::Path;

/// Walks every enabled mod in load order.
pub fn collect(staging: &Path, registry: &Registry) -> Result<OverlayInput> {
    registry
        .mods
        .iter()
        .filter(|m| m.enabled)
        .try_fold(OverlayInput::default(), |mut input, m| {
            let root = staging.join(&m.name);
            deployable_files(&root)?
                .into_iter()
                .for_each(|(rel, place)| match place {
                    Placement::Pack => {
                        if let Some(vpath) = virtual_path(&rel) {
                            let replaced = input.packs.insert(vpath, root.join(&rel));
                            input.conflicts += usize::from(replaced.is_some());
                        }
                    }
                    Placement::PrebuiltGroup => {
                        let group = rel.components().next().map(|c| root.join(c));
                        if let Some(group) = group.filter(|g| !input.prebuilt.contains(g)) {
                            input.prebuilt.push(group);
                        }
                    }
                    _ => {}
                });
            Ok(input)
        })
}
