pub mod user_endpoints;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    infra::{
        database::PostgresDatabase,
        http_api::user_endpoints::{login_end, register_end},
    },
    use_cases::user_database::UserDatabase,
};

#[derive(Clone)]
pub struct AuthState {
    db: Arc<PostgresDatabase>,
    jwt_secret: String,
}

pub async fn start_http_api(addr: String, jwt_secret: String, db: Arc<PostgresDatabase>) {
    let auth_state = AuthState { db, jwt_secret };

    let app = Router::new()
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
