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

    pub fn parse_hook_input(&self, stdin_json: &Value) -> Result<AgentEvent> {
        // Here we attempt to identify common structures between agents.
        // We'll define a minimal unified parser, but different sources might need custom mapping.

        let event_name = stdin_json["hook_event_name"]
            .as_str()
            .or_else(|| stdin_json["event"].as_str())
            .unwrap_or("Unknown");

        let event_type = self.map_event_type(event_name, stdin_json);

        Ok(AgentEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: self.agent_source.clone(),
            category: self.derive_category(&self.agent_source),
            event_type,
            metadata: Some(EventMetadata {
                extra: stdin_json.clone(),
            }),
        })
    }

    fn map_event_type(&self, event_name: &str, json: &Value) -> AgentEventType {
        match event_name {
            "SessionStart" | "start" => AgentEventType::AgentStarted { instruction: None },
            "USER_INPUT" => {
                let instruction = json["content"].as_str().map(|s| s.to_string());
                AgentEventType::AgentStarted { instruction }
            }
            "Stop" | "stop" | "exit" => AgentEventType::AgentStopped,
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
            .unwrap_or("");

        let args = &json["toolCall"]["args"];
        let fallback_args = &json["tool_arguments"];

        let args = if args.is_null() { fallback_args } else { args };

        match tool_name {
            "Bash" | "run_command" | "execute" => {
                let cmd = args["command"]
                    .as_str()
                    .or_else(|| args["cmd"].as_str())
                    .or_else(|| args["CommandLine"].as_str())
                    .unwrap_or("")
                    .to_string();
                AgentEventType::RunningCommand { cmd }
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
                    .unwrap_or("")
                    .to_string();
                AgentEventType::WritingFile { path }
            }
            "view_file" | "read_file" | "cat" => {
                let path = args["AbsolutePath"]
                    .as_str()
                    .or_else(|| args["path"].as_str())
                    .unwrap_or("")
                    .to_string();
                AgentEventType::ReadingFile { path }
            }
            "search_web" | "read_url_content" => {
                let url = args["query"]
                    .as_str()
                    .or_else(|| args["Url"].as_str())
                    .unwrap_or("")
                    .to_string();
                AgentEventType::BrowsingWeb { url }
            }
            name if name.starts_with("mcp__") => AgentEventType::Processing {
                description: format!("MCP: {}", name),
            },
            other => AgentEventType::Processing {
                description: format!("Using tool {}", other),
            },
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
