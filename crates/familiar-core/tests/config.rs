use familiar_core::config::FamiliarConfig;

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
