use std::sync::Arc;
use tokio::sync::RwLock;

use crate::event::{AgentEvent, AgentEventType};
use crate::event_bus::EventBus;
use crate::state::{AgentState, AgentStatus, FamiliarMood, RenderState};

#[derive(Debug, Clone)]
pub struct StateMachine {
    render_state: Arc<RwLock<RenderState>>,
    event_bus: EventBus,
    celebration_secs: i64,
    sleep_timeout_secs: i64,
}

impl StateMachine {
    pub fn new(event_bus: EventBus, celebration_secs: u32, sleep_timeout_secs: u32) -> Self {
        Self {
            render_state: Arc::new(RwLock::new(RenderState::default())),
            event_bus,
            celebration_secs: celebration_secs as i64,
            sleep_timeout_secs: sleep_timeout_secs as i64,
        }
    }

    pub async fn get_state(&self) -> RenderState {
        self.render_state.read().await.clone()
    }

    pub async fn start_processing(&self) {
        let mut rx = self.event_bus.subscribe();
        let state_ref = self.render_state.clone();

        // Background timer to clean up completed agents and handle Idle -> Sleepy inactivity timeout
        let state_ref_cleanup = self.render_state.clone();
        let celebration_secs = self.celebration_secs;
        let sleep_timeout_secs = self.sleep_timeout_secs;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                let mut state = state_ref_cleanup.write().await;
                let now = chrono::Utc::now();

                // Handle cleanup of completed agents
                state.agents.retain(|agent| {
                    if agent.status == AgentStatus::Completed {
                        if let Some(last) = agent.last_event_at {
                            if now.signed_duration_since(last).num_seconds() > celebration_secs {
                                return false; // Remove if completed for > celebration_secs seconds
                            }
                        }
                    }
                    true
                });

                state.active_agent_count = state.agents.len();
                Self::update_mood(&mut state, now, sleep_timeout_secs);
            }
        });

        let sleep_timeout_secs = self.sleep_timeout_secs;
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let mut state = state_ref.write().await;
                Self::apply_event(&mut state, &event, sleep_timeout_secs);
            }
        });
    }

    fn apply_event(state: &mut RenderState, event: &AgentEvent, sleep_timeout_secs: i64) {
        state.last_activity_at = event.timestamp;
        let agent_id = event.id.to_string(); // In a real app we might group by process/session ID

        // Find or create agent state
        let agent_idx = state.agents.iter().position(|a| a.id == agent_id);

        if let Some(idx) = agent_idx {
            let agent = &mut state.agents[idx];
            agent.last_event_at = Some(event.timestamp);

            match &event.event_type {
                AgentEventType::AgentStopped => {
                    state.agents.remove(idx);
                }
                AgentEventType::AgentStarted {
                    instruction: Some(inst),
                } => {
                    agent.user_instruction = Some(inst.clone());
                }
                AgentEventType::AgentStarted { instruction: None } => {}
                AgentEventType::Thinking => {
                    agent.status = AgentStatus::Thinking;
                    agent.current_activity = Some("Thinking...".to_string());
                }
                AgentEventType::Processing { description } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(description.clone());
                }
                AgentEventType::ReadingFile { path } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Reading {}", path));
                }
                AgentEventType::WritingFile { path } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Writing {}", path));
                }
                AgentEventType::RunningCommand { cmd, instruction } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Running `{}`", cmd));
                    if let Some(inst) = instruction {
                        agent.user_instruction = Some(inst.clone());
                    }
                }
                AgentEventType::SearchingCode { query } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Searching `{}`", query));
                }
                AgentEventType::BrowsingWeb { url } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Browsing {}", url));
                }
                AgentEventType::TaskCompleted { summary } => {
                    agent.status = AgentStatus::Completed;
                    agent.current_activity = Some(summary.clone());
                }
                AgentEventType::TaskFailed { error } => {
                    agent.status = AgentStatus::Failed;
                    agent.current_activity = Some(error.clone());
                }
                AgentEventType::WaitingForInput => {
                    agent.status = AgentStatus::WaitingInput;
                    agent.current_activity = Some("Waiting for user input...".to_string());
                }
                _ => {}
            }
        } else {
            // Implicitly create agent if it doesn't exist and it's not a stop event
            if !matches!(event.event_type, AgentEventType::AgentStopped) {
                let mut new_agent = AgentState {
                    id: agent_id.clone(),
                    source: event.source.clone(),
                    category: event.category.clone(),
                    status: AgentStatus::Idle,
                    current_activity: None,
                    user_instruction: None,
                    progress: None,
                    started_at: Some(event.timestamp),
                    last_event_at: Some(event.timestamp),
                };

                // Apply initial state for this first event
                match &event.event_type {
                    AgentEventType::AgentStarted { instruction } => {
                        new_agent.user_instruction = instruction.clone();
                        new_agent.status = AgentStatus::Working;
                        new_agent.current_activity = Some("Started session".to_string());
                    }
                    AgentEventType::Thinking => {
                        new_agent.status = AgentStatus::Thinking;
                        new_agent.current_activity = Some("Thinking...".to_string());
                    }
                    AgentEventType::Processing { description } => {
                        new_agent.status = AgentStatus::Working;
                        new_agent.current_activity = Some(description.clone());
                    }
                    AgentEventType::ReadingFile { path } => {
                        new_agent.status = AgentStatus::Working;
                        new_agent.current_activity = Some(format!("Reading {}", path));
                    }
                    AgentEventType::WritingFile { path } => {
                        new_agent.status = AgentStatus::Working;
                        new_agent.current_activity = Some(format!("Writing {}", path));
                    }
                    AgentEventType::RunningCommand { cmd, instruction } => {
                        new_agent.status = AgentStatus::Working;
                        new_agent.current_activity = Some(format!("Running `{}`", cmd));
                        if let Some(inst) = instruction {
                            new_agent.user_instruction = Some(inst.clone());
                        }
                    }
                    AgentEventType::SearchingCode { query } => {
                        new_agent.status = AgentStatus::Working;
                        new_agent.current_activity = Some(format!("Searching `{}`", query));
                    }
                    AgentEventType::BrowsingWeb { url } => {
                        new_agent.status = AgentStatus::Working;
                        new_agent.current_activity = Some(format!("Browsing {}", url));
                    }
                    AgentEventType::TaskCompleted { summary } => {
                        new_agent.status = AgentStatus::Completed;
                        new_agent.current_activity = Some(summary.clone());
                    }
                    AgentEventType::TaskFailed { error } => {
                        new_agent.status = AgentStatus::Failed;
                        new_agent.current_activity = Some(error.clone());
                    }
                    AgentEventType::WaitingForInput => {
                        new_agent.status = AgentStatus::WaitingInput;
                        new_agent.current_activity = Some("Waiting for user input...".to_string());
                    }
                    _ => {}
                }

                state.agents.push(new_agent);
            }
        }

        // Recompute aggregates
        state.active_agent_count = state.agents.len();

        state.agents_by_category.clear();
        for agent in &state.agents {
            state
                .agents_by_category
                .entry(agent.category.clone())
                .or_default()
                .push(agent.clone());
        }

        let now = chrono::Utc::now();
        Self::update_mood(state, now, sleep_timeout_secs);

        tracing::info!(
            agent_id = %agent_id,
            event = ?event.event_type,
            mood = ?state.mood,
            "State updated from event"
        );
    }

    fn update_mood(state: &mut RenderState, now: chrono::DateTime<chrono::Utc>, sleep_timeout_secs: i64) {
        if state
            .agents
            .iter()
            .any(|a| a.status == AgentStatus::Working)
        {
            state.mood = FamiliarMood::Busy;
        } else if state
            .agents
            .iter()
            .any(|a| a.status == AgentStatus::Thinking)
        {
            state.mood = FamiliarMood::Thinking;
        } else if state
            .agents
            .iter()
            .any(|a| a.status == AgentStatus::Completed)
        {
            state.mood = FamiliarMood::Celebrating;
        } else if state
            .agents
            .iter()
            .any(|a| a.status == AgentStatus::WaitingInput)
        {
            state.mood = FamiliarMood::Watching;
        } else {
            let idle_secs = now.signed_duration_since(state.last_activity_at).num_seconds();
            if idle_secs >= sleep_timeout_secs {
                state.mood = FamiliarMood::Sleepy;
            } else {
                state.mood = FamiliarMood::Idle;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{AgentCategory, AgentEvent, AgentEventType, AgentSource};
    use crate::event_bus::EventBus;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_state_machine_receives_stop_signal() {
        let bus = EventBus::new(100, 1000);
        let machine = StateMachine::new(bus.clone(), 4, 300);
        machine.start_processing().await;

        let agent_id = Uuid::new_v4();

        // 1. Send SessionStart to create the agent
        bus.publish(AgentEvent {
            id: agent_id,
            timestamp: chrono::Utc::now(),
            source: AgentSource::Antigravity,
            category: AgentCategory::Coding,
            event_type: AgentEventType::AgentStarted {
                instruction: Some("Do a task".into()),
            },
            metadata: None,
        })
        .await
        .unwrap();

        // 2. Send Thinking to transition it
        bus.publish(AgentEvent {
            id: agent_id,
            timestamp: chrono::Utc::now(),
            source: AgentSource::Antigravity,
            category: AgentCategory::Coding,
            event_type: AgentEventType::Thinking,
            metadata: None,
        })
        .await
        .unwrap();

        // Let the processing loop tick
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let state = machine.get_state().await;
        assert_eq!(state.agents.len(), 1);
        assert_eq!(state.agents[0].status, AgentStatus::Thinking);

        // 2. Send Stop signal (TaskCompleted)
        bus.publish(AgentEvent {
            id: agent_id,
            timestamp: chrono::Utc::now(),
            source: AgentSource::Antigravity,
            category: AgentCategory::Coding,
            event_type: AgentEventType::TaskCompleted {
                summary: "Task finished".to_string(),
            },
            metadata: None,
        })
        .await
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let state = machine.get_state().await;
        // Should have transitioned to Completed
        assert_eq!(state.agents.len(), 1);
        assert_eq!(state.agents[0].status, AgentStatus::Completed);
        assert_eq!(state.mood, FamiliarMood::Celebrating);
    }

    #[tokio::test]
    async fn test_idle_to_sleep_inactivity_timeout() {
        let bus = EventBus::new(100, 1000);
        // Set sleep_timeout_secs = 1 second for fast test
        let machine = StateMachine::new(bus.clone(), 4, 1);
        machine.start_processing().await;

        // Initially in Idle mood
        let state = machine.get_state().await;
        assert_eq!(state.mood, FamiliarMood::Idle);

        // Wait 1.2 seconds for background timer tick
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

        // Should transition to Sleepy
        let state = machine.get_state().await;
        assert_eq!(state.mood, FamiliarMood::Sleepy);
    }
}
