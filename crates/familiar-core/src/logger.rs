use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Platform default directory for log files.
///
/// Unix keeps the historical `/tmp` location. Windows uses the user's temp
/// directory, because `"/tmp"` would otherwise resolve to the drive root
/// (e.g. `C:\tmp`), which may not exist and is not writable by standard users.
pub fn default_log_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::temp_dir()
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/tmp")
    }
}

pub fn init_logger<P: AsRef<Path>>(log_dir: P, file_name: &str) -> Result<WorkerGuard> {
    // File appender
    let file_appender = tracing_appender::rolling::never(log_dir, file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Formatter
    let fmt_layer = fmt::layer().with_writer(non_blocking).with_target(false);

    // Standard stdout logger
    let stdout_layer = fmt::layer().with_writer(std::io::stdout);

    // Filter
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,familiar_core=debug"));

    // Register
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(stdout_layer)
        .init();

    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::default_log_dir;

    #[test]
    fn default_log_dir_exists() {
        // Guards against regressions to non-existent locations like `C:\tmp`.
        assert!(default_log_dir().exists());
    }

    #[cfg(windows)]
    #[test]
    fn default_log_dir_is_not_drive_root() {
        let dir = default_log_dir();
        assert!(dir.is_absolute());
        assert_ne!(dir.parent(), None);
        assert!(
            dir.file_name().is_some(),
            "log dir must not be a bare drive root"
        );
    }
}
