use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Active,
    Sleeping,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FamiliarMood {
    Happy,
    Neutral,
    Sad,
    Focused,
    Curious,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStats {
    pub events_processed: u64,
    pub errors_encountered: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub interactions: u32,
    pub active_time_seconds: u64,
    pub tasks_completed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: uuid::Uuid,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub status: AgentStatus,
    pub mood: FamiliarMood,
    pub energy_level: u8,
    pub current_activity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderState {
    pub agent: AgentState,
    pub stats: DailyStats,
    pub sources: HashMap<String, SourceStats>,
    pub active_notifications: Vec<Notification>,
}
