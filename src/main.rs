#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use axum::{routing::get, routing::post, Router};
use frontend_handler::SharedFrontendPackage;
use response_handler::init_environment;
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;
use tray_icon::menu::Menu;
use tray_icon::Icon;
use tray_icon::{
    menu::{AboutMetadata, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, TrayIconEvent,
};

mod ai_handler;
mod frontend_handler;
mod response_handler;

use tao::event_loop::{ControlFlow, EventLoopBuilder};

static ICON: &[u8] = include_bytes!("../resources/tm.png");
const DEFAULT_PORT: u16 = 6655;

fn tm_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

fn create_tray_icon(port: u16, has_frontend: bool) -> Box<dyn FnOnce() + 'static> {
    let img = image::load_from_memory(ICON).unwrap().into_rgba8();
    let (width, height) = img.dimensions();
    let icon = Icon::from_rgba(img.into_raw(), width, height).unwrap();

    let menu = Menu::new();
    let open_i = MenuItem::new("Open Tailrmade", true, None);
    let quit_i = MenuItem::new("Quit", true, None);
    let menu_channel = MenuEvent::receiver();
    let tray_channel = TrayIconEvent::receiver();

    let about = PredefinedMenuItem::about(
        None,
        Some(AboutMetadata {
            name: Some("Tailrmade Companion".to_string()),
            website: Some("https://tailrmade.dev".to_string()),
            ..Default::default()
        }),
    );
    let separator = PredefinedMenuItem::separator();
    if has_frontend {
        menu.append_items(&[&about, &separator, &open_i, &quit_i])
            .unwrap();
    } else {
        menu.append_items(&[&about, &separator, &quit_i]).unwrap();
    }

    let event_loop = EventLoopBuilder::new().build();
    let mut tray_icon = Some(
        TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(if has_frontend {
                format!("Tailrmade Local running on port {port}")
            } else {
                format!("TM Companion running on port {port}")
            })
            .with_icon(icon)
            .build()
            .unwrap(),
    );

    Box::new(move || {
        event_loop.run(move |_event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            if let Ok(event) = menu_channel.try_recv() {
                if event.id == open_i.id() {
                    let _ = webbrowser::open(&tm_url(port));
                } else if event.id == quit_i.id() {
                    tray_icon.take();
                    *control_flow = ControlFlow::Exit;
                }
                println!("{event:?}");
            }

            if let Ok(event) = tray_channel.try_recv() {
                println!("{event:?}");
            }
        });
    })
}

#[tokio::main]
async fn main() {
    init_environment();
    let port = std::env::var("TM_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let frontend: SharedFrontendPackage = Arc::new(frontend_handler::FrontendPackage::discover());
    if let Some(package) = frontend.as_ref() {
        println!("Loaded {}", package.description());
    } else {
        println!("No TM package found; running companion-only mode");
    }
    let has_frontend = frontend.is_some();

    // TODO fix tray icon, it crashes on mac
    #[cfg(target_os = "windows")]
    let event_loop_function = create_tray_icon(port, has_frontend);

    let _server_thread = tokio::task::spawn(start_server(port, frontend));

    #[cfg(target_os = "windows")]
    {
        if has_frontend && std::env::var_os("TM_NO_OPEN").is_none() {
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let _ = webbrowser::open(&tm_url(port));
            });
        }
        event_loop_function();
    }

    #[cfg(not(target_os = "windows"))]
    _server_thread.await.unwrap();

    std::process::exit(0);
}

async fn start_server(port: u16, frontend: SharedFrontendPackage) {
    tracing_subscriber::fmt::init();
    println!("Starting server on: {port}");

    // build our application with a route
    let app = Router::new()
        .route("/forward", post(response_handler::tm_request))
        .route("/ping", get(response_handler::ping))
        .route("/ai/request", post(ai_handler::request))
        .route("/ai/claude-stream", post(ai_handler::claude_stream))
        .fallback(get(frontend_handler::serve))
        .layer(CorsLayer::permissive())
        .with_state(frontend);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::debug!("listening on {}", addr);

    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
