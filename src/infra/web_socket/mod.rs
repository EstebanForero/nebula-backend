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

    ws.on_upgrade(move |socket| handle_socket(socket, room_id, state))
}

async fn handle_socket(mut socket: axum::extract::ws::WebSocket, room_id: String, state: AppState) {
    todo!()
}
