use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentSource {
    ClaudeCode,
    Codex,
    Antigravity,
    Qoder,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentCategory {
    Coding,
    Workflow,
    DevOps,
    General,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentEventType {
    // Lifecycle
    AgentStarted {
        instruction: Option<String>,
    },
    AgentStopped,

    // Core states
    Thinking,
    Processing {
        description: String,
    },

    // Coding specific
    ReadingFile {
        path: String,
    },
    WritingFile {
        path: String,
    },
    RunningCommand {
        cmd: String,
        instruction: Option<String>,
    },
    SearchingCode {
        query: String,
    },
    BrowsingWeb {
        url: String,
    },

    // Results
    TaskCompleted {
        summary: String,
    },
    TaskFailed {
        error: String,
    },
    WaitingForInput,

    // Subagents
    SubagentStarted {
        agent_type: String,
    },
    SubagentStopped {
        agent_type: String,
    },
}

impl AgentEventType {
    /// Returns a stable, non-sensitive event name suitable for logs and metrics.
    ///
    /// Event payload fields can contain prompts, commands, paths, or error text
    /// and must not be formatted into persistent operational logs.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::AgentStarted { .. } => "AgentStarted",
            Self::AgentStopped => "AgentStopped",
            Self::Thinking => "Thinking",
            Self::Processing { .. } => "Processing",
            Self::ReadingFile { .. } => "ReadingFile",
            Self::WritingFile { .. } => "WritingFile",
            Self::RunningCommand { .. } => "RunningCommand",
            Self::SearchingCode { .. } => "SearchingCode",
            Self::BrowsingWeb { .. } => "BrowsingWeb",
            Self::TaskCompleted { .. } => "TaskCompleted",
            Self::TaskFailed { .. } => "TaskFailed",
            Self::WaitingForInput => "WaitingForInput",
            Self::SubagentStarted { .. } => "SubagentStarted",
            Self::SubagentStopped { .. } => "SubagentStopped",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: AgentSource,
    pub category: AgentCategory,
    pub event_type: AgentEventType,
    pub metadata: Option<EventMetadata>,
}

#[cfg(test)]
mod tests {
    use super::AgentEventType;

    #[test]
    fn event_kind_does_not_include_sensitive_payload_fields() {
        let event = AgentEventType::RunningCommand {
            cmd: "secret command".to_string(),
            instruction: Some("secret prompt".to_string()),
        };

        assert_eq!(event.kind(), "RunningCommand");
        assert!(!event.kind().contains("secret"));
    }
}
