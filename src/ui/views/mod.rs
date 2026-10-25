//! Rendering functions. Each view reads [`PyluneApp`](super::PyluneApp)
//! state (or mutates only its own input widgets) and returns the
//! [`Action`](super::Action)s the user triggered.

pub mod banner;
pub mod confirm;
pub mod header;
pub mod incoming;
pub mod jobs;
pub mod log;
pub mod mod_details;
pub mod mod_files;
pub mod mod_list;
pub mod nexus;
pub mod profiles;
pub mod readme;
pub mod settings;
pub mod setup;
pub mod status_bar;

use super::Action;

/// Keeps only the actions whose button was clicked.
pub fn clicked(pairs: impl IntoIterator<Item = (bool, Action)>) -> Vec<Action> {
    pairs
        .into_iter()
        .filter(|(c, _)| *c)
        .map(|(_, a)| a)
        .collect()
}
