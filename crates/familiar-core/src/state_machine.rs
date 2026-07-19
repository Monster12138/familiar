use anyhow::Result;
use crate::state::{AgentState, AgentStatus, FamiliarMood};
use crate::event::AgentEvent;

#[derive(Debug)]
pub struct StateMachine {
    current_state: AgentState,
}

impl StateMachine {
    pub fn new(initial_state: AgentState) -> Self {
        Self {
            current_state: initial_state,
        }
    }

    pub fn default_state() -> AgentState {
        AgentState {
            status: AgentStatus::Idle,
            mood: FamiliarMood::Neutral,
            energy_level: 100,
            current_activity: None,
        }
    }

    pub fn get_state(&self) -> &AgentState {
        &self.current_state
    }

    pub fn process_event(&mut self, _event: &AgentEvent) -> Result<()> {
        // Basic state transition logic based on event would go here
        Ok(())
    }
}
