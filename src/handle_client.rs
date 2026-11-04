use axum::extract::ws::{Message, WebSocket};

use futures_util::{SinkExt, StreamExt};

use std::sync::Arc;

use crate::chat_room::ChatRoom;

pub async fn handle_socket(socket: WebSocket, room: Arc<ChatRoom>) {
    let username = room.add_user().await;

    let (mut sender, mut receiver) = socket.split();
    let mut rx = room.subscribe();

    let welcome_message =
        format!("SYSTEM|Добро пожаловать, {username}! Используйте /help для списка команд.");

    if sender
        .send(Message::Text(welcome_message.into()))
        .await
        .is_err()
    {
        room.remove_user(&username).await;
        return;
    }

    room.broadcast(format!("SYSTEM|{username} присоединился к чату."));

    let room_for_send = Arc::clone(&room);

    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(message) => {
                    if sender.send(Message::Text(message.into())).await.is_err() {
                        break;
                    }
                }

                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }

                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    let mut current_username = username.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            let message = match result {
                Ok(Message::Text(text)) => text.to_string(),

                Ok(Message::Close(_)) => break,

                Ok(_) => continue,

                Err(_) => break,
            };

            let message = message.trim();

            if message.is_empty() {
                continue;
            }

            if message == "/help" {
                room.broadcast(format!(
                    "PRIVATE|{current_username}|Команды: /help, /users, /nick <имя>, /quit"
                ));

                continue;
            }

            if message == "/users" {
                let users = room.users_list().await;

                room.broadcast(format!(
                    "PRIVATE|{current_username}|Сейчас в чате: {}",
                    users.join(", ")
                ));

                continue;
            }

            if let Some(new_name) = message.strip_prefix("/nick ") {
                match room.rename_user(&current_username, new_name).await {
                    Ok(new_username) => {
                        let old_username =
                            std::mem::replace(&mut current_username, new_username.clone());

                        room.broadcast(format!(
                            "SYSTEM|{old_username} теперь известен как {new_username}."
                        ));
                    }

                    Err(error) => {
                        room.broadcast(format!("PRIVATE|{current_username}|Ошибка: {error}"));
                    }
                }

                continue;
            }

            if message == "/quit" {
                break;
            }

            room.broadcast(format!("MESSAGE|{current_username}|{message}"));
        }

        room.remove_user(&current_username).await;

        room.broadcast(format!("SYSTEM|{current_username} покинул чат."));
    });

    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }

        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    room_for_send.remove_user(&username).await;
}
