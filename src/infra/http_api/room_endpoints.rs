use axum::{
    Extension,
    extract::{Json, Path, Query, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    domain::room::RoomVisibility,
    infra::http_api::AppState,
    use_cases::room_service::{
        create_room, get_all_public_rooms, get_user_rooms_use, join_room, obtain_messages,
        send_message, user_is_in_room,
    },
};

#[derive(Deserialize, Serialize)]
pub struct RoomInfo {
    password: Option<String>,
    name: String,
    visibility: RoomVisibility,
}

#[derive(Deserialize, Serialize)]
pub struct Pegination {
    page: u32,
    page_size: u8,
    room_id: Uuid,
}

#[derive(Deserialize, Serialize)]
pub struct MessageInfo {
    room_id: Uuid,
    content: String,
}

pub async fn create_room_end(
    State(state): State<AppState>,
    Extension(user_id): Extension<Uuid>,
    Json(room_info): Json<RoomInfo>,
) -> impl IntoResponse {
    match create_room(
        state.db,
        room_info.visibility,
        room_info.password,
        room_info.name,
        user_id,
    )
    .await
    {
        Ok(_) => (StatusCode::OK, "".to_string()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn join_room_end(
    State(state): State<AppState>,
    Extension(user_id): Extension<Uuid>,
    Path(room_id): Path<Uuid>,
) -> impl IntoResponse {
    match join_room(state.db, room_id, user_id).await {
        Ok(_) => (StatusCode::OK, "".to_string()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn get_user_rooms_end(
    State(state): State<AppState>,
    Extension(user_id): Extension<Uuid>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match get_user_rooms_use(state.db, user_id).await {
        Ok(rooms) => Ok((StatusCode::OK, Json(rooms))),
        Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err.to_string())),
    }
}

pub async fn get_all_public_rooms_end(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match get_all_public_rooms(state.db).await {
        Ok(rooms) => Ok((StatusCode::OK, Json(rooms))),
        Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err.to_string())),
    }
}

pub async fn send_message_end(
    State(state): State<AppState>,
    Extension(user_id): Extension<Uuid>,
    Json(message_info): Json<MessageInfo>,
) -> impl IntoResponse {
    match send_message(
        state.db,
        message_info.room_id,
        user_id,
        message_info.content,
        state.redis_publisher,
        state.message_processing,
    )
    .await
    {
        Ok(_) => (StatusCode::OK, "".to_string()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn get_messages(
    State(state): State<AppState>,
    pegination: Query<Pegination>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match obtain_messages(
        state.db,
        pegination.page,
        pegination.page_size,
        pegination.room_id,
    )
    .await
    {
        Ok(messages) => Ok((StatusCode::OK, Json(messages))),
        Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err.to_string())),
    }
}
