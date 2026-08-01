use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{self, Read};
use tokio::io::AsyncWriteExt;

// This would typically use UnixStream for IPC, but for MVP let's just print to stdout
// or we can simulate it. The Familiar API crate will listen on a port or socket.
pub async fn run(source_name: &str, event_name: &str) -> Result<()> {
    // 1. Read JSON from stdin (non-blocking or timeout?)
    // Actually, if we just read to EOF, and the framework didn't send anything, it'll just be empty.
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read stdin")?;

    let json: Value = if buffer.trim().is_empty() {
        serde_json::json!({})
    } else {
        match serde_json::from_str(&buffer) {
            Ok(v) => v,
            Err(_) => serde_json::json!({"raw_payload": buffer}),
        }
    };

    // 2. We inject our source name so the backend knows where it came from
    let payload = serde_json::json!({
        "source_client": source_name,
        "hook_event_name": event_name,
        "payload": json
    });

    let mut payload_bytes = serde_json::to_vec(&payload)?;
    payload_bytes.push(b'\n');

    // 3. Send to familiar daemon (Try Unix Socket first, fallback to TCP loopback 127.0.0.1:9528)
    let res = async {
        #[cfg(unix)]
        {
            if let Ok(mut stream) = tokio::net::UnixStream::connect("/tmp/familiar.sock").await {
                stream.write_all(&payload_bytes).await?;
                return Ok::<(), anyhow::Error>(());
            }
        }
        let mut stream = tokio::net::TcpStream::connect("127.0.0.1:9528").await?;
        stream.write_all(&payload_bytes).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    let output_json = get_hook_response_json(event_name, res.is_err());
    println!("{}", output_json);
    Ok(())
}

fn get_hook_response_json(event_name: &str, offline: bool) -> String {
    match event_name {
        "PreToolUse" | "PermissionRequest" => {
            let reason = if offline {
                "Familiar offline"
            } else {
                "Familiar notified"
            };
            serde_json::json!({
                "decision": "allow",
                "reason": reason
            })
            .to_string()
        }
        "PostToolUse" => serde_json::json!({}).to_string(),
        "PreInvocation" => serde_json::json!({
            "injectSteps": []
        })
        .to_string(),
        "PostInvocation" => serde_json::json!({
            "injectSteps": []
        })
        .to_string(),
        "Stop" | "SessionEnd" | "SubagentStop" => serde_json::json!({
            "decision": "allow"
        })
        .to_string(),
        _ => serde_json::json!({}).to_string(),
    }
}
