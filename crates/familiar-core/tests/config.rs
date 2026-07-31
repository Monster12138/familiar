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
