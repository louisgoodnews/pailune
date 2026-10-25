//! Profile page actions.

use crate::config::store;
use crate::error::Result;
use crate::profiles::{ProfileBook, ops};
use crate::ui::{Level, ProfileAction, PyluneApp, StatusLine};

impl PyluneApp {
    pub(super) fn profile_action(&mut self, action: ProfileAction) -> Result<StatusLine> {
        match action {
            ProfileAction::Save(name) => {
                let name = name.trim().to_string();
                let book = ops::upsert(&self.profiles.book, ops::capture(&name, &self.registry))?;
                self.store_book(book)?;
                self.profiles.name_draft.clear();
                Ok(StatusLine::new(
                    Level::Success,
                    format!("Profile '{name}' saved"),
                ))
            }
            ProfileAction::Load(name) => {
                let profile = ops::find(&self.profiles.book, &name)?.clone();
                let (next, outcome) = ops::apply(&profile, &self.registry);
                self.set_registry(next, true)?;
                self.store_book(ProfileBook {
                    active: Some(name.clone()),
                    ..self.profiles.book.clone()
                })?;
                Ok(match outcome.missing.as_slice() {
                    [] => StatusLine::new(
                        Level::Success,
                        format!("Profile '{name}' loaded — press Apply"),
                    ),
                    missing => StatusLine::new(
                        Level::Warning,
                        format!(
                            "Profile '{name}' loaded; not staged anymore: {}",
                            missing.join(", ")
                        ),
                    ),
                })
            }
            ProfileAction::Delete(name) => {
                self.confirm = None;
                self.store_book(ops::delete(&self.profiles.book, &name))?;
                Ok(StatusLine::new(
                    Level::Success,
                    format!("Profile '{name}' deleted"),
                ))
            }
        }
    }

    fn store_book(&mut self, book: ProfileBook) -> Result<()> {
        store::save(&self.paths.profiles, &book)?;
        self.profiles.book = book;
        Ok(())
    }
}
