use anyhow::{Result, Context};
use std::io::{self, Read};
use serde_json::Value;

// This would typically use UnixStream for IPC, but for MVP let's just print to stdout
// or we can simulate it. The Familiar API crate will listen on a port or socket.
pub async fn run(source_name: &str) -> Result<()> {
    // 1. Read JSON from stdin
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).context("Failed to read stdin")?;
    
    if buffer.trim().is_empty() {
        return Ok(());
    }

    let json: Value = serde_json::from_str(&buffer).context("Failed to parse JSON")?;

    // 2. We inject our source name so the backend knows where it came from
    let mut payload = serde_json::json!({
        "source_client": source_name,
        "payload": json
    });

    // 3. Send to familiar daemon (assuming it listens on a local port or socket)
    // For now we'll do a simple HTTP POST to the local daemon (easier cross-platform than UnixSocket)
    let client = reqwest::Client::new();
    let res = client.post("http://127.0.0.1:9528/api/v1/notify")
        .json(&payload)
        .send()
        .await;

    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            // It's okay if daemon is not running; we just silently ignore
            tracing::debug!("Failed to notify familiar daemon: {}", e);
            Ok(())
        }
    }
}
