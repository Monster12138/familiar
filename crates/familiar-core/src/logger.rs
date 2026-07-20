use anyhow::Result;
use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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
