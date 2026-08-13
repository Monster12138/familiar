use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::hook_trait::AgentHook;
use familiar_core::event::{AgentCategory, AgentEvent};

#[derive(Debug, Clone)]
pub struct ClaudeCodeHook {}

impl Default for ClaudeCodeHook {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeCodeHook {
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

    pub fn clean_legacy_claude_json() -> Result<()> {
        let Some(home) = dirs::home_dir() else {
            return Ok(());
        };
        let legacy_path = home.join(".claude.json");
        if !legacy_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&legacy_path)?;
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.as_object_mut() {
                let mut modified = false;
                if let Some(v) = obj.get("on_pre_tool_use") {
                    if v.as_str().unwrap_or("").contains("familiar-cli") {
                        obj.remove("on_pre_tool_use");
                        modified = true;
                    }
                }
                if let Some(v) = obj.get("on_post_tool_use") {
                    if v.as_str().unwrap_or("").contains("familiar-cli") {
                        obj.remove("on_post_tool_use");
                        modified = true;
                    }
                }
                if modified {
                    let new_content = serde_json::to_string_pretty(&json)?;
                    std::fs::write(&legacy_path, new_content)?;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl AgentHook for ClaudeCodeHook {
    fn name(&self) -> &str {
        "claude-code"
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
        Some(home.join(".claude").join("settings.json"))
    }

    fn get_injection_payload(&self) -> Option<serde_json::Value> {
        let bin_path = crate::bin_path::resolve_cli_bin_path();
        let hook = |event: &str| serde_json::json!({ "hooks": [{ "type": "command", "command": format!("\"{}\" hook --source claude-code --event {}", bin_path, event) }] });
        Some(serde_json::json!({
            "hooks": {
                "SessionStart": [hook("SessionStart")],
                "UserPromptSubmit": [hook("UserPromptSubmit")],
                "PreToolUse": [serde_json::json!({ "matcher": "*", "hooks": [{ "type": "command", "command": format!("\"{}\" hook --source claude-code --event PreToolUse", bin_path) }] })],
                "PostToolUse": [serde_json::json!({ "matcher": "*", "hooks": [{ "type": "command", "command": format!("\"{}\" hook --source claude-code --event PostToolUse", bin_path) }] })],
                "PostToolUseFailure": [serde_json::json!({ "matcher": "*", "hooks": [{ "type": "command", "command": format!("\"{}\" hook --source claude-code --event PostToolUseFailure", bin_path) }] })],
                "PermissionRequest": [serde_json::json!({ "matcher": "*", "hooks": [{ "type": "command", "command": format!("\"{}\" hook --source claude-code --event PermissionRequest", bin_path) }] })],
                "SubagentStart": [hook("SubagentStart")],
                "SubagentStop": [hook("SubagentStop")],
                "Stop": [hook("Stop")],
                "SessionEnd": [hook("SessionEnd")]
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
                                        if cmd.contains("familiar-cli")
                                            && cmd.contains("claude-code")
                                        {
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
        let _ = Self::clean_legacy_claude_json();

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
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
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

        let new_content = serde_json::to_string_pretty(&config_json)?;
        std::fs::write(&path, new_content)?;
        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let _ = Self::clean_legacy_claude_json();

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
                                        !(cmd.contains("familiar-cli")
                                            && cmd.contains("claude-code"))
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

        let mut config_json = serde_json::json!({});
        let mut before_content = String::new();

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&content) {
                config_json = existing.clone();
                before_content = serde_json::to_string_pretty(&existing)?;
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

        let after_content = serde_json::to_string_pretty(&config_json)?;
        Ok((before_content, after_content))
    }

    fn preview_uninstall(&self) -> Result<(String, String)> {
        let path = self
            .config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        if !path.exists() {
            return Ok((String::new(), String::new()));
        }

        let content = std::fs::read_to_string(&path)?;
        let before_content =
            if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&content) {
                serde_json::to_string_pretty(&existing)?
            } else {
                String::new()
            };

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
                                        !(cmd.contains("familiar-cli")
                                            && cmd.contains("claude-code"))
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

            let after_content = serde_json::to_string_pretty(&json)?;
            return Ok((before_content, after_content));
        }

        Ok((String::new(), String::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_code_hook_logic() {
        let hook = ClaudeCodeHook::new();
        let payload = hook.get_injection_payload().unwrap();
        assert!(payload.get("hooks").is_some());

        // All official events that the shared adapter maps must be registered
        // so the claude-code source actually reports them.
        for event in [
            "SessionStart",
            "UserPromptSubmit",
            "PreToolUse",
            "PostToolUse",
            "PostToolUseFailure",
            "PermissionRequest",
            "SubagentStart",
            "SubagentStop",
            "Stop",
            "SessionEnd",
        ] {
            assert!(
                payload["hooks"].get(event).is_some(),
                "expected hook entry for {}",
                event
            );
        }

        // Tool-scoped events carry the official matcher field.
        for event in [
            "PreToolUse",
            "PostToolUse",
            "PostToolUseFailure",
            "PermissionRequest",
        ] {
            assert_eq!(
                payload["hooks"][event][0]["matcher"].as_str(),
                Some("*"),
                "expected matcher on {}",
                event
            );
        }

        // Every entry routes through familiar-cli with the claude-code source.
        let hooks_obj = payload["hooks"].as_object().unwrap();
        for (event, entries) in hooks_obj {
            let inner = entries[0]["hooks"][0]["command"].as_str().unwrap();
            assert!(
                inner.contains("familiar-cli") && inner.contains("--source claude-code"),
                "hook command for {} does not reference familiar-cli claude-code: {}",
                event,
                inner
            );
        }

        assert_eq!(
            hook.config_path().unwrap().file_name().unwrap(),
            "settings.json"
        );
    }
}
