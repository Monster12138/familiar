use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{self, Read};
use tokio::io::AsyncWriteExt;

// This would typically use UnixStream for IPC, but for MVP let's just print to stdout
// or we can simulate it. The Familiar API crate will listen on a port or socket.
pub async fn run(source_name: &str, event_name: &str, stdin_json: Option<&str>) -> Result<()> {
    // 1. Resolve the JSON payload: `--stdin-json` wins (manual testing,
    //    single-quoted for cmd/PowerShell/sh), otherwise read from stdin.
    //    If stdin is empty, treat it as an empty object.
    let json: Value = if let Some(input) = stdin_json {
        parse_payload(input.trim().trim_matches('\''))
    } else {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read stdin")?;
        if buffer.trim().is_empty() {
            serde_json::json!({})
        } else {
            parse_payload(&buffer)
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

    // 3. Send to familiar daemon (UDS first on Unix, TCP loopback fallback).
    // Endpoints follow the user config (`hooks.socket_path` / `hooks.tcp_port`).
    let (socket_path, tcp_port) = load_endpoint_config();
    let tcp_addr = tcp_endpoint(tcp_port);
    let res = async {
        #[cfg(unix)]
        {
            let sock = socket_path.unwrap_or_else(|| "/tmp/familiar.sock".to_string());
            if let Ok(mut stream) = tokio::net::UnixStream::connect(&sock).await {
                stream.write_all(&payload_bytes).await?;
                return Ok::<(), anyhow::Error>(());
            }
        }
        #[cfg(not(unix))]
        let _ = socket_path;
        let mut stream = tokio::net::TcpStream::connect(&tcp_addr).await?;
        stream.write_all(&payload_bytes).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    let output_json = get_hook_response_json(source_name, event_name, res.is_err());
    println!("{}", output_json);
    Ok(())
}

/// Parse a JSON payload string, falling back to a `raw_payload` wrapper
/// when the string is not valid JSON (e.g. pasted plain text).
fn parse_payload(input: &str) -> Value {
    match serde_json::from_str(input) {
        Ok(v) => v,
        Err(_) => serde_json::json!({ "raw_payload": input }),
    }
}

/// Best-effort lookup of hook endpoints from the user config file.
///
/// The hook CLI runs from any working directory on behalf of coding agents,
/// so only absolute user config locations are consulted. Missing entries fall
/// back to the defaults (`/tmp/familiar.sock`, port 19527).
fn load_endpoint_config() -> (Option<String>, Option<u16>) {
    for path in familiar_core::platform::user_config_file_candidates() {
        if let Ok(config) = familiar_core::config::FamiliarConfig::load_from_file(&path) {
            return (config.hooks.socket_path, config.hooks.tcp_port);
        }
    }
    (None, None)
}

fn tcp_endpoint(tcp_port: Option<u16>) -> String {
    format!("127.0.0.1:{}", tcp_port.unwrap_or(19527))
}

fn get_hook_response_json(source_name: &str, event_name: &str, offline: bool) -> String {
    if source_name == "qoder" {
        match event_name {
            "PreToolUse" => {
                let reason = if offline {
                    "Familiar offline"
                } else {
                    "Familiar notified"
                };
                serde_json::json!({
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "permissionDecision": "allow",
                        "permissionDecisionReason": reason
                    }
                })
                .to_string()
            }
            _ => serde_json::json!({}).to_string(),
        }
    } else {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_endpoint_falls_back_to_default_port() {
        assert_eq!(tcp_endpoint(None), "127.0.0.1:19527");
        assert_eq!(tcp_endpoint(Some(1234)), "127.0.0.1:1234");
    }

    #[test]
    fn test_qoder_hook_response_json() {
        let resp = get_hook_response_json("qoder", "PreToolUse", false);
        let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(
            json["hookSpecificOutput"]["permissionDecision"].as_str(),
            Some("allow")
        );
        assert_eq!(
            json["hookSpecificOutput"]["permissionDecisionReason"].as_str(),
            Some("Familiar notified")
        );

        let stop_resp = get_hook_response_json("qoder", "Stop", false);
        let stop_json: serde_json::Value = serde_json::from_str(&stop_resp).unwrap();
        assert_eq!(stop_json, serde_json::json!({}));
    }
}
