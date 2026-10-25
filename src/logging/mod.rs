//! Diagnostics: a rolling application log plus a structured event journal.
//!
//! Every [`Event`] goes to three sinks: the `tracing` log file
//! (`logs/pylune.log.YYYY-MM-DD`), the `events` table in SQLite, and — when it
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
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warn => "warn",
            Severity::Error => "error",
        }
    }

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
    pub fn as_str(self) -> &'static str {
        KIND_NAMES
            .iter()
            .find(|(k, _)| *k == self)
            .map_or("job", |(_, n)| n)
    }

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
