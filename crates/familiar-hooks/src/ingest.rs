use crate::{adapter::CliAgentHookAdapter, antigravity::AntigravityHook};
use familiar_core::{event::AgentSource, event_bus::EventBus};
use serde_json::Value;
use tokio::io::AsyncBufReadExt;

pub async fn handle_line(line: &str, bus: &EventBus) {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return;
    };
    let Some(source) = value.get("source_client").and_then(Value::as_str) else {
        return;
    };
    let Some(payload) = value.get("payload") else {
        return;
    };
    let event_name = value
        .get("hook_event_name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let parsed = if source == "antigravity" {
        AntigravityHook::new().parse(event_name, payload)
    } else {
        let source = match source {
            "codex" => AgentSource::Codex,
            "claude-code" => AgentSource::ClaudeCode,
            "qoder" => AgentSource::Qoder,
            "deepseek-harness" => AgentSource::DeepSeekHarness,
            other => AgentSource::Custom(other.to_string()),
        };
        let adapter = CliAgentHookAdapter::new(source);
        let mut full_payload = payload.clone();
        if let Some(object) = full_payload.as_object_mut() {
            object.insert(
                "hook_event_name".to_string(),
                Value::String(event_name.to_string()),
            );
        }
        adapter.parse_hook_input(&full_payload)
    };
    if let Ok(event) = parsed {
        let _ = bus.publish(event).await;
    }
}

pub async fn serve_tcp(bus: EventBus, port: u16) -> anyhow::Result<()> {
    let address = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&address).await?;
    tracing::info!(%address, "hook TCP listener started");
    loop {
        let (stream, _) = listener.accept().await?;
        let bus = bus.clone();
        tokio::spawn(async move {
            let (reader, _) = tokio::io::split(stream);
            let mut reader = tokio::io::BufReader::new(reader);
            let mut line = String::new();
            while reader
                .read_line(&mut line)
                .await
                .ok()
                .filter(|size| *size > 0)
                .is_some()
            {
                handle_line(&line, &bus).await;
                line.clear();
            }
        });
    }
}

#[cfg(unix)]
pub async fn serve_unix(bus: EventBus, path: String) -> anyhow::Result<()> {
    let _ = std::fs::remove_file(&path);
    let _cleanup = SocketCleanup(path.clone());
    let listener = tokio::net::UnixListener::bind(&path)?;
    tracing::info!(socket = %path, "hook UDS listener started");
    loop {
        let (stream, _) = listener.accept().await?;
        let bus = bus.clone();
        tokio::spawn(async move {
            let (reader, _) = tokio::io::split(stream);
            let mut reader = tokio::io::BufReader::new(reader);
            let mut line = String::new();
            while reader
                .read_line(&mut line)
                .await
                .ok()
                .filter(|size| *size > 0)
                .is_some()
            {
                handle_line(&line, &bus).await;
                line.clear();
            }
        });
    }
}

#[cfg(unix)]
struct SocketCleanup(String);

#[cfg(unix)]
impl Drop for SocketCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
