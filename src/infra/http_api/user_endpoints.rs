use axum::{
    Extension,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    infra::http_api::AppState,
    use_cases::auth_service::{get_user_by_id_use, login, register},
};

#[derive(Deserialize, Serialize)]
pub struct AuthInfo {
    identifier: String,
    password: String,
}

#[derive(Deserialize, Serialize)]
pub struct RegisterInfo {
    username: String,
    email: String,
    password: String,
}

pub async fn login_end(
    State(state): State<AppState>,
    Json(auth_info): Json<AuthInfo>,
) -> impl IntoResponse {
    match login(
        state.db,
        auth_info.identifier,
        auth_info.password,
        state.jwt_secret,
    )
    .await
    {
        Ok(token) => (StatusCode::OK, token),
        Err(err) => (StatusCode::UNAUTHORIZED, err.to_string()),
    }
}

pub async fn register_end(
    State(state): State<AppState>,
    Json(register_info): Json<RegisterInfo>,
) -> impl IntoResponse {
    match register(
        state.db,
        register_info.username,
        register_info.password,
        register_info.email,
    )
    .await
    {
        Ok(_) => (StatusCode::OK, "".to_string()),
        Err(err) => (StatusCode::UNAUTHORIZED, err.to_string()),
    }
}

pub async fn get_user_info_end(
    State(state): State<AppState>,
    Extension(user_id): Extension<Uuid>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match get_user_by_id_use(state.db, user_id).await {
        Ok(user) => Ok((StatusCode::OK, Json(user))),
        Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err.to_string())),
    }
}
