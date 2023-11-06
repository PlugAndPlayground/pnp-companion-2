#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use axum::{routing::post, Router};

use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tray_icon::menu::Menu;
use tray_icon::Icon;
use tray_icon::{
    menu::{AboutMetadata, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, TrayIconEvent,
};
mod response_handler;
use tao::event_loop::{ControlFlow, EventLoopBuilder};

static ICON: &'static [u8] = include_bytes!("../resources/pnp.png");
static PORT: u16 = 6655;

fn create_tray_icon() -> Box<dyn FnOnce() + 'static> {
    let img = image::load_from_memory(ICON).unwrap().into_rgba8();
    let (width, height) = img.dimensions();
    let icon = Icon::from_rgba(img.into_raw(), width as u32, height as u32).unwrap();
    let menu = Menu::new();
    let quit_i = MenuItem::new("Quit", true, None);
    let menu_channel = MenuEvent::receiver();
    let tray_channel = TrayIconEvent::receiver();
    menu.append_items(&[
        &PredefinedMenuItem::about(
            None,
            Some(AboutMetadata {
                name: Some("Plug and Playground Companion".to_string()),
                website: Some("https://plugandplayground.dev".to_string()),
                ..Default::default()
            }),
        ),
        &PredefinedMenuItem::separator(),
        &quit_i,
    ])
    .unwrap();
    let event_loop = EventLoopBuilder::new().build();

    let mut tray_icon = Some(
        TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("tao - awesome windowing lib")
            .with_icon(icon)
            .build()
            .unwrap(),
    );

    return Box::new(move || {
        event_loop.run(move |_event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            if let Ok(event) = menu_channel.try_recv() {
                if event.id == quit_i.id() {
                    tray_icon.take();
                    *control_flow = ControlFlow::Exit;
                }
                println!("{event:?}");
            }

            if let Ok(event) = tray_channel.try_recv() {
                println!("{event:?}");
            }
        });
    });
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // TODO fix tray icon, it crashes on mac
    #[cfg(target_os = "windows")]
    let event_loop_function = create_tray_icon();
    let _server_thread = tokio::task::spawn(start_server());

    #[cfg(target_os = "windows")]
    event_loop_function();
    #[cfg(not(target_os = "windows"))]
    server_thread.await;

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
