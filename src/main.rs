//#![windows_subsystem = "windows"]
use axum::{response::IntoResponse, response::Response, routing::post, Json, Router};
use base64::{engine::general_purpose, Engine as _};
use regex::Regex;
use reqwest::{RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use std::env;
use std::{collections::HashMap, net::SocketAddr};
use tower_http::cors::CorsLayer;
use tray_icon::menu::Menu;
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};
use xml2json_rs::JsonBuilder;
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

    //start_tray_icon();
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
        .await
        .unwrap();
}
