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

    /// Pulls the human-readable error message (falling back to the short code)
    /// from a Claude Code `StopFailure` payload.
    fn extract_stop_failure_error(json: &Value) -> String {
        json["error_message"]
            .as_str()
            .or_else(|| json["error"].as_str())
            .or_else(|| json["payload"]["error_message"].as_str())
            .or_else(|| json["payload"]["error"].as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "API error".to_string())
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

    fn extract_subagent_type(json: &Value) -> String {
        json["subagent_type"]
            .as_str()
            .or_else(|| json["agent_type"].as_str())
            .or_else(|| json["payload"]["subagent_type"].as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("Unknown")
            .to_string()
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
            // Claude Code v2.1.78+: fires when a turn ends due to a model API
            // error (rate limit, auth, billing, network, max_output_tokens...).
            "StopFailure" => AgentEventType::TaskFailed {
                error: Self::extract_stop_failure_error(json),
            },
            "PreToolUse" | "tool_call" => self.parse_pre_tool_use(json),
            "PostToolUse" | "tool_result" => AgentEventType::Processing {
                description: "Tool finished".into(),
            },
            "PostToolUseFailure" => AgentEventType::Processing {
                description: "Tool failed".into(),
            },
            "PermissionRequest" => AgentEventType::WaitingForInput,
            "SubagentStart" => AgentEventType::SubagentStarted {
                agent_type: Self::extract_subagent_type(json),
            },
            "SubagentStop" => AgentEventType::SubagentStopped {
                agent_type: Self::extract_subagent_type(json),
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

        let args_option = json
            .get("toolCall")
            .and_then(|t| t.get("args"))
            .or_else(|| json.get("tool_input"))
            .or_else(|| json.get("tool_arguments"))
            .or_else(|| json.get("args"))
            .or_else(|| json.get("input"))
            .or_else(|| {
                json.get("payload")
                    .and_then(|p| p.get("toolCall"))
                    .and_then(|t| t.get("args"))
            })
            .or_else(|| json.get("payload").and_then(|p| p.get("tool_input")))
            .or_else(|| json.get("payload").and_then(|p| p.get("tool_arguments")));

        let empty_json = serde_json::json!({});
        let args = args_option.unwrap_or(&empty_json);
        let instruction = Self::extract_instruction(json);

        match tool_name {
            "run_in_terminal" | "Bash" | "run_command" | "execute" => {
                let cmd = args["command"]
                    .as_str()
                    .or_else(|| args["cmd"].as_str())
                    .or_else(|| args["CommandLine"].as_str())
                    .or_else(|| args["script"].as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(if tool_name.is_empty() {
                        "Command"
                    } else {
                        tool_name
                    })
                    .to_string();
                AgentEventType::RunningCommand { cmd, instruction }
            }
            "create_file"
            | "search_replace"
            | "delete_file"
            | "Edit"
            | "Write"
            | "apply_patch"
            | "write_to_file"
            | "replace_file_content"
            | "multi_replace_file_content" => {
                let path = args["file_path"]
                    .as_str()
                    .or_else(|| args["path"].as_str())
                    .or_else(|| args["filePath"].as_str())
                    .or_else(|| args["TargetFile"].as_str())
                    .or_else(|| args["Target"].as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(if tool_name.is_empty() {
                        "file"
                    } else {
                        tool_name
                    })
                    .to_string();
                AgentEventType::WritingFile { path }
            }
            "read_file" | "Read" | "view_file" | "cat" => {
                let path = args["file_path"]
                    .as_str()
                    .or_else(|| args["path"].as_str())
                    .or_else(|| args["filePath"].as_str())
                    .or_else(|| args["AbsolutePath"].as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("file")
                    .to_string();
                AgentEventType::ReadingFile { path }
            }
            "grep_code" | "Grep" | "search_file" | "Glob" => {
                let query = args["query"]
                    .as_str()
                    .or_else(|| args["pattern"].as_str())
                    .or_else(|| args["path"].as_str())
                    .unwrap_or("")
                    .to_string();
                AgentEventType::SearchingCode { query }
            }
            "search_web" | "WebSearch" | "fetch_content" | "WebFetch" | "read_url_content" => {
                let url = args["query"]
                    .as_str()
                    .or_else(|| args["url"].as_str())
                    .or_else(|| args["Url"].as_str())
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
            AgentSource::Qoder => AgentCategory::Coding,
            AgentSource::Custom(_) => AgentCategory::General,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(event_name: &str, payload: serde_json::Value) -> AgentEvent {
        let adapter = CliAgentHookAdapter::new(AgentSource::ClaudeCode);
        let mut full = payload.clone();
        if let Some(obj) = full.as_object_mut() {
            obj.insert(
                "hook_event_name".to_string(),
                Value::String(event_name.to_string()),
            );
        }
        adapter.parse_hook_input(&full).unwrap()
    }

    #[test]
    fn test_stop_failure_maps_to_task_failed_with_message() {
        let event = parse(
            "StopFailure",
            json!({
                "conversation_id": "11111111-1111-1111-1111-111111111111",
                "error_type": "rate_limit",
                "error": "429",
                "error_message": "You've been rate limited"
            }),
        );
        match event.event_type {
            AgentEventType::TaskFailed { error } => {
                assert_eq!(error, "You've been rate limited");
            }
            other => panic!("expected TaskFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_stop_failure_falls_back_to_short_code() {
        let event = parse(
            "StopFailure",
            json!({
                "conversation_id": "11111111-1111-1111-1111-111111111111",
                "error_type": "authentication_failed",
                "error": "401"
            }),
        );
        match event.event_type {
            AgentEventType::TaskFailed { error } => {
                assert_eq!(error, "401");
            }
            other => panic!("expected TaskFailed, got {:?}", other),
        }
    }
}
