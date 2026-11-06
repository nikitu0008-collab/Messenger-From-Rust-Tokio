use serde::Serialize;

/// Сообщения, которые сервер отправляет клиенту.
///
/// Сериализуются в JSON с полем `type`, например:
/// `{"type":"message","from":"Никита","text":"Привет!"}`.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ServerEvent {
    /// Успешное подключение. Содержит итоговое имя и список участников.
    Welcome {
        username: String,
        users: Vec<String>,
    },
    /// Обычное сообщение в общий чат.
    Message { from: String, text: String },
    /// Системное уведомление (вход, выход, смена ника).
    System { text: String },
    /// Актуальный список участников онлайн.
    Users { users: Vec<String> },
    /// Подтверждение смены ника.
    Renamed { username: String },
    /// Ошибка или подсказка по команде.
    Error { text: String },
}

impl ServerEvent {
    /// Сериализует событие в JSON-строку для отправки в WebSocket.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("ServerEvent всегда сериализуется в JSON")
    }
}
