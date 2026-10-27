//! Profile page actions.

use crate::db;
use crate::error::{PailuneError, Result};
use crate::logging::{Event, EventKind, Severity};
use crate::profiles::{Profile, ops};
use crate::ui::{Level, PailuneApp, ProfileAction, StatusLine};

impl PailuneApp {
    /// Handles profile page actions.
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
            ProfileAction::Export(name) => self.export_profile(&name)?,
            ProfileAction::Import => self.import_profile()?,
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

    /// A copy of the profile called `name`.
    fn find(&self, name: &str) -> Result<Profile> {
        ops::find(&self.profiles.book, name).cloned()
    }

    /// Saves the current selection as a profile (overwriting keeps its options).
    fn save_profile(&mut self, name: &str) -> Result<StatusLine> {
        if name.is_empty() {
            return Err(PailuneError::Invalid(
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

    /// Applies a profile to the mod list.
    fn load_profile(&mut self, name: &str) -> Result<StatusLine> {
        let profile = self.find(name)?;
        let (_, outcome) = ops::apply(&profile, &self.registry);
        self.update_registry(|r| Ok(ops::apply(&profile, r).0))?;
        db::profiles::set_active(&self.conn, Some(name))?;
        // A pending mod list belonged to the setup that was just replaced.
        self.mods.list_import = None;
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

    /// Journals a profile event.
    fn log_profile(&self, message: &str) {
        self.journal(Event::new(
            Severity::Info,
            EventKind::Profile,
            None,
            message,
        ));
    }

    /// Writes a profile to a JSON file chosen by the user.
    fn export_profile(&mut self, name: &str) -> Result<StatusLine> {
        let profile = self.find(name)?;
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("{name}.pailune-profile.json"))
            .save_file()
        else {
            return Ok(StatusLine::new(Level::Info, "Export cancelled"));
        };
        let text = serde_json::to_string_pretty(&profile)?;
        std::fs::write(&path, text).map_err(|source| crate::error::PailuneError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(StatusLine::new(
            Level::Success,
            format!("Profile '{name}' exported to {}", path.display()),
        ))
    }

    /// Reads a profile file; a name clash gets a numbered suffix.
    fn import_profile(&mut self) -> Result<StatusLine> {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Pailune profile", &["json"])
            .pick_file()
        else {
            return Ok(StatusLine::new(Level::Info, "Import cancelled"));
        };
        let text =
            std::fs::read_to_string(&path).map_err(|source| crate::error::PailuneError::Io {
                path: path.clone(),
                source,
            })?;
        let profile: Profile = serde_json::from_str(&text)?;
        let taken = |n: &str| self.profiles.book.profiles.iter().any(|p| p.name == n);
        let name = (1..)
            .map(|i| match i {
                1 => profile.name.clone(),
                i => format!("{} ({i})", profile.name),
            })
            .find(|n| !taken(n))
            .unwrap_or_default();
        db::profiles::save(
            &mut self.conn,
            &Profile {
                name: name.clone(),
                ..profile
            },
        )?;
        self.log_profile(&format!(
            "imported profile '{name}' from {}",
            path.display()
        ));
        Ok(StatusLine::new(
            Level::Success,
            format!("Profile '{name}' imported — load it to use it"),
        ))
    }
}
