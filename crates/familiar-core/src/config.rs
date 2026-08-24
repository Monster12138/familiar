use crate::state::{AgentStatus, EventStatusMap};
use anyhow::Result;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamiliarConfig {
    #[serde(default)]
    pub runtime: RuntimeConfig,
    pub general: GeneralConfig,
    pub hooks: HooksConfig,
    pub renderer: RendererConfig,
    pub api: ApiConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub remote: RemoteConfig,
    pub notifications: NotificationsConfig,
    pub achievements: AchievementsConfig,
    #[serde(default)]
    pub sessions: SessionsConfig,
    #[serde(default)]
    pub cleanup: CleanupConfig,
    #[serde(default)]
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeMode {
    #[default]
    Local,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub mode: RuntimeMode,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            mode: RuntimeMode::Local,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub cert_path: Option<String>,
    #[serde(default)]
    pub key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerAuthConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Persistent token file used by the server for bearer authentication.
    #[serde(default)]
    pub token_file: Option<String>,
    /// Generate `token_file` on first server startup when it does not exist.
    #[serde(default)]
    pub auto_generate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateStreamConfig {
    #[serde(default = "default_max_updates_per_second")]
    pub max_updates_per_second: u32,
    #[serde(default = "default_summary_chars")]
    pub max_task_summary_chars: usize,
    #[serde(default = "default_summary_chars")]
    pub max_activity_summary_chars: usize,
}

fn default_max_updates_per_second() -> u32 {
    10
}

fn default_summary_chars() -> usize {
    160
}

impl Default for StateStreamConfig {
    fn default() -> Self {
        Self {
            max_updates_per_second: default_max_updates_per_second(),
            max_task_summary_chars: default_summary_chars(),
            max_activity_summary_chars: default_summary_chars(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    #[serde(default)]
    pub bind: Option<String>,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub auth: ServerAuthConfig,
    #[serde(default)]
    pub state_stream: StateStreamConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default = "default_state_stream_path")]
    pub path: String,
    #[serde(default)]
    pub tls: bool,
    /// Path to the local file containing the remote server bearer token.
    /// The token itself is intentionally never serialized into this config.
    #[serde(default)]
    pub token_file: Option<String>,
    #[serde(default = "default_connect_timeout_secs")]
    pub connect_timeout_secs: u64,
    #[serde(default = "default_reconnect_initial_secs")]
    pub reconnect_initial_secs: u64,
    #[serde(default = "default_reconnect_max_secs")]
    pub reconnect_max_secs: u64,
}

fn default_state_stream_path() -> String {
    "/api/v1/state-stream".to_string()
}

fn default_connect_timeout_secs() -> u64 {
    10
}

fn default_reconnect_initial_secs() -> u64 {
    1
}

fn default_reconnect_max_secs() -> u64 {
    30
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            path: default_state_stream_path(),
            tls: false,
            token_file: None,
            connect_timeout_secs: default_connect_timeout_secs(),
            reconnect_initial_secs: default_reconnect_initial_secs(),
            reconnect_max_secs: default_reconnect_max_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub language: String,
    pub data_retention_days: u32,
    /// Whether the first-run onboarding page has been completed. Defaults to
    /// false so existing config files without the key still show it once.
    #[serde(default)]
    pub onboarded: bool,
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
    /// Opacity applied while the pointer hovers over the pet, so it does not
    /// block the content behind it. Values >= `opacity` disable the effect.
    #[serde(default = "default_hover_opacity")]
    pub hover_opacity: f32,
    #[serde(default = "default_true")]
    pub show_task_bubble: bool,
    #[serde(default = "default_true")]
    pub show_pet: bool,
    #[serde(default = "default_true")]
    pub show_dashboard: bool,
    #[serde(default)]
    pub click_through: bool,
    /// Fade the pet out while the pointer is over its sprite.
    #[serde(default)]
    pub hide_on_hover: bool,
    /// Snap the pet to the nearest screen corner when a drag ends close to it.
    #[serde(default = "default_true")]
    pub snap_to_corner: bool,
    /// Distance threshold (physical pixels) within which a corner attracts the
    /// pet on drag end.
    #[serde(default = "default_snap_threshold")]
    pub snap_threshold: f32,
    #[serde(default)]
    pub show_window_frame: bool,
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

fn default_hover_opacity() -> f32 {
    0.35
}

fn default_snap_threshold() -> f32 {
    60.0
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

/// How often the app auto-checks for updates (used with `last_check_at`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateInterval {
    #[default]
    Daily,
    Weekly,
}

/// In-app update settings. `last_check_at` is written by the app (not the UI)
/// as a Unix-epoch-seconds timestamp each time a successful check runs; it
/// gates the auto-check interval across restarts. `skipped_version` suppresses
/// the reminder for one specific version until a newer one appears, while
/// `ignored_versions` suppresses it permanently.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateConfig {
    #[serde(default = "default_true")]
    pub check_on_startup: bool,
    #[serde(default)]
    pub interval: UpdateInterval,
    #[serde(default)]
    pub skipped_version: Option<String>,
    #[serde(default)]
    pub ignored_versions: Vec<String>,
    #[serde(default)]
    pub last_check_at: Option<u64>,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            check_on_startup: true,
            interval: UpdateInterval::Daily,
            skipped_version: None,
            ignored_versions: Vec::new(),
            last_check_at: None,
        }
    }
}

impl Default for FamiliarConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                language: "zh-CN".to_string(),
                data_retention_days: 90,
                onboarded: false,
            },
            runtime: RuntimeConfig::default(),
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
                    click_through: false,
                    hide_on_hover: false,
                    hover_opacity: 0.35,
                    snap_to_corner: true,
                    snap_threshold: 60.0,
                    show_window_frame: false,
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
                port: 19528,
            },
            server: ServerConfig {
                bind: Some("127.0.0.1:19528".to_string()),
                ..ServerConfig::default()
            },
            remote: RemoteConfig::default(),
            notifications: NotificationsConfig {
                dnd_start: "22:00".to_string(),
                dnd_end: "08:00".to_string(),
                min_level: "info".to_string(),
            },
            achievements: AchievementsConfig { enabled: true },
            sessions: SessionsConfig::default(),
            cleanup: CleanupConfig::default(),
            update: UpdateConfig::default(),
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
