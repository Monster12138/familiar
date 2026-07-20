use serde_json::json;
use std::time::Duration;
use tempfile::tempdir;
use tokio::time::sleep;

use familiar_core::event::AgentEventType;
use familiar_core::event_bus::EventBus;
use familiar_core::logger::init_logger;
use familiar_core::state_machine::StateMachine;
use familiar_hooks::antigravity::AntigravityHook;

#[tokio::test]
async fn test_hook_to_state_machine_flow() {
    // 1. Setup temporary directory for log file
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let log_dir = temp_dir.path().to_path_buf();

    // 2. Initialize tracing logger to write to the temp directory
    let _guard = init_logger(&log_dir, "familiar.log").expect("Failed to init logger");

    // 3. Setup Core components
    let event_bus = EventBus::new(100, 100);
    let state_machine = StateMachine::new(event_bus.clone());

    // Start the state machine processing loop
    state_machine.start_processing().await;

    // 4. Setup Hooks
    let antigravity_hook = AntigravityHook::new();

    // 5. Simulate AGY sending a hook payload (e.g., running a command)
    let agy_payload = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "run_command",
        "tool_arguments": {
            "CommandLine": "cargo test",
            "Cwd": "/Users/sam.gl/workspace/rust/familiar"
        }
    });

    // 6. Parse and publish the event
    let event = antigravity_hook
        .parse(&agy_payload)
        .expect("Failed to parse payload");

    // Assert parsing logic worked correctly
    match event.event_type {
        AgentEventType::RunningCommand { ref cmd } => {
            assert_eq!(cmd, "cargo test");
        }
        _ => panic!("Expected RunningCommand event"),
    }

    event_bus
        .publish(event)
        .await
        .expect("Failed to publish event");

    // 7. Wait for async processing
    sleep(Duration::from_millis(100)).await;

    // 8. Verify the state machine was updated
    let state = state_machine.get_state().await;
    assert_eq!(state.active_agent_count, 1);

    let agent_state = &state.agents[0];
    assert_eq!(
        agent_state.current_activity.as_deref(),
        Some("Running `cargo test`")
    );

    // 9. Verify the log file was created and contains the expected log output
    let log_file_path = log_dir.join("familiar.log");
    assert!(log_file_path.exists(), "Log file should be created");

    let log_content = std::fs::read_to_string(&log_file_path).expect("Failed to read log file");

    // Check that the tracing::info! macro wrote our event
    assert!(log_content.contains("State updated from event"));
    assert!(log_content.contains("RunningCommand"));
    assert!(log_content.contains("cargo test"));
}
