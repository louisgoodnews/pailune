//! Preference sections of `config.toml`: appearance, behaviour, mod
//! management and backups. Defaults follow `docs/ux-principles.md` (R-21:
//! safe defaults, R-04: undo instead of confirmation).

use serde::{Deserialize, Serialize};

/// Colour scheme of the window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the desktop's light/dark preference.
    System,
    #[default]
    Dark,
    Light,
}

/// Spacing between controls.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    Compact,
    #[default]
    Comfortable,
}

/// Smallest and largest UI scale.
pub const SCALE_RANGE: std::ops::RangeInclusive<f32> = 0.8..=2.0;

/// `[ui]` — appearance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: Theme,
    /// Zoom factor of the whole interface (1.0 = 100 %).
    pub scale: f32,
    pub density: Density,
    /// Disable animations (WCAG 2.3.3).
    pub reduced_motion: bool,
}

impl Eq for UiConfig {}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            scale: 1.0,
            density: Density::Comfortable,
            reduced_motion: false,
        }
    }
}

/// `[behavior]` — how the app reacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BehaviorConfig {
    /// Ask before deleting a mod (deletes can be undone either way).
    pub confirm_deletes: bool,
    /// Check Nexus for mod updates when Pailune starts.
    pub check_updates_on_start: bool,
    /// Show the getting-started guide on the next start.
    pub show_guide: bool,
    /// Pop up notifications (toasts); the notification center always
    /// collects them.
    pub toasts: bool,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            confirm_deletes: false,
            check_updates_on_start: false,
            show_guide: true,
            toasts: true,
        }
    }
}

/// `[mods]` — mod management.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModsConfig {
    /// Enable newly installed mods right away.
    pub enable_new: bool,
    /// Deleted mods kept in "Recently deleted".
    pub trash_keep: usize,
}

impl Default for ModsConfig {
    fn default() -> Self {
        Self {
            enable_new: false,
            trash_keep: 20,
        }
    }
}

/// `[backup]` — automatic backups of Pailune's own state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BackupConfig {
    /// Take a settings-and-mod-list backup once a day at start.
    pub auto_daily: bool,
    /// Automatic backups kept; older ones are pruned.
    pub keep_auto: usize,
    /// Back up the game's saves every time Play is pressed.
    pub saves_before_play: bool,
    /// Automatic save backups kept.
    pub keep_saves: usize,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            auto_daily: true,
            keep_auto: 10,
            saves_before_play: true,
            keep_saves: 10,
        }
    }
}
