use crate::ws::ws_handler;
use axum::{routing::get, Router};

pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws_handler))
}

async fn health_check() -> &'static str {
    "OK"
}
