use crate::event::{AgentCategory, AgentSource};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Thinking,
    Working,
    WaitingInput,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FamiliarMood {
    Interacting,
    Thinking,
    Busy,
    Sleepy,
    Alarmed,
    Celebrating,
    Watching,
    Idle,
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
    pub id: String,
    pub source: AgentSource,
    pub category: AgentCategory,
    pub status: AgentStatus,
    pub current_activity: Option<String>,
    pub user_instruction: Option<String>,
    pub progress: Option<f32>,
    pub started_at: Option<DateTime<Utc>>,
    pub last_event_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderState {
    pub agents: Vec<AgentState>,
    pub active_agent_count: usize,
    pub agents_by_category: HashMap<AgentCategory, Vec<AgentState>>,
    pub stats: DailyStats,
    pub sources: HashMap<String, SourceStats>,
    pub mood: FamiliarMood,
    pub notifications: Vec<Notification>,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            agents: Vec::new(),
            active_agent_count: 0,
            agents_by_category: HashMap::new(),
            stats: DailyStats {
                interactions: 0,
                active_time_seconds: 0,
                tasks_completed: 0,
            },
            sources: HashMap::new(),
            mood: FamiliarMood::Sleepy,
            notifications: Vec::new(),
        }
    }
}
