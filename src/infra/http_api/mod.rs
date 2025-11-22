mod middleware_auth;
pub mod room_endpoints;
pub mod user_endpoints;
use std::sync::Arc;

use axum::{
    Extension, Router, middleware,
    routing::{get, post},
};
use dashmap::DashMap;
use tokio::sync::broadcast;
use tracing::info;
use uuid::Uuid;

use crate::{
    domain::room::Message,
    infra::{
        database::PostgresDatabase,
        http_api::{
            room_endpoints::{
                create_room_end, get_all_public_rooms_end, get_user_rooms_end, join_room_end,
                send_message_end,
            },
            user_endpoints::{login_end, register_end},
        },
        rabbit_mq::RabbitMQ,
        redis::RedisPublisher,
        web_socket::ws_handler,
    },
    use_cases::room_service::{create_room, get_user_rooms_use},
};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<PostgresDatabase>,
    jwt_secret: String,
    pub rooms_channels: Arc<DashMap<Uuid, broadcast::Sender<Message>>>,
    redis_publisher: Arc<RedisPublisher>,
    message_processing: Arc<RabbitMQ>,
}

pub async fn start_http_api(
    addr: String,
    jwt_secret: String,
    db: Arc<PostgresDatabase>,
    rooms_channels: Arc<DashMap<Uuid, broadcast::Sender<Message>>>,
    redis_publisher: Arc<RedisPublisher>,
    message_processing: Arc<RabbitMQ>,
) {
    let auth_state = AppState {
        db,
        jwt_secret,
        rooms_channels,
        redis_publisher,
        message_processing,
    };

    let app = Router::new()
        .route("/health/auth", get(auth_health_check))
        .route("/ws/room/{room_id}", get(ws_handler))
        .route("/rooms/public", get(get_all_public_rooms_end))
        .route("/rooms", get(get_user_rooms_end))
        .route("/room", post(create_room_end))
        .route("/room/join", post(join_room_end))
        .route("/message", post(send_message_end))
        .route_layer(middleware::from_fn_with_state(
            auth_state.clone(),
            middleware_auth::middleware_fn,
        ))
        .route("/", get(health_check))
        .route("/register", post(register_end))
        .route("/login", post(login_end))
        .with_state(auth_state);

    let listener = tokio::net::TcpListener::bind(addr.clone()).await.unwrap();
    info!("Starting server in: {addr}");
    axum::serve(listener, app).await.unwrap();
}

pub async fn health_check() -> &'static str {
    "hello"
}

pub async fn auth_health_check(Extension(user_id): Extension<Uuid>) -> String {
    format!("hello user with id: {user_id}")
}
