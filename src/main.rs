#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai_handler;
mod config;
mod desktop;
mod frontend_handler;
mod logging;
mod origin_policy;
mod response_handler;
mod secrets;
mod server;

use std::sync::Arc;

fn main() {
    logging::init();
    config::init();
    logging::set_verbose(config::get().verbose_logging);

    let headless = std::env::args().any(|arg| arg == "--headless")
        || std::env::var_os("TM_HEADLESS").is_some();

    if headless {
        run_headless();
    } else {
        desktop::run();
    }
}

/// Server only: no tray, no windows. For Linux servers, containers and scripts.
fn run_headless() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        tokio::task::spawn_blocking(secrets::prewarm);
        let frontend = Arc::new(frontend_handler::FrontendPackage::discover(None));
        match frontend.as_ref() {
            Some(package) => tracing::info!("Serving {}", package.description()),
            None => tracing::info!("No TM package found; running companion-only mode"),
        }
        tracing::info!("Config directory: {}", config::config_dir().display());
        let (port, _) = config::port();
        if server::run(port, frontend).await.is_err() {
            std::process::exit(1);
        }
    });
}
