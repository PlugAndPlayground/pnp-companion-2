//#![windows_subsystem = "windows"]
use axum::{routing::post, Router};

use std::net::SocketAddr;
use tokio::signal;
use tower_http::cors::CorsLayer;
use tray_icon::menu::Menu;
use tray_icon::{Icon, TrayIconBuilder};
mod response_handler;

static ICON: &'static [u8] = include_bytes!("../resources/pnp.png");
static PORT: u16 = 6655;

#[tokio::main]
async fn main() {
    start_server().await;
}

fn start_tray_icon() {
    //std::thread::spawn(|| {
    let img = image::load_from_memory(ICON).unwrap().into_rgba8();
    let (width, height) = img.dimensions();
    let icon = Icon::from_rgba(img.into_raw(), width as u32, height as u32).unwrap();
    //let icon = Icon::from_rgba(ICON.to_vec(), 16, 16).unwrap();
    let menu = Menu::new();
    let _tray_icon = TrayIconBuilder::new()
        .with_tooltip(format!("PNP companion running on port {}", PORT))
        .with_icon(icon)
        //.with_menu(Box::new(menu))
        .build()
        .unwrap();

    // Create the menu
    //});
}

async fn start_server() {
    let img = image::load_from_memory(ICON).unwrap().into_rgba8();
    let (width, height) = img.dimensions();
    let icon = Icon::from_rgba(img.into_raw(), width as u32, height as u32).unwrap();
    //let icon = Icon::from_rgba(ICON.to_vec(), 16, 16).unwrap();
    let menu = Menu::new();
    let _tray_icon = TrayIconBuilder::new()
        .with_tooltip(format!("PNP companion running on port {}", PORT))
        .with_icon(icon)
        //.with_menu(Box::new(menu))
        .build()
        .unwrap();

    //start_tray_icon(); // TODO FIX THIS
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
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}
