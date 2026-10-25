//! The getting-started steps and how far the user is.

/// One step of the first-run path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    FindGame,
    AddMod,
    EnableMod,
    Apply,
    Play,
}

/// Steps in order, with title and a one-sentence explanation.
pub const STEPS: [(Step, &str, &str); 5] = [
    (
        Step::FindGame,
        "Find the game",
        "Pylune needs the Crimson Desert folder (it contains bin64/ and meta/).",
    ),
    (
        Step::AddMod,
        "Add a mod",
        "Import a downloaded archive or folder, drop it onto the window, or download from Nexus Mods.",
    ),
    (
        Step::EnableMod,
        "Enable it",
        "Tick the checkbox in front of the mod. Mods lower in the list win when two change the same file.",
    ),
    (
        Step::Apply,
        "Apply",
        "Apply links the enabled mods into the game. Your original files are kept and come back with Unlink all.",
    ),
    (
        Step::Play,
        "Play",
        "Start the game from Pylune; pending changes are applied first.",
    ),
];

/// The three ideas a new user needs before anything else (R-18).
pub const CONCEPTS: [&str; 3] = [
    "Mods are kept in Pylune's own folder — installing, enabling and ordering never touches the game.",
    "Apply links the enabled mods into the game folder; replaced originals are kept as .bak.",
    "Unlink all puts the game back exactly as it was.",
];

/// Facts the steps are checked against.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Facts {
    pub game_ready: bool,
    pub has_mods: bool,
    pub has_enabled: bool,
    /// Links are deployed and nothing is pending.
    pub applied: bool,
    pub played: bool,
}

impl Facts {
    /// Whether a step is complete.
    pub fn done(&self, step: Step) -> bool {
        match step {
            Step::FindGame => self.game_ready,
            Step::AddMod => self.has_mods,
            Step::EnableMod => self.has_enabled,
            Step::Apply => self.applied,
            Step::Play => self.played,
        }
    }

    /// Completed steps; finding the game counts from the start when it was
    /// detected automatically (endowed progress, R-17).
    pub fn completed(&self) -> usize {
        STEPS.iter().filter(|(s, _, _)| self.done(*s)).count()
    }

    /// The first step that is not complete yet.
    pub fn current(&self) -> Option<Step> {
        STEPS.iter().map(|(s, _, _)| *s).find(|s| !self.done(*s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_follows_the_facts() {
        let mut facts = Facts {
            game_ready: true,
            ..Facts::default()
        };
        assert_eq!(
            (facts.completed(), facts.current()),
            (1, Some(Step::AddMod))
        );
        facts.has_mods = true;
        facts.has_enabled = true;
        facts.applied = true;
        facts.played = true;
        assert_eq!((facts.completed(), facts.current()), (5, None));
    }
}
