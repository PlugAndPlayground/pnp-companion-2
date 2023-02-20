use axum::{
    http::StatusCode, response::IntoResponse, response::Response, routing::post, Json, Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
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
        // `GET /` goes to `root`
        .route("/", post(pnp_request))
        .layer(CorsLayer::permissive());
    // `POST /users` goes to `create_user`
    //.route("/users", post(create_user));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 6655));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn pnp_request(Json(payload): Json<CompanionInput>) -> Response {
    let received = payload;
    let client = reqwest::Client::new();

    /*let fun = match received.finalMethod.as_str() {
        "Post" => client.post,
        _ => client.get,
    };*/

    let final_result = client.get(received.finalURL);

    let response = CompanionResponse {
        status: 200,
        response: String::from("all good boss"),
    };
    //println!("Handling response to {:#?}", payload.);
    Json(response).into_response()
}
#[derive(Deserialize, Clone)]
struct CompanionInput {
    finalURL: String,
    finalHeaders: String,
    finalBody: String,
    finalMethod: String,
}
#[derive(Serialize)]
struct CompanionResponse {
    status: i32,
    response: String,
}
