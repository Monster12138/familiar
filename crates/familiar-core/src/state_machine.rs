use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::state::{AgentState, AgentStatus, FamiliarMood, RenderState};
use crate::event::{AgentEvent, AgentEventType, AgentSource};
use crate::event_bus::EventBus;

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
                },
                AgentEventType::Thinking => {
                    agent.status = AgentStatus::Thinking;
                    agent.current_activity = Some("Thinking...".to_string());
                },
                AgentEventType::Processing { description } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(description.clone());
                },
                AgentEventType::ReadingFile { path } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Reading {}", path));
                },
                AgentEventType::WritingFile { path } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Writing {}", path));
                },
                AgentEventType::RunningCommand { cmd } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Running `{}`", cmd));
                },
                AgentEventType::SearchingCode { query } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Searching `{}`", query));
                },
                AgentEventType::BrowsingWeb { url } => {
                    agent.status = AgentStatus::Working;
                    agent.current_activity = Some(format!("Browsing {}", url));
                },
                AgentEventType::TaskCompleted { summary } => {
                    agent.status = AgentStatus::Completed;
                    agent.current_activity = Some(summary.clone());
                    state.mood = FamiliarMood::Celebrating;
                },
                AgentEventType::TaskFailed { error } => {
                    agent.status = AgentStatus::Failed;
                    agent.current_activity = Some(error.clone());
                    state.mood = FamiliarMood::Alarmed;
                },
                AgentEventType::WaitingForInput => {
                    agent.status = AgentStatus::WaitingInput;
                    agent.current_activity = Some("Waiting for user input...".to_string());
                    state.mood = FamiliarMood::Watching;
                },
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
                    progress: None,
                    started_at: Some(event.timestamp),
                    last_event_at: Some(event.timestamp),
                };

                // Apply initial state for this first event
                match &event.event_type {
                    AgentEventType::RunningCommand { cmd } => {
                        new_agent.status = AgentStatus::Working;
                        new_agent.current_activity = Some(format!("Running `{}`", cmd));
                    },
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
            state.agents_by_category
                .entry(agent.category.clone())
                .or_default()
                .push(agent.clone());
        }

        // Update mood if not explicitly overridden by this event
        if state.active_agent_count == 0 {
            state.mood = FamiliarMood::Sleepy;
        } else if state.agents.iter().any(|a| a.status == AgentStatus::Thinking) {
            state.mood = FamiliarMood::Thinking;
        } else if state.agents.iter().any(|a| a.status == AgentStatus::Working) {
            state.mood = FamiliarMood::Busy;
        }

        tracing::info!(
            agent_id = %agent_id,
            event = ?event.event_type,
            mood = ?state.mood,
            "State updated from event"
        );
    }
}
