use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamiliarConfig {
    pub core: CoreConfig,
    pub ui: UiConfig,
    pub plugins: PluginConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub db_path: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub animations_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled_plugins: Vec<String>,
    pub plugin_dir: String,
}

impl Default for FamiliarConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig {
                db_path: "familiar.db".to_string(),
                log_level: "info".to_string(),
            },
            ui: UiConfig {
                theme: "default".to_string(),
                animations_enabled: true,
            },
            plugins: PluginConfig {
                enabled_plugins: Vec::new(),
                plugin_dir: "plugins".to_string(),
            },
        }
    }
}

impl FamiliarConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: FamiliarConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
