use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use axum::http::StatusCode;
use tokio;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase", serialize = "camelCase"))]
struct SetInput {
    machine_id: String,
    is_on: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct GetOutput {
    example: String,
}

async fn set_data(Json(body): Json<SetInput>) {
    println!("{body:?}");
}

async fn get_data() -> Result<Json<GetOutput>, StatusCode> {
    Ok(Json::from(GetOutput {
        example: "hello".to_string(),
    }))
}

async fn index() -> axum::response::Html<String> {
    axum::response::Html(std::fs::read_to_string("./pages/index.html").unwrap())
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/get", get(get_data))
        .route("/set", post(set_data).get(index));

    axum::serve(
        tokio::net::TcpListener::bind("localhost:3000")
            .await
            .unwrap(),
        app,
    )
    .await
    .unwrap();
}
