//! Gathering packable files and prebuilt groups from the enabled mods.

use super::OverlayInput;
use crate::error::Result;
use crate::mod_engine::model::Registry;
use crate::mod_engine::plan::{Placement, css_target, deployable_files, virtual_path};
use std::path::Path;

/// Walks every enabled mod in load order.
pub fn collect(staging: &Path, registry: &Registry) -> Result<OverlayInput> {
    registry
        .mods
        .iter()
        .filter(|m| m.enabled)
        .try_fold(OverlayInput::default(), |mut input, m| {
            let root = staging.join(&m.name);
            deployable_files(&root, m.variant.as_deref())?
                .into_iter()
                .for_each(|f| match f.place {
                    Placement::Pack => {
                        if let Some(vpath) = virtual_path(&f.rel) {
                            let replaced = input.packs.insert(vpath, f.source);
                            input.conflicts += usize::from(replaced.is_some());
                        }
                    }
                    Placement::JsonPatch => input.patches.push((m.name.clone(), f.source)),
                    Placement::CssPatch => {
                        if let Some(target) = css_target(&f.rel) {
                            input.css_patches.push((m.name.clone(), target, f.source));
                        }
                    }
                    Placement::PrebuiltGroup => {
                        // The group folder is the file's parent (inside a variant, too).
                        let group = f.source.parent().map(Path::to_path_buf);
                        if let Some(group) = group.filter(|g| !input.prebuilt.contains(g)) {
                            input.prebuilt.push(group);
                        }
                    }
                    _ => {}
                });
            Ok(input)
        })
}
