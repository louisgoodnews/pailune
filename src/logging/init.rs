//! Installs the global `tracing` subscriber writing to a daily log file.

use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

/// Starts file logging; keep the returned guard alive until exit so buffered
/// lines are flushed. `RUST_LOG` overrides the configured `level`.
pub fn init(logs_dir: &Path, level: &str) -> Option<WorkerGuard> {
    let appender = tracing_appender::rolling::daily(logs_dir, "pylune.log");
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
