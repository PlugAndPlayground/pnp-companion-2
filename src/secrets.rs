//! API key storage.
//!
//! Lookup order for a key name:
//! 1. Process environment, only for the known provider keys (developer override)
//! 2. System keychain (macOS Keychain, Windows Credential Manager, Secret Service on Linux)
//! 3. `.env` in the per-user config directory
//! 4. Legacy `.env` next to the executable or in the working directory
//!
//! `$TM_KEY{NAME}` placeholders resolve through the same lookup. Arbitrary
//! process environment variables are deliberately not reachable that way,
//! since any allowed browser origin can put a placeholder in a request.

use crate::config;
use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, OnceLock, RwLock},
};

const KEYCHAIN_SERVICE: &str = "app.tailrmade.companion";

pub struct KnownKey {
    pub name: &'static str,
    pub label: &'static str,
}

pub const KNOWN_KEYS: &[KnownKey] = &[
    KnownKey {
        name: "ANTHROPIC_API_KEY",
        label: "Anthropic (Claude)",
    },
    KnownKey {
        name: "OPENAI_API_KEY",
        label: "OpenAI",
    },
    KnownKey {
        name: "GEMINI_API_KEY",
        label: "Google Gemini",
    },
    KnownKey {
        name: "DEEPSEEK_API_KEY",
        label: "DeepSeek",
    },
    KnownKey {
        name: "MOONSHOT_API_KEY",
        label: "Moonshot (Kimi)",
    },
];

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Source {
    Environment,
    Keychain,
    EnvFile { path: String },
}

struct EnvFile {
    path: PathBuf,
    values: HashMap<String, String>,
}

#[derive(Default)]
struct Store {
    env_files: Vec<EnvFile>,
    /// Keychain cache: `Some(None)` means "looked up, not present".
    keychain: HashMap<String, Option<String>>,
    keychain_error: Option<String>,
}

static STORE: OnceLock<RwLock<Store>> = OnceLock::new();
/// Keychain calls can display an OS authorization dialog. Keep them serialized
/// so concurrent startup/settings lookups cannot create duplicate prompts.
static KEYCHAIN_ACCESS: Mutex<()> = Mutex::new(());

fn store() -> &'static RwLock<Store> {
    STORE.get_or_init(|| {
        RwLock::new(Store {
            env_files: read_env_files(),
            ..Store::default()
        })
    })
}

fn env_file_candidates() -> Vec<PathBuf> {
    let mut paths = vec![config::env_file_path()];
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
    {
        paths.push(dir.join(".env"));
    }
    if let Ok(dir) = std::env::current_dir() {
        paths.push(dir.join(".env"));
    }
    let mut unique = Vec::new();
    for path in paths {
        let key = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !unique.iter().any(|(k, _): &(PathBuf, PathBuf)| *k == key) {
            unique.push((key, path));
        }
    }
    unique.into_iter().map(|(_, path)| path).collect()
}

fn read_env_files() -> Vec<EnvFile> {
    env_file_candidates()
        .into_iter()
        .filter_map(|path| {
            let iter = dotenvy::from_path_iter(&path).ok()?;
            let values = iter
                .filter_map(Result::ok)
                .filter(|(_, value)| !value.trim().is_empty())
                .collect::<HashMap<_, _>>();
            Some(EnvFile { path, values })
        })
        .collect()
}

/// Re-reads `.env` files. Keychain lookups remain cached for this process so a
/// settings refresh does not repeatedly ask the OS to authorize the same app.
pub fn reload() {
    let mut store = store().write().unwrap();
    store.env_files = read_env_files();
}

fn entry(name: &str) -> keyring::Result<keyring::Entry> {
    keyring::Entry::new(KEYCHAIN_SERVICE, name)
}

fn keychain_get(name: &str) -> Option<String> {
    if let Some(cached) = store().read().unwrap().keychain.get(name) {
        return cached.clone();
    }

    let _access = KEYCHAIN_ACCESS.lock().unwrap();
    // Another lookup may have populated the cache while this thread waited.
    if let Some(cached) = store().read().unwrap().keychain.get(name) {
        return cached.clone();
    }

    let result = entry(name).and_then(|e| e.get_password());
    let mut store = store().write().unwrap();
    let value = match result {
        Ok(value) => Some(value),
        Err(keyring::Error::NoEntry) => None,
        Err(error) => {
            store
                .keychain_error
                .get_or_insert_with(|| error.to_string());
            // Do not cache failures, the keychain may become available later.
            return None;
        }
    };
    store.keychain.insert(name.to_string(), value.clone());
    value
}

pub fn is_known(name: &str) -> bool {
    KNOWN_KEYS.iter().any(|k| k.name == name)
}

pub fn lookup(name: &str) -> Option<(String, Source)> {
    if is_known(name) {
        if let Ok(value) = std::env::var(name) {
            if !value.trim().is_empty() {
                return Some((value, Source::Environment));
            }
        }
    }
    if let Some(value) = keychain_get(name).filter(|v| !v.trim().is_empty()) {
        return Some((value, Source::Keychain));
    }
    let store = store().read().unwrap();
    store.env_files.iter().find_map(|file| {
        file.values.get(name).map(|value| {
            (
                value.clone(),
                Source::EnvFile {
                    path: file.path.display().to_string(),
                },
            )
        })
    })
}

pub fn get(name: &str) -> Option<String> {
    lookup(name).map(|(value, _)| value)
}

pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

pub fn set(name: &str, value: &str) -> Result<(), String> {
    if !valid_name(name) {
        return Err("Key names may only use A-Z, 0-9 and _".into());
    }
    let value = value.trim();
    if value.is_empty() {
        return Err("Value is empty".into());
    }
    let _access = KEYCHAIN_ACCESS.lock().unwrap();
    entry(name)
        .and_then(|e| e.set_password(value))
        .map_err(|error| format!("Could not save to the system keychain: {error}"))?;
    store()
        .write()
        .unwrap()
        .keychain
        .insert(name.to_string(), Some(value.to_string()));
    Ok(())
}

pub fn delete(name: &str) -> Result<(), String> {
    let _access = KEYCHAIN_ACCESS.lock().unwrap();
    match entry(name).and_then(|e| e.delete_credential()) {
        Ok(()) | Err(keyring::Error::NoEntry) => {}
        Err(error) => {
            return Err(format!(
                "Could not remove from the system keychain: {error}"
            ))
        }
    }
    store()
        .write()
        .unwrap()
        .keychain
        .insert(name.to_string(), None);
    Ok(())
}

pub fn keychain_error() -> Option<String> {
    store().read().unwrap().keychain_error.clone()
}

/// Shows only the last four characters of a key.
pub fn mask(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        return "••••".into();
    }
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("••••{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_names() {
        assert!(valid_name("ANTHROPIC_API_KEY"));
        assert!(valid_name("MY_KEY_2"));
        assert!(!valid_name(""));
        assert!(!valid_name("lower"));
        assert!(!valid_name("WITH SPACE"));
        assert!(!valid_name("A}B"));
    }

    #[test]
    fn masks_values() {
        assert_eq!(mask("short"), "••••");
        assert_eq!(mask("sk-ant-1234567890abcd"), "••••abcd");
    }

    #[test]
    fn arbitrary_process_env_is_not_exposed() {
        std::env::set_var("TM_TEST_NOT_A_KEY", "secret");
        assert!(!is_known("TM_TEST_NOT_A_KEY"));
        // Not known, not in keychain or .env files, so not resolvable.
        assert!(lookup("TM_TEST_NOT_A_KEY").map(|(_, s)| s) != Some(Source::Environment));
    }
}
