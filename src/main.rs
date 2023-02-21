use axum::{response::IntoResponse, response::Response, routing::post, Json, Router};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    let response = client.get(received.finalURL).send().await;

    let mut status_code = StatusCode::IM_A_TEAPOT;
    let return_string = match response {
        Ok(res) => {
            status_code = res.status();
            let payload = res.text().await;
            match payload {
                Ok(text) => text,
                Err(e) => String::from(e.to_string()),
            }
        }
        Err(e) => String::from(e.to_string()),
    };
    //println!("returning: {}", return_string);
    //let res = match sent {
    //    _ => "bazinga",
    //};

    //print!("{}", final_result);

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
    //finalMethod: String,
}
#[derive(Serialize)]
struct CompanionResponse {
    status: u16,
    response: String,
}
