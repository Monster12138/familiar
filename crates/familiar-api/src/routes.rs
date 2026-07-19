use axum::{Router, routing::get};
use crate::ws::ws_handler;

pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws_handler))
}

async fn health_check() -> &'static str {
    "OK"
}
