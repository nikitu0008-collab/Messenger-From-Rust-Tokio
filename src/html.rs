use axum::response::Html;

/// Отдаёт встроенную HTML-страницу чата.
pub async fn index() -> Html<&'static str> {
    Html(include_str!("../assets/index.html"))
}
