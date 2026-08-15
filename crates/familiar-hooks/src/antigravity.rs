use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::hook_trait::AgentHook;
use familiar_core::event::{AgentCategory, AgentEvent, AgentEventType, AgentSource};

#[derive(Debug, Clone)]
pub struct AntigravityHook {}

impl Default for AntigravityHook {
    fn default() -> Self {
        Self::new()
    }
}

impl AntigravityHook {
    pub fn new() -> Self {
        Self {}
    }

    fn extract_clean_text(s: &str) -> String {
        let mut text = s.to_string();
        if let Some(start) = text.find("<USER_REQUEST>") {
            if let Some(end) = text.find("</USER_REQUEST>") {
                let start_idx = start + "<USER_REQUEST>".len();
                text = text[start_idx..end].trim().to_string();
            }
        }
        text
    }

    /// Loads an existing config file as a JSON object, refusing to fall back
    /// to an empty document: overwriting a malformed or non-object config
    /// would silently discard the user's other hooks and settings.
    fn parse_existing_config(path: &std::path::Path, content: &str) -> Result<serde_json::Value> {
        let existing = serde_json::from_str::<serde_json::Value>(content).map_err(|e| {
            anyhow::anyhow!(
                "{} is not valid JSON ({e}); refusing to overwrite it",
                path.display()
            )
        })?;
        if !existing.is_object() {
            anyhow::bail!(
                "{} is not a JSON object; refusing to overwrite it",
                path.display()
            );
        }
        Ok(existing)
    }

    fn map_native_hook(&self, event_name: &str, json: &Value) -> Option<AgentEvent> {
        let agent_id = json
            .get("conversationId")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .unwrap_or_else(uuid::Uuid::nil);

        match event_name {
            "SessionStart" => {
                let instruction = json["instruction"]
                    .as_str()
                    .or_else(|| json["task"].as_str())
                    .or_else(|| json["prompt"].as_str())
                    .map(Self::extract_clean_text);
                Some(AgentEvent {
                    id: agent_id,
                    timestamp: chrono::Utc::now(),
                    source: AgentSource::Antigravity,
                    category: AgentCategory::Coding,
                    event_type: AgentEventType::AgentStarted { instruction },
                    metadata: None,
                })
            }
            "PreToolUse" => {
                let name = json["toolCall"]["name"]
                    .as_str()
                    .or_else(|| json["tool_name"].as_str())
                    .or_else(|| json["tool"].as_str())
                    .unwrap_or("unknown");

                let cmd_str = if name == "run_command" || name == "Bash" {
                    json["toolCall"]["args"]["CommandLine"]
                        .as_str()
                        .or_else(|| json["toolCall"]["args"]["command"].as_str())
                        .or_else(|| json["tool_arguments"]["CommandLine"].as_str())
                        .or_else(|| json["tool_arguments"]["command"].as_str())
                        .unwrap_or(name)
                        .to_string()
                } else {
                    format!("Using tool {}", name)
                };

                let instruction = json["transcriptPath"].as_str().and_then(|path| {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        for line in content.lines().rev() {
                            if line.contains("\"type\":\"USER_INPUT\"") {
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                                    if let Some(content_str) =
                                        val.get("content").and_then(|c| c.as_str())
                                    {
                                        return Some(Self::extract_clean_text(content_str));
                                    }
                                }
                            }
                        }
                    }
                    None
                });

                Some(AgentEvent {
                    id: agent_id,
                    timestamp: chrono::Utc::now(),
                    source: AgentSource::Antigravity,
                    category: AgentCategory::Coding,
                    event_type: AgentEventType::RunningCommand {
                        cmd: cmd_str,
                        instruction,
                    },
                    metadata: None,
                })
            }
            "PostToolUse" => Some(AgentEvent {
                id: agent_id,
                timestamp: chrono::Utc::now(),
                source: AgentSource::Antigravity,
                category: AgentCategory::Coding,
                event_type: AgentEventType::Processing {
                    description: "Tool finished".to_string(),
                },
                metadata: None,
            }),
            "PreInvocation" | "PostInvocation" => Some(AgentEvent {
                id: agent_id,
                timestamp: chrono::Utc::now(),
                source: AgentSource::Antigravity,
                category: AgentCategory::Coding,
                event_type: AgentEventType::Thinking,
                metadata: None,
            }),
            "Stop" | "SessionEnd" => Some(AgentEvent {
                id: agent_id,
                timestamp: chrono::Utc::now(),
                source: AgentSource::Antigravity,
                category: AgentCategory::Coding,
                event_type: AgentEventType::TaskCompleted {
                    summary: "Task finished".to_string(),
                },
                metadata: None,
            }),
            // Fallback for legacy transcript mocking
            "test" => {
                let step_type = json["type"].as_str().unwrap_or("");
                if step_type == "PLANNER_RESPONSE" {
                    Some(AgentEvent {
                        id: agent_id,
                        timestamp: chrono::Utc::now(),
                        source: AgentSource::Antigravity,
                        category: AgentCategory::Coding,
                        event_type: AgentEventType::RunningCommand {
                            cmd: "Working...".into(),
                            instruction: None,
                        },
                        metadata: None,
                    })
                } else if step_type == "USER_INPUT" {
                    let mut instruction = json["content"].as_str().map(|s| s.to_string());
                    if let Some(ref mut text) = instruction {
                        if let Some(start) = text.find("<USER_REQUEST>") {
                            if let Some(end) = text.find("</USER_REQUEST>") {
                                let start_idx = start + "<USER_REQUEST>".len();
                                *text = text[start_idx..end].trim().to_string();
                            }
                        }
                    }
                    Some(AgentEvent {
                        id: agent_id,
                        timestamp: chrono::Utc::now(),
                        source: AgentSource::Antigravity,
                        category: AgentCategory::Coding,
                        event_type: AgentEventType::AgentStarted { instruction },
                        metadata: None,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn parse(&self, event_name: &str, json: &Value) -> anyhow::Result<AgentEvent> {
        self.map_native_hook(event_name, json)
            .ok_or_else(|| anyhow::anyhow!("Could not map json to event"))
    }
}

#[async_trait]
impl AgentHook for AntigravityHook {
    fn name(&self) -> &str {
        "antigravity"
    }

    fn category(&self) -> AgentCategory {
        AgentCategory::Coding
    }

    async fn start(&self, sender: mpsc::Sender<AgentEvent>) -> Result<()> {
        tokio::spawn(async move {
            let brain_dir = dirs::home_dir().unwrap().join(".gemini/antigravity/brain");
            let mut file_cursors: std::collections::HashMap<std::path::PathBuf, usize> =
                std::collections::HashMap::new();

            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                let mut active_dirs = Vec::new();
                let now = std::time::SystemTime::now();

                // Find all directories modified in the last 15 seconds
                if let Ok(entries) = std::fs::read_dir(&brain_dir) {
                    for entry in entries.flatten() {
                        if let Ok(meta) = entry.metadata() {
                            if meta.is_dir() {
                                if let Ok(time) = meta.modified() {
                                    if let Ok(duration) = now.duration_since(time) {
                                        if duration.as_secs() < 15 {
                                            active_dirs.push(entry.path());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                for dir in active_dirs {
                    let transcript = dir.join(".system_generated/logs/transcript_full.jsonl");
                    if transcript.exists() {
                        if let Ok(content) = std::fs::read_to_string(&transcript) {
                            let lines: Vec<&str> = content.lines().collect();
                            let current_count = lines.len();

                            // If it's a newly discovered conversation in this session, don't replay history
                            // Just set the cursor to the end, minus 2 lines to catch the very message that triggered this
                            let cursor = file_cursors
                                .entry(transcript.clone())
                                .or_insert_with(|| current_count.saturating_sub(2));

                            let agent_id = dir
                                .file_name()
                                .and_then(|os_str| os_str.to_str())
                                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                .unwrap_or_else(uuid::Uuid::nil);

                            if current_count > *cursor {
                                for line in &lines[*cursor..] {
                                    if line.contains("\"type\":\"USER_INPUT\"") {
                                        if let Ok(json) = serde_json::from_str::<Value>(line) {
                                            if let Some(content_str) =
                                                json.get("content").and_then(|c| c.as_str())
                                            {
                                                let instruction =
                                                    Self::extract_clean_text(content_str);
                                                let _ = sender
                                                    .send(AgentEvent {
                                                        id: agent_id,
                                                        timestamp: chrono::Utc::now(),
                                                        source: AgentSource::Antigravity,
                                                        category: AgentCategory::Coding,
                                                        event_type: AgentEventType::AgentStarted {
                                                            instruction: Some(instruction),
                                                        },
                                                        metadata: None,
                                                    })
                                                    .await;
                                                let _ = sender
                                                    .send(AgentEvent {
                                                        id: agent_id,
                                                        timestamp: chrono::Utc::now(),
                                                        source: AgentSource::Antigravity,
                                                        category: AgentCategory::Coding,
                                                        event_type: AgentEventType::Thinking,
                                                        metadata: None,
                                                    })
                                                    .await;
                                            }
                                        }
                                    } else if line.contains("\"type\":\"PLANNER_RESPONSE\"")
                                        && line.contains("\"status\":\"DONE\"")
                                    {
                                        let _ = sender
                                            .send(AgentEvent {
                                                id: agent_id,
                                                timestamp: chrono::Utc::now(),
                                                source: AgentSource::Antigravity,
                                                category: AgentCategory::Coding,
                                                event_type: AgentEventType::TaskCompleted {
                                                    summary: "Finished thinking".into(),
                                                },
                                                metadata: None,
                                            })
                                            .await;
                                    }
                                }
                                *cursor = current_count;
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn config_path(&self) -> Option<std::path::PathBuf> {
        let home = dirs::home_dir()?;
        Some(home.join(".gemini").join("config").join("hooks.json"))
    }

    fn get_injection_payload(&self) -> Option<serde_json::Value> {
        let bin_path = crate::bin_path::resolve_cli_bin_path();
        Some(serde_json::json!({
            "familiar": {
                "PreInvocation": [{
                    "type": "command",
                    "command": format!("\"{}\" hook --source antigravity --event PreInvocation", bin_path)
                }],
                "PostInvocation": [{
                    "type": "command",
                    "command": format!("\"{}\" hook --source antigravity --event PostInvocation", bin_path)
                }],
                "Stop": [{
                    "type": "command",
                    "command": format!("\"{}\" hook --source antigravity --event Stop", bin_path)
                }],
                "PreToolUse": [{
                    "matcher": "*",
                    "hooks": [{
                        "type": "command",
                        "command": format!("\"{}\" hook --source antigravity --event PreToolUse", bin_path)
                    }]
                }],
                "PostToolUse": [{
                    "matcher": "*",
                    "hooks": [{
                        "type": "command",
                        "command": format!("\"{}\" hook --source antigravity --event PostToolUse", bin_path)
                    }]
                }]
            }
        }))
    }

    fn is_injected(&self) -> bool {
        let path = match self.config_path() {
            Some(p) => p,
            None => return false,
        };

        if !path.exists() {
            return false;
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            json.get("familiar").is_some()
        } else {
            false
        }
    }

    fn inject(&self) -> Result<()> {
        let path = self
            .config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        let payload = self
            .get_injection_payload()
            .ok_or_else(|| anyhow::anyhow!("No payload defined"))?;

        let mut config_json = serde_json::json!({});

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            if !content.trim().is_empty() {
                config_json = Self::parse_existing_config(&path, &content)?;
            }
            // Backup old config before writing
            let bak_path = crate::hook_trait::backup_path(&path, "bak")?;
            std::fs::copy(&path, &bak_path)?;
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // Merge payload
        if let (Some(obj), Some(payload_obj)) = (config_json.as_object_mut(), payload.as_object()) {
            for (k, v) in payload_obj {
                obj.insert(k.clone(), v.clone());
            }
        }

        let new_content = serde_json::to_string_pretty(&config_json)?;
        std::fs::write(&path, new_content)?;

        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let path = self
            .config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        if !path.exists() {
            return Ok(());
        }

        // Backup before uninstall
        let bak_path = crate::hook_trait::backup_path(&path, "bak.uninstall")?;
        std::fs::copy(&path, &bak_path)?;

        let content = std::fs::read_to_string(&path)?;
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.as_object_mut() {
                obj.remove("familiar");
            }
            let new_content = serde_json::to_string_pretty(&json)?;
            std::fs::write(&path, new_content)?;
        }

        Ok(())
    }

    fn preview_inject(&self) -> Result<(String, String)> {
        let path = self
            .config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        let payload = self
            .get_injection_payload()
            .ok_or_else(|| anyhow::anyhow!("No payload defined"))?;

        let mut config_json = serde_json::json!({});
        let mut before_content = String::new();

        if path.exists() {
            before_content = std::fs::read_to_string(&path).unwrap_or_default();
            if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&before_content) {
                config_json = existing;
            }
        }

        if let (Some(obj), Some(payload_obj)) = (config_json.as_object_mut(), payload.as_object()) {
            for (k, v) in payload_obj {
                obj.insert(k.clone(), v.clone());
            }
        }

        let after_content = serde_json::to_string_pretty(&config_json)?;
        Ok((before_content, after_content))
    }

    fn preview_uninstall(&self) -> Result<(String, String)> {
        let path = self
            .config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        if !path.exists() {
            return Ok((String::new(), String::new()));
        }

        let before_content = std::fs::read_to_string(&path)?;
        let mut after_content = before_content.clone();

        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&before_content) {
            if let Some(obj) = json.as_object_mut() {
                obj.remove("familiar");
            }
            after_content = serde_json::to_string_pretty(&json)?;
        }

        Ok((before_content, after_content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(event_name: &str) -> AgentEvent {
        let hook = AntigravityHook::new();
        let json = json!({
            "conversationId": "11111111-1111-1111-1111-111111111111",
        });
        hook.parse(event_name, &json).unwrap()
    }

    #[test]
    fn test_post_tool_use_maps_to_processing() {
        // Must match the shared adapter (adapter.rs) so all agents show the
        // same mid-round Working state instead of Antigravity-only Thinking.
        let event = parse("PostToolUse");
        assert!(
            matches!(event.event_type, AgentEventType::Processing { .. }),
            "PostToolUse should map to Processing, got {:?}",
            event.event_type
        );
    }

    #[test]
    fn test_stop_maps_to_task_completed() {
        let event = parse("Stop");
        assert!(
            matches!(event.event_type, AgentEventType::TaskCompleted { .. }),
            "Stop should map to TaskCompleted, got {:?}",
            event.event_type
        );
    }
}
