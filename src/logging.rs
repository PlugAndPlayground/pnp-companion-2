//! Logging to stdout and to daily log files in `<config dir>/logs`.
//!
//! Logs carry metadata only: providers, models, status codes, timings, token
//! counts, paths and hosts. Never log API keys, request or response bodies,
//! full URLs or file contents, at any level.
//!
//! The default level is INFO. "Verbose logging" in settings switches the
//! companion's own logs to DEBUG without a restart. `RUST_LOG` overrides both.

use crate::config;
use std::{fs, path::PathBuf, sync::OnceLock};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt, layer::SubscriberExt, reload, util::SubscriberInitExt, EnvFilter, Registry,
};

const KEEP_LOG_FILES: usize = 7;

static FILTER: OnceLock<reload::Handle<EnvFilter, Registry>> = OnceLock::new();

pub fn log_dir() -> PathBuf {
    config::config_dir().join("logs")
}

/// Installs the global subscriber. Call once, before anything logs.
pub fn init() {
    let (filter, handle) = reload::Layer::new(filter(false));
    let dir = log_dir();
    let file = fs::create_dir_all(&dir)
        .map_err(|error| error.to_string())
        .and_then(|_| {
            RollingFileAppender::builder()
                .rotation(Rotation::DAILY)
                .filename_prefix("companion")
                .filename_suffix("log")
                .max_log_files(KEEP_LOG_FILES)
                .build(&dir)
                .map_err(|error| error.to_string())
        });
    let file_error = file.as_ref().err().cloned();

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .with(
            file.ok()
                .map(|file| fmt::layer().with_ansi(false).with_writer(file)),
        )
        .init();
    let _ = FILTER.set(handle);

    if let Some(error) = file_error {
        tracing::warn!(dir = %dir.display(), %error, "Could not open log file; logging to stdout only");
    }
}

/// Switches between INFO and DEBUG. Has no effect when `RUST_LOG` is set.
pub fn set_verbose(verbose: bool) {
    if let Some(handle) = FILTER.get() {
        let _ = handle.reload(filter(verbose));
    }
}

fn filter(verbose: bool) -> EnvFilter {
    if let Ok(filter) = EnvFilter::try_from_default_env() {
        return filter;
    }
    EnvFilter::new(if verbose {
        "info,tm_companion=debug"
    } else {
        "info"
    })
}
