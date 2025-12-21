use axum::response::Html;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

type UserId = u64;

#[derive(Clone)]
pub struct AppState {}

// Query parameters for POST /users/{id}
#[derive(Debug, Deserialize)]
pub struct SendMessageQuery {
    msg: String,
}

// GET /users - returns map of "id: remote address"
async fn list_users(State(state): State<AppState>) {
    println!("GET /users - listing all connected users");
    todo!();
}

// DELETE /users/{id} - disconnect specific user
async fn kick_user(State(state): State<AppState>, Path(user_id): Path<UserId>) {
    println!("DELETE /users/{} - kicking user", user_id);
    todo!();
}

// POST /users/{id}?msg=text - send message to specific user
async fn send_message_to_user(
    State(state): State<AppState>,
    Path(user_id): Path<UserId>,
    Query(params): Query<SendMessageQuery>,
) {
    println!(
        "POST /users/{}?msg={} - sending message to user",
        user_id, params.msg
    );
    todo!();
}

async fn admin_panel() -> Html<&'static str> {
    Html(include_str!("../static/admin.html"))
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(admin_panel)) // Add this
        .route("/users", get(list_users))
        .route("/users/:id", delete(kick_user))
        .route("/users/:id", post(send_message_to_user))
        .with_state(state)
}

pub async fn run_admin_server(state: AppState) -> anyhow::Result<()> {
    let app = create_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Admin HTTP server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = AppState {};

    run_admin_server(state).await?;

    Ok(())
}
