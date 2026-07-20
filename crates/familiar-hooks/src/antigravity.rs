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
}
