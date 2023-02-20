use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    startServer().await;
}

async fn startServer() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", post(pnpRequest))
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

// basic handler that responds with a static string
async fn root() -> &'static str {
    println!("gettin");
    "Hello, World!"
}

/*async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    // insert your application logic here
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}*/

async fn pnpRequest(Json(payload): Json<CompanionInput>) -> (StatusCode, Json<CompanionResponse>) {
    let response = CompanionResponse { success: true };
    (StatusCode::CREATED, Json(response))
}
#[derive(Deserialize)]
struct CompanionInput {
    waddup: bool,
}
#[derive(Serialize)]
struct CompanionResponse {
    success: bool,
}

// the input to our `create_user` handler
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
struct User {
    id: u64,
    username: String,
}
