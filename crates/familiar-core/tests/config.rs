use familiar_core::config::{
    DashboardAlignment, DashboardLayout, DashboardPosition, DashboardStyle, FamiliarConfig,
};

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
