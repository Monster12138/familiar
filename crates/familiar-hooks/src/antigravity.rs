use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom};
use tokio::sync::mpsc;

use crate::hook_trait::AgentHook;
use familiar_core::event::{AgentCategory, AgentEvent, AgentEventType, AgentSource};

#[derive(Debug, Clone)]
pub struct AntigravityHook {
    transcript_path: String,
}

impl AntigravityHook {
    pub fn new(transcript_path: String) -> Self {
        Self { transcript_path }
    }

    fn map_transcript_line(&self, json: &Value) -> Option<AgentEvent> {
        // ... (existing logic)
        let step_type = json["type"].as_str()?;
        let agent_id = uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111")
            .unwrap_or_else(|_| uuid::Uuid::nil());

        match step_type {
            "PLANNER_RESPONSE" => {
                let tool_calls = &json["tool_calls"];
                if tool_calls.is_null()
                    || (tool_calls.is_array() && tool_calls.as_array().unwrap().is_empty())
                {
                    return Some(AgentEvent {
                        id: agent_id,
                        timestamp: chrono::Utc::now(),
                        source: AgentSource::Antigravity,
                        category: AgentCategory::Coding,
                        event_type: AgentEventType::WaitingForInput,
                        metadata: None,
                    });
                }

                if let Some(tc_array) = tool_calls.as_array() {
                    let tc = &tc_array[0];
                    let name = tc["name"].as_str().unwrap_or("unknown");
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
                } else {
                    None
                }
            }
            "USER_INPUT" => {
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
            }
            "RUN_COMMAND"
            | "VIEW_FILE"
            | "REPLACE_FILE_CONTENT"
            | "MULTI_REPLACE_FILE_CONTENT"
            | "READ_URL_CONTENT"
            | "GREP_SEARCH"
            | "TOOL_RESPONSE" => Some(AgentEvent {
                id: agent_id,
                timestamp: chrono::Utc::now(),
                source: AgentSource::Antigravity,
                category: AgentCategory::Coding,
                event_type: AgentEventType::Processing {
                    description: "Tool finished".into(),
                },
                metadata: None,
            }),
            _ => None,
        }
    }

    pub fn parse(&self, json: &Value) -> anyhow::Result<AgentEvent> {
        self.map_transcript_line(json)
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
        let path = self.transcript_path.clone();
        let hook_clone = self.clone();

        tokio::spawn(async move {
            loop {
                let mut file = match File::open(&path).await {
                    Ok(f) => f,
                    Err(_) => {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                };
                // Seek to end to only read new events
                let _ = file.seek(SeekFrom::End(0)).await;
                let mut reader = BufReader::new(file);
                let mut line = String::new();

                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                        Ok(_) => {
                            if let Ok(json) = serde_json::from_str::<Value>(&line) {
                                if let Some(event) = hook_clone.map_transcript_line(&json) {
                                    let _ = sender.send(event).await;
                                }
                            }
                        }
                        Err(_) => {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            break;
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
}
