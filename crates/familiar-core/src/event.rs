use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentSource {
    ClaudeCode,
    Codex,
    Antigravity,
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
    AgentStarted { instruction: Option<String> },
    AgentStopped,

    // Core states
    Thinking,
    Processing { description: String },

    // Coding specific
    ReadingFile { path: String },
    WritingFile { path: String },
    RunningCommand { cmd: String },
    SearchingCode { query: String },
    BrowsingWeb { url: String },

    // Results
    TaskCompleted { summary: String },
    TaskFailed { error: String },
    WaitingForInput,

    // Subagents
    SubagentStarted { agent_type: String },
    SubagentStopped { agent_type: String },
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
