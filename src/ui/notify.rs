//! Notifications: what finished in the background (installed, applied,
//! scanned, failed …) shown as toasts and kept in a notification center for
//! the session. The Log tab stays the permanent record.

use super::{Level, PyluneApp, StatusLine};
use crate::error::Result;
use crate::jobs::JobKind;
use std::time::{Duration, Instant};

/// How long a toast stays (errors stay until dismissed).
pub const TOAST_TIME: Duration = Duration::from_secs(6);

/// Toasts shown at once.
pub const TOASTS: usize = 3;

/// Notifications kept per session.
const KEEP: usize = 100;

/// One notification.
#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub id: u64,
    /// Unix seconds, for display.
    pub at: i64,
    /// When it arrived, for the toast timeout.
    pub arrived: Instant,
    pub level: Level,
    pub title: String,
    pub text: String,
    /// Seen in the notification center.
    pub read: bool,
    /// Its toast was closed (or timed out).
    pub dismissed: bool,
}

/// Notification center state.
#[derive(Debug, Clone, Default)]
pub struct Notifications {
    /// Oldest first.
    pub items: Vec<Notification>,
    next: u64,
    /// The center is open.
    pub open: bool,
}

/// Notification intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoticeAction {
    /// Open or close the center (opening marks everything read).
    ToggleCenter,
    /// Close one toast.
    Dismiss(u64),
    /// Remove every notification.
    Clear,
}

impl Notifications {
    /// Adds a notification (dropping the oldest beyond the limit).
    pub fn push(&mut self, level: Level, title: &str, text: &str) {
        self.next += 1;
        self.items.push(Notification {
            id: self.next,
            at: crate::db::now(),
            arrived: Instant::now(),
            level,
            title: title.to_string(),
            text: text.to_string(),
            read: self.open,
            dismissed: false,
        });
        let excess = self.items.len().saturating_sub(KEEP);
        self.items.drain(..excess);
    }

    pub fn unread(&self) -> usize {
        self.items.iter().filter(|n| !n.read).count()
    }

    /// The toasts to show now, newest first.
    pub fn toasts(&self, now: Instant) -> Vec<&Notification> {
        self.items
            .iter()
            .rev()
            .filter(|n| !n.dismissed && !n.read)
            .filter(|n| n.level == Level::Error || now.duration_since(n.arrived) < TOAST_TIME)
            .take(TOASTS)
            .collect()
    }
}

/// The title of a finished job's notification; `None` for routine jobs.
fn job_title(kind: &JobKind, ok: bool) -> Option<&'static str> {
    let title = match (kind, ok) {
        (JobKind::Import { replace: None, .. }, true) => "Mod installed",
        (JobKind::Import { .. }, true) => "Mod updated",
        (JobKind::Download(_), true) => "Download installed",
        (JobKind::Apply, true) => "Mods applied",
        (JobKind::UnlinkAll, true) => "Mods unlinked",
        (JobKind::CleanSlate, true) => "Clean slate done",
        (JobKind::Remove(_), true) => "Mod deleted",
        (JobKind::RestoreDeleted(_), true) => "Mod restored",
        (JobKind::CreateBackup { .. }, true) => "Backup created",
        (JobKind::RestoreBackup(_), true) => "Backup restored",
        (JobKind::BackupSaves, true) => "Saves backed up",
        (JobKind::RestoreSaves(_), true) => "Saves restored",
        (JobKind::CheckUpdates, true) => "Update check finished",
        (JobKind::FetchCollection { .. }, true) => "Collection loaded",
        (JobKind::RefreshBackup, true) => "Metadata backup refreshed",
        (JobKind::AnalyzeConflicts(_) | JobKind::DescribeFile { .. }, _) => return None,
        (JobKind::ValidateKey | JobKind::FetchMod(_), true) => return None,
        (_, true) => return None,
        (_, false) => "Something failed",
    };
    Some(title)
}

impl PyluneApp {
    /// Notifies about a finished job; its message is the status line it set.
    pub(super) fn notify_job(&mut self, kind: &JobKind, ok: bool) {
        if let Some(title) = job_title(kind, ok) {
            let level = match ok {
                true => self.status.level,
                false => Level::Error,
            };
            let text = self.status.text.clone();
            self.notices.push(level, title, &text);
        }
    }

    /// Handles notification intents.
    pub(super) fn notice_action(&mut self, action: NoticeAction) -> Result<Option<StatusLine>> {
        let notes = &mut self.notices;
        match action {
            NoticeAction::ToggleCenter => {
                notes.open = !notes.open;
                if notes.open {
                    notes.items.iter_mut().for_each(|n| n.read = true);
                }
            }
            NoticeAction::Dismiss(id) => notes
                .items
                .iter_mut()
                .filter(|n| n.id == id)
                .for_each(|n| n.dismissed = true),
            NoticeAction::Clear => notes.items.clear(),
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toasts_time_out_except_errors_and_the_center_marks_read() {
        let mut notes = Notifications::default();
        (0..5).for_each(|i| notes.push(Level::Success, "Mod installed", &format!("mod {i}")));
        let now = Instant::now();
        let shown: Vec<&str> = notes.toasts(now).iter().map(|n| n.text.as_str()).collect();
        assert_eq!(
            shown,
            ["mod 4", "mod 3", "mod 2"],
            "newest first, three at once"
        );
        let later = now + TOAST_TIME + Duration::from_secs(1);
        assert!(notes.toasts(later).is_empty(), "timed out");
        notes.push(Level::Error, "Something failed", "boom");
        let later = Instant::now() + TOAST_TIME * 10;
        assert_eq!(notes.toasts(later).len(), 1, "errors stay");
        assert_eq!(notes.unread(), 6);
        (0..KEEP).for_each(|_| notes.push(Level::Info, "x", "y"));
        assert_eq!(notes.items.len(), KEEP);
        assert!(job_title(&JobKind::AnalyzeConflicts(1), true).is_none());
        assert_eq!(job_title(&JobKind::Apply, false), Some("Something failed"));
    }
}
