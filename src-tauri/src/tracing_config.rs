//! Unified tracing configuration for Codictate
//!
//! Provides structured logging with session correlation,
//! dual output to stdout (colored) and file (plain),
//! non-blocking file writes, and dynamic log level changes.

use once_cell::sync::OnceCell;
use std::sync::Mutex;

use tracing::Level;
use tracing_appender::{
    non_blocking::{NonBlockingBuilder, WorkerGuard},
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Global guard to keep the non-blocking writer alive
static WORKER_GUARD: OnceCell<Mutex<Option<WorkerGuard>>> = OnceCell::new();

/// Current file log level (modified at runtime via atomic)
static FILE_LOG_LEVEL: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(4);

fn level_to_u8(level: Level) -> u8 {
    match level {
        Level::ERROR => 1,
        Level::WARN => 2,
        Level::INFO => 3,
        Level::DEBUG => 4,
        Level::TRACE => 5,
    }
}

/// Set the file log level dynamically at runtime.
/// Note: Due to tracing's static layer architecture, this affects new spans/events
/// but requires a filter that reads this value dynamically.
pub fn set_file_log_level(level: Level) {
    FILE_LOG_LEVEL.store(level_to_u8(level), std::sync::atomic::Ordering::Relaxed);
    tracing::info!("File log level changed to {:?}", level);
}

/// Initialize the tracing subscriber with dual output:
/// - Stdout: Colored, respects RUST_LOG env var
/// - File: Plain text, daily rotation, 7 days retention, non-blocking
///
/// Returns Ok(()) on success. The worker guard is stored globally.
pub fn init_tracing(log_dir: &std::path::Path) -> anyhow::Result<()> {
    // Create file appender with daily rotation
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .max_log_files(7)
        .filename_prefix("codictate")
        .filename_suffix("log")
        .build(log_dir)?;

    // Wrap with non-blocking writer for async performance
    let (non_blocking_writer, guard) = NonBlockingBuilder::default()
        .lossy(false) // Don't drop logs under pressure
        .finish(file_appender);

    // Store guard globally to prevent dropping
    WORKER_GUARD.get_or_init(|| Mutex::new(Some(guard)));

    // Console layer: colored, respects RUST_LOG
    let console_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let console_layer = fmt::layer()
        .with_ansi(true)
        .with_target(true)
        .with_level(true)
        .compact()
        .with_filter(console_filter);

    // File layer: plain text with dynamic level filtering
    // Reads FILE_LOG_LEVEL atomic to allow runtime changes
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_level(true)
        .with_writer(non_blocking_writer)
        .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
            let current_level = FILE_LOG_LEVEL.load(std::sync::atomic::Ordering::Relaxed);
            let event_level = match *metadata.level() {
                Level::ERROR => 1,
                Level::WARN => 2,
                Level::INFO => 3,
                Level::DEBUG => 4,
                Level::TRACE => 5,
            };
            event_level <= current_level
        }));

    // Combine layers
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!("Tracing initialized, log dir: {}", log_dir.display());

    Ok(())
}

/// Log a message from the frontend with session correlation.
/// This allows frontend logs to appear in the unified log file.
pub fn log_from_frontend(level: &str, session_id: Option<&str>, target: &str, message: &str) {
    let session = session_id.unwrap_or("-");

    match level.to_lowercase().as_str() {
        "error" => tracing::error!(session = session, target = target, "{}", message),
        "warn" => tracing::warn!(session = session, target = target, "{}", message),
        "info" => tracing::info!(session = session, target = target, "{}", message),
        "debug" => tracing::debug!(session = session, target = target, "{}", message),
        "trace" => tracing::trace!(session = session, target = target, "{}", message),
        _ => tracing::info!(session = session, target = target, "{}", message),
    }
}
