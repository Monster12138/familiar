use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::hook_trait::AgentHook;
use familiar_core::event::{AgentCategory, AgentEvent};

/// DeepSeek Harness does not consume an external hooks configuration file the
/// way Claude Code does: hooks are wired through the `dsh-hooks-claude-code`
/// bridge plugin, whose `configPath` points at a plain hooks.json. This hook
/// manages that standalone file (`~/.dsh/familiar-hooks.json`) so the settings
/// panel can inject, preview and uninstall Familiar's entries without touching
/// the user's Claude Code config. The commands use the `deepseek-harness`
/// source so Familiar can tell DSH activity apart from real Claude Code.
///
/// The file only registers the events the bridge actually supports
/// (SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop,
/// SubagentStart, SubagentStop); unsupported points like PermissionRequest or
/// SessionEnd are silently dropped by the bridge, so injecting them would be
/// dead configuration.
#[derive(Debug, Clone)]
pub struct DeepSeekHarnessHook {}

impl Default for DeepSeekHarnessHook {
    fn default() -> Self {
        Self::new()
    }
}

impl DeepSeekHarnessHook {
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
impl AgentHook for DeepSeekHarnessHook {
    fn name(&self) -> &str {
        "deepseek-harness"
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
        Some(home.join(".dsh").join("familiar-hooks.json"))
    }

    fn get_injection_payload(&self) -> Option<serde_json::Value> {
        // Commands run through the DSH bridge's shell executor (PowerShell on
        // Windows), which cannot parse a quoted-path call like
        // `"C:\path\familiar-cli.exe" args`. Emit a bare unquoted path — valid
        // in PowerShell and bash alike as long as the CLI lives on a
        // space-free path (the standard install layout).
        let hook = |event: &str| serde_json::json!({ "hooks": [{ "type": "command", "command": crate::bin_path::hook_command("deepseek-harness", event, false) }] });
        Some(serde_json::json!({
            "hooks": {
                "SessionStart": [hook("SessionStart")],
                "UserPromptSubmit": [hook("UserPromptSubmit")],
                "PreToolUse": [serde_json::json!({ "matcher": "*", "hooks": [{ "type": "command", "command": crate::bin_path::hook_command("deepseek-harness", "PreToolUse", false) }] })],
                "PostToolUse": [serde_json::json!({ "matcher": "*", "hooks": [{ "type": "command", "command": crate::bin_path::hook_command("deepseek-harness", "PostToolUse", false) }] })],
                "Stop": [hook("Stop")],
                "SubagentStart": [hook("SubagentStart")],
                "SubagentStop": [hook("SubagentStop")]
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
                                            && cmd.contains("deepseek-harness")
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
                                        !(cmd.contains("familiar-cli")
                                            && cmd.contains("deepseek-harness"))
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
                                            && cmd.contains("deepseek-harness"))
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
    fn test_deepseek_harness_hook_payload() {
        let hook = DeepSeekHarnessHook::new();
        let payload = hook.get_injection_payload().unwrap();
        assert!(payload.get("hooks").is_some());

        // The bridge supports exactly these points; unsupported ones must not
        // be injected as dead configuration.
        for event in [
            "SessionStart",
            "UserPromptSubmit",
            "PreToolUse",
            "PostToolUse",
            "Stop",
            "SubagentStart",
            "SubagentStop",
        ] {
            assert!(
                payload["hooks"].get(event).is_some(),
                "expected hook entry for {}",
                event
            );
        }
        for event in ["PermissionRequest", "PostToolUseFailure", "SessionEnd"] {
            assert!(
                payload["hooks"].get(event).is_none(),
                "unsupported event {} should not be injected",
                event
            );
        }

        // Tool-scoped events carry the official matcher field.
        for event in ["PreToolUse", "PostToolUse"] {
            assert_eq!(
                payload["hooks"][event][0]["matcher"].as_str(),
                Some("*"),
                "expected matcher on {}",
                event
            );
        }

        // Every entry routes through familiar-cli with the deepseek-harness source.
        let hooks_obj = payload["hooks"].as_object().unwrap();
        for (event, entries) in hooks_obj {
            let inner = entries[0]["hooks"][0]["command"].as_str().unwrap();
            assert!(
                inner.contains("familiar-cli") && inner.contains("--source deepseek-harness"),
                "hook command for {} does not reference familiar-cli deepseek-harness: {}",
                event,
                inner
            );
        }

        assert_eq!(
            hook.config_path().unwrap().file_name().unwrap(),
            "familiar-hooks.json"
        );
    }
}
