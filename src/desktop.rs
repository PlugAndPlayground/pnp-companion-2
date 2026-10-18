//! Desktop shell: tray icon, settings window, autostart and updates.

use crate::{config, frontend_handler, secrets, server};
use serde::Serialize;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, Wry,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;

const TRAY_ID: &str = "main";
const SETTINGS_WINDOW: &str = "settings";
const BACKGROUND_ARG: &str = "--background";
const HOSTED_URL: &str = "https://tailrmade.app";

/// Updater public key, provided at build time by CI. Without it the updater
/// is left out and the settings window hides the update button.
const UPDATER_PUBKEY: Option<&str> = option_env!("TM_UPDATER_PUBKEY");

struct AppState {
    frontend: frontend_handler::SharedFrontendPackage,
    status_item: MenuItem<Wry>,
}

pub fn run() {
    let mut builder = tauri::Builder::default()
        // Must be registered first: a second launch focuses the running one.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            open_settings(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![BACKGROUND_ARG]),
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());

    if let Some(pubkey) = updater_pubkey() {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().pubkey(pubkey).build());
    }

    let app = builder
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_key,
            remove_key,
            set_port,
            set_autostart,
            open_config_dir,
            open_tailrmade,
            restart,
            check_update,
            install_update,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let bundled = app.path().resource_dir().ok().map(|dir| dir.join("tm"));
            let frontend = Arc::new(frontend_handler::FrontendPackage::discover(bundled));
            match frontend.as_ref() {
                Some(package) => tracing::info!("Serving {}", package.description()),
                None => tracing::info!("No TM package found; running companion-only mode"),
            }
            tracing::info!("Config directory: {}", config::config_dir().display());

            let status_item = build_tray(app.handle())?;
            app.manage(AppState { frontend: frontend.clone(), status_item });

            tauri::async_runtime::spawn_blocking(secrets::prewarm);
            start_server(app.handle().clone(), frontend.clone());

            let background = std::env::args().any(|arg| arg == BACKGROUND_ARG);
            if !background && std::env::var_os("TM_NO_OPEN").is_none() {
                if frontend.is_some() {
                    open_tailrmade_url(app.handle());
                } else if !any_key_configured() {
                    open_settings(app.handle());
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Tailrmade Companion");

    app.run(|_app, event| {
        // Closing the settings window must not quit the companion. Explicit
        // exits (Quit menu, restart) carry an exit code and pass through.
        if let RunEvent::ExitRequested { code: None, api, .. } = event {
            api.prevent_exit();
        }
    });
}

fn updater_pubkey() -> Option<&'static str> {
    UPDATER_PUBKEY.map(str::trim).filter(|key| !key.is_empty())
}

fn any_key_configured() -> bool {
    secrets::KNOWN_KEYS
        .iter()
        .any(|key| secrets::lookup(key.name).is_some())
}

fn build_tray(app: &AppHandle) -> tauri::Result<MenuItem<Wry>> {
    let status = MenuItem::with_id(app, "status", "Starting…", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open Tailrmade", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Tailrmade Companion", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let separator2 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(app, &[&status, &separator, &open, &settings, &separator2, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("bundle icon configured in tauri.conf.json");

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Tailrmade Companion")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => open_tailrmade_url(app),
            "settings" => open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(status)
}

fn start_server(app: AppHandle, frontend: frontend_handler::SharedFrontendPackage) {
    let (port, _) = config::port();
    // Report "running" once the listener is bound, which happens immediately.
    let reporter = app.clone();
    tauri::async_runtime::spawn(async move {
        for _ in 0..50 {
            if !matches!(server::status(), server::Status::Starting) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        refresh_tray(&reporter);
    });
    tauri::async_runtime::spawn(async move {
        if let Err(error) = server::run(port, frontend).await {
            refresh_tray(&app);
            app.dialog()
                .message(error)
                .title("Tailrmade Companion")
                .kind(MessageDialogKind::Error)
                .show(|_| {});
        }
    });
}

fn refresh_tray(app: &AppHandle) {
    let (text, tooltip) = match server::status() {
        server::Status::Starting => ("Starting…".to_string(), "Tailrmade Companion".to_string()),
        server::Status::Running { port } => (
            format!("Running on port {port}"),
            format!("Tailrmade Companion – port {port}"),
        ),
        server::Status::Failed { port, .. } => (
            format!("Not running (port {port} unavailable)"),
            "Tailrmade Companion – not running".to_string(),
        ),
    };
    if let Some(state) = app.try_state::<AppState>() {
        let _ = state.status_item.set_text(text);
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

/// The locally served Tailrmade when a package is bundled, the hosted app otherwise.
fn tailrmade_url(app: &AppHandle) -> String {
    let local = app
        .try_state::<AppState>()
        .is_some_and(|state| state.frontend.is_some());
    if !local {
        return HOSTED_URL.to_string();
    }
    let port = match server::status() {
        server::Status::Running { port } => port,
        _ => config::port().0,
    };
    format!("http://localhost:{port}")
}

fn open_tailrmade_url(app: &AppHandle) {
    let _ = app.opener().open_url(tailrmade_url(app), None::<&str>);
}

pub fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    match WebviewWindowBuilder::new(app, SETTINGS_WINDOW, WebviewUrl::App("settings.html".into()))
        .title("Tailrmade Companion")
        .inner_size(560.0, 720.0)
        .min_inner_size(420.0, 480.0)
        .center()
        .build()
    {
        Ok(window) => {
            let _ = window.set_focus();
        }
        Err(error) => tracing::error!(%error, "Could not open settings window"),
    }
}

// ---------------------------------------------------------------------------
// Commands for the settings window
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyInfo {
    name: String,
    label: Option<String>,
    known: bool,
    source: Option<secrets::Source>,
    masked: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsState {
    version: String,
    server: server::Status,
    port: u16,
    port_source: config::PortSource,
    restart_needed: bool,
    config_dir: String,
    env_file: String,
    frontend: Option<String>,
    autostart: bool,
    updater_enabled: bool,
    keychain_error: Option<String>,
    keys: Vec<KeyInfo>,
}

fn key_info(name: &str, label: Option<&str>, known: bool) -> KeyInfo {
    let found = secrets::lookup(name);
    KeyInfo {
        name: name.to_string(),
        label: label.map(String::from),
        known,
        masked: found.as_ref().map(|(value, _)| secrets::mask(value)),
        source: found.map(|(_, source)| source),
    }
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_settings(app: AppHandle) -> Result<SettingsState, String> {
    let keys = blocking(|| {
        secrets::reload();
        let mut keys: Vec<KeyInfo> = secrets::KNOWN_KEYS
            .iter()
            .map(|key| key_info(key.name, Some(key.label), true))
            .collect();
        for name in config::get().custom_key_names {
            if !secrets::is_known(&name) {
                keys.push(key_info(&name, None, false));
            }
        }
        keys
    })
    .await?;

    let (port, port_source) = config::port();
    let status = server::status();
    let restart_needed = match status {
        server::Status::Running { port: running } => running != port,
        server::Status::Failed { port: tried, .. } => tried != port,
        server::Status::Starting => false,
    };
    let state = app.state::<AppState>();
    Ok(SettingsState {
        version: app.package_info().version.to_string(),
        server: status,
        port,
        port_source,
        restart_needed,
        config_dir: config::config_dir().display().to_string(),
        env_file: config::env_file_path().display().to_string(),
        frontend: state
            .frontend
            .as_ref()
            .as_ref()
            .map(|package| package.root().display().to_string()),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        updater_enabled: updater_pubkey().is_some(),
        keychain_error: secrets::keychain_error(),
        keys,
    })
}

#[tauri::command]
async fn save_key(name: String, value: String) -> Result<(), String> {
    let name = name.trim().to_uppercase();
    blocking(move || {
        secrets::set(&name, &value)?;
        if !secrets::is_known(&name) {
            config::update(|config| {
                if !config.custom_key_names.contains(&name) {
                    config.custom_key_names.push(name.clone());
                }
            })
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    })
    .await?
}

#[tauri::command]
async fn remove_key(name: String) -> Result<(), String> {
    blocking(move || {
        secrets::delete(&name)?;
        if !secrets::is_known(&name) {
            config::update(|config| config.custom_key_names.retain(|n| n != &name))
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    })
    .await?
}

#[tauri::command]
fn set_port(port: Option<u16>) -> Result<(), String> {
    if let Some(port) = port {
        if port < 1024 {
            return Err("Use a port between 1024 and 65535".into());
        }
    }
    let port = port.filter(|p| *p != config::DEFAULT_PORT);
    config::update(|config| config.port = port).map_err(|error| error.to_string())
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|error| error.to_string())
}

#[tauri::command]
fn open_config_dir(app: AppHandle) -> Result<(), String> {
    let dir = config::config_dir();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn open_tailrmade(app: AppHandle) {
    open_tailrmade_url(&app);
}

#[tauri::command]
fn restart(app: AppHandle) {
    app.restart();
}

#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_updater::UpdaterExt;
    if updater_pubkey().is_none() {
        return Err("Updates are not enabled in this build".into());
    }
    let update = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string())?;
    Ok(update.map(|update| update.version))
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let update = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string())?
        .ok_or("Already up to date")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| error.to_string())?;
    app.restart();
}
