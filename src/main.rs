use axum::{response::IntoResponse, response::Response, routing::post, Json, Router};
use reqwest::{RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};
use tower_http::cors::CorsLayer;

static PORT: u16 = 6655;

#[tokio::main]
async fn main() {
    start_server().await;
}

async fn start_server() {
    // initialize tracing
    tracing_subscriber::fmt::init();
    println!("Starting server on: {}", PORT);

    // build our application with a route
    let app = Router::new()
        .route("/", post(pnp_request))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], PORT));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn pnp_request(Json(payload): Json<CompanionInput>) -> Response {
    let received = payload.to_owned();
    let client = reqwest::Client::new();

    let attach_headers = |mut to_send: RequestBuilder| -> RequestBuilder {
        for (key, value) in received.final_headers {
            to_send = to_send.header(key, value);
        }
        return to_send;
    };

    let response = match payload.final_method.as_str() {
        "Get" => {
            let mut to_send = client.get(received.final_url);
            to_send = attach_headers(to_send);
            Some(to_send.send().await)
        }
        "Post" => {
            let mut to_send: RequestBuilder = client.post(received.final_url);
            to_send = attach_headers(to_send);
            Some(to_send.body(received.final_body).send().await)
        }
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
    Json(response).into_response()
}
#[derive(Deserialize, Clone)]
struct CompanionInput {
    #[serde(rename = "finalURL")]
    final_url: String,
    #[serde(rename = "finalHeaders")]
    final_headers: HashMap<String, String>,
    #[serde(rename = "finalBody")]
    final_body: String,
    #[serde(rename = "finalMethod")]
    final_method: String,
}
#[derive(Serialize)]
struct CompanionResponse {
    status: u16,
    response: String,
}
