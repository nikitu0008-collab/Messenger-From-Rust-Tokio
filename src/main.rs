mod chat_room;
mod handle_client;
mod html;

use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};
use std::{net::SocketAddr, sync::Arc};

use chat_room::ChatRoom;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(ChatRoom::new());

    let app = Router::new()
        .route("/", get(html::index_handler))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    println!("╔══════════════════════════════════════╗");
    println!("║       Messenger Rust + Tokio         ║");
    println!("╠══════════════════════════════════════╣");
    println!("║ Server: http://{addr}       ║");
    println!("╚══════════════════════════════════════╝");

    axum::serve(listener, app).await.expect("Server error");
}

async fn ws_handler(ws: WebSocketUpgrade, State(room): State<Arc<ChatRoom>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_client::handle_socket(socket, room))
}
