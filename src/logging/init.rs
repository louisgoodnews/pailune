//! Installs the global `tracing` subscriber writing to a daily log file.

use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

/// Starts file logging; keep the returned guard alive until exit so buffered
/// lines are flushed. `RUST_LOG` overrides the configured `level`.
pub fn init(logs_dir: &Path, level: &str, keep_days: usize) -> Option<WorkerGuard> {
    // Daily files, the newest `keep_days` kept.
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("pailune.log")
        .max_log_files(keep_days.max(1))
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
