use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::hook_trait::AgentHook;
use familiar_core::event::{AgentCategory, AgentEvent};

#[derive(Debug, Clone)]
pub struct QoderHook {}

impl Default for QoderHook {
    fn default() -> Self {
        Self::new()
    }
}

impl QoderHook {
    pub fn new() -> Self {
        Self {}
    }

    fn merge_hooks(existing: &mut serde_json::Value, payload: &serde_json::Value) {
        if let (Some(existing_obj), Some(payload_obj)) =
            (existing.as_object_mut(), payload.as_object())
        {
            for (k, v) in payload_obj {
                if !existing_obj.contains_key(k) {
                    existing_obj.insert(k.clone(), v.clone());
                } else if let (Some(existing_arr), Some(payload_arr)) = (
                    existing_obj.get_mut(k).and_then(|v| v.as_array_mut()),
                    v.as_array(),
                ) {
                    for item in payload_arr {
                        if !existing_arr.contains(item) {
                            existing_arr.push(item.clone());
                        }
                    }
                }
            }
        }
    }

    fn get_bin_path() -> String {
        if let Ok(exe) = std::env::current_exe() {
            if exe.file_name().and_then(|s| s.to_str()) == Some("familiar-cli") {
                return exe.to_string_lossy().to_string();
            }
            if let Some(parent) = exe.parent() {
                let cli = parent.join("familiar-cli");
                if cli.exists() {
                    return cli.to_string_lossy().to_string();
                }
                if let Some(grandparent) = parent.parent() {
                    let bin_cli = grandparent
                        .join("Resources")
                        .join("bin")
                        .join("familiar-cli");
                    if bin_cli.exists() {
                        return bin_cli.to_string_lossy().to_string();
                    }
                    let res_cli = grandparent.join("Resources").join("familiar-cli");
                    if res_cli.exists() {
                        return res_cli.to_string_lossy().to_string();
                    }
                }
                let res_cli = parent.join("Resources").join("familiar-cli");
                if res_cli.exists() {
                    return res_cli.to_string_lossy().to_string();
                }
            }
        }
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let dev_cli =
                std::path::PathBuf::from(&manifest_dir).join("../../target/release/familiar-cli");
            if dev_cli.exists() {
                return dev_cli.to_string_lossy().to_string();
            }
            let dev_cli_debug =
                std::path::PathBuf::from(&manifest_dir).join("../../target/debug/familiar-cli");
            if dev_cli_debug.exists() {
                return dev_cli_debug.to_string_lossy().to_string();
            }
        }
        "familiar-cli".to_string()
    }
}

#[async_trait]
impl AgentHook for QoderHook {
    fn name(&self) -> &str {
        "qoder"
    }

    fn category(&self) -> AgentCategory {
        AgentCategory::Coding
    }

    async fn start(&self, _sender: mpsc::Sender<AgentEvent>) -> Result<()> {
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn config_path(&self) -> Option<std::path::PathBuf> {
        let home = dirs::home_dir()?;
        Some(home.join(".qoder").join("settings.json"))
    }

    fn get_injection_payload(&self) -> Option<serde_json::Value> {
        let bin_path = Self::get_bin_path();
        Some(serde_json::json!({
            "hooks": {
                "UserPromptSubmit": [
                    {
                        "hooks": [
                            {
                                "type": "command",
                                "command": format!("{} hook --source qoder --event UserPromptSubmit", bin_path)
                            }
                        ]
                    }
                ],
                "PreToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": format!("{} hook --source qoder --event PreToolUse", bin_path)
                            }
                        ]
                    }
                ],
                "PostToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": format!("{} hook --source qoder --event PostToolUse", bin_path)
                            }
                        ]
                    }
                ],
                "PostToolUseFailure": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": format!("{} hook --source qoder --event PostToolUseFailure", bin_path)
                            }
                        ]
                    }
                ],
                "Stop": [
                    {
                        "hooks": [
                            {
                                "type": "command",
                                "command": format!("{} hook --source qoder --event Stop", bin_path)
                            }
                        ]
                    }
                ]
            }
        }))
    }

    fn is_injected(&self) -> bool {
        let path = match self.config_path() {
            Some(p) => p,
            None => return false,
        };

        if !path.exists() {
            return false;
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            let hooks_obj = json.get("hooks").and_then(|v| v.as_object());
            if let Some(hooks) = hooks_obj {
                for (_, event_val) in hooks.iter() {
                    if let Some(arr) = event_val.as_array() {
                        for item in arr {
                            if let Some(inner_hooks) = item.get("hooks").and_then(|v| v.as_array())
                            {
                                for inner_hook in inner_hooks {
                                    if let Some(cmd) =
                                        inner_hook.get("command").and_then(|v| v.as_str())
                                    {
                                        if cmd.contains("familiar-cli") && cmd.contains("qoder") {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn inject(&self) -> Result<()> {
        let path = self
            .config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        let payload = self
            .get_injection_payload()
            .ok_or_else(|| anyhow::anyhow!("No payload defined"))?;

        let mut config_json = serde_json::json!({});

        if path.exists() {
            let bak_path = path.with_extension(format!("bak.{}", chrono::Utc::now().timestamp()));
            std::fs::copy(&path, &bak_path)?;
            let content = std::fs::read_to_string(&path)?;
            if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&content) {
                config_json = existing;
            }
        } else if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if !config_json.is_object() {
            config_json = serde_json::json!({});
        }

        if let Some(payload_hooks) = payload.get("hooks") {
            if !config_json.as_object().unwrap().contains_key("hooks") {
                config_json
                    .as_object_mut()
                    .unwrap()
                    .insert("hooks".to_string(), serde_json::json!({}));
            }
            if let Some(config_hooks) = config_json.get_mut("hooks") {
                Self::merge_hooks(config_hooks, payload_hooks);
            }
        }

        let new_content = serde_json::to_string_pretty(&config_json)?;
        std::fs::write(&path, new_content)?;
        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let path = self
            .config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        if !path.exists() {
            return Ok(());
        }

        let bak_path =
            path.with_extension(format!("bak.uninstall.{}", chrono::Utc::now().timestamp()));
        std::fs::copy(&path, &bak_path)?;

        let content = std::fs::read_to_string(&path)?;
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(hooks_obj) = json.get_mut("hooks").and_then(|v| v.as_object_mut()) {
                for (_, event_array) in hooks_obj.iter_mut() {
                    if let Some(arr) = event_array.as_array_mut() {
                        for item in arr.iter_mut() {
                            if let Some(inner_hooks) =
                                item.get_mut("hooks").and_then(|v| v.as_array_mut())
                            {
                                inner_hooks.retain(|hook| {
                                    if let Some(cmd) = hook.get("command").and_then(|v| v.as_str())
                                    {
                                        !(cmd.contains("familiar-cli") && cmd.contains("qoder"))
                                    } else {
                                        true
                                    }
                                });
                            }
                        }
                        arr.retain(|item| {
                            if let Some(inner_hooks) = item.get("hooks").and_then(|v| v.as_array())
                            {
                                !inner_hooks.is_empty()
                            } else {
                                true
                            }
                        });
                    }
                }

                let empty_keys: Vec<String> = hooks_obj
                    .iter()
                    .filter(|(_, v)| v.as_array().is_some_and(|arr| arr.is_empty()))
                    .map(|(k, _)| k.clone())
                    .collect();
                for k in empty_keys {
                    hooks_obj.remove(&k);
                }
            }

            if let Some(obj) = json.as_object_mut() {
                if let Some(hooks) = obj.get("hooks") {
                    if hooks.as_object().is_some_and(|o| o.is_empty()) {
                        obj.remove("hooks");
                    }
                }
            }

            let new_content = serde_json::to_string_pretty(&json)?;
            std::fs::write(&path, new_content)?;
        }
        Ok(())
    }

    fn preview_inject(&self) -> Result<(String, String)> {
        let path = self
            .config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        let payload = self
            .get_injection_payload()
            .ok_or_else(|| anyhow::anyhow!("No payload defined"))?;

        let mut before_str = "{}".to_string();
        let mut config_json = serde_json::json!({});

        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                before_str = content.clone();
                if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&content) {
                    config_json = existing;
                }
            }
        }

        if !config_json.is_object() {
            config_json = serde_json::json!({});
        }

        if let Some(payload_hooks) = payload.get("hooks") {
            if !config_json.as_object().unwrap().contains_key("hooks") {
                config_json
                    .as_object_mut()
                    .unwrap()
                    .insert("hooks".to_string(), serde_json::json!({}));
            }
            if let Some(config_hooks) = config_json.get_mut("hooks") {
                Self::merge_hooks(config_hooks, payload_hooks);
            }
        }

        let after_str = serde_json::to_string_pretty(&config_json)?;
        Ok((before_str, after_str))
    }

    fn preview_uninstall(&self) -> Result<(String, String)> {
        let path = self
            .config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;

        let before_str = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string())
        } else {
            "{}".to_string()
        };

        let mut config_json = serde_json::from_str::<serde_json::Value>(&before_str)
            .unwrap_or_else(|_| serde_json::json!({}));

        if let Some(hooks_obj) = config_json.get_mut("hooks").and_then(|v| v.as_object_mut()) {
            for (_, event_array) in hooks_obj.iter_mut() {
                if let Some(arr) = event_array.as_array_mut() {
                    for item in arr.iter_mut() {
                        if let Some(inner_hooks) =
                            item.get_mut("hooks").and_then(|v| v.as_array_mut())
                        {
                            inner_hooks.retain(|hook| {
                                if let Some(cmd) = hook.get("command").and_then(|v| v.as_str()) {
                                    !(cmd.contains("familiar-cli") && cmd.contains("qoder"))
                                } else {
                                    true
                                }
                            });
                        }
                    }
                    arr.retain(|item| {
                        if let Some(inner_hooks) = item.get("hooks").and_then(|v| v.as_array()) {
                            !inner_hooks.is_empty()
                        } else {
                            true
                        }
                    });
                }
            }

            let empty_keys: Vec<String> = hooks_obj
                .iter()
                .filter(|(_, v)| v.as_array().is_some_and(|arr| arr.is_empty()))
                .map(|(k, _)| k.clone())
                .collect();
            for k in empty_keys {
                hooks_obj.remove(&k);
            }
        }

        if let Some(obj) = config_json.as_object_mut() {
            if let Some(hooks) = obj.get("hooks") {
                if hooks.as_object().is_some_and(|o| o.is_empty()) {
                    obj.remove("hooks");
                }
            }
        }

        let after_str = serde_json::to_string_pretty(&config_json)?;
        Ok((before_str, after_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::CliAgentHookAdapter;
    use familiar_core::event::{AgentEventType, AgentSource};
    use serde_json::json;

    #[test]
    fn test_qoder_hook_metadata_and_payload() {
        let hook = QoderHook::new();
        assert_eq!(hook.name(), "qoder");
        assert_eq!(hook.category(), AgentCategory::Coding);

        let payload = hook.get_injection_payload().unwrap();
        let hooks_obj = payload["hooks"].as_object().unwrap();
        assert!(hooks_obj.contains_key("UserPromptSubmit"));
        assert!(hooks_obj.contains_key("PreToolUse"));
        assert!(hooks_obj.contains_key("PostToolUse"));
        assert!(hooks_obj.contains_key("PostToolUseFailure"));
        assert!(hooks_obj.contains_key("Stop"));
    }

    #[test]
    fn test_qoder_adapter_parsing() {
        let adapter = CliAgentHookAdapter::new(AgentSource::Qoder);

        // 1. UserPromptSubmit
        let prompt_input = json!({
            "session_id": "qoder-session-1",
            "hook_event_name": "UserPromptSubmit",
            "prompt": "Fix sorting function"
        });
        let event = adapter.parse_hook_input(&prompt_input).unwrap();
        assert_eq!(event.source, AgentSource::Qoder);
        match event.event_type {
            AgentEventType::AgentStarted { instruction } => {
                assert_eq!(instruction.as_deref(), Some("Fix sorting function"));
            }
            _ => panic!("Expected AgentStarted"),
        }

        // 2. PreToolUse run_in_terminal
        let tool_input = json!({
            "session_id": "qoder-session-1",
            "hook_event_name": "PreToolUse",
            "tool_name": "run_in_terminal",
            "tool_input": {
                "command": "npm test"
            }
        });
        let event = adapter.parse_hook_input(&tool_input).unwrap();
        match event.event_type {
            AgentEventType::RunningCommand { cmd, .. } => {
                assert_eq!(cmd, "npm test");
            }
            _ => panic!("Expected RunningCommand"),
        }

        // 3. PreToolUse read_file
        let read_input = json!({
            "session_id": "qoder-session-1",
            "hook_event_name": "PreToolUse",
            "tool_name": "read_file",
            "tool_input": {
                "file_path": "/src/main.rs"
            }
        });
        let event = adapter.parse_hook_input(&read_input).unwrap();
        match event.event_type {
            AgentEventType::ReadingFile { path } => {
                assert_eq!(path, "/src/main.rs");
            }
            _ => panic!("Expected ReadingFile"),
        }

        // 4. PostToolUseFailure
        let fail_input = json!({
            "session_id": "qoder-session-1",
            "hook_event_name": "PostToolUseFailure"
        });
        let event = adapter.parse_hook_input(&fail_input).unwrap();
        match event.event_type {
            AgentEventType::Processing { description } => {
                assert_eq!(description, "Tool failed");
            }
            _ => panic!("Expected Processing (Tool failed)"),
        }

        // 5. Stop
        let stop_input = json!({
            "session_id": "qoder-session-1",
            "hook_event_name": "Stop"
        });
        let event = adapter.parse_hook_input(&stop_input).unwrap();
        match event.event_type {
            AgentEventType::TaskCompleted { summary } => {
                assert_eq!(summary, "Task finished");
            }
            _ => panic!("Expected TaskCompleted"),
        }
    }
}
