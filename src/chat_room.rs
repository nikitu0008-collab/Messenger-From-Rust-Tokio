use std::{
    collections::HashSet,
    sync::atomic::{AtomicUsize, Ordering},
};

use tokio::sync::{RwLock, broadcast};

pub struct ChatRoom {
    users: RwLock<HashSet<String>>,
    tx: broadcast::Sender<String>,
    next_user_id: AtomicUsize,
}

impl ChatRoom {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);

        Self {
            users: RwLock::new(HashSet::new()),
            tx,
            next_user_id: AtomicUsize::new(1),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    pub fn broadcast(&self, message: impl Into<String>) {
        let _ = self.tx.send(message.into());
    }

    pub async fn add_user(&self) -> String {
        let id = self.next_user_id.fetch_add(1, Ordering::Relaxed);
        let username = format!("Guest{id}");

        let mut users = self.users.write().await;
        users.insert(username.clone());

        username
    }

    pub async fn rename_user(&self, old_name: &str, new_name: &str) -> Result<String, String> {
        let new_name = new_name.trim();

        if new_name.is_empty() {
            return Err("Имя не может быть пустым.".to_string());
        }

        if new_name.len() > 20 {
            return Err("Имя не должно быть длиннее 20 символов.".to_string());
        }

        let mut users = self.users.write().await;

        if users.contains(new_name) {
            return Err("Это имя уже занято.".to_string());
        }

        users.remove(old_name);
        users.insert(new_name.to_string());

        Ok(new_name.to_string())
    }

    pub async fn remove_user(&self, username: &str) {
        let mut users = self.users.write().await;
        users.remove(username);
    }

    pub async fn users_list(&self) -> Vec<String> {
        let users = self.users.read().await;

        let mut list: Vec<String> = users.iter().cloned().collect();
        list.sort();

        list
    }
}
