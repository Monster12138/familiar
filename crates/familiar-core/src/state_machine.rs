use std::sync::Arc;
use tokio::sync::RwLock;

use crate::event::{AgentEvent, AgentEventType};
use crate::event_bus::EventBus;
use crate::state::{AgentState, AgentStatus, FamiliarMood, RenderState};

#[derive(Debug, Clone)]
pub struct StateMachine {
    render_state: Arc<RwLock<RenderState>>,
    event_bus: EventBus,
}

impl StateMachine {
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            render_state: Arc::new(RwLock::new(RenderState::default())),
            event_bus,
        }
    }

    pub async fn get_state(&self) -> RenderState {
        self.render_state.read().await.clone()
    }

    pub async fn start_processing(&self) {
        let mut rx = self.event_bus.subscribe();
        let state_ref = self.render_state.clone();

        // Background timer to clean up completed agents after 4 seconds to let the celebration animation play out
        let state_ref_cleanup = self.render_state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                let mut state = state_ref_cleanup.write().await;
                let now = chrono::Utc::now();
                let initial_len = state.agents.len();
                
                // Then handle cleanup of completed agents
                state.agents.retain(|agent| {
                    if agent.status == AgentStatus::Completed {
                        if let Some(last) = agent.last_event_at {
                            if now.signed_duration_since(last).num_seconds() > 4 {
                                return false; // Remove if completed for > 4 seconds
                            }
                        }
                    }
                    true
                });

                if state.agents.len() != initial_len {
                    // Update global mood if agents were removed
                    if state.agents.is_empty() {
                        state.mood = FamiliarMood::Sleepy;
                    } else if state.agents.iter().any(|a| a.status == AgentStatus::Working) {
                        state.mood = FamiliarMood::Busy;
                    } else if state.agents.iter().any(|a| a.status == AgentStatus::Thinking) {
                        state.mood = FamiliarMood::Thinking;
                    } else if state.agents.iter().any(|a| a.status == AgentStatus::Completed) {
                        state.mood = FamiliarMood::Celebrating;
                    } else if state.agents.iter().any(|a| a.status == AgentStatus::WaitingInput) {
                        state.mood = FamiliarMood::Watching;
                    } else {
                        state.mood = FamiliarMood::Idle;
                    }
                }
            }
        });

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let mut state = state_ref.write().await;
                Self::apply_event(&mut state, &event);
            }
        });
    }

    fn apply_event(state: &mut RenderState, event: &AgentEvent) {
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
                AgentEventType::AgentStarted { instruction: Some(inst) } => {
                    agent.user_instruction = Some(inst.clone());
                }
                AgentEventType::AgentStarted { instruction: None } => {}
                AgentEventType::Thinking => {
                    agent.status = AgentStatus::Thinking;
                    agent.current_activity = Some("Thinking...".to_string());
                    state.mood = FamiliarMood::Thinking;
                }
                AgentEventType::Processing { description } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(description.clone());
                    state.mood = FamiliarMood::Busy;
                }
                AgentEventType::ReadingFile { path } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Reading {}", path));
                    state.mood = FamiliarMood::Busy;
                }
                AgentEventType::WritingFile { path } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Writing {}", path));
                    state.mood = FamiliarMood::Busy;
                }
                AgentEventType::RunningCommand { cmd, instruction } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Running `{}`", cmd));
                    if let Some(inst) = instruction {
                        agent.user_instruction = Some(inst.clone());
                    }
                    state.mood = FamiliarMood::Busy;
                }
                AgentEventType::SearchingCode { query } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Searching `{}`", query));
                    state.mood = FamiliarMood::Busy;
                }
                AgentEventType::BrowsingWeb { url } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Browsing {}", url));
                    state.mood = FamiliarMood::Busy;
                }
                AgentEventType::TaskCompleted { summary } => {
                    agent.status = AgentStatus::Completed;
                    agent.current_activity = Some(summary.clone());
                    state.mood = FamiliarMood::Celebrating;
                }
                AgentEventType::TaskFailed { error } => {
                    agent.status = AgentStatus::Failed;
                    agent.current_activity = Some(error.clone());
                    state.mood = FamiliarMood::Alarmed;
                }
                AgentEventType::WaitingForInput => {
                    agent.status = AgentStatus::WaitingInput;
                    agent.current_activity = Some("Waiting for user input...".to_string());
                    state.mood = FamiliarMood::Watching;
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
                    AgentEventType::RunningCommand { cmd, instruction } => {
                        new_agent.status = AgentStatus::Working;
                        new_agent.current_activity = Some(format!("Running `{}`", cmd));
                        if let Some(inst) = instruction {
                            new_agent.user_instruction = Some(inst.clone());
                        }
                    }
                    AgentEventType::AgentStarted { instruction } => {
                        new_agent.user_instruction = instruction.clone();
                    }
                    // (We can extend this to match all others, but for test purpose let's keep it simple or just leave it idle)
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

        // Aggregate global mood based on all active sessions
        if state.active_agent_count == 0 {
            state.mood = FamiliarMood::Sleepy;
        } else if state.agents.iter().any(|a| a.status == AgentStatus::Working) {
            state.mood = FamiliarMood::Busy;
        } else if state.agents.iter().any(|a| a.status == AgentStatus::Thinking) {
            state.mood = FamiliarMood::Thinking;
        } else if state.agents.iter().any(|a| a.status == AgentStatus::Completed) {
            state.mood = FamiliarMood::Celebrating;
        } else if state.agents.iter().any(|a| a.status == AgentStatus::WaitingInput) {
            state.mood = FamiliarMood::Watching;
        } else {
            state.mood = FamiliarMood::Idle;
        }

        tracing::info!(
            agent_id = %agent_id,
            event = ?event.event_type,
            mood = ?state.mood,
            "State updated from event"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{AgentEvent, AgentEventType, AgentSource, AgentCategory};
    use crate::event_bus::EventBus;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_state_machine_receives_stop_signal() {
        let bus = EventBus::new(100, 1000);
        let machine = StateMachine::new(bus.clone());
        machine.start_processing().await;

        let agent_id = Uuid::new_v4();

        // 1. Send SessionStart to create the agent
        bus.publish(AgentEvent {
            id: agent_id,
            timestamp: chrono::Utc::now(),
            source: AgentSource::Antigravity,
            category: AgentCategory::Coding,
            event_type: AgentEventType::AgentStarted { instruction: Some("Do a task".into()) },
            metadata: None,
        }).await.unwrap();
        
        // 2. Send Thinking to transition it
        bus.publish(AgentEvent {
            id: agent_id,
            timestamp: chrono::Utc::now(),
            source: AgentSource::Antigravity,
            category: AgentCategory::Coding,
            event_type: AgentEventType::Thinking,
            metadata: None,
        }).await.unwrap();

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
            event_type: AgentEventType::TaskCompleted { summary: "Task finished".to_string() },
            metadata: None,
        }).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let state = machine.get_state().await;
        // Should have transitioned to Completed
        assert_eq!(state.agents.len(), 1);
        assert_eq!(state.agents[0].status, AgentStatus::Completed);
        assert_eq!(state.mood, FamiliarMood::Celebrating);
    }
}
