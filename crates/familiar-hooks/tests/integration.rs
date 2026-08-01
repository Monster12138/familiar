use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

use familiar_core::event::AgentEventType;
use familiar_core::event_bus::EventBus;
use familiar_core::state_machine::StateMachine;
use familiar_hooks::antigravity::AntigravityHook;

#[tokio::test]
async fn test_hook_to_state_machine_flow() {
    // 1. Setup Core components
    let event_bus = EventBus::new(100, 100);
    let state_machine = StateMachine::new(event_bus.clone(), 4, 300);

    // Start the state machine processing loop
    state_machine.start_processing().await;

    // 2. Setup Hooks
    let antigravity_hook = AntigravityHook::new();

    // 3. Simulate AGY sending a hook payload (e.g., running a command)
    let agy_payload = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "run_command",
        "tool_arguments": {
            "CommandLine": "cargo test",
            "Cwd": "/path/to/project"
        }
    });

    // 4. Parse and publish the event
    let event = antigravity_hook
        .parse("PreToolUse", &agy_payload)
        .expect("Failed to parse payload");

    // Assert parsing logic worked correctly
    match event.event_type {
        AgentEventType::RunningCommand { ref cmd, .. } => {
            assert_eq!(cmd, "cargo test");
        }
        _ => panic!("Expected RunningCommand event"),
    }

    event_bus
        .publish(event)
        .await
        .expect("Failed to publish event");

    // 5. Wait for async processing
    sleep(Duration::from_millis(100)).await;

    // 6. Verify the state machine was updated
    let state = state_machine.get_state().await;
    assert_eq!(state.active_agent_count, 1);

    let agent_state = &state.agents[0];
    assert_eq!(
        agent_state.current_activity.as_deref(),
        Some("Running `cargo test`")
    );
}
