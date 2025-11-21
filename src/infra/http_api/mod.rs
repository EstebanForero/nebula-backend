pub mod user_endpoints;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};

use crate::{
    infra::{
        database::PostgresDatabase,
        http_api::user_endpoints::{login_end, register_end},
        web_socket::ws_handler,
    },
    use_cases::{auth_service::Claims, user_database::UserDatabase},
};

#[derive(Clone)]
pub struct AppState {
    db: Arc<PostgresDatabase>,
    jwt_secret: String,
}

pub async fn start_http_api(addr: String, jwt_secret: String, db: Arc<PostgresDatabase>) {
    let auth_state = AppState { db, jwt_secret };

    let app = Router::new()
        .route("/", get(health_check))
        .route("/register", post(register_end))
        .route("/login", post(login_end))
        .route_layer(middleware::from_fn_with_state(
            auth_state.clone(),
            middleware,
        ))
        .route("/ws/room/{room_id}", get(ws_handler))
        .with_state(auth_state);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub async fn health_check() -> &'static str {
    "hello"
}

pub async fn middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let bearer_token = match request.headers().get("authorization") {
        Some(auth) => auth.to_str(),
        None => {
            return Response::builder()
                .status(401)
                .body("invalid/missing auth token".into())
                .unwrap();
        }
    };

    let bearer_token = if let Ok(bearer_token) = bearer_token {
        bearer_token
    } else {
        return Response::builder()
            .status(401)
            .body("invalid/missing auth token".into())
            .unwrap();
    };

    let jwt_token = match bearer_token.strip_prefix("Bearer") {
        Some(token) => token.to_string(),
        None => {
            return Response::builder()
                .status(401)
                .body("worng header format".into())
                .unwrap();
        }
    };

    let my_claims: Claims = match decode(
        jwt_token,
        &DecodingKey::from_secret(state.jwt_secret.as_ref()),
        &Validation::default(),
    ) {
        Ok(clamis) => clamis.claims,
        Err(_) => {
            return Response::builder()
                .status(401)
                .body("worng header format".into())
                .unwrap();
        }
    };

    request.extensions_mut().insert(my_claims.sub);

    next.run(request).await
}
