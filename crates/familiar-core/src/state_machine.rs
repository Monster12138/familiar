use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::event::{AgentEvent, AgentEventType};
use crate::event_bus::EventBus;
use crate::state::{AgentState, AgentStatus, EventStatusMap, FamiliarMood, RenderState};

#[derive(Debug, Clone)]
pub struct StateMachine {
    render_state: Arc<RwLock<RenderState>>,
    event_bus: EventBus,
    celebration_secs: i64,
    sleep_timeout_secs: i64,
    event_status_map: Arc<std::sync::RwLock<EventStatusMap>>,
    revision: Arc<AtomicU64>,
}

impl StateMachine {
    pub fn new(event_bus: EventBus, celebration_secs: u32, sleep_timeout_secs: u32) -> Self {
        Self::with_event_map(
            event_bus,
            celebration_secs,
            sleep_timeout_secs,
            Arc::new(std::sync::RwLock::new(EventStatusMap::new())),
        )
    }

    /// Constructs a state machine that consults a shared, hot-reloadable
    /// event-status override map. The map is populated by the config-save path
    /// and read on every event, so mapping changes apply without a restart.
    pub fn with_event_map(
        event_bus: EventBus,
        celebration_secs: u32,
        sleep_timeout_secs: u32,
        event_status_map: Arc<std::sync::RwLock<EventStatusMap>>,
    ) -> Self {
        Self {
            render_state: Arc::new(RwLock::new(RenderState::default())),
            event_bus,
            celebration_secs: celebration_secs as i64,
            sleep_timeout_secs: sleep_timeout_secs as i64,
            event_status_map,
            revision: Arc::new(AtomicU64::new(1)),
        }
    }

    pub async fn get_state(&self) -> RenderState {
        self.render_state.read().await.clone()
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::SeqCst)
    }

    /// Removes an agent session from the render state. Returns `true` if an
    /// agent with the given id was present. A later event for the same id
    /// (e.g. the agent is still active) will re-create it.
    pub async fn remove_agent(&self, id: &str) -> bool {
        let mut state = self.render_state.write().await;
        let len_before = state.agents.len();
        state.agents.retain(|a| a.id != id);
        if state.agents.len() == len_before {
            return false;
        }

        // Recompute aggregates (mirrors the tail of apply_event). Clone the
        // agent list first: the guard's Deref hides field-level borrows, so
        // iterating `state.agents` while writing `state.agents_by_category`
        // would otherwise conflict.
        state.active_agent_count = state.agents.len();
        state.agents_by_category.clear();
        let agents = state.agents.clone();
        for agent in &agents {
            state
                .agents_by_category
                .entry(agent.category.clone())
                .or_default()
                .push(agent.clone());
        }
        Self::update_mood(&mut state, chrono::Utc::now(), self.sleep_timeout_secs);
        self.revision.fetch_add(1, Ordering::SeqCst);
        true
    }

    pub async fn start_processing(&self) {
        let mut rx = self.event_bus.subscribe();
        let state_ref = self.render_state.clone();
        let revision_ref = self.revision.clone();

        // Background timer to clean up completed agents and handle Idle -> Sleepy inactivity timeout
        let state_ref_cleanup = self.render_state.clone();
        let celebration_secs = self.celebration_secs;
        let sleep_timeout_secs = self.sleep_timeout_secs;
        let revision_ref_cleanup = self.revision.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                let mut state = state_ref_cleanup.write().await;
                let now = chrono::Utc::now();
                let len_before = state.agents.len();
                let mood_before = state.mood.clone();

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

                if len_before != state.agents.len() || mood_before != state.mood {
                    revision_ref_cleanup.fetch_add(1, Ordering::SeqCst);
                }
            }
        });

        let sleep_timeout_secs = self.sleep_timeout_secs;
        let event_status_map = self.event_status_map.clone();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let mut state = state_ref.write().await;
                Self::apply_event(&mut state, &event, sleep_timeout_secs, &event_status_map);
                revision_ref.fetch_add(1, Ordering::SeqCst);
            }
        });
    }

    /// A new prompt starts a fresh round. Clear any stale terminal state
    /// (`Completed`/`Failed`) so the pet leaves the celebrating state and the
    /// cleanup timer is not kept deferring removal by refreshed timestamps.
    /// `target` is the mapped `AgentStarted` status (defaults to `Working`).
    fn reset_stale_terminal_status(agent: &mut AgentState, target: AgentStatus) {
        if matches!(agent.status, AgentStatus::Completed | AgentStatus::Failed) {
            agent.status = target;
            agent.current_activity = Some("Started session".to_string());
        }
    }

    fn apply_event(
        state: &mut RenderState,
        event: &AgentEvent,
        sleep_timeout_secs: i64,
        event_status_map: &std::sync::RwLock<EventStatusMap>,
    ) {
        state.last_activity_at = event.timestamp;
        let agent_id = event.id.to_string(); // In a real app we might group by process/session ID

        // Clone the (tiny) override map once per event so no lock guard spans
        // the match below. `resolve` returns the configured status for a kind,
        // falling back to the built-in behavior when unmapped.
        let status_map = event_status_map
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let resolve = |kind: &str, fallback: AgentStatus| -> AgentStatus {
            status_map.get(kind).cloned().unwrap_or(fallback)
        };

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
                    Self::reset_stale_terminal_status(
                        agent,
                        resolve("AgentStarted", AgentStatus::Working),
                    );
                }
                AgentEventType::AgentStarted { instruction: None } => {
                    Self::reset_stale_terminal_status(
                        agent,
                        resolve("AgentStarted", AgentStatus::Working),
                    );
                }
                AgentEventType::Thinking => {
                    agent.status = resolve("Thinking", AgentStatus::Thinking);
                    agent.current_activity = Some("Thinking...".to_string());
                }
                AgentEventType::Processing { description } => {
                    agent.status = resolve("Processing", AgentStatus::Working);
                    agent.current_activity = Some(description.clone());
                }
                AgentEventType::ReadingFile { path } => {
                    agent.status = resolve("ReadingFile", AgentStatus::Working);
                    agent.current_activity = Some(format!("Reading {}", path));
                }
                AgentEventType::WritingFile { path } => {
                    agent.status = resolve("WritingFile", AgentStatus::Working);
                    agent.current_activity = Some(format!("Writing {}", path));
                }
                AgentEventType::RunningCommand { cmd, instruction } => {
                    agent.status = resolve("RunningCommand", AgentStatus::Working);
                    agent.current_activity = Some(format!("Running `{}`", cmd));
                    if let Some(inst) = instruction {
                        agent.user_instruction = Some(inst.clone());
                    }
                }
                AgentEventType::SearchingCode { query } => {
                    agent.status = resolve("SearchingCode", AgentStatus::Working);
                    agent.current_activity = Some(format!("Searching `{}`", query));
                }
                AgentEventType::BrowsingWeb { url } => {
                    agent.status = resolve("BrowsingWeb", AgentStatus::Working);
                    agent.current_activity = Some(format!("Browsing {}", url));
                }
                AgentEventType::TaskCompleted { summary } => {
                    agent.status = resolve("TaskCompleted", AgentStatus::Completed);
                    agent.current_activity = Some(summary.clone());
                }
                AgentEventType::TaskFailed { error } => {
                    agent.status = resolve("TaskFailed", AgentStatus::Failed);
                    agent.current_activity = Some(error.clone());
                }
                AgentEventType::WaitingForInput => {
                    agent.status = resolve("WaitingForInput", AgentStatus::WaitingInput);
                    agent.current_activity = Some("Waiting for user input...".to_string());
                }
                _ => {
                    // Subagent events are a no-op by default but can be mapped.
                    if let Some(st) = status_map.get(event.event_type.kind()) {
                        agent.status = st.clone();
                    }
                }
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
                        new_agent.status = resolve("AgentStarted", AgentStatus::Working);
                        new_agent.current_activity = Some("Started session".to_string());
                    }
                    AgentEventType::Thinking => {
                        new_agent.status = resolve("Thinking", AgentStatus::Thinking);
                        new_agent.current_activity = Some("Thinking...".to_string());
                    }
                    AgentEventType::Processing { description } => {
                        new_agent.status = resolve("Processing", AgentStatus::Working);
                        new_agent.current_activity = Some(description.clone());
                    }
                    AgentEventType::ReadingFile { path } => {
                        new_agent.status = resolve("ReadingFile", AgentStatus::Working);
                        new_agent.current_activity = Some(format!("Reading {}", path));
                    }
                    AgentEventType::WritingFile { path } => {
                        new_agent.status = resolve("WritingFile", AgentStatus::Working);
                        new_agent.current_activity = Some(format!("Writing {}", path));
                    }
                    AgentEventType::RunningCommand { cmd, instruction } => {
                        new_agent.status = resolve("RunningCommand", AgentStatus::Working);
                        new_agent.current_activity = Some(format!("Running `{}`", cmd));
                        if let Some(inst) = instruction {
                            new_agent.user_instruction = Some(inst.clone());
                        }
                    }
                    AgentEventType::SearchingCode { query } => {
                        new_agent.status = resolve("SearchingCode", AgentStatus::Working);
                        new_agent.current_activity = Some(format!("Searching `{}`", query));
                    }
                    AgentEventType::BrowsingWeb { url } => {
                        new_agent.status = resolve("BrowsingWeb", AgentStatus::Working);
                        new_agent.current_activity = Some(format!("Browsing {}", url));
                    }
                    AgentEventType::TaskCompleted { summary } => {
                        new_agent.status = resolve("TaskCompleted", AgentStatus::Completed);
                        new_agent.current_activity = Some(summary.clone());
                    }
                    AgentEventType::TaskFailed { error } => {
                        new_agent.status = resolve("TaskFailed", AgentStatus::Failed);
                        new_agent.current_activity = Some(error.clone());
                    }
                    AgentEventType::WaitingForInput => {
                        new_agent.status = resolve("WaitingForInput", AgentStatus::WaitingInput);
                        new_agent.current_activity = Some("Waiting for user input...".to_string());
                    }
                    _ => {
                        if let Some(st) = status_map.get(event.event_type.kind()) {
                            new_agent.status = st.clone();
                        }
                    }
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
            event_type = event.event_type.kind(),
            mood = ?state.mood,
            "State updated from event"
        );
    }

    fn update_mood(
        state: &mut RenderState,
        now: chrono::DateTime<chrono::Utc>,
        sleep_timeout_secs: i64,
    ) {
        // An error state is the most important thing to surface, so it wins
        // over any other agent's activity.
        if state.agents.iter().any(|a| a.status == AgentStatus::Failed) {
            state.mood = FamiliarMood::Alarmed;
        } else if state
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
            let idle_secs = now
                .signed_duration_since(state.last_activity_at)
                .num_seconds();
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
    async fn test_new_prompt_resets_completed_to_working() {
        let bus = EventBus::new(100, 1000);
        let machine = StateMachine::new(bus.clone(), 4, 300);
        machine.start_processing().await;

        let agent_id = Uuid::new_v4();
        let event = |event_type| AgentEvent {
            id: agent_id,
            timestamp: chrono::Utc::now(),
            source: AgentSource::ClaudeCode,
            category: AgentCategory::Coding,
            event_type,
            metadata: None,
        };

        // Round 1: prompt start, then Stop (TaskCompleted).
        bus.publish(event(AgentEventType::AgentStarted {
            instruction: Some("First task".into()),
        }))
        .await
        .unwrap();
        bus.publish(event(AgentEventType::TaskCompleted {
            summary: "Task finished".into(),
        }))
        .await
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.agents[0].status, AgentStatus::Completed);
        assert_eq!(state.mood, FamiliarMood::Celebrating);

        // Round 2: a new prompt arrives before the completed agent is cleaned
        // up. It must leave the stale Completed state immediately instead of
        // staying in Celebrating for the whole round.
        bus.publish(event(AgentEventType::AgentStarted {
            instruction: Some("Second task".into()),
        }))
        .await
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.agents[0].status, AgentStatus::Working);
        assert_eq!(
            state.agents[0].current_activity.as_deref(),
            Some("Started session")
        );
        assert_eq!(state.mood, FamiliarMood::Busy);
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

    #[tokio::test]
    async fn test_remove_agent_drops_session_and_recomputes_state() {
        let bus = EventBus::new(100, 1000);
        let machine = StateMachine::new(bus.clone(), 4, 300);
        machine.start_processing().await;

        let agent_id = Uuid::new_v4();
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

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.agents.len(), 1);

        // Unknown id leaves the state untouched
        assert!(!machine.remove_agent("missing").await);

        // Known id is removed and aggregates are recomputed
        assert!(machine.remove_agent(&agent_id.to_string()).await);
        let state = machine.get_state().await;
        assert!(state.agents.is_empty());
        assert_eq!(state.active_agent_count, 0);
        assert!(state.agents_by_category.is_empty());
    }

    #[tokio::test]
    async fn test_state_machine_revision_increments_on_event() {
        let bus = EventBus::new(100, 1000);
        let machine = StateMachine::new(bus.clone(), 4, 300);
        machine.start_processing().await;

        let initial_rev = machine.revision();
        let agent_id = Uuid::new_v4();

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

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let new_rev = machine.revision();
        assert!(
            new_rev > initial_rev,
            "Revision should increment after receiving an event"
        );
    }

    fn machine_with_map(
        map: EventStatusMap,
    ) -> (
        StateMachine,
        Arc<std::sync::RwLock<EventStatusMap>>,
        EventBus,
    ) {
        let bus = EventBus::new(100, 1000);
        let arc = Arc::new(std::sync::RwLock::new(map));
        let machine = StateMachine::with_event_map(bus.clone(), 4, 300, arc.clone());
        (machine, arc, bus)
    }

    fn event_for(agent_id: Uuid, event_type: AgentEventType) -> AgentEvent {
        AgentEvent {
            id: agent_id,
            timestamp: chrono::Utc::now(),
            source: AgentSource::Antigravity,
            category: AgentCategory::Coding,
            event_type,
            metadata: None,
        }
    }

    #[tokio::test]
    async fn test_event_status_map_override_applied() {
        let mut map = EventStatusMap::new();
        map.insert("Thinking".to_string(), AgentStatus::Working);
        let (machine, _arc, bus) = machine_with_map(map);
        machine.start_processing().await;

        let agent_id = Uuid::new_v4();
        bus.publish(event_for(agent_id, AgentEventType::Thinking))
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.agents[0].status, AgentStatus::Working);
        assert_eq!(state.mood, FamiliarMood::Busy);
    }

    #[tokio::test]
    async fn test_event_status_map_unmapped_falls_back() {
        let (machine, _arc, bus) = machine_with_map(EventStatusMap::new());
        machine.start_processing().await;

        let agent_id = Uuid::new_v4();
        bus.publish(event_for(agent_id, AgentEventType::Thinking))
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.agents[0].status, AgentStatus::Thinking);
        assert_eq!(state.mood, FamiliarMood::Thinking);
    }

    #[tokio::test]
    async fn test_event_status_map_hot_reload() {
        let (machine, arc, bus) = machine_with_map(EventStatusMap::new());
        machine.start_processing().await;

        let agent_id = Uuid::new_v4();
        bus.publish(event_for(agent_id, AgentEventType::Thinking))
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.agents[0].status, AgentStatus::Thinking);

        // Update the shared map as the config-save path would; the next event
        // must pick up the new mapping without a restart.
        arc.write()
            .unwrap()
            .insert("Thinking".to_string(), AgentStatus::Working);
        bus.publish(event_for(agent_id, AgentEventType::Thinking))
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.agents[0].status, AgentStatus::Working);
        assert_eq!(state.mood, FamiliarMood::Busy);
    }

    #[tokio::test]
    async fn test_agent_started_reset_uses_mapped_status() {
        let mut map = EventStatusMap::new();
        map.insert("AgentStarted".to_string(), AgentStatus::Thinking);
        let (machine, _arc, bus) = machine_with_map(map);
        machine.start_processing().await;

        let agent_id = Uuid::new_v4();
        bus.publish(event_for(
            agent_id,
            AgentEventType::AgentStarted {
                instruction: Some("First task".into()),
            },
        ))
        .await
        .unwrap();
        bus.publish(event_for(
            agent_id,
            AgentEventType::TaskCompleted {
                summary: "Task finished".into(),
            },
        ))
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.agents[0].status, AgentStatus::Completed);

        // A new prompt resets the stale Completed state to the mapped status.
        bus.publish(event_for(
            agent_id,
            AgentEventType::AgentStarted {
                instruction: Some("Second task".into()),
            },
        ))
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.agents[0].status, AgentStatus::Thinking);
        assert_eq!(
            state.agents[0].current_activity.as_deref(),
            Some("Started session")
        );
    }

    #[tokio::test]
    async fn test_subagent_event_mapped() {
        let mut map = EventStatusMap::new();
        map.insert("SubagentStarted".to_string(), AgentStatus::Thinking);
        let (machine, _arc, bus) = machine_with_map(map);
        machine.start_processing().await;

        let agent_id = Uuid::new_v4();
        bus.publish(event_for(
            agent_id,
            AgentEventType::SubagentStarted {
                agent_type: "general-purpose".into(),
            },
        ))
        .await
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.agents[0].status, AgentStatus::Thinking);
    }

    #[tokio::test]
    async fn test_task_failed_sets_alarmed_mood() {
        let (machine, _arc, bus) = machine_with_map(EventStatusMap::new());
        machine.start_processing().await;

        let agent_id = Uuid::new_v4();
        bus.publish(event_for(
            agent_id,
            AgentEventType::AgentStarted {
                instruction: Some("Do a task".into()),
            },
        ))
        .await
        .unwrap();
        bus.publish(event_for(
            agent_id,
            AgentEventType::TaskFailed {
                error: "rate limited".into(),
            },
        ))
        .await
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.agents[0].status, AgentStatus::Failed);
        assert_eq!(state.mood, FamiliarMood::Alarmed);
    }

    #[tokio::test]
    async fn test_alarmed_wins_over_working_agent() {
        let (machine, _arc, bus) = machine_with_map(EventStatusMap::new());
        machine.start_processing().await;

        // Agent A fails...
        let failed_id = Uuid::new_v4();
        bus.publish(event_for(
            failed_id,
            AgentEventType::AgentStarted {
                instruction: Some("A".into()),
            },
        ))
        .await
        .unwrap();
        bus.publish(event_for(
            failed_id,
            AgentEventType::TaskFailed {
                error: "network down".into(),
            },
        ))
        .await
        .unwrap();

        // ...while agent B is still working.
        let working_id = Uuid::new_v4();
        bus.publish(event_for(
            working_id,
            AgentEventType::AgentStarted {
                instruction: Some("B".into()),
            },
        ))
        .await
        .unwrap();
        bus.publish(event_for(working_id, AgentEventType::Thinking))
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = machine.get_state().await;
        assert_eq!(state.mood, FamiliarMood::Alarmed);
    }
}
