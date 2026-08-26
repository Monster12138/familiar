use familiar_core::config::{
    CleanupConfig, DashboardAlignment, DashboardLayout, DashboardPosition, DashboardStyle,
    EventStatus, FamiliarConfig, RuntimeMode, UpdateConfig,
};
use familiar_core::state::AgentStatus;

#[test]
fn legacy_config_defaults_to_showing_on_all_desktops() {
    let legacy_config =
        include_str!("../../../config/default.toml").replace("show_on_all_desktops = true\n", "");
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-config-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert!(config.renderer.desktop_pet.show_on_all_desktops);
}

#[test]
fn legacy_config_defaults_onboarded_to_false() {
    let legacy_config =
        include_str!("../../../config/default.toml").replace("onboarded = false\n", "");
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-onboarded-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    // Existing config files without the key still load and show onboarding.
    assert!(!config.general.onboarded);
}

#[test]
fn scale_serializes_with_one_decimal_place() {
    let mut config = FamiliarConfig::default();
    config.renderer.desktop_pet.scale = 1.3_f32;

    let serialized = toml::to_string_pretty(&config).expect("serialize config");

    assert!(
        serialized.lines().any(|line| line == "scale = 1.3"),
        "unexpected serialized scale:\n{serialized}"
    );
}

#[test]
fn legacy_config_defaults_empty_hidden_sessions() {
    let legacy_config = include_str!("../../../config/default.toml")
        .replace("[sessions]\nhidden_sessions = []\n", "");
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-sessions-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert!(config.sessions.hidden_sessions.is_empty());
}

#[test]
fn legacy_config_defaults_to_classic_dashboard_style() {
    let legacy_config =
        include_str!("../../../config/default.toml").replace("dashboard_style = \"classic\"\n", "");
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-dashboard-style-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert_eq!(
        config.renderer.desktop_pet.dashboard_style,
        DashboardStyle::Classic
    );
}

#[test]
fn dashboard_style_serializes_minimal_variant() {
    let mut config = FamiliarConfig::default();
    config.renderer.desktop_pet.dashboard_style = DashboardStyle::Minimal;

    let serialized = toml::to_string_pretty(&config).expect("serialize config");

    assert!(
        serialized
            .lines()
            .any(|line| line == "dashboard_style = \"minimal\""),
        "unexpected serialized dashboard style:\n{serialized}"
    );
}

#[test]
fn removed_dashboard_style_falls_back_to_classic() {
    let legacy_config = include_str!("../../../config/default.toml").replace(
        "dashboard_style = \"classic\"",
        "dashboard_style = \"capsule\"",
    );
    let path = std::env::temp_dir().join(format!(
        "familiar-removed-dashboard-style-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert_eq!(
        config.renderer.desktop_pet.dashboard_style,
        DashboardStyle::Classic
    );
}

#[test]
fn legacy_config_defaults_to_bottom_dashboard_position() {
    let legacy_config = include_str!("../../../config/default.toml")
        .replace("dashboard_position = \"bottom\"\n", "");
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-dashboard-position-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert_eq!(
        config.renderer.desktop_pet.dashboard_position,
        DashboardPosition::Bottom
    );
}

#[test]
fn dashboard_position_serializes_as_kebab_case_string() {
    let mut config = FamiliarConfig::default();
    config.renderer.desktop_pet.dashboard_position = DashboardPosition::Right;

    let serialized = toml::to_string_pretty(&config).expect("serialize config");

    assert!(
        serialized
            .lines()
            .any(|line| line == "dashboard_position = \"right\""),
        "unexpected serialized dashboard position:\n{serialized}"
    );
}

#[test]
fn legacy_config_defaults_to_vertical_dashboard_layout() {
    let legacy_config = include_str!("../../../config/default.toml")
        .replace("dashboard_layout = \"vertical\"\n", "");
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-dashboard-layout-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert_eq!(
        config.renderer.desktop_pet.dashboard_layout,
        DashboardLayout::Vertical
    );
}

#[test]
fn dashboard_layout_serializes_as_kebab_case_string() {
    let mut config = FamiliarConfig::default();
    config.renderer.desktop_pet.dashboard_layout = DashboardLayout::Horizontal;

    let serialized = toml::to_string_pretty(&config).expect("serialize config");

    assert!(
        serialized
            .lines()
            .any(|line| line == "dashboard_layout = \"horizontal\""),
        "unexpected serialized dashboard layout:\n{serialized}"
    );
}

#[test]
fn legacy_config_defaults_to_bottom_dashboard_alignment() {
    let legacy_config = include_str!("../../../config/default.toml")
        .replace("dashboard_alignment = \"bottom\"\n", "");
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-dashboard-alignment-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert_eq!(
        config.renderer.desktop_pet.dashboard_alignment,
        DashboardAlignment::Bottom
    );
}

#[test]
fn dashboard_alignment_serializes_as_kebab_case_string() {
    let mut config = FamiliarConfig::default();
    config.renderer.desktop_pet.dashboard_alignment = DashboardAlignment::Center;

    let serialized = toml::to_string_pretty(&config).expect("serialize config");

    assert!(
        serialized
            .lines()
            .any(|line| line == "dashboard_alignment = \"center\""),
        "unexpected serialized dashboard alignment:\n{serialized}"
    );
}

#[test]
fn legacy_config_defaults_empty_event_status_map() {
    // default.toml has no event_status_map section; it must deserialize empty.
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-event-status-{}.toml",
        std::process::id()
    ));
    std::fs::write(&path, include_str!("../../../config/default.toml")).expect("write config");
    let config = FamiliarConfig::load_from_file(&path).expect("load config");
    std::fs::remove_file(path).expect("remove config");

    assert!(config.renderer.desktop_pet.event_status_map.is_empty());
}

#[test]
fn event_status_map_serializes_known_statuses() {
    let mut config = FamiliarConfig::default();
    config
        .renderer
        .desktop_pet
        .event_status_map
        .insert("Thinking".to_string(), EventStatus::Working);
    config
        .renderer
        .desktop_pet
        .event_status_map
        .insert("WaitingForInput".to_string(), EventStatus::Pending);

    let serialized = toml::to_string_pretty(&config).expect("serialize config");

    assert!(
        serialized
            .lines()
            .any(|line| line == "Thinking = \"working\""),
        "unexpected serialized event status map:\n{serialized}"
    );
    assert!(
        serialized
            .lines()
            .any(|line| line == "WaitingForInput = \"pending\""),
        "unexpected serialized event status map:\n{serialized}"
    );
}

#[test]
fn legacy_config_defaults_to_local_runtime_and_new_fields_serialize() {
    let legacy = r#"
[general]
language = "en-US"
data_retention_days = 90

[hooks]
enabled = []
socket_path = "/tmp/familiar.sock"
tcp_port = 19527

[renderer]
enabled = []

[renderer.desktop-pet]
sprite = "tabby-cat"
scale = 1.0
position = "bottom-right"
always_on_top = true
opacity = 1.0

[renderer.menu-bar]
show_active_count = true
show_today_stats = true

[renderer.dashboard]
port = 9527

[api]
enabled = true
port = 19527

[notifications]
dnd_start = "22:00"
dnd_end = "08:00"
min_level = "info"

[achievements]
enabled = true
"#;
    let path = std::env::temp_dir().join(format!(
        "familiar-runtime-config-{}.toml",
        std::process::id()
    ));
    std::fs::write(&path, legacy).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");
    assert_eq!(config.runtime.mode, RuntimeMode::Local);
    assert!(
        config.renderer.desktop_pet.sprite_pool.is_empty(),
        "legacy config without sprite_pool must default to an empty pool"
    );

    let serialized = toml::to_string_pretty(&FamiliarConfig::default()).expect("serialize config");
    assert!(serialized.contains("mode = \"local\""));
    assert!(serialized.contains("max_updates_per_second = 10"));
}

#[test]
fn sprite_pool_round_trips_through_config() {
    let base = include_str!("../../../config/default.toml");
    let with_pool = base.replacen(
        "sprite_pool = []",
        "sprite_pool = [\"huajuan-cat\", \"douhua-cat\"]",
        1,
    );
    let path = std::env::temp_dir().join(format!("familiar-pool-{}.toml", std::process::id()));
    std::fs::write(&path, with_pool).expect("write pool config");
    let config = FamiliarConfig::load_from_file(&path).expect("load pool config");
    std::fs::remove_file(path).expect("remove pool config");
    assert_eq!(
        config.renderer.desktop_pet.sprite_pool,
        vec!["huajuan-cat".to_string(), "douhua-cat".to_string()]
    );
}

#[test]
fn event_status_map_unknown_value_falls_back_to_unknown() {
    let content = format!(
        "{}\n[renderer.desktop-pet.event_status_map]\nTaskCompleted = \"bogus\"\n",
        include_str!("../../../config/default.toml")
    );
    let path = std::env::temp_dir().join(format!(
        "familiar-unknown-event-status-{}.toml",
        std::process::id()
    ));
    std::fs::write(&path, content).expect("write config");
    let config = FamiliarConfig::load_from_file(&path).expect("load config");
    std::fs::remove_file(path).expect("remove config");

    assert_eq!(
        config
            .renderer
            .desktop_pet
            .event_status_map
            .get("TaskCompleted"),
        Some(&EventStatus::Unknown)
    );
}

#[test]
fn event_status_agent_map_skips_unknown_and_agent_stopped() {
    let mut config = FamiliarConfig::default();
    config
        .renderer
        .desktop_pet
        .event_status_map
        .insert("TaskCompleted".to_string(), EventStatus::Completed);
    config
        .renderer
        .desktop_pet
        .event_status_map
        .insert("AgentStopped".to_string(), EventStatus::Working);
    config
        .renderer
        .desktop_pet
        .event_status_map
        .insert("RunningCommand".to_string(), EventStatus::Unknown);

    let map = config.renderer.desktop_pet.event_status_agent_map();
    assert_eq!(map.get("TaskCompleted"), Some(&AgentStatus::Completed));
    // AgentStopped is not mappable and Unknown values are dropped so they
    // fall back to the built-in behavior.
    assert!(!map.contains_key("AgentStopped"));
    assert!(!map.contains_key("RunningCommand"));
}

#[test]
fn legacy_config_defaults_to_cleanup_settings() {
    // Normalize line endings so the strip matches regardless of CRLF/LF.
    let legacy_config = include_str!("../../../config/default.toml")
        .replace("\r\n", "\n")
        .replace(
            "[cleanup]\nbackup_files = true\nlog_files = true\nage_days = 90",
            "",
        );
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-cleanup-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert_eq!(config.cleanup, CleanupConfig::default());
}

#[test]
fn cleanup_serializes_expected_section() {
    let serialized = toml::to_string_pretty(&FamiliarConfig::default()).expect("serialize config");

    assert!(
        serialized.lines().any(|line| line == "backup_files = true"),
        "unexpected serialized cleanup section:\n{serialized}"
    );
    assert!(
        serialized.lines().any(|line| line == "log_files = true"),
        "unexpected serialized cleanup section:\n{serialized}"
    );
    assert!(
        serialized.lines().any(|line| line == "age_days = 90"),
        "unexpected serialized cleanup section:\n{serialized}"
    );
}

#[test]
fn cleanup_partial_section_fills_missing_fields() {
    // Normalize line endings so the replacement matches regardless of CRLF/LF.
    let content = include_str!("../../../config/default.toml")
        .replace("\r\n", "\n")
        .replace(
            "[cleanup]\nbackup_files = true\nlog_files = true\nage_days = 90",
            "[cleanup]\nage_days = 30",
        );
    let path = std::env::temp_dir().join(format!(
        "familiar-partial-cleanup-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, content).expect("write config");
    let config = FamiliarConfig::load_from_file(&path).expect("load config");
    std::fs::remove_file(path).expect("remove config");

    assert!(config.cleanup.backup_files);
    assert!(config.cleanup.log_files);
    assert_eq!(config.cleanup.age_days, 30);
}

#[test]
fn legacy_config_defaults_to_update_settings() {
    // Normalize line endings so the strip matches regardless of CRLF/LF.
    let legacy_config = include_str!("../../../config/default.toml")
        .replace("\r\n", "\n")
        .replace(
            "[update]\ncheck_on_startup = true\ninterval = \"daily\"\nignored_versions = []\n",
            "",
        );
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-update-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert_eq!(config.update, UpdateConfig::default());
}

#[test]
fn update_serializes_expected_section() {
    let serialized = toml::to_string_pretty(&FamiliarConfig::default()).expect("serialize config");

    assert!(
        serialized
            .lines()
            .any(|line| line == "check_on_startup = true"),
        "unexpected serialized update section:\n{serialized}"
    );
    assert!(
        serialized
            .lines()
            .any(|line| line == "interval = \"daily\""),
        "unexpected serialized update section:\n{serialized}"
    );
    assert!(
        serialized
            .lines()
            .any(|line| line == "ignored_versions = []"),
        "unexpected serialized update section:\n{serialized}"
    );
}

#[test]
fn update_partial_section_fills_missing_fields() {
    // Normalize line endings so the replacement matches regardless of CRLF/LF.
    let content = include_str!("../../../config/default.toml")
        .replace("\r\n", "\n")
        .replace(
            "[update]\ncheck_on_startup = true\ninterval = \"daily\"\nignored_versions = []\n",
            "[update]\ncheck_on_startup = false\n",
        );
    let path = std::env::temp_dir().join(format!(
        "familiar-partial-update-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, content).expect("write config");
    let config = FamiliarConfig::load_from_file(&path).expect("load config");
    std::fs::remove_file(path).expect("remove config");

    assert!(!config.update.check_on_startup);
    assert_eq!(config.update.interval, Default::default());
    assert!(config.update.skipped_version.is_none());
    assert!(config.update.ignored_versions.is_empty());
    assert!(config.update.last_check_at.is_none());
}

#[test]
fn legacy_config_defaults_to_no_click_through() {
    let legacy_config =
        include_str!("../../../config/default.toml").replace("click_through = false\n", "");
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-click-through-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert!(!config.renderer.desktop_pet.click_through);
}

#[test]
fn click_through_serializes_as_bool() {
    let mut config = FamiliarConfig::default();
    config.renderer.desktop_pet.click_through = true;

    let serialized = toml::to_string_pretty(&config).expect("serialize config");

    assert!(
        serialized
            .lines()
            .any(|line| line == "click_through = true"),
        "unexpected serialized click_through:\n{serialized}"
    );
}

#[test]
fn legacy_config_defaults_to_no_hover_hide() {
    let legacy_config =
        include_str!("../../../config/default.toml").replace("hide_on_hover = false\n", "");
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-hover-hide-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert!(!config.renderer.desktop_pet.hide_on_hover);
}

#[test]
fn hover_hide_serializes_as_bool() {
    let mut config = FamiliarConfig::default();
    config.renderer.desktop_pet.hide_on_hover = true;

    let serialized = toml::to_string_pretty(&config).expect("serialize config");

    assert!(
        serialized
            .lines()
            .any(|line| line == "hide_on_hover = true"),
        "unexpected serialized hover-hide config:\n{serialized}"
    );
}

#[test]
fn legacy_config_defaults_to_no_window_frame() {
    let legacy_config =
        include_str!("../../../config/default.toml").replace("show_window_frame = false\n", "");
    let path = std::env::temp_dir().join(format!(
        "familiar-legacy-window-frame-{}.toml",
        std::process::id()
    ));

    std::fs::write(&path, legacy_config).expect("write legacy config");
    let config = FamiliarConfig::load_from_file(&path).expect("load legacy config");
    std::fs::remove_file(path).expect("remove legacy config");

    assert!(!config.renderer.desktop_pet.show_window_frame);
}

#[test]
fn show_window_frame_serializes_as_bool() {
    let mut config = FamiliarConfig::default();
    config.renderer.desktop_pet.show_window_frame = true;

    let serialized = toml::to_string_pretty(&config).expect("serialize config");

    assert!(
        serialized
            .lines()
            .any(|line| line == "show_window_frame = true"),
        "unexpected serialized show_window_frame:\n{serialized}"
    );
}
