use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::hook_trait::AgentHook;
use familiar_core::event::{AgentCategory, AgentEvent};

#[derive(Debug, Clone)]
pub struct CodexHook {}

impl Default for CodexHook {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexHook {
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

    /// Loads an existing config file as a JSON object, refusing to fall back
    /// to an empty document: overwriting a malformed or non-object config
    /// would silently discard the user's other hooks and settings.
    fn parse_existing_config(path: &std::path::Path, content: &str) -> Result<serde_json::Value> {
        let existing = serde_json::from_str::<serde_json::Value>(content).map_err(|e| {
            anyhow::anyhow!(
                "{} is not valid JSON ({e}); refusing to overwrite it",
                path.display()
            )
        })?;
        if !existing.is_object() {
            anyhow::bail!(
                "{} is not a JSON object; refusing to overwrite it",
                path.display()
            );
        }
        Ok(existing)
    }
}

#[async_trait]
impl AgentHook for CodexHook {
    fn name(&self) -> &str {
        "codex"
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
        Some(home.join(".codex").join("hooks.json"))
    }

    fn get_injection_payload(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "hooks": {
                "SessionStart": [{ "hooks": [{ "type": "command", "command": crate::bin_path::hook_command("codex", "SessionStart", true) }] }],
                "SessionEnd": [{ "hooks": [{ "type": "command", "command": crate::bin_path::hook_command("codex", "SessionEnd", true) }] }],
                "PreToolUse": [{ "hooks": [{ "type": "command", "command": crate::bin_path::hook_command("codex", "PreToolUse", true) }] }],
                "PostToolUse": [{ "hooks": [{ "type": "command", "command": crate::bin_path::hook_command("codex", "PostToolUse", true) }] }],
                "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": crate::bin_path::hook_command("codex", "UserPromptSubmit", true) }] }],
                "Stop": [{ "hooks": [{ "type": "command", "command": crate::bin_path::hook_command("codex", "Stop", true) }] }]
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
                if let Some(pre_tool) = hooks.get("PreToolUse").and_then(|v| v.as_array()) {
                    for item in pre_tool {
                        if let Some(inner_hooks) = item.get("hooks").and_then(|v| v.as_array()) {
                            for inner_hook in inner_hooks {
                                if let Some(cmd) =
                                    inner_hook.get("command").and_then(|v| v.as_str())
                                {
                                    if cmd.contains("familiar-cli") && cmd.contains("codex") {
                                        return true;
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
            let content = std::fs::read_to_string(&path)?;
            if !content.trim().is_empty() {
                config_json = Self::parse_existing_config(&path, &content)?;
            }
            let bak_path = crate::hook_trait::backup_path(&path, "bak")?;
            std::fs::copy(&path, &bak_path)?;
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
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

        let bak_path = crate::hook_trait::backup_path(&path, "bak.uninstall")?;
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
                                        !(cmd.contains("familiar-cli") && cmd.contains("codex"))
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
                                        !(cmd.contains("familiar-cli") && cmd.contains("codex"))
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
