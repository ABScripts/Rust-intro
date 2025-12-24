use axum::response::Html;
use axum::response::IntoResponse;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::sync::{Mutex, broadcast};
use tokio::task::JoinSet;

use client::client::client_admin::ClientAdmin;
use client::client::writer::ClientWriterActorHandle;
use protocol::client_message::ClientMessage;

type UserId = u64;

#[derive(Clone)]
pub struct AppState {
    write_to_server: ClientWriterActorHandle,
    tx_to_sse: broadcast::Sender<String>,
}

// Query parameters for POST /users/{id}
#[derive(Debug, Deserialize)]
pub struct SendMessageQuery {
    msg: String,
}

// GET /users - returns map of "id: remote address"
async fn list_users(State(state): State<AppState>) -> impl IntoResponse {
    println!("GET /users - listing all connected users");
    // todo!();
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

async fn sse_event_sender(State(state): State<AppState>) -> impl IntoResponse {
    let mut rx = state.tx_to_sse.subscribe();

    let stream = async_stream::stream! {
        while let Ok(msg) = rx.recv().await {
            tracing::info!("{msg}");
            yield Ok::<_, std::convert::Infallible>(axum::response::sse::Event::default()
                .data(msg)
                .into())
        }
    };

    axum::response::sse::Sse::new(stream)
}

async fn get_admin_panel_html() -> Html<&'static str> {
    Html(include_str!("../static/admin.html"))
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(get_admin_panel_html))
        .route("/users", get(list_users))
        .route("/api/events", get(sse_event_sender))
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
    tracing_subscriber::fmt::init();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3456));
    let default_admin_username = "admin".to_string();
    let client = ClientAdmin::connect(addr, default_admin_username).await?;

    let (tx_to_sse, _) = broadcast::channel(100);

    let state = AppState {
        write_to_server: client.get_writer(),
        tx_to_sse: tx_to_sse.clone(),
    };

    let (tx_to_server, mut rx_from_client) = tokio::sync::mpsc::channel(100);

    let mut join_set = JoinSet::new();
    // runs client operations
    join_set.spawn({
        let tx_to_server = tx_to_server.clone();
        async move {
            let _ = client.run(tx_to_server).await;
        }
    });

    // hands over messages from client reader -> server sse handler
    join_set.spawn({
        let tx_to_sse = tx_to_sse.clone();
        async move {
            while let Some(msg) = rx_from_client.recv().await {
                // we don't want to display certain annoying messages
                match &msg {
                    ClientMessage::KeepAlive(_) => {
                        tracing::trace!("Ignore keepalive event {:?}", msg);
                        continue;
                    }
                    _ => {}
                }

                let msg_str = format!("{:?}", msg);
                println!("SSE: {}", msg_str);
                let _ = tx_to_sse.send(msg_str);
            }
        }
    });

    // 1) handles sse which pipes data to UI
    // 2) handles api requests
    join_set.spawn(async move {
        let _ = run_admin_server(state).await;
    });

    Ok(join_set.join_next().await.unwrap()?)
}
