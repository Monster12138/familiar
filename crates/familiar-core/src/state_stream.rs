use crate::event::{AgentCategory, AgentSource};
use crate::state::{
    AgentState, AgentStatus, DailyStats, FamiliarMood, Notification, RenderState, SourceStats,
};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub const STATE_STREAM_PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteAgentStateV1 {
    pub id: String,
    pub source: AgentSource,
    pub category: AgentCategory,
    pub status: AgentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteNotificationV1 {
    pub id: Uuid,
    pub message: String,
    pub created_at_ms: i64,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateSnapshotV1 {
    #[serde(rename = "type")]
    pub message_type: String,
    pub v: u16,
    pub server_id: Uuid,
    pub revision: u64,
    pub timestamp_ms: i64,
    #[serde(default)]
    pub agents: Vec<RemoteAgentStateV1>,
    #[serde(default)]
    pub active_agent_count: usize,
    #[serde(default)]
    pub mood: FamiliarMood,
    #[serde(default)]
    pub stats: DailyStats,
    #[serde(default)]
    pub sources: HashMap<String, SourceStats>,
    #[serde(default)]
    pub notifications: Vec<RemoteNotificationV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerHelloV1 {
    #[serde(rename = "type")]
    pub message_type: String,
    pub v: u16,
    pub server_id: Uuid,
    pub server_version: String,
    pub heartbeat_secs: u64,
}

impl StateSnapshotV1 {
    pub fn from_render_state(
        state: &RenderState,
        server_id: Uuid,
        revision: u64,
        max_task_summary_chars: usize,
        max_activity_summary_chars: usize,
    ) -> Self {
        Self {
            message_type: "state".to_string(),
            v: STATE_STREAM_PROTOCOL_VERSION,
            server_id,
            revision,
            timestamp_ms: state.last_activity_at.timestamp_millis(),
            agents: state
                .agents
                .iter()
                .map(|agent| RemoteAgentStateV1 {
                    id: agent.id.clone(),
                    source: agent.source.clone(),
                    category: agent.category.clone(),
                    status: agent.status.clone(),
                    progress: agent.progress,
                    started_at_ms: agent.started_at.map(|value| value.timestamp_millis()),
                    task_summary: agent
                        .user_instruction
                        .as_deref()
                        .and_then(|value| sanitize_summary(value, max_task_summary_chars)),
                    activity_summary: agent
                        .current_activity
                        .as_deref()
                        .and_then(|value| {
                            sanitize_activity_summary(
                                value,
                                &agent.status,
                                max_activity_summary_chars,
                            )
                        }),
                    last_event_at_ms: agent.last_event_at.map(|value| value.timestamp_millis()),
                })
                .collect(),
            active_agent_count: state.active_agent_count,
            mood: state.mood.clone(),
            stats: state.stats.clone(),
            sources: state.sources.clone(),
            notifications: state
                .notifications
                .iter()
                .filter_map(|notification| {
                    sanitize_summary(&notification.message, max_activity_summary_chars).map(
                        |message| RemoteNotificationV1 {
                            id: notification.id,
                            message,
                            created_at_ms: notification.created_at.timestamp_millis(),
                            read: notification.read,
                        },
                    )
                })
                .collect(),
        }
    }

    pub fn to_render_state(&self) -> RenderState {
        let agents = self
            .agents
            .iter()
            .map(|agent| AgentState {
                id: agent.id.clone(),
                source: agent.source.clone(),
                category: agent.category.clone(),
                status: agent.status.clone(),
                progress: agent.progress,
                started_at: agent
                    .started_at_ms
                    .and_then(|value| Utc.timestamp_millis_opt(value).single()),
                current_activity: agent.activity_summary.clone(),
                user_instruction: agent.task_summary.clone(),
                last_event_at: agent
                    .last_event_at_ms
                    .and_then(|value| Utc.timestamp_millis_opt(value).single()),
            })
            .collect::<Vec<_>>();
        let mut state = RenderState {
            agents,
            active_agent_count: self.agents.len(),
            mood: self.mood.clone(),
            stats: self.stats.clone(),
            sources: self.sources.clone(),
            notifications: self
                .notifications
                .iter()
                .filter_map(|notification| {
                    Utc.timestamp_millis_opt(notification.created_at_ms)
                        .single()
                        .map(|created_at| Notification {
                            id: notification.id,
                            message: notification.message.clone(),
                            created_at,
                            read: notification.read,
                        })
                })
                .collect(),
            last_activity_at: Utc
                .timestamp_millis_opt(self.timestamp_ms)
                .single()
                .unwrap_or_else(Utc::now),
            ..RenderState::default()
        };
        for agent in &state.agents {
            state
                .agents_by_category
                .entry(agent.category.clone())
                .or_default()
                .push(agent.clone());
        }
        state
    }
}

fn sanitize_summary(value: &str, max_chars: usize) -> Option<String> {
    if max_chars == 0 {
        return None;
    }
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return None;
    }
    let mut chars = normalized.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        let mut result = truncated;
        result.push('…');
        Some(result)
    } else {
        Some(truncated)
    }
}

fn sanitize_activity_summary(
    value: &str,
    status: &AgentStatus,
    max_chars: usize,
) -> Option<String> {
    let generic = if value.starts_with("Running `") {
        "Running command"
    } else if value.starts_with("Reading ") {
        "Reading file"
    } else if value.starts_with("Writing ") {
        "Writing file"
    } else if value.starts_with("Searching `") {
        "Searching code"
    } else if value.starts_with("Browsing ") {
        "Browsing web"
    } else if matches!(status, AgentStatus::Failed) {
        "Task failed"
    } else {
        value
    };
    sanitize_summary(generic, max_chars)
}

#[cfg(test)]
mod tests {
    use super::{
        sanitize_activity_summary, sanitize_summary, ServerHelloV1, StateSnapshotV1,
        STATE_STREAM_PROTOCOL_VERSION,
    };
    use crate::{
        event::{AgentCategory, AgentEvent, AgentEventType, AgentSource},
        event_bus::EventBus,
        state::AgentStatus,
        state_machine::StateMachine,
    };
    use chrono::Utc;
    use serde_json::Value;
    use uuid::Uuid;

    #[test]
    fn summary_is_normalized_and_unicode_safe() {
        assert_eq!(
            sanitize_summary("  hello\n  world ", 20),
            Some("hello world".into())
        );
        assert_eq!(sanitize_summary("你好世界", 2), Some("你好…".into()));
        assert_eq!(sanitize_summary("secret", 0), None);
        assert_eq!(
            sanitize_activity_summary("Running `cat secret.txt`", &AgentStatus::Working, 160),
            Some("Running command".into())
        );
    }

    #[test]
    fn hello_is_a_flat_json_object() {
        let hello = ServerHelloV1 {
            message_type: "hello".into(),
            v: STATE_STREAM_PROTOCOL_VERSION,
            server_id: Uuid::nil(),
            server_version: "test".into(),
            heartbeat_secs: 30,
        };
        let value = serde_json::to_value(hello).unwrap();
        assert_eq!(value["type"], "hello");
        assert_eq!(value["v"], STATE_STREAM_PROTOCOL_VERSION);
    }

    #[tokio::test]
    async fn snapshot_contains_self_contained_task_and_activity_summaries() {
        let bus = EventBus::new(8, 8);
        let machine = StateMachine::new(bus.clone(), 4, 300);
        machine.start_processing().await;
        bus.publish(AgentEvent {
            session_id: None,
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: AgentSource::Codex,
            category: AgentCategory::Coding,
            event_type: AgentEventType::AgentStarted {
                instruction: Some("fix the sync failure".into()),
            },
            metadata: None,
        })
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let snapshot = StateSnapshotV1::from_render_state(
            &machine.get_state().await,
            Uuid::new_v4(),
            machine.revision(),
            160,
            160,
        );
        let encoded: Value = serde_json::to_value(snapshot).unwrap();
        assert_eq!(encoded["agents"][0]["task_summary"], "fix the sync failure");
        assert_eq!(encoded["agents"][0]["activity_summary"], "Started session");
        assert_eq!(encoded["active_agent_count"], 1);
        assert_eq!(encoded["stats"]["interactions"], 1);
        assert_eq!(encoded["sources"]["Codex"]["events_processed"], 1);

        let decoded: StateSnapshotV1 = serde_json::from_value(encoded).unwrap();
        let round_trip = decoded.to_render_state();
        assert_eq!(round_trip.stats.interactions, 1);
        assert_eq!(
            round_trip.agents[0].user_instruction.as_deref(),
            Some("fix the sync failure")
        );
    }
}
