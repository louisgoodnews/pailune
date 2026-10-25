//! Onboarding (board item B-03): a getting-started checklist whose steps are
//! real actions checked against the app's state, and a searchable
//! "How do I…" guide. See `docs/ux-principles.md` R-16 … R-20.

pub mod steps;
pub mod topics;

/// Which part of the guide window is shown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GuidePage {
    #[default]
    GettingStarted,
    Topics,
}

/// Guide window state.
#[derive(Debug, Clone, Default)]
pub struct GuideState {
    pub open: bool,
    pub page: GuidePage,
    /// Search text of the "How do I…" page.
    pub query: String,
    /// The game was started from Pylune in this session.
    pub played: bool,
}

/// Guide intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuideAction {
    Open(GuidePage),
    Close,
    /// Remember whether the guide opens at start.
    ShowAtStart(bool),
    /// Reopen the game-folder dialog.
    ChooseGame,
}
