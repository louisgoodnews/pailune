//! Installs the global `tracing` subscriber writing to a daily log file.

use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

/// Number of daily log files kept.
const KEEP_DAYS: usize = 14;

/// Starts file logging; keep the returned guard alive until exit so buffered
/// lines are flushed. `RUST_LOG` overrides the configured `level`.
pub fn init(logs_dir: &Path, level: &str) -> Option<WorkerGuard> {
    // Daily files, the newest two weeks kept.
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("pylune.log")
        .max_log_files(KEEP_DAYS)
        .build(logs_dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .with_thread_names(true)
        .try_init()
        .ok()
        .map(|()| guard)
}
