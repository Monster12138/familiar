use anyhow::Result;
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use familiar_core::event::{AgentCategory, AgentEvent, AgentEventType, AgentSource, EventMetadata};

#[derive(Debug, Clone)]
pub struct CliAgentHookAdapter {
    agent_source: AgentSource,
}

impl CliAgentHookAdapter {
    pub fn new(source: AgentSource) -> Self {
        Self {
            agent_source: source,
        }
    }

    fn deterministic_uuid(s: &str) -> Uuid {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut hasher);
        let h1 = hasher.finish();
        s.as_bytes().hash(&mut hasher);
        let h2 = hasher.finish();

        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&h1.to_be_bytes());
        bytes[8..].copy_from_slice(&h2.to_be_bytes());
        Uuid::from_bytes(bytes)
    }

    pub fn parse_hook_input(&self, stdin_json: &Value) -> Result<AgentEvent> {
        let event_name = stdin_json["hook_event_name"]
            .as_str()
            .or_else(|| stdin_json["event"].as_str())
            .unwrap_or("Unknown");

        let id_str = stdin_json["conversationId"]
            .as_str()
            .or_else(|| stdin_json["conversation_id"].as_str())
            .or_else(|| stdin_json["session_id"].as_str())
            .or_else(|| stdin_json["sessionId"].as_str())
            .or_else(|| stdin_json["thread_id"].as_str())
            .or_else(|| stdin_json["payload"]["conversationId"].as_str())
            .or_else(|| stdin_json["payload"]["conversation_id"].as_str())
            .or_else(|| stdin_json["payload"]["session_id"].as_str())
            .or_else(|| stdin_json["payload"]["sessionId"].as_str())
            .or_else(|| stdin_json["payload"]["thread_id"].as_str());

        let id = match id_str {
            Some(s) if !s.is_empty() => {
                if let Ok(u) = Uuid::parse_str(s) {
                    u
                } else {
                    Self::deterministic_uuid(s)
                }
            }
            _ => {
                let source_key = format!("default_session_{:?}", self.agent_source);
                Self::deterministic_uuid(&source_key)
            }
        };

        let event_type = self.map_event_type(event_name, stdin_json);

        Ok(AgentEvent {
            id,
            timestamp: Utc::now(),
            source: self.agent_source.clone(),
            category: self.derive_category(&self.agent_source),
            event_type,
            metadata: Some(EventMetadata {
                extra: stdin_json.clone(),
            }),
        })
    }

    fn extract_instruction(json: &Value) -> Option<String> {
        let direct = json["content"]
            .as_str()
            .or_else(|| json["prompt"].as_str())
            .or_else(|| json["user_prompt"].as_str())
            .or_else(|| json["payload"]["content"].as_str())
            .or_else(|| json["payload"]["prompt"].as_str())
            .or_else(|| json["payload"]["user_prompt"].as_str());

        if let Some(s) = direct {
            let clean = Self::extract_clean_text(s);
            if !clean.is_empty() {
                return Some(clean);
            }
        }

        let transcript_path = json["transcriptPath"]
            .as_str()
            .or_else(|| json["payload"]["transcriptPath"].as_str());

        if let Some(path) = transcript_path {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines().rev() {
                    if line.contains("\"type\":\"USER_INPUT\"") {
                        if let Ok(val) = serde_json::from_str::<Value>(line) {
                            if let Some(content_str) = val.get("content").and_then(|c| c.as_str()) {
                                let clean = Self::extract_clean_text(content_str);
                                if !clean.is_empty() {
                                    return Some(clean);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
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

    fn map_event_type(&self, event_name: &str, json: &Value) -> AgentEventType {
        match event_name {
            "SessionStart" | "start" => {
                let instruction = Self::extract_instruction(json);
                AgentEventType::AgentStarted { instruction }
            }
            "USER_INPUT" | "UserPromptSubmit" => {
                let instruction = Self::extract_instruction(json);
                AgentEventType::AgentStarted { instruction }
            }
            "Stop" | "stop" | "exit" | "SessionEnd" => AgentEventType::TaskCompleted {
                summary: "Task finished".into(),
            },
            "PreToolUse" | "tool_call" => self.parse_pre_tool_use(json),
            "PostToolUse" | "tool_result" => AgentEventType::Processing {
                description: "Tool finished".into(),
            },
            "PermissionRequest" => AgentEventType::WaitingForInput,
            "SubagentStart" => AgentEventType::SubagentStarted {
                agent_type: "Unknown".into(),
            },
            "SubagentStop" => AgentEventType::SubagentStopped {
                agent_type: "Unknown".into(),
            },
            _ => AgentEventType::Processing {
                description: event_name.to_string(),
            },
        }
    }

    fn parse_pre_tool_use(&self, json: &Value) -> AgentEventType {
        let tool_name = json["toolCall"]["name"]
            .as_str()
            .or_else(|| json["tool_name"].as_str())
            .or_else(|| json["tool"].as_str())
            .or_else(|| json["name"].as_str())
            .or_else(|| json["payload"]["toolCall"]["name"].as_str())
            .or_else(|| json["payload"]["tool_name"].as_str())
            .unwrap_or("");

        let args_option = json.get("toolCall").and_then(|t| t.get("args"))
            .or_else(|| json.get("tool_arguments"))
            .or_else(|| json.get("args"))
            .or_else(|| json.get("input"))
            .or_else(|| json.get("payload").and_then(|p| p.get("toolCall")).and_then(|t| t.get("args")))
            .or_else(|| json.get("payload").and_then(|p| p.get("tool_arguments")));

        let empty_json = serde_json::json!({});
        let args = args_option.unwrap_or(&empty_json);
        let instruction = Self::extract_instruction(json);

        match tool_name {
            "Bash" | "run_command" | "execute" => {
                let cmd = args["command"]
                    .as_str()
                    .or_else(|| args["cmd"].as_str())
                    .or_else(|| args["CommandLine"].as_str())
                    .or_else(|| args["script"].as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(if tool_name.is_empty() { "Command" } else { tool_name })
                    .to_string();
                AgentEventType::RunningCommand { cmd, instruction }
            }
            "Edit"
            | "Write"
            | "apply_patch"
            | "write_to_file"
            | "replace_file_content"
            | "multi_replace_file_content" => {
                let path = args["path"]
                    .as_str()
                    .or_else(|| args["TargetFile"].as_str())
                    .or_else(|| args["Target"].as_str())
                    .or_else(|| args["filePath"].as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(if tool_name.is_empty() { "file" } else { tool_name })
                    .to_string();
                AgentEventType::WritingFile { path }
            }
            "view_file" | "read_file" | "cat" => {
                let path = args["AbsolutePath"]
                    .as_str()
                    .or_else(|| args["path"].as_str())
                    .or_else(|| args["filePath"].as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("file")
                    .to_string();
                AgentEventType::ReadingFile { path }
            }
            "search_web" | "read_url_content" => {
                let url = args["query"]
                    .as_str()
                    .or_else(|| args["Url"].as_str())
                    .or_else(|| args["url"].as_str())
                    .unwrap_or("")
                    .to_string();
                AgentEventType::BrowsingWeb { url }
            }
            name if name.starts_with("mcp__") => AgentEventType::Processing {
                description: format!("MCP: {}", name),
            },
            other => {
                let display_name = if other.is_empty() { "tool" } else { other };
                AgentEventType::Processing {
                    description: format!("Using tool {}", display_name),
                }
            }
        }
    }

    fn derive_category(&self, source: &AgentSource) -> AgentCategory {
        match source {
            AgentSource::ClaudeCode => AgentCategory::Coding,
            AgentSource::Codex => AgentCategory::Coding,
            AgentSource::Antigravity => AgentCategory::Coding,
            AgentSource::Custom(_) => AgentCategory::General,
        }
    }
}
