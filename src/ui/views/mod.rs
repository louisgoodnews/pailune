//! Rendering functions. Each view reads [`PyluneApp`](super::PyluneApp)
//! state (or mutates only its own input widgets) and returns the
//! [`Action`](super::Action)s the user triggered.

pub mod confirm;
pub mod header;
pub mod mod_list;
pub mod settings;
pub mod status_bar;
