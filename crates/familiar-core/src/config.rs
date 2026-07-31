use anyhow::Result;
use serde::{Deserialize, Serialize, Serializer};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamiliarConfig {
    pub general: GeneralConfig,
    pub hooks: HooksConfig,
    pub renderer: RendererConfig,
    pub api: ApiConfig,
    pub notifications: NotificationsConfig,
    pub achievements: AchievementsConfig,
    #[serde(default)]
    pub sessions: SessionsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub language: String,
    pub auto_start: bool,
    pub data_retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    pub enabled: Vec<String>,
    pub socket_path: Option<String>,
    pub tcp_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererConfig {
    pub enabled: Vec<String>,
    #[serde(rename = "desktop-pet")]
    pub desktop_pet: DesktopPetConfig,
    #[serde(rename = "menu-bar")]
    pub menu_bar: MenuBarConfig,
    pub dashboard: DashboardConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopPetConfig {
    pub sprite: String,
    #[serde(serialize_with = "serialize_one_decimal")]
    pub scale: f32,
    pub position: String,
    pub always_on_top: bool,
    #[serde(default = "default_true")]
    pub show_on_all_desktops: bool,
    pub opacity: f32,
    #[serde(default = "default_true")]
    pub show_task_bubble: bool,
    #[serde(default = "default_true")]
    pub show_pet: bool,
    #[serde(default = "default_true")]
    pub show_dashboard: bool,
    #[serde(default = "default_celebration_secs")]
    pub celebration_secs: u32,
}

fn default_true() -> bool {
    true
}

fn serialize_one_decimal<S>(value: &f32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded = (f64::from(*value) * 10.0).round() / 10.0;
    serializer.serialize_f64(rounded)
}

fn default_celebration_secs() -> u32 {
    4
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuBarConfig {
    pub show_active_count: bool,
    pub show_today_stats: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub enabled: bool,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    pub dnd_start: String,
    pub dnd_end: String,
    pub min_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementsConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SessionsConfig {
    #[serde(default)]
    pub hidden_sessions: Vec<String>,
}

impl Default for FamiliarConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                language: "zh-CN".to_string(),
                auto_start: true,
                data_retention_days: 90,
            },
            hooks: HooksConfig {
                enabled: vec!["claude-code".to_string(), "codex".to_string()],
                socket_path: Some("/tmp/familiar.sock".to_string()),
                tcp_port: Some(9528),
            },
            renderer: RendererConfig {
                enabled: vec!["desktop-pet".to_string(), "menu-bar".to_string()],
                desktop_pet: DesktopPetConfig {
                    sprite: "pixel-cat".to_string(),
                    scale: 2.0,
                    position: "bottom-right".to_string(),
                    always_on_top: true,
                    show_on_all_desktops: true,
                    opacity: 0.95,
                    show_task_bubble: true,
                    show_pet: true,
                    show_dashboard: true,
                    celebration_secs: 4,
                },
                menu_bar: MenuBarConfig {
                    show_active_count: true,
                    show_today_stats: true,
                },
                dashboard: DashboardConfig { port: 9527 },
            },
            api: ApiConfig {
                enabled: true,
                port: 9528,
            },
            notifications: NotificationsConfig {
                dnd_start: "22:00".to_string(),
                dnd_end: "08:00".to_string(),
                min_level: "info".to_string(),
            },
            achievements: AchievementsConfig { enabled: true },
            sessions: SessionsConfig::default(),
        }
    }
}

impl FamiliarConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: FamiliarConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}
