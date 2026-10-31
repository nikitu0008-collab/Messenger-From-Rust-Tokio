mod ChatRoom;
mod Buffer;
mod HandleClient;
mod HTML;
mod URL_DECODE;

use axum::{
    response::Html,
    routing::get,
    Router,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let html_lines: Vec<String> = HTML::getHTMLVector();
    
    let full_html: String = html_lines.join("\n");

    let app = Router::new().route(
        "/", get(move||async move{
            Html(full_html)
        }),
    );
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    println!("Server started from: http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
