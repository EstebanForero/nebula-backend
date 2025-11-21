use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::IntoResponse,
};

use crate::infra::http_api::AppState;

pub async fn ws_handler(
    Path(room_id): Path<String>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    println!("User joining room: {}", room_id);

    // 3. Upgrade to WebSocket
    ws.on_upgrade(move |socket| handle_socket(socket, room_id, state))
}
