use crate::commands::AppConfigState;
use familiar_core::{
    config::RemoteConfig,
    state::{AgentStatus, FamiliarMood, RenderState},
    state_stream::{StateSnapshotV1, STATE_STREAM_PROTOCOL_VERSION},
};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::HashSet;
use tauri::Emitter;
use tokio_tungstenite::{
    connect_async, tungstenite::client::IntoClientRequest, tungstenite::Message,
};

pub fn start(
    app: tauri::AppHandle,
    config: RemoteConfig,
    app_config: std::sync::Arc<AppConfigState>,
) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run(app.clone(), config, app_config).await {
            tracing::error!(%error, "remote state provider stopped");
            let _ = app.emit("connection_status_changed", "offline");
        }
    });
}

async fn run(
    app: tauri::AppHandle,
    config: RemoteConfig,
    app_config: std::sync::Arc<AppConfigState>,
) -> anyhow::Result<()> {
    let endpoint = config
        .endpoint
        .ok_or_else(|| anyhow::anyhow!("remote.endpoint is required in remote mode"))?;
    let scheme = if config.tls { "wss" } else { "ws" };
    let url = format!("{scheme}://{}{}", endpoint, config.path);
    let token = config
        .token_env
        .as_deref()
        .and_then(|name| std::env::var(name).ok());
    let mut backoff = config.reconnect_initial_secs.max(1);
    let mut last_server_id = None;
    let mut last_revision = 0u64;

    loop {
        tracing::info!(endpoint = %url, tls = config.tls, "connecting to remote state stream");
        let _ = app.emit("connection_status_changed", "connecting");
        let mut request = url.clone().into_client_request()?;
        if let Some(token) = token.as_deref() {
            request
                .headers_mut()
                .insert("authorization", format!("Bearer {token}").parse()?);
        }
        let connection = tokio::time::timeout(
            std::time::Duration::from_secs(config.connect_timeout_secs.max(1)),
            connect_async(request),
        )
        .await;
        let (mut socket, _) = match connection {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => {
                tracing::warn!(%error, "remote state connection failed");
                let status = match &error {
                    tokio_tungstenite::tungstenite::Error::Http(response)
                        if response.status().as_u16() == 401 => "authentication_failed",
                    _ => "reconnecting",
                };
                let _ = app.emit("connection_status_changed", status);
                tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                backoff = (backoff.saturating_mul(2)).min(config.reconnect_max_secs.max(1));
                continue;
            }
            Err(_) => {
                let _ = app.emit("connection_status_changed", "reconnecting");
                tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                backoff = (backoff.saturating_mul(2)).min(config.reconnect_max_secs.max(1));
                continue;
            }
        };
        backoff = config.reconnect_initial_secs.max(1);
        tracing::info!(endpoint = %url, "connected to remote state stream");
        let _ = app.emit("connection_status_changed", "connected");

        while let Some(message) = socket.next().await {
            match message? {
                Message::Text(text) => {
                    let value: Value = serde_json::from_str(&text)?;
                    if value.get("type").and_then(Value::as_str) != Some("state") {
                        continue;
                    }
                    let snapshot: StateSnapshotV1 = serde_json::from_value(value)?;
                    if snapshot.v != STATE_STREAM_PROTOCOL_VERSION {
                        tracing::warn!(
                            version = snapshot.v,
                            expected = STATE_STREAM_PROTOCOL_VERSION,
                            "unsupported remote state stream version"
                        );
                        let _ = app.emit("connection_status_changed", "incompatible_protocol");
                        continue;
                    }
                    if last_server_id == Some(snapshot.server_id)
                        && snapshot.revision <= last_revision
                    {
                        continue;
                    }
                    if last_server_id != Some(snapshot.server_id) {
                        last_server_id = Some(snapshot.server_id);
                    }
                    last_revision = snapshot.revision;
                    tracing::debug!(
                        server_id = %snapshot.server_id,
                        revision = snapshot.revision,
                        agents = snapshot.agents.len(),
                        "received remote state snapshot"
                    );
                    let hidden_sessions = app_config
                        .hidden_sessions
                        .read()
                        .map(|sessions| sessions.clone())
                        .unwrap_or_default();
                    let state =
                        filter_hidden_sessions(snapshot.to_render_state(), &hidden_sessions);
                    let _ = app.emit("state_changed", state.clone());
                    let _ = app.emit("settings_state_changed", state);
                }
                Message::Ping(payload) => socket.send(Message::Pong(payload)).await?,
                Message::Close(_) => break,
                _ => {}
            }
        }
        tracing::warn!(endpoint = %url, "remote state stream disconnected");
        let _ = app.emit("connection_status_changed", "reconnecting");
    }
}

fn filter_hidden_sessions(
    mut state: RenderState,
    hidden_sessions: &HashSet<String>,
) -> RenderState {
    if hidden_sessions.is_empty() {
        return state;
    }
    state
        .agents
        .retain(|agent| !hidden_sessions.contains(&agent.id));
    state.active_agent_count = state.agents.len();
    state.agents_by_category.clear();
    for agent in &state.agents {
        state
            .agents_by_category
            .entry(agent.category.clone())
            .or_default()
            .push(agent.clone());
    }
    state.mood = if state.agents.iter().any(|a| a.status == AgentStatus::Failed) {
        FamiliarMood::Alarmed
    } else if state
        .agents
        .iter()
        .any(|a| a.status == AgentStatus::Working)
    {
        FamiliarMood::Busy
    } else if state
        .agents
        .iter()
        .any(|a| a.status == AgentStatus::Thinking)
    {
        FamiliarMood::Thinking
    } else if state
        .agents
        .iter()
        .any(|a| a.status == AgentStatus::Completed)
    {
        FamiliarMood::Celebrating
    } else if state
        .agents
        .iter()
        .any(|a| a.status == AgentStatus::Pending)
    {
        FamiliarMood::Watching
    } else {
        FamiliarMood::Idle
    };
    state
}
