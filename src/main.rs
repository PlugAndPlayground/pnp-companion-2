use axum::{response::IntoResponse, response::Response, routing::post, Json, Router};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::{collections::HashMap, error::Error};
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    start_server().await;
}

async fn start_server() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        .route("/", post(pnp_request))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], 6655));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn pnp_request(Json(payload): Json<CompanionInput>) -> Response {
    let received = payload.to_owned();
    let client = reqwest::Client::new();

    let response = match payload.finalMethod.as_str() {
        "Get" => Some(client.get(received.finalURL).send().await),
        "Post" => Some(client.post(received.finalURL).send().await),
        _ => None,
    };

    let mut status_code = StatusCode::IM_A_TEAPOT;
    let return_string = match response {
        Some(res) => match res {
            Ok(res) => {
                status_code = res.status();
                let payload = res.text().await;
                match payload {
                    Ok(text) => text,
                    Err(e) => String::from(e.to_string()),
                }
            }
            Err(e) => String::from(e.to_string()),
        },
        None => String::from("{\"Response\":\"Invalid method\"}"),
    };

    let response = CompanionResponse {
        status: status_code.as_u16(),
        response: return_string,
    };
    //println!("Handling response to {:#?}", payload.);
    Json(response).into_response()
}
#[derive(Deserialize, Clone)]
struct CompanionInput {
    finalURL: String,
    //finalHeaders: HashMap<String, String>,
    //finalBody: HashMap<String, String>,
    finalMethod: String,
}
#[derive(Serialize)]
struct CompanionResponse {
    status: u16,
    response: String,
}
