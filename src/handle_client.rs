use axum::extract::ws::{
    Message,
    WebSocket,
};

use futures_util::{
    SinkExt,
    StreamExt,
};

use std::sync::Arc;

use crate::AppState;

pub async fn handle_socket(
    socket: WebSocket,
    state: Arc<AppState>,
) {
    // Разделяем WebSocket на отправку и получение сообщений
    let (mut sender, mut receiver) = socket.split();

    // Каждый клиент получает собственный receiver
    // для подписки на broadcast-канал
    let mut rx = state.tx.subscribe();

    // Задача отправки сообщений клиенту
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(message) => {
                    if sender
                        .send(Message::Text(message.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }

                // Клиент не успел получить некоторые сообщения
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }

                // Канал закрыт
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    // Клонируем Sender для задачи получения сообщений
    let tx = state.tx.clone();

    // Задача получения сообщений от клиента
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                // Получили текстовое сообщение
                Ok(Message::Text(text)) => {
                    let _ = tx.send(text.to_string());
                }

                // Клиент закрыл соединение
                Ok(Message::Close(_)) => {
                    break;
                }

                // Игнорируем другие типы сообщений
                Ok(_) => {}

                // Ошибка WebSocket
                Err(_) => {
                    break;
                }
            }
        }
    });

    // Ждём завершения одной из задач
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }

        _ = &mut recv_task => {
            send_task.abort();
        }
    }
}
