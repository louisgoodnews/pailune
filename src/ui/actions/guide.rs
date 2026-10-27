//! Guide actions: opening, closing and the "show at start" preference.

use crate::config::file;
use crate::error::Result;
use crate::steam::GameStatus;
use crate::ui::guide::GuideAction;
use crate::ui::guide::steps::Facts;
use crate::ui::{Level, PailuneApp, StatusLine};

impl PailuneApp {
    /// The state the getting-started steps are checked against.
    pub fn guide_facts(&self) -> Facts {
        let mods = &self.registry.mods;
        Facts {
            game_ready: self.game == GameStatus::Ready,
            has_mods: !mods.is_empty(),
            has_enabled: mods.iter().any(|m| m.enabled),
            applied: self.deployed_links > 0 && !self.dirty,
            played: self.guide.played,
        }
    }

    /// Handles guide intents.
    pub(super) fn guide_action(&mut self, action: GuideAction) -> Result<Option<StatusLine>> {
        match action {
            GuideAction::Open(page) => {
                self.guide.open = true;
                self.guide.page = page;
                Ok(None)
            }
            GuideAction::Close => {
                self.guide.open = false;
                // A finished guide no longer opens by itself.
                let finished = self.guide_facts().current().is_none();
                match finished && self.config.behavior.show_guide {
                    true => self.set_show_guide(false).map(|()| {
                        Some(StatusLine::new(
                            Level::Success,
                            "Guide finished — reopen it any time with ❓ Guide or F1",
                        ))
                    }),
                    false => Ok(None),
                }
            }
            GuideAction::ShowAtStart(show) => self.set_show_guide(show).map(|()| None),
            GuideAction::ChooseGame => {
                self.setup_open = true;
                Ok(None)
            }
        }
    }

    /// Stores the preference in the configuration and the settings draft.
    fn set_show_guide(&mut self, show: bool) -> Result<()> {
        let mut next = self.config.clone();
        next.behavior.show_guide = show;
        file::save(&self.paths.config_file, &next)?;
        self.config = next;
        self.draft.config.behavior.show_guide = show;
        Ok(())
    }
}
