mod buffer;
mod chat_room;
mod handle_client;
mod html;
mod url_decode;

use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};

use std::{
    net::SocketAddr,
    sync::Arc,
};

use tokio::sync::broadcast;

pub struct AppState {
    pub tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<String>(100);

    let state = Arc::new(AppState {
        tx,
    });

    let app = Router::new()
        .route("/", get(html::index_handler))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    println!("Server started at: http://{addr}");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        handle_client::handle_socket(socket, state)
    })
}
