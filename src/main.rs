#![windows_subsystem = "windows"]
use axum::{routing::post, Router};

use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tray_icon::menu::{Menu, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};
mod response_handler;

static ICON: &'static [u8] = include_bytes!("../resources/pnp.png");
static PORT: u16 = 6655;

fn create_tray_icon() -> tray_icon::TrayIcon {
    let img = image::load_from_memory(ICON).unwrap().into_rgba8();
    let (width, height) = img.dimensions();
    let icon = Icon::from_rgba(img.into_raw(), width as u32, height as u32).unwrap();
    let menu = Menu::new();
    // TODO FIX TRAY ICON OPENING https://github.com/tauri-apps/tray-icon/issues/89

    return TrayIconBuilder::new()
        .with_tooltip(format!("PNP companion running on port {}", PORT))
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .unwrap();
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // TODO fix tray icon, it crashes on mac
    #[cfg(target_os = "windows")]
    let _tray_icon = create_tray_icon();
    tokio::task::spawn(start_server());

    // Capture Ctrl+C to shutdown gracefully
    let ctrl_c_task = tokio::signal::ctrl_c();

    tokio::select! {
        _ = ctrl_c_task => {
            println!("Received Ctrl+C. Shutting down...");
        }
    }

    // Exit the program
    std::process::exit(0);
}

async fn start_server() {
    tracing_subscriber::fmt::init();
    println!("Starting server on: {}", PORT);

    // build our application with a route
    let app = Router::new()
        .route("/", post(response_handler::pnp_request))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], PORT));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        //        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
