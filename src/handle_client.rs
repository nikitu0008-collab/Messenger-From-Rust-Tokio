use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::chat_room::ChatRoom;
use crate::protocol::ServerEvent;

/// Ограничение на длину одного сообщения чата (в символах).
const MAX_MESSAGE_CHARS: usize = 2_000;

/// Полный жизненный цикл одного WebSocket-соединения.
pub async fn handle_socket(socket: WebSocket, room: Arc<ChatRoom>, requested_name: Option<String>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let joined = room.join(requested_name.as_deref(), tx.clone()).await;
    let id = joined.id;
    let mut username = joined.username.clone();

    let users = room.users().await;
    let online = room.client_count().await;
    info!(client_id = id, user = %username, online, "client joined the room");

    let welcome = ServerEvent::Welcome {
        username: username.clone(),
        users,
    };

    if tx.send(welcome.to_json()).is_err() {
        room.leave(id).await;
        return;
    }

    room.broadcast_except(
        id,
        &ServerEvent::System {
            text: format!("{username} присоединился к чату"),
        },
    )
    .await;
    room.broadcast_users().await;

    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        while let Some(payload) = rx.recv().await {
            if sender.send(Message::Text(payload)).await.is_err() {
                break;
            }
        }
    });

    let recv_room = Arc::clone(&room);
    let recv_tx = tx.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            let text = match result {
                Ok(Message::Text(text)) => text.as_str().to_string(),
                Ok(Message::Binary(_)) => {
                    send_error(
                        &recv_tx,
                        "Поддерживаются только текстовые сообщения.".to_string(),
                    );
                    continue;
                }
                Ok(Message::Close(_)) => break,
                Ok(_) => continue,
                Err(error) => {
                    debug!(%error, "websocket receive error");
                    break;
                }
            };

            let text = text.trim();

            if text.is_empty() {
                continue;
            }

            if text.chars().count() > MAX_MESSAGE_CHARS {
                send_error(
                    &recv_tx,
                    format!("Сообщение слишком длинное (максимум {MAX_MESSAGE_CHARS} символов)."),
                );
                continue;
            }

            let mut parts = text.splitn(2, char::is_whitespace);
            let command = parts.next().unwrap_or_default();
            let argument = parts.next().unwrap_or_default().trim();

            match command {
                "/help" => {
                    let _ = recv_tx.send(
                        ServerEvent::System {
                            text: "Команды: /help — справка, /users — кто онлайн, /nick <имя> — сменить ник, /quit — выйти."
                                .to_string(),
                        }
                        .to_json(),
                    );
                }

                "/users" => {
                    let users = recv_room.users().await;
                    let _ = recv_tx.send(ServerEvent::Users { users }.to_json());
                }

                "/nick" => {
                    if argument.is_empty() {
                        send_error(&recv_tx, "Использование: /nick <имя>".to_string());
                        continue;
                    }

                    match recv_room.rename(id, argument).await {
                        Ok(new_name) => {
                            let old_name = std::mem::replace(&mut username, new_name.clone());

                            let _ = recv_tx.send(
                                ServerEvent::Renamed {
                                    username: new_name.clone(),
                                }
                                .to_json(),
                            );

                            recv_room
                                .broadcast(&ServerEvent::System {
                                    text: format!("{old_name} теперь известен как {new_name}"),
                                })
                                .await;
                            recv_room.broadcast_users().await;
                        }

                        Err(error) => send_error(&recv_tx, error.message().to_string()),
                    }
                }

                "/quit" => break,

                _ if command.starts_with('/') => {
                    send_error(
                        &recv_tx,
                        format!("Неизвестная команда: {command}. Введите /help."),
                    );
                }

                _ => {
                    debug!(client_id = id, user = %username, "chat message");
                    recv_room
                        .broadcast(&ServerEvent::Message {
                            from: username.clone(),
                            text: text.to_string(),
                        })
                        .await;
                }
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    if let Some(username) = room.leave(id).await {
        info!(client_id = id, user = %username, "client disconnected");

        room.broadcast(&ServerEvent::System {
            text: format!("{username} покинул чат"),
        })
        .await;
        room.broadcast_users().await;
    }
}

fn send_error(tx: &mpsc::UnboundedSender<String>, text: String) {
    if tx.send(ServerEvent::Error { text }.to_json()).is_err() {
        warn!("failed to send websocket error: receiver is gone");
    }
}
