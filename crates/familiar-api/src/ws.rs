use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use familiar_core::{
    config::StateStreamConfig,
    state::RenderState,
    state_machine::StateMachine,
    state_stream::{ServerHelloV1, StateSnapshotV1, STATE_STREAM_PROTOCOL_VERSION},
};
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Clone)]
pub struct StateStreamState {
    pub machine: StateMachine,
    pub server_id: Uuid,
    pub server_version: String,
    pub heartbeat_secs: u64,
    pub auth_token: Option<Arc<str>>,
    pub snapshots: watch::Receiver<Option<Arc<str>>>,
}

impl StateStreamState {
    pub fn new(
        machine: StateMachine,
        config: StateStreamConfig,
        auth_token: Option<String>,
        server_version: impl Into<String>,
    ) -> Self {
        let server_id = Uuid::new_v4();
        let (sender, receiver) = watch::channel(None);
        let mut updates = machine.subscribe_state();
        let revision_machine = machine.clone();
        let initial_machine = machine.clone();
        tokio::spawn(async move {
            let mut last_encoded: Option<Arc<str>> = None;
            let mut last_sent = tokio::time::Instant::now() - Duration::from_secs(1);
            let min_interval = if config.max_updates_per_second == 0 {
                Duration::ZERO
            } else {
                Duration::from_secs_f64(1.0 / f64::from(config.max_updates_per_second))
            };

            // Publish the current state before waiting for the next event so
            // a newly connected client does not have to wait for activity.
            let initial_state = initial_machine.get_state().await;
            if let Ok(encoded) = encode_snapshot(
                &initial_state,
                initial_machine.revision(),
                server_id,
                &config,
            ) {
                if sender.send(Some(encoded.clone())).is_err() {
                    return;
                }
                last_encoded = Some(encoded);
                last_sent = tokio::time::Instant::now();
            }
            loop {
                if updates.changed().await.is_err() {
                    break;
                }
                if last_sent.elapsed() < min_interval {
                    tokio::time::sleep(min_interval - last_sent.elapsed()).await;
                }
                // A watch channel keeps only the newest state. Reading after
                // the throttle window coalesces all intermediate updates.
                let state = updates.borrow_and_update().clone();
                let revision = revision_machine.revision();
                let encoded = match encode_snapshot(&state, revision, server_id, &config) {
                    Ok(value) => value,
                    Err(error) => {
                        tracing::warn!(%error, "failed to encode state stream snapshot");
                        continue;
                    }
                };
                if last_encoded.as_deref() == Some(encoded.as_ref()) {
                    continue;
                }
                if sender.send(Some(encoded.clone())).is_err() {
                    break;
                }
                last_encoded = Some(encoded);
                last_sent = tokio::time::Instant::now();
            }
        });
        Self {
            machine,
            server_id,
            server_version: server_version.into(),
            heartbeat_secs: 30,
            auth_token: auth_token.map(Arc::<str>::from),
            snapshots: receiver,
        }
    }
}

fn encode_snapshot(
    state: &RenderState,
    revision: u64,
    server_id: Uuid,
    config: &StateStreamConfig,
) -> anyhow::Result<Arc<str>> {
    let snapshot = StateSnapshotV1::from_render_state(
        state,
        server_id,
        revision,
        config.max_task_summary_chars,
        config.max_activity_summary_chars,
    );
    Ok(Arc::<str>::from(serde_json::to_string(&snapshot)?))
}

pub async fn ws_handler(
    State(state): State<StateStreamState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if let Some(expected) = state.auth_token.as_deref() {
        let supplied = headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));
        if supplied != Some(expected) {
            return (axum::http::StatusCode::UNAUTHORIZED, "unauthorized").into_response();
        }
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
        .into_response()
}

async fn handle_socket(mut socket: WebSocket, state: StateStreamState) {
    let hello = ServerHelloV1 {
        message_type: "hello".to_string(),
        v: STATE_STREAM_PROTOCOL_VERSION,
        server_id: state.server_id,
        server_version: state.server_version,
        heartbeat_secs: state.heartbeat_secs,
    };
    if send_json(&mut socket, &hello).await.is_err() {
        return;
    }

    let mut snapshots = state.snapshots.clone();
    let mut heartbeat = tokio::time::interval(Duration::from_secs(state.heartbeat_secs.max(1)));
    loop {
        tokio::select! {
            changed = snapshots.changed() => {
                if changed.is_err() { return; }
                let snapshot = snapshots.borrow().clone();
                let Some(snapshot) = snapshot else { continue };
                if socket.send(Message::Text(snapshot.to_string())).await.is_err() { return; }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() { return; }
                    }
                    Some(Ok(Message::Close(_))) | None => return,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => return,
                }
            }
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(Vec::new())).await.is_err() { return; }
            }
        }
    }
}

async fn send_json<T: serde::Serialize>(socket: &mut WebSocket, value: &T) -> anyhow::Result<()> {
    let encoded = serde_json::to_string(value)?;
    socket.send(Message::Text(encoded)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::StateStreamState;
    use familiar_core::{
        config::StateStreamConfig, event_bus::EventBus, state_machine::StateMachine,
    };

    #[tokio::test]
    async fn state_stream_publishes_initial_snapshot() {
        let bus = EventBus::new(8, 8);
        let machine = StateMachine::new(bus, 4, 300);
        let state = StateStreamState::new(machine, StateStreamConfig::default(), None, "test");
        let mut snapshots = state.snapshots.clone();
        tokio::time::timeout(std::time::Duration::from_secs(1), snapshots.changed())
            .await
            .expect("initial state should be published")
            .expect("snapshot publisher should remain alive");
        let encoded = snapshots.borrow().clone().expect("snapshot payload");
        let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(value["type"], "state");
        assert_eq!(value["active_agent_count"], 0);
    }
}
