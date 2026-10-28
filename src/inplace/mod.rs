//! The in-place route (D-14, `docs/design/in-place.md`): patched game files
//! reach the game through the index of the group that holds the vanilla
//! file. The game's large archives are never rewritten; Pailune adds one
//! archive per changed group (linked from its store) and rewrites only the
//! small index files, which are backed up first (base) and on every change
//! (versions).
//!
//! A version holds every index file Pailune has changed since the base, not
//! only the ones of that Apply, so rolling back to it restores the whole
//! state; files a version no longer changes are stored with their base bytes.

pub mod build;
pub mod classify;
pub mod store;

#[allow(unused_imports)]
pub use build::{InPlacePlan, build, ensure_base_backup, revert};
pub use classify::is_inplace_candidate;
pub use store::Store;

#[cfg(test)]
mod tests;
