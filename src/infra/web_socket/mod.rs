use axum::{
    Extension,
    extract::{Path, State, WebSocketUpgrade},
    response::IntoResponse,
};
use tokio::sync::broadcast;
use tracing::{error, info};
use uuid::Uuid;

use crate::infra::http_api::AppState;

pub async fn ws_handler(
    Path(room_id): Path<Uuid>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(user_id): Extension<Uuid>,
) -> impl IntoResponse {
    info!("User with id: {user_id} joining room: {}", room_id);

    // Use method user is in room

    ws.on_upgrade(move |socket| handle_socket(socket, room_id, user_id, state))
}

async fn handle_socket(
    socket: axum::extract::ws::WebSocket,
    room_id: Uuid,
    user_id: Uuid,
    state: AppState,
) {
    let room_tx = state
        .rooms_channels
        .entry(room_id)
        .or_insert_with(|| broadcast::channel(1_000).0)
        .value()
        .clone();

    let mut receiver = room_tx.subscribe();

    let mut sender = socket;

    while let Ok(msg) = receiver.recv().await {
        let msg_json = if let Ok(msg_json) = serde_json::to_string(&msg) {
            msg_json
        } else {
            error!("Error converting message to a string: {msg:?}");
            break;
        };

        if sender.send(msg_json.into()).await.is_err() {
            break;
        }
    }
}
