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
use uuid::Uuid;

use crate::{
    domain::room::Message,
    infra::{
        database::PostgresDatabase,
        http_api::user_endpoints::{login_end, register_end},
        web_socket::ws_handler,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<PostgresDatabase>,
    jwt_secret: String,
    pub rooms_channels: Arc<DashMap<Uuid, broadcast::Sender<Message>>>,
}

pub async fn start_http_api(
    addr: String,
    jwt_secret: String,
    db: Arc<PostgresDatabase>,
    rooms_channels: Arc<DashMap<Uuid, broadcast::Sender<Message>>>,
) {
    let auth_state = AppState {
        db,
        jwt_secret,
        rooms_channels,
    };

    let app = Router::new()
        .route("/health/auth", get(auth_health_check))
        .route("/ws/room/{room_id}", get(ws_handler))
        .route_layer(middleware::from_fn_with_state(
            auth_state.clone(),
            middleware_auth::middleware_fn,
        ))
        .route("/", get(health_check))
        .route("/register", post(register_end))
        .route("/login", post(login_end))
        .with_state(auth_state);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub async fn health_check() -> &'static str {
    "hello"
}

pub async fn auth_health_check(Extension(user_id): Extension<Uuid>) -> String {
    format!("hello user with id: {user_id}")
}
