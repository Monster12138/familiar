use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::hook_trait::AgentHook;
use familiar_core::event::{AgentCategory, AgentEvent, AgentEventType, AgentSource};

#[derive(Debug, Clone)]
pub struct AntigravityHook {}

impl AntigravityHook {
    pub fn new() -> Self {
        Self {}
    }

    fn map_native_hook(&self, event_name: &str, json: &Value) -> Option<AgentEvent> {
        let agent_id = uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111")
            .unwrap_or_else(|_| uuid::Uuid::nil());

        match event_name {
            "SessionStart" => Some(AgentEvent {
                id: agent_id,
                timestamp: chrono::Utc::now(),
                source: AgentSource::Antigravity,
                category: AgentCategory::Coding,
                event_type: AgentEventType::AgentStarted { instruction: None },
                metadata: None,
            }),
            "PreToolUse" => {
                let name = json["toolCall"]["name"].as_str().unwrap_or("unknown");
                Some(AgentEvent {
                    id: agent_id,
                    timestamp: chrono::Utc::now(),
                    source: AgentSource::Antigravity,
                    category: AgentCategory::Coding,
                    event_type: AgentEventType::RunningCommand {
                        cmd: format!("Using tool {}", name),
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
                    description: "Tool finished".into(),
                },
                metadata: None,
            }),
            "Stop" | "SessionEnd" => Some(AgentEvent {
                id: agent_id,
                timestamp: chrono::Utc::now(),
                source: AgentSource::Antigravity,
                category: AgentCategory::Coding,
                event_type: AgentEventType::WaitingForInput,
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
                        event_type: AgentEventType::RunningCommand { cmd: "Working...".into() },
                        metadata: None,
                    })
                } else if step_type == "USER_INPUT" {
                    Some(AgentEvent {
                        id: agent_id,
                        timestamp: chrono::Utc::now(),
                        source: AgentSource::Antigravity,
                        category: AgentCategory::Coding,
                        event_type: AgentEventType::AgentStarted { instruction: Some(json["content"].as_str().unwrap_or("").to_string()) },
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

    async fn start(&self, _sender: mpsc::Sender<AgentEvent>) -> Result<()> {
        // File-tailing sidecar is deprecated in favor of UDS IPC via hooks.json
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn config_path(&self) -> Option<std::path::PathBuf> {
        let home = dirs::home_dir()?;
        Some(home.join(".gemini").join("antigravity").join("hooks.json"))
    }

    fn get_injection_payload(&self) -> Option<serde_json::Value> {
        let bin_path = "familiar-cli"; // Assumes it's in PATH, or could be absolute
        Some(serde_json::json!({
            "on_pre_tool_use": format!("{} hook --source antigravity --event PreToolUse", bin_path),
            "on_post_tool_use": format!("{} hook --source antigravity --event PostToolUse", bin_path)
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
            let pre = json["on_pre_tool_use"].as_str().unwrap_or("");
            let post = json["on_post_tool_use"].as_str().unwrap_or("");
            pre.contains("familiar-cli") || post.contains("familiar-cli")
        } else {
            false
        }
    }

    fn inject(&self) -> Result<()> {
        let path = self.config_path().ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        let payload = self.get_injection_payload().ok_or_else(|| anyhow::anyhow!("No payload defined"))?;

        let mut config_json = serde_json::json!({});

        if path.exists() {
            // Backup old config
            let bak_path = path.with_extension(format!("bak.{}", chrono::Utc::now().timestamp()));
            std::fs::copy(&path, &bak_path)?;

            let content = std::fs::read_to_string(&path)?;
            if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&content) {
                config_json = existing;
            }
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
        let path = self.config_path().ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        if !path.exists() {
            return Ok(());
        }

        // Backup before uninstall
        let bak_path = path.with_extension(format!("bak.uninstall.{}", chrono::Utc::now().timestamp()));
        std::fs::copy(&path, &bak_path)?;

        let content = std::fs::read_to_string(&path)?;
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.as_object_mut() {
                // If it's ours, remove it
                if let Some(v) = obj.get("on_pre_tool_use") {
                    if v.as_str().unwrap_or("").contains("familiar-cli") {
                        obj.remove("on_pre_tool_use");
                    }
                }
                if let Some(v) = obj.get("on_post_tool_use") {
                    if v.as_str().unwrap_or("").contains("familiar-cli") {
                        obj.remove("on_post_tool_use");
                    }
                }
            }
            let new_content = serde_json::to_string_pretty(&json)?;
            std::fs::write(&path, new_content)?;
        }

        Ok(())
    }
}
