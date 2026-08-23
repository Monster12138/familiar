use crate::ws::{display_ws_handler, ws_handler, StateStreamState};
use axum::{extract::State, http::HeaderMap, response::IntoResponse, routing::get, Json, Router};
use familiar_hooks::manager;
use serde::Serialize;
use std::collections::BTreeMap;

pub fn create_router(state: StateStreamState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/state-stream", get(ws_handler))
        .route("/api/v1/display-stream", get(display_ws_handler))
        .route("/api/v1/hooks/status", get(hooks_status))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}

#[derive(Debug, Serialize)]
struct RemoteHookStatus {
    injected: bool,
}

async fn hooks_status(
    State(state): State<StateStreamState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !state.is_authorized(&headers) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(BTreeMap::<String, RemoteHookStatus>::new()),
        )
            .into_response();
    }

    let statuses = manager::statuses()
        .into_iter()
        .map(|(agent, status)| {
            (
                agent,
                RemoteHookStatus {
                    injected: status.injected,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    Json(statuses).into_response()
}

#[cfg(test)]
mod tests {
    use super::RemoteHookStatus;

    #[test]
    fn remote_status_does_not_include_config_paths() {
        let encoded = serde_json::to_value(RemoteHookStatus { injected: true }).unwrap();
        assert_eq!(encoded, serde_json::json!({"injected": true}));
        assert!(encoded.get("config_path").is_none());
    }
}
