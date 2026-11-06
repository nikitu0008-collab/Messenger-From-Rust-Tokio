# Messenger на Rust + Tokio

Первая версия чата для колледжа — аналог [Messenger-From-C-Boost-Beast](https://github.com/nikitu0008-collab/Messenger-From-C-Boost-Beast), но на **Rust + Tokio + Axum**.

[![CI](https://github.com/nikitu0008-collab/Messenger-From-Rust-Tokio/actions/workflows/ci.yml/badge.svg)](https://github.com/nikitu0008-collab/Messenger-From-Rust-Tokio/actions/workflows/ci.yml)

## Возможности

- HTTP-сервер отдаёт готовую страницу чата.
- WebSocket-соединение для обмена сообщениями в реальном времени.
- Общая комната и рассылка сообщений всем участникам.
- Список пользователей онлайн, который обновляется автоматически.
- Вход под своим именем, уникальные ники и гостевые имена.
- Команды: `/help`, `/users`, `/nick <имя>`, `/quit`.
- Системные уведомления о входе, выходе и смене ника.
- Адаптивный интерфейс для компьютера и телефона.
- In-memory хранение: без базы данных и внешних сервисов.
- Unit-тесты для комнаты чата и CI через GitHub Actions.

## Стек

| Компонент | Технология |
|---|---|
| Язык | Rust 2021 |
| Асинхронный runtime | Tokio |
| HTTP / WebSocket | Axum 0.7 |
| Сериализация | Serde + serde_json |
| Логирование | tracing |
| Frontend | HTML5, CSS3, Vanilla JS |

## Требования

- Rust 1.85 или новее (проще всего установить через [rustup](https://rustup.rs/)).
- Свободный порт `8080` (или другой, заданный через `PORT`).

## Быстрый старт

```bash
git clone https://github.com/nikitu0008-collab/Messenger-From-Rust-Tokio.git
cd Messenger-From-Rust-Tokio
cargo run
```

После запуска откройте:

```text
http://localhost:8080
```

Введите имя и нажмите «Войти в чат». Чтобы проверить чат вдвоём, откройте страницу во втором окне браузера.

### Другой порт

```bash
# Linux / macOS
PORT=3000 cargo run

# Windows PowerShell
$env:PORT=3000; cargo run
```

### Release-сборка

```bash
cargo build --release
./target/release/messenger
```

## Настройки

| Переменная | По умолчанию | Описание |
|---|---|---|
| `PORT` | `8080` | Порт HTTP/WebSocket-сервера |

Сервер слушает `0.0.0.0`, поэтому при необходимости к нему можно подключиться из локальной сети по IP компьютера.

## Команды чата

| Команда | Действие |
|---|---|
| `/help` | Показать справку |
| `/users` | Показать участников онлайн |
| `/nick <имя>` | Сменить ник (до 24 символов) |
| `/quit` | Отключиться от чата |

Ник также можно передать сразу при подключении:

```text
ws://localhost:8080/ws?name=Никита
```

## Протокол

Клиент отправляет в WebSocket обычный текст. Сервер отвечает JSON-событиями с полем `type`.

```jsonc
// подключение
{"type":"welcome","username":"Никита","users":["Никита","Гость2"]}

// сообщение в чат
{"type":"message","from":"Никита","text":"Привет!"}

// системное уведомление
{"type":"system","text":"Гость2 присоединился к чату"}

// список онлайн
{"type":"users","users":["Гость2","Никита"]}

// смена ника
{"type":"renamed","username":"Niki"}

// ошибка команды
{"type":"error","text":"Этот ник уже занят."}
```

## Структура проекта

```text
Messenger-From-Rust-Tokio/
├── src/
│   ├── main.rs          # запуск сервера, роуты, конфигурация порта
│   ├── chat_room.rs     # участники, ники, broadcast, unit-тесты
│   ├── handle_client.rs # жизненный цикл WebSocket-соединения и команды
│   ├── protocol.rs      # JSON-события сервера
│   └── html.rs          # отдача HTML-страницы
├── assets/
│   └── index.html       # интерфейс чата (HTML/CSS/JS)
├── .github/workflows/ci.yml
├── Cargo.toml
├── LICENSE
└── README.md
```

## Проверка и тесты

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Чем отличается от C++ версии

| C++ + Boost.Beast | Rust + Tokio |
|---|---|
| Ручной пул потоков | Асинхронные задачи Tokio |
| `std::mutex` и общий вектор клиентов | `RwLock` и каналы Tokio |
| `std::thread` на каждое соединение | Лёгкие async-задачи |
| Ручной разбор JSON-строк | Типизированные события Serde |
| Строковый протокол `MSG|имя|текст` | JSON-события |

## Ограничения первой версии

- История сообщений хранится только в памяти и не сохраняется между перезапусками.
- Нет базы данных, авторизации и приватных сообщений.
- Нет ограничения частоты сообщений (rate limit).
- Для публичного интернета нужен reverse proxy с HTTPS/WSS.

## Планы

- Приватные сообщения между участниками.
- История чата и PostgreSQL/SQLite.
- Аутентификация и роли.
- Rate limiting и модерация.
- Docker-образ и docker-compose.
- WebSocket-интеграционные тесты.

## Лицензия

MIT — см. [LICENSE](LICENSE).

## Автор

NikiZIP (nikitu0008-collab), IT TOP.
