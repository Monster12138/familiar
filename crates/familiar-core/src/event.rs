use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentSource {
    System,
    User,
    Application,
    Network,
    Hardware,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentCategory {
    Info,
    Warning,
    Error,
    Action,
    Metric,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentEventType {
    Created,
    Updated,
    Deleted,
    StateChanged,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub tags: Vec<String>,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: AgentSource,
    pub category: AgentCategory,
    pub event_type: AgentEventType,
    pub message: String,
    pub metadata: Option<EventMetadata>,
}
