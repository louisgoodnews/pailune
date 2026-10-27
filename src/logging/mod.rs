//! Diagnostics: a rolling application log plus a structured event journal.
//!
//! Every [`Event`] goes to three sinks: the `tracing` log file
//! (`logs/pailune.log.YYYY-MM-DD`), the `events` table in SQLite, and — when it
//! concerns a mod — that mod's own log (`logs/mods/<mod>.log`).

pub mod init;
pub mod journal;

/// How serious an event is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warn,
    Error,
}

/// What an event is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Install,
    Update,
    Remove,
    Deploy,
    Unlink,
    Reset,
    Backup,
    Download,
    Launch,
    Profile,
    Settings,
    Job,
}

/// One journal entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Unix timestamp (seconds).
    pub ts: i64,
    pub severity: Severity,
    pub kind: EventKind,
    pub mod_name: Option<String>,
    pub message: String,
}

impl Severity {
    /// Name stored in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warn => "warn",
            Severity::Error => "error",
        }
    }

    /// Severity from its stored name (unknown names read as info).
    pub fn parse(text: &str) -> Self {
        match text {
            "warn" => Severity::Warn,
            "error" => Severity::Error,
            _ => Severity::Info,
        }
    }
}

/// All kinds with their stored names, used for both directions of mapping.
const KIND_NAMES: [(EventKind, &str); 12] = [
    (EventKind::Install, "install"),
    (EventKind::Update, "update"),
    (EventKind::Remove, "remove"),
    (EventKind::Deploy, "deploy"),
    (EventKind::Unlink, "unlink"),
    (EventKind::Reset, "reset"),
    (EventKind::Backup, "backup"),
    (EventKind::Download, "download"),
    (EventKind::Launch, "launch"),
    (EventKind::Profile, "profile"),
    (EventKind::Settings, "settings"),
    (EventKind::Job, "job"),
];

impl EventKind {
    /// Name stored in the database.
    pub fn as_str(self) -> &'static str {
        KIND_NAMES
            .iter()
            .find(|(k, _)| *k == self)
            .map_or("job", |(_, n)| n)
    }

    /// Kind from its stored name (unknown names read as job).
    pub fn parse(text: &str) -> Self {
        KIND_NAMES
            .iter()
            .find(|(_, n)| *n == text)
            .map_or(EventKind::Job, |(k, _)| *k)
    }
}

impl Event {
    /// Creates an event stamped with the current time.
    pub fn new(
        severity: Severity,
        kind: EventKind,
        mod_name: Option<&str>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            ts: crate::db::now(),
            severity,
            kind,
            mod_name: mod_name.map(str::to_string),
            message: message.into(),
        }
    }
}

/// Formats a Unix timestamp as `YYYY-MM-DD HH:MM:SS` (UTC).
pub fn timestamp(ts: i64) -> String {
    let (days, secs) = (ts.div_euclid(86_400), ts.rem_euclid(86_400));
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// Journal events as plain text, one per line (for the clipboard and bug
/// reports): time, severity, kind, mod and message.
pub fn as_text(events: &[Event]) -> String {
    events
        .iter()
        .map(|e| {
            format!(
                "{} UTC  {:<5} {:<8} {}: {}\n",
                timestamp(e.ts),
                e.severity.as_str(),
                e.kind.as_str(),
                e.mod_name.as_deref().unwrap_or("-"),
                e.message
            )
        })
        .collect()
}
