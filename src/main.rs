mod chat_room;
mod handle_client;
mod html;
mod protocol;

use std::{future::IntoFuture, net::SocketAddr, sync::Arc};

use axum::{
    extract::{Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Deserialize;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use crate::chat_room::ChatRoom;

const DEFAULT_PORT: u16 = 8080;

#[derive(Debug, Deserialize)]
struct JoinQuery {
    name: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).init();

    let port = match std::env::var("PORT") {
        Ok(value) => match value.parse::<u16>() {
            Ok(port) => port,
            Err(_) => {
                warn!(%value, "invalid PORT, using default {DEFAULT_PORT}");
                DEFAULT_PORT
            }
        },
        Err(_) => DEFAULT_PORT,
    };

    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let room = Arc::new(ChatRoom::new());

    let app = Router::new()
        .route("/", get(html::index))
        .route("/chat", get(ws_handler))
        .route("/ws", get(ws_handler))
        .route("/health", get(health))
        .fallback(not_found)
        .with_state(room);

    let listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => {
            error!(%error, %address, "failed to bind TCP listener");
            std::process::exit(1);
        }
    };

    info!("Messenger Rust + Tokio");
    info!("Open http://localhost:{port} in your browser");
    info!("WebSocket endpoint: /ws?name=<nickname>");

    let server = axum::serve(listener, app).into_future();

    tokio::select! {
        result = server => {
            if let Err(error) = result {
                error!(%error, "server error");
            }
        }
        _ = shutdown_signal() => {
            info!("server stopped");
        }
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(room): State<Arc<ChatRoom>>,
    Query(query): Query<JoinQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_client::handle_socket(socket, room, query.name))
}

async fn health() -> &'static str {
    "ok"
}

async fn not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "404 Not Found")
}

async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_ok() {
        info!("shutdown signal received");
    }
}
