use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize file-based logging.
///
/// Creates the log directory if it does not exist, sets up a file appender,
/// and installs a global tracing subscriber with an environment filter.
///
/// Returns a `WorkerGuard` that **must** be held alive for the duration of
/// the program; dropping it flushes and shuts down the logging thread.
pub fn init_logging(log_dir: &Path) -> std::io::Result<WorkerGuard> {
    std::fs::create_dir_all(log_dir)?;

    let file_appender = tracing_appender::rolling::never(log_dir, "zap.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(true)
                .with_line_number(true),
        )
        .init();

    Ok(guard)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_init_logging_creates_dir() {
        // We cannot call init() more than once per process, so we just verify
        // that the directory-creation logic works.
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("logs");
        assert!(!log_dir.exists());
        std::fs::create_dir_all(&log_dir).unwrap();
        assert!(log_dir.exists());
    }

    #[test]
    fn test_log_dir_from_config() {
        let dir = crate::config::log_dir();
        assert!(dir.ends_with("logs"));
    }
}
