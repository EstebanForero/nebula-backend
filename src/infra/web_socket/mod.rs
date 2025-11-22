use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::Response,
};
use serde::Deserialize;
use tokio::sync::broadcast;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    infra::http_api::{AppState, middleware_auth::extract_user_id_from_jwt},
    use_cases::room_service::user_is_in_room,
};

#[derive(Deserialize)]
pub struct WsAuth {
    pub token: String,
}

pub async fn ws_handler(
    Path(room_id): Path<Uuid>,
    ws: WebSocketUpgrade,
    Query(WsAuth { token }): Query<WsAuth>,
    State(state): State<AppState>,
) -> Response {
    let user_id = match extract_user_id_from_jwt(token, &state.jwt_secret) {
        Ok(id) => id,
        Err(res) => return res,
    };

    info!("User with id: {user_id} joining room: {}", room_id);

    let is_in_room = match user_is_in_room(state.db.clone(), user_id, room_id).await {
        Ok(res) => res,
        Err(_) => {
            error!("Error verifying room, check the room id");
            return Response::builder()
                .status(400)
                .body("Invalid room".into())
                .unwrap();
        }
    };

    if !is_in_room {
        return Response::builder()
            .status(403)
            .body("Not eough access to enter the room".into())
            .unwrap();
    }

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
        if msg.sender_id == user_id {
            continue;
        }

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
