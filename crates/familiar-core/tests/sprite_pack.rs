use familiar_core::sprite_pack::{SpritePackManager, SpritePackManifest};

#[test]
fn test_parse_sprite_pack_manifest() {
    let json = r#"{
        "id": "test-pack",
        "name": "Test Pack",
        "author": "Tester",
        "created_at": "2026-07-31",
        "email": "test@example.com",
        "version": "1.0.0",
        "description": "Test sprite pack",
        "preview": "idle.png",
        "states": {
            "idle": "idle.png",
            "working": "working.gif"
        }
    }"#;

    let manifest: SpritePackManifest = serde_json::from_str(json).expect("parse manifest");
    assert_eq!(manifest.id, "test-pack");
    assert_eq!(manifest.name, "Test Pack");
    assert_eq!(manifest.author, "Tester");
    assert_eq!(manifest.states.get("working").unwrap(), "working.gif");
}

#[test]
fn test_discover_default_cat_pack() {
    let packs = SpritePackManager::discover_packs();
    assert!(!packs.is_empty(), "Should discover built-in sprite packs");
    let has_default = packs
        .iter()
        .any(|p| p.manifest.id == "british-blue" || p.manifest.id == "tabby-cat");
    assert!(has_default, "Should discover built-in packs");
}

#[test]
fn test_import_rejects_manifest_without_name() {
    let dir = std::env::temp_dir().join("fpack-test-no-name");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("pack.json"),
        r#"{"id":"bad","name":"","author":"a","created_at":"","email":"","version":"1","states":{"idle":"idle.png"}}"#,
    )
    .unwrap();
    std::fs::write(dir.join("idle.png"), b"fake").unwrap();

    let result = SpritePackManager::import_pack(&dir);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("name"), "Error should mention name: {msg}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_import_rejects_manifest_without_idle_state() {
    let dir = std::env::temp_dir().join("fpack-test-no-idle");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("pack.json"),
        r#"{"id":"bad2","name":"Bad","author":"a","created_at":"","email":"","version":"1","states":{"working":"w.png"}}"#,
    )
    .unwrap();

    let result = SpritePackManager::import_pack(&dir);
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("idle"), "Error should mention idle: {msg}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_import_rejects_unsupported_format_version() {
    let dir = std::env::temp_dir().join("fpack-test-bad-version");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("pack.json"),
        r#"{"format_version":999,"id":"future-pack","name":"Future","author":"a","created_at":"","email":"","version":"1","states":{"idle":"idle.png"}}"#,
    )
    .unwrap();

    let result = SpritePackManager::import_pack(&dir);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("unsupported format version"),
        "Error should mention unsupported format version: {msg}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
