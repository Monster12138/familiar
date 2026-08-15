use crate::state::{AgentStatus, EventStatusMap};
use anyhow::Result;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::BTreeMap;
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
    #[serde(default)]
    pub cleanup: CleanupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub language: String,
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
    #[serde(default)]
    pub dashboard_style: DashboardStyle,
    #[serde(default)]
    pub dashboard_position: DashboardPosition,
    #[serde(default)]
    pub dashboard_layout: DashboardLayout,
    #[serde(default)]
    pub dashboard_alignment: DashboardAlignment,
    #[serde(default = "default_celebration_secs")]
    pub celebration_secs: u32,
    #[serde(default = "default_sleep_timeout_secs")]
    pub sleep_timeout_secs: u32,
    #[serde(default)]
    pub event_status_map: BTreeMap<String, EventStatus>,
}

impl DesktopPetConfig {
    /// Build the runtime lookup map used by the state machine. Unknown values
    /// and the non-mappable `AgentStopped` key are dropped so they fall back
    /// to the built-in behavior.
    pub fn event_status_agent_map(&self) -> EventStatusMap {
        self.event_status_map
            .iter()
            .filter(|(k, _)| k.as_str() != "AgentStopped")
            .filter_map(|(k, v)| v.to_agent_status().map(|s| (k.clone(), s)))
            .collect()
    }
}

fn default_true() -> bool {
    true
}

fn default_cleanup_age_days() -> u32 {
    90
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DashboardStyle {
    Minimal,
    #[default]
    #[serde(other)]
    Classic,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DashboardPosition {
    Left,
    #[default]
    Bottom,
    Right,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DashboardLayout {
    Horizontal,
    #[default]
    #[serde(other)]
    Vertical,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DashboardAlignment {
    Top,
    Center,
    #[default]
    #[serde(other)]
    Bottom,
}

/// Config-facing pet status value (kebab-case in TOML). Distinct from
/// `AgentStatus`, which serializes PascalCase as part of the frontend JSON
/// contract and must not change. `Unknown` catches unrecognized config values
/// so they fall back to the built-in behavior at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventStatus {
    Idle,
    Thinking,
    Working,
    Pending,
    Completed,
    Failed,
    #[serde(other)]
    Unknown,
}

impl EventStatus {
    pub fn to_agent_status(self) -> Option<AgentStatus> {
        match self {
            Self::Idle => Some(AgentStatus::Idle),
            Self::Thinking => Some(AgentStatus::Thinking),
            Self::Working => Some(AgentStatus::Working),
            Self::Pending => Some(AgentStatus::Pending),
            Self::Completed => Some(AgentStatus::Completed),
            Self::Failed => Some(AgentStatus::Failed),
            Self::Unknown => None,
        }
    }
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

fn default_sleep_timeout_secs() -> u32 {
    300
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

/// Data-cleanup settings. `age_days` is the retention window for both backup
/// and log files; `0` disables the age limit. Field-level serde defaults keep
/// a partially-written `[cleanup]` section (e.g. only `age_days`) loadable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleanupConfig {
    #[serde(default = "default_true")]
    pub backup_files: bool,
    #[serde(default = "default_true")]
    pub log_files: bool,
    #[serde(default = "default_cleanup_age_days")]
    pub age_days: u32,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            backup_files: true,
            log_files: true,
            age_days: 90,
        }
    }
}

impl Default for FamiliarConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                language: "zh-CN".to_string(),
                data_retention_days: 90,
            },
            hooks: HooksConfig {
                enabled: vec!["claude-code".to_string(), "codex".to_string()],
                socket_path: Some("/tmp/familiar.sock".to_string()),
                tcp_port: Some(19527),
            },
            renderer: RendererConfig {
                enabled: vec!["desktop-pet".to_string(), "menu-bar".to_string()],
                desktop_pet: DesktopPetConfig {
                    sprite: "tabby-cat".to_string(),
                    scale: 2.0,
                    position: "bottom-right".to_string(),
                    always_on_top: true,
                    show_on_all_desktops: true,
                    opacity: 0.95,
                    show_task_bubble: true,
                    show_pet: true,
                    show_dashboard: true,
                    dashboard_style: DashboardStyle::Classic,
                    dashboard_position: DashboardPosition::Bottom,
                    dashboard_layout: DashboardLayout::Vertical,
                    dashboard_alignment: DashboardAlignment::Bottom,
                    celebration_secs: 4,
                    sleep_timeout_secs: 300,
                    event_status_map: BTreeMap::new(),
                },
                menu_bar: MenuBarConfig {
                    show_active_count: true,
                    show_today_stats: true,
                },
                dashboard: DashboardConfig { port: 9527 },
            },
            api: ApiConfig {
                enabled: true,
                port: 19527,
            },
            notifications: NotificationsConfig {
                dnd_start: "22:00".to_string(),
                dnd_end: "08:00".to_string(),
                min_level: "info".to_string(),
            },
            achievements: AchievementsConfig { enabled: true },
            sessions: SessionsConfig::default(),
            cleanup: CleanupConfig::default(),
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
