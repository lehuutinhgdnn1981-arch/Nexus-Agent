//! NEXUS — observability: tracing/log init.

use std::path::Path;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Khởi tạo tracing subscriber với 2 layer:
///   - stdout tại INFO level (dev visibility)
///   - rolling daily file appender tại DEBUG level (persistent log)
///
/// Log file nằm ở `<log_dir>/app.log` (rotation daily, suffix `YYYY-MM-DD`).
pub fn init<P: AsRef<Path>>(log_dir: P) -> Result<(), tracing_subscriber::Error> {
    let log_dir = log_dir.as_ref();
    std::fs::create_dir_all(log_dir).ok();

    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "app.log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("nexus=info,tauri=info,sqlx=warn"));

    let stdout_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_target(false)
        .compact();

    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false)
        .json();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .try_init()
}

/// Khởi tạo tracing subscriber chỉ với stdout (dùng cho test / CLI example).
pub fn init_stdout() -> Result<(), tracing_subscriber::Error> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("nexus=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().compact())
        .try_init()
}
