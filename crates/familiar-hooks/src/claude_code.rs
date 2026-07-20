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
        Some(home.join(".claude.json"))
    }

    fn get_injection_payload(&self) -> Option<serde_json::Value> {
        let bin_path = "familiar-cli";
        Some(serde_json::json!({
            "on_pre_tool_use": format!("{} hook --source claude-code --event PreToolUse", bin_path),
            "on_post_tool_use": format!("{} hook --source claude-code --event PostToolUse", bin_path)
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
            let pre = json["on_pre_tool_use"].as_str().unwrap_or("");
            let post = json["on_post_tool_use"].as_str().unwrap_or("");
            pre.contains("familiar-cli") || post.contains("familiar-cli")
        } else {
            false
        }
    }

    fn inject(&self) -> Result<()> {
        let path = self.config_path().ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        let payload = self.get_injection_payload().ok_or_else(|| anyhow::anyhow!("No payload defined"))?;

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

        if let (Some(obj), Some(payload_obj)) = (config_json.as_object_mut(), payload.as_object()) {
            for (k, v) in payload_obj {
                obj.insert(k.clone(), v.clone());
            }
        }

        let new_content = serde_json::to_string_pretty(&config_json)?;
        std::fs::write(&path, new_content)?;
        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let path = self.config_path().ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        if !path.exists() {
            return Ok(());
        }

        let bak_path = path.with_extension(format!("bak.uninstall.{}", chrono::Utc::now().timestamp()));
        std::fs::copy(&path, &bak_path)?;

        let content = std::fs::read_to_string(&path)?;
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.as_object_mut() {
                let mut keys_to_remove = Vec::new();
                
                if let Some(v) = obj.get("on_pre_tool_use") {
                    if v.as_str().unwrap_or("").contains("familiar-cli") {
                        keys_to_remove.push("on_pre_tool_use".to_string());
                    }
                }
                if let Some(v) = obj.get("on_post_tool_use") {
                    if v.as_str().unwrap_or("").contains("familiar-cli") {
                        keys_to_remove.push("on_post_tool_use".to_string());
                    }
                }

                for k in keys_to_remove {
                    obj.remove(&k);
                }
            }

            let new_content = serde_json::to_string_pretty(&json)?;
            std::fs::write(&path, new_content)?;
        }
        Ok(())
    }

    fn preview_inject(&self) -> Result<(String, String)> {
        let path = self.config_path().ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        let payload = self.get_injection_payload().ok_or_else(|| anyhow::anyhow!("No payload defined"))?;
        
        let mut config_json = serde_json::json!({});
        let mut before_content = String::new();

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&content) {
                config_json = existing.clone();
                before_content = serde_json::to_string_pretty(&existing)?;
            }
        }

        if let (Some(obj), Some(payload_obj)) = (config_json.as_object_mut(), payload.as_object()) {
            for (k, v) in payload_obj {
                obj.insert(k.clone(), v.clone());
            }
        }

        let after_content = serde_json::to_string_pretty(&config_json)?;
        Ok((before_content, after_content))
    }

    fn preview_uninstall(&self) -> Result<(String, String)> {
        let path = self.config_path().ok_or_else(|| anyhow::anyhow!("Could not get config path"))?;
        if !path.exists() {
            return Ok((String::new(), String::new()));
        }

        let content = std::fs::read_to_string(&path)?;
        let before_content = if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&content) {
            serde_json::to_string_pretty(&existing)?
        } else {
            String::new()
        };

        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.as_object_mut() {
                let mut keys_to_remove = Vec::new();
                if let Some(v) = obj.get("on_pre_tool_use") {
                    if v.as_str().unwrap_or("").contains("familiar-cli") {
                        keys_to_remove.push("on_pre_tool_use".to_string());
                    }
                }
                if let Some(v) = obj.get("on_post_tool_use") {
                    if v.as_str().unwrap_or("").contains("familiar-cli") {
                        keys_to_remove.push("on_post_tool_use".to_string());
                    }
                }

                for k in keys_to_remove {
                    obj.remove(&k);
                }
            }

            let after_content = serde_json::to_string_pretty(&json)?;
            return Ok((before_content, after_content));
        }

        Ok((String::new(), String::new()))
    }
}
