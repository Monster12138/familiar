use crate::ws::{ws_handler, StateStreamState};
use axum::{routing::get, Router};

pub fn create_router(state: StateStreamState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/state-stream", get(ws_handler))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}
