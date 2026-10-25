//! Profile page actions.

use crate::db;
use crate::error::{PyluneError, Result};
use crate::logging::{Event, EventKind, Severity};
use crate::profiles::{Profile, ops};
use crate::ui::{Level, ProfileAction, PyluneApp, StatusLine};

impl PyluneApp {
    pub(super) fn profile_action(&mut self, action: ProfileAction) -> Result<StatusLine> {
        let line = match action {
            ProfileAction::Save(name) => self.save_profile(name.trim())?,
            ProfileAction::Load(name) => self.load_profile(&name)?,
            ProfileAction::Delete(name) => {
                self.confirm = None;
                db::profiles::delete(&self.conn, &name)?;
                self.log_profile(&format!("deleted profile '{name}'"));
                StatusLine::new(Level::Success, format!("Profile '{name}' deleted"))
            }
            ProfileAction::SaveOptions(name) => {
                let options = self
                    .profiles
                    .option_drafts
                    .get(&name)
                    .cloned()
                    .unwrap_or_default();
                let profile = Profile {
                    launch_options: options.trim().to_string(),
                    ..self.find(&name)?
                };
                db::profiles::save(&mut self.conn, &profile)?;
                self.log_profile(&format!(
                    "launch options of '{name}' set to {:?}",
                    profile.launch_options
                ));
                StatusLine::new(Level::Success, format!("Launch options of '{name}' saved"))
            }
        };
        self.profiles.book = db::profiles::load(&self.conn)?;
        Ok(line)
    }

    fn find(&self, name: &str) -> Result<Profile> {
        ops::find(&self.profiles.book, name).cloned()
    }

    fn save_profile(&mut self, name: &str) -> Result<StatusLine> {
        if name.is_empty() {
            return Err(PyluneError::Invalid(
                "profile name must not be empty".into(),
            ));
        }
        // Overwriting keeps the profile's launch options.
        let options = self
            .find(name)
            .map(|p| p.launch_options)
            .unwrap_or_default();
        db::profiles::save(
            &mut self.conn,
            &ops::capture(name, &self.registry, &options),
        )?;
        db::profiles::set_active(&self.conn, Some(name))?;
        self.profiles.name_draft.clear();
        self.log_profile(&format!("saved profile '{name}'"));
        Ok(StatusLine::new(
            Level::Success,
            format!("Profile '{name}' saved"),
        ))
    }

    fn load_profile(&mut self, name: &str) -> Result<StatusLine> {
        let profile = self.find(name)?;
        let (_, outcome) = ops::apply(&profile, &self.registry);
        self.update_registry(|r| Ok(ops::apply(&profile, r).0))?;
        db::profiles::set_active(&self.conn, Some(name))?;
        self.log_profile(&format!("loaded profile '{name}'"));
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

    fn log_profile(&self, message: &str) {
        self.journal(Event::new(
            Severity::Info,
            EventKind::Profile,
            None,
            message,
        ));
    }
}
