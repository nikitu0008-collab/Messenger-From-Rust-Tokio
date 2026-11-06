use std::collections::HashMap;

use tokio::sync::{mpsc, RwLock};

use crate::protocol::ServerEvent;

/// Максимальная длина ника в символах (не в байтах).
pub const MAX_NAME_CHARS: usize = 24;

/// Результат успешного подключения к комнате.
#[derive(Debug, Clone)]
pub struct Joined {
    pub id: u64,
    pub username: String,
}

/// Ошибки смены ника.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameError {
    Empty,
    Taken,
    NotFound,
}

impl RenameError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Empty => "Ник не может быть пустым.",
            Self::Taken => "Этот ник уже занят.",
            Self::NotFound => "Соединение не найдено.",
        }
    }
}

struct Client {
    username: String,
    tx: mpsc::UnboundedSender<String>,
}

struct RoomState {
    clients: HashMap<u64, Client>,
    next_id: u64,
    next_guest: u64,
}

impl RoomState {
    fn is_name_taken(&self, name: &str, except_id: Option<u64>) -> bool {
        let name = name.to_lowercase();
        self.clients
            .iter()
            .any(|(id, client)| Some(*id) != except_id && client.username.to_lowercase() == name)
    }

    fn unique_name(&self, base: &str) -> String {
        if !self.is_name_taken(base, None) {
            return base.to_string();
        }

        for suffix in 2..10_000 {
            let suffix = format!(" ({suffix})");
            let keep = MAX_NAME_CHARS.saturating_sub(suffix.chars().count());
            let mut candidate: String = base.chars().take(keep).collect();
            candidate.push_str(&suffix);

            if !self.is_name_taken(&candidate, None) {
                return candidate;
            }
        }

        // Практически недостижимый запасной вариант.
        let mut id = self.next_id;
        loop {
            let candidate = format!("Гость{id}");
            if !self.is_name_taken(&candidate, None) {
                return candidate;
            }
            id += 1;
        }
    }
}

/// Общая комната чата: хранит подключённых клиентов и рассылает им события.
pub struct ChatRoom {
    state: RwLock<RoomState>,
}

impl Default for ChatRoom {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatRoom {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(RoomState {
                clients: HashMap::new(),
                next_id: 1,
                next_guest: 1,
            }),
        }
    }

    /// Подключает клиента. Если имя не передано или невалидно, выдаёт `ГостьN`.
    /// При совпадении имён добавляет числовой суффикс.
    pub async fn join(
        &self,
        requested_name: Option<&str>,
        tx: mpsc::UnboundedSender<String>,
    ) -> Joined {
        let mut state = self.state.write().await;

        let base = match requested_name.and_then(sanitize_name) {
            Some(name) => name,
            None => {
                let guest = format!("Гость{}", state.next_guest);
                state.next_guest += 1;
                guest
            }
        };

        let username = state.unique_name(&base);
        let id = state.next_id;
        state.next_id += 1;

        state.clients.insert(
            id,
            Client {
                username: username.clone(),
                tx,
            },
        );

        Joined { id, username }
    }

    /// Удаляет клиента и возвращает его последний ник.
    pub async fn leave(&self, id: u64) -> Option<String> {
        self.state
            .write()
            .await
            .clients
            .remove(&id)
            .map(|client| client.username)
    }

    /// Меняет ник клиента, проверяя корректность и уникальность.
    pub async fn rename(&self, id: u64, requested_name: &str) -> Result<String, RenameError> {
        let name = sanitize_name(requested_name).ok_or(RenameError::Empty)?;
        let mut state = self.state.write().await;

        if !state.clients.contains_key(&id) {
            return Err(RenameError::NotFound);
        }

        if state.is_name_taken(&name, Some(id)) {
            return Err(RenameError::Taken);
        }

        let client = state.clients.get_mut(&id).expect("клиент проверен выше");

        client.username = name.clone();
        Ok(name)
    }

    /// Отсортированный список ников онлайн.
    pub async fn users(&self) -> Vec<String> {
        let state = self.state.read().await;
        let mut users: Vec<String> = state
            .clients
            .values()
            .map(|client| client.username.clone())
            .collect();
        users.sort_by_key(|name| name.to_lowercase());
        users
    }

    pub async fn client_count(&self) -> usize {
        self.state.read().await.clients.len()
    }

    /// Рассылает событие всем клиентам.
    pub async fn broadcast(&self, event: &ServerEvent) {
        self.broadcast_inner(None, event).await;
    }

    /// Рассылает событие всем, кроме указанного клиента.
    pub async fn broadcast_except(&self, except_id: u64, event: &ServerEvent) {
        self.broadcast_inner(Some(except_id), event).await;
    }

    /// Рассылает актуальный список участников.
    pub async fn broadcast_users(&self) {
        let state = self.state.read().await;
        let mut users: Vec<String> = state
            .clients
            .values()
            .map(|client| client.username.clone())
            .collect();
        users.sort_by_key(|name| name.to_lowercase());

        let payload = ServerEvent::Users { users }.to_json();
        for client in state.clients.values() {
            let _ = client.tx.send(payload.clone());
        }
    }

    async fn broadcast_inner(&self, except_id: Option<u64>, event: &ServerEvent) {
        let payload = event.to_json();
        let state = self.state.read().await;

        for (id, client) in &state.clients {
            if Some(*id) == except_id {
                continue;
            }
            // Ошибка отправки означает, что получатель уже отключился.
            // Его удалит задача обработки соединения.
            let _ = client.tx.send(payload.clone());
        }
    }
}

/// Очищает ник: убирает управляющие символы, обрезает пробелы и длину.
pub fn sanitize_name(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|ch| !ch.is_control())
        .collect::<String>()
        .trim()
        .to_string();

    if cleaned.is_empty() {
        return None;
    }

    let name: String = cleaned.chars().take(MAX_NAME_CHARS).collect();
    let name = name.trim().to_string();

    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel() -> (
        mpsc::UnboundedSender<String>,
        mpsc::UnboundedReceiver<String>,
    ) {
        mpsc::unbounded_channel()
    }

    #[test]
    fn sanitize_name_trims_and_limits() {
        assert_eq!(sanitize_name("  Никита  ").as_deref(), Some("Никита"));
        assert_eq!(sanitize_name("   "), None);
        assert_eq!(sanitize_name("\n\t"), None);

        let long = "a".repeat(MAX_NAME_CHARS + 10);
        assert_eq!(
            sanitize_name(&long).unwrap().chars().count(),
            MAX_NAME_CHARS
        );
    }

    #[tokio::test]
    async fn guests_receive_incrementing_names() {
        let room = ChatRoom::new();
        let (tx1, _rx1) = channel();
        let (tx2, _rx2) = channel();

        let first = room.join(None, tx1).await;
        let second = room.join(None, tx2).await;

        assert_eq!(first.username, "Гость1");
        assert_eq!(second.username, "Гость2");
        assert_eq!(room.client_count().await, 2);
    }

    #[tokio::test]
    async fn duplicate_names_get_a_suffix() {
        let room = ChatRoom::new();
        let (tx1, _rx1) = channel();
        let (tx2, _rx2) = channel();

        let first = room.join(Some("Niki"), tx1).await;
        let second = room.join(Some("Niki"), tx2).await;

        assert_eq!(first.username, "Niki");
        assert_eq!(second.username, "Niki (2)");
    }

    #[tokio::test]
    async fn rename_rejects_taken_and_empty_names() {
        let room = ChatRoom::new();
        let (tx1, _rx1) = channel();
        let (tx2, _rx2) = channel();

        let first = room.join(Some("Niki"), tx1).await;
        let second = room.join(Some("Other"), tx2).await;

        assert_eq!(
            room.rename(second.id, "Niki").await,
            Err(RenameError::Taken)
        );
        assert_eq!(room.rename(second.id, "  ").await, Err(RenameError::Empty));
        assert_eq!(room.rename(second.id, "New").await.unwrap(), "New");
        assert_eq!(room.rename(first.id, "new").await, Err(RenameError::Taken));
    }

    #[tokio::test]
    async fn rename_updates_users_list() {
        let room = ChatRoom::new();
        let (tx, _rx) = channel();
        let joined = room.join(Some("Old"), tx).await;

        room.rename(joined.id, "New").await.unwrap();

        assert_eq!(room.users().await, vec!["New".to_string()]);
    }

    #[tokio::test]
    async fn broadcast_except_skips_sender() {
        let room = ChatRoom::new();
        let (tx1, mut rx1) = channel();
        let (tx2, mut rx2) = channel();

        let first = room.join(Some("One"), tx1).await;
        let _second = room.join(Some("Two"), tx2).await;

        room.broadcast_except(
            first.id,
            &ServerEvent::Message {
                from: "One".to_string(),
                text: "hello".to_string(),
            },
        )
        .await;

        assert!(rx1.try_recv().is_err());
        let payload = rx2.try_recv().expect("второй клиент получил сообщение");
        assert!(payload.contains("\"type\":\"message\""));
        assert!(payload.contains("\"text\":\"hello\""));
    }

    #[tokio::test]
    async fn users_are_sorted_case_insensitively() {
        let room = ChatRoom::new();
        let (tx1, _rx1) = channel();
        let (tx2, _rx2) = channel();
        let (tx3, _rx3) = channel();

        room.join(Some("banana"), tx1).await;
        room.join(Some("Apple"), tx2).await;
        room.join(Some("cherry"), tx3).await;

        assert_eq!(
            room.users().await,
            vec![
                "Apple".to_string(),
                "banana".to_string(),
                "cherry".to_string()
            ]
        );
    }
}
