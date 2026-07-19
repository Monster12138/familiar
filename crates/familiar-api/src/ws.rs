use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};

pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut _socket: WebSocket) {
    // Basic stub for websocket handler
}
