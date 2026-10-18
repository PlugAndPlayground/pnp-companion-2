//! Per-user configuration.
//!
//! Everything the companion persists lives in one directory per user:
//! - macOS:   ~/Library/Application Support/Tailrmade
//! - Windows: %APPDATA%\Tailrmade
//! - Linux:   ~/.config/tailrmade
//!
//! `TM_CONFIG_DIR` overrides the location. The directory holds `config.json`
//! (non-secret settings) and an optional `.env` fallback for API keys.

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
};

pub const DEFAULT_PORT: u16 = 6655;
const CONFIG_FILE: &str = "config.json";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    /// Port override; `None` means the default port.
    pub port: Option<u16>,
    /// Names of custom keys saved through the settings window, so they can be
    /// listed again (the keychain cannot enumerate entries).
    pub custom_key_names: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PortSource {
    Default,
    Config,
    Environment,
}

static CONFIG: OnceLock<RwLock<Config>> = OnceLock::new();

pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("TM_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    let name = if cfg!(target_os = "linux") {
        "tailrmade"
    } else {
        "Tailrmade"
    };
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(name)
}

pub fn env_file_path() -> PathBuf {
    config_dir().join(".env")
}

fn config_file_path() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

fn cell() -> &'static RwLock<Config> {
    CONFIG.get_or_init(|| RwLock::new(load_from_disk()))
}

fn load_from_disk() -> Config {
    let path = config_file_path();
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|error| {
            tracing::warn!(path = %path.display(), %error, "Ignoring unreadable config file");
            Config::default()
        }),
        Err(_) => Config::default(),
    }
}

/// Creates the config directory and a commented `.env` template on first run.
pub fn init() {
    let dir = config_dir();
    if let Err(error) = fs::create_dir_all(&dir) {
        tracing::warn!(dir = %dir.display(), %error, "Could not create config directory");
        return;
    }
    let env_path = env_file_path();
    if !env_path.exists() {
        let template = "# Tailrmade Companion API keys (fallback).\n\
# Prefer the Settings window, which stores keys in the system keychain.\n\
# Keys set here are used when no keychain entry exists.\n\
#\n\
# ANTHROPIC_API_KEY=\n\
# OPENAI_API_KEY=\n\
# GEMINI_API_KEY=\n\
# DEEPSEEK_API_KEY=\n\
# MOONSHOT_API_KEY=\n";
        if write_private(&env_path, template).is_ok() {
            tracing::info!(path = %env_path.display(), "Created .env template");
        }
    }
    let _ = cell();
}

pub fn get() -> Config {
    cell().read().unwrap().clone()
}

pub fn update(change: impl FnOnce(&mut Config)) -> std::io::Result<()> {
    let mut guard = cell().write().unwrap();
    let mut next = guard.clone();
    change(&mut next);
    fs::create_dir_all(config_dir())?;
    let text = serde_json::to_string_pretty(&next).expect("config serializes");
    write_private(&config_file_path(), &text)?;
    *guard = next;
    Ok(())
}

pub fn port() -> (u16, PortSource) {
    if let Some(port) = std::env::var("TM_PORT").ok().and_then(|v| v.parse().ok()) {
        return (port, PortSource::Environment);
    }
    match get().port {
        Some(port) => (port, PortSource::Config),
        None => (DEFAULT_PORT, PortSource::Default),
    }
}

/// Writes a file readable only by the current user where the OS supports it.
fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(contents.as_bytes())
    }
    #[cfg(not(unix))]
    {
        fs::write(path, contents)
    }
}
