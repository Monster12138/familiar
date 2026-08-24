use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Current sprite pack format version.
pub const SPRITE_PACK_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpritePackManifest {
    /// Sprite pack format version. Incremented when the pack format changes.
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub author: String,
    pub created_at: String,
    pub email: String,
    pub version: String,
    /// SPDX license expression for the pack's visual assets.
    #[serde(default)]
    pub license: Option<String>,
    /// Optional homepage or source repository for attribution.
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub states: HashMap<String, String>,
    /// SHA-256 digest of the pack's asset files for integrity verification.
    #[serde(default)]
    pub sha256: Option<String>,
}

fn default_format_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpritePackInfo {
    pub manifest: SpritePackManifest,
    pub path: String,
    pub is_builtin: bool,
    /// Changes when the manifest or an asset changes, allowing WebViews to
    /// invalidate cached URLs after a pack is imported over an existing ID.
    pub asset_revision: String,
    pub preview_url: String,
    pub state_urls: HashMap<String, String>,
}

pub struct SpritePackManager;

impl SpritePackManager {
    pub fn get_search_dirs() -> Vec<(PathBuf, bool)> {
        let mut dirs = Vec::new();

        // Custom user sprite pack directories FIRST (platform dir, then the
        // legacy ~/.config/familiar/sprites location)
        for dir in crate::platform::user_sprite_dir_candidates() {
            dirs.push((dir, false));
        }

        // Built-in sprite pack locations
        dirs.push((PathBuf::from("sprites"), true));
        dirs.push((PathBuf::from("../../sprites"), true));

        if let Ok(exe_dir) = std::env::current_exe() {
            dirs.push((exe_dir.join("../Resources/sprites"), true));
            dirs.push((exe_dir.join("../sprites"), true));
            if let Some(parent) = exe_dir.parent() {
                dirs.push((parent.join("sprites"), true));
                dirs.push((parent.join("../Resources/sprites"), true));
                if let Some(grandparent) = parent.parent() {
                    dirs.push((grandparent.join("sprites"), true));
                }
            }
        }

        dirs
    }

    pub fn get_user_sprite_dir() -> PathBuf {
        crate::platform::user_sprite_dir()
    }

    pub fn discover_packs() -> Vec<SpritePackInfo> {
        Self::discover_packs_with_extra(None)
    }

    pub fn discover_packs_with_extra(extra_dir: Option<&Path>) -> Vec<SpritePackInfo> {
        let mut search_dirs = Self::get_search_dirs();
        if let Some(extra) = extra_dir {
            // User-imported directories must remain ahead of bundled resources
            // so importing a pack with the same ID intentionally overrides the
            // built-in copy.
            let user_dir_count = crate::platform::user_sprite_dir_candidates().len();
            search_dirs.insert(user_dir_count, (extra.to_path_buf(), true));
            search_dirs.insert(user_dir_count, (extra.join("sprites"), true));
        }

        discover_packs_in_dirs(search_dirs)
    }

    pub fn load_pack_from_dir(pack_dir: &Path, is_builtin: bool) -> Result<SpritePackInfo> {
        let pack_json_path = pack_dir.join("pack.json");
        let manifest_json_path = pack_dir.join("manifest.json");

        let content = if pack_json_path.exists() {
            fs::read_to_string(&pack_json_path)?
        } else if manifest_json_path.exists() {
            fs::read_to_string(&manifest_json_path)?
        } else {
            return Err(anyhow!(
                "No pack.json or manifest.json found in {:?}",
                pack_dir
            ));
        };

        let mut manifest: SpritePackManifest = serde_json::from_str(&content)?;

        // Fallback ID if empty
        if manifest.id.is_empty() {
            if let Some(dir_name) = pack_dir.file_name().and_then(|s| s.to_str()) {
                manifest.id = dir_name.to_string();
            }
        }

        let abs_dir = pack_dir
            .canonicalize()
            .unwrap_or_else(|_| pack_dir.to_path_buf());
        let abs_dir_str = abs_dir.to_string_lossy().to_string();
        let asset_revision = asset_revision(pack_dir, &manifest);

        let preview_file = manifest
            .preview
            .clone()
            .unwrap_or_else(|| "idle.png".to_string());
        let preview_url = format!("/sprites/{}/{}", manifest.id, preview_file);

        let mut state_urls = HashMap::new();
        for (state, file_name) in &manifest.states {
            state_urls.insert(
                state.clone(),
                format!("/sprites/{}/{}", manifest.id, file_name),
            );
        }

        Ok(SpritePackInfo {
            manifest,
            path: abs_dir_str,
            is_builtin,
            asset_revision,
            preview_url,
            state_urls,
        })
    }

    pub fn import_pack<P: AsRef<Path>>(source_path: P) -> Result<SpritePackInfo> {
        let src_path = source_path.as_ref();
        if !src_path.exists() {
            return Err(anyhow!("Source path does not exist: {:?}", src_path));
        }

        let user_dir = Self::get_user_sprite_dir();
        fs::create_dir_all(&user_dir)?;

        let temp_dir =
            std::env::temp_dir().join(format!("familiar-import-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir)?;

        let is_archive = src_path.is_file()
            && src_path
                .extension()
                .is_some_and(|e| e == "zip" || e == "fpack");

        let pack_src_dir = if is_archive {
            // Extract ZIP / .fpack archive
            let file = fs::File::open(src_path)?;
            let mut archive =
                zip::ZipArchive::new(file).map_err(|e| anyhow!("Failed to read archive: {e}"))?;
            archive.extract(&temp_dir)?;

            find_archive_pack_dir(&temp_dir)?
        } else if src_path.is_file()
            && src_path
                .file_name()
                .is_some_and(|n| n == "pack.json" || n == "manifest.json")
        {
            src_path.parent().unwrap_or(src_path).to_path_buf()
        } else if src_path.is_dir() {
            src_path.to_path_buf()
        } else {
            return Err(anyhow!("Unsupported source format: {:?}", src_path));
        };

        // Validate manifest exists
        let pack_json = pack_src_dir.join("pack.json");
        let manifest_json = pack_src_dir.join("manifest.json");
        if !pack_json.exists() && !manifest_json.exists() {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(anyhow!(
                "Invalid sprite pack: missing pack.json manifest file"
            ));
        }

        // Validate manifest content
        let info = Self::load_pack_from_dir(&pack_src_dir, false).map_err(|e| {
            let _ = fs::remove_dir_all(&temp_dir);
            anyhow!("Invalid sprite pack manifest: {e}")
        })?;

        if let Err(e) = Self::validate_manifest(&info.manifest) {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(e);
        }

        let pack_id = info.manifest.id.clone();
        let target_dir = user_dir.join(&pack_id);

        // Copy directory contents to target_dir
        copy_dir_all(&pack_src_dir, &target_dir)?;

        // Clean up temp_dir if used
        let _ = fs::remove_dir_all(&temp_dir);

        Self::load_pack_from_dir(&target_dir, false)
    }

    pub fn delete_user_pack(pack_id: &str) -> Result<()> {
        delete_pack_from_dir(&Self::get_user_sprite_dir(), pack_id)
    }

    /// Validate that a sprite pack manifest has all required fields.
    fn validate_manifest(manifest: &SpritePackManifest) -> Result<()> {
        if manifest.format_version > SPRITE_PACK_FORMAT_VERSION {
            return Err(anyhow!(
                "Invalid sprite pack: unsupported format version {} (current max supported version: {})",
                manifest.format_version,
                SPRITE_PACK_FORMAT_VERSION
            ));
        }
        if manifest.id.is_empty() {
            return Err(anyhow!(
                "Invalid sprite pack: missing required field \"id\""
            ));
        }
        if manifest.name.is_empty() {
            return Err(anyhow!(
                "Invalid sprite pack: missing required field \"name\""
            ));
        }
        if manifest.states.is_empty() {
            return Err(anyhow!(
                "Invalid sprite pack: \"states\" must contain at least one state mapping"
            ));
        }
        if !manifest.states.contains_key("idle") {
            return Err(anyhow!(
                "Invalid sprite pack: \"states\" must contain an \"idle\" state"
            ));
        }
        Ok(())
    }
}

fn delete_pack_from_dir(user_sprite_dir: &Path, pack_id: &str) -> Result<()> {
    let mut components = Path::new(pack_id).components();
    let valid_id = matches!(
        (components.next(), components.next()),
        (Some(std::path::Component::Normal(name)), None)
            if name == std::ffi::OsStr::new(pack_id)
    );
    if !valid_id {
        return Err(anyhow!("Invalid sprite pack ID"));
    }

    let target = user_sprite_dir.join(pack_id);
    if !target.is_dir() {
        return Err(anyhow!("Custom sprite pack '{pack_id}' not found"));
    }

    fs::remove_dir_all(target)?;
    Ok(())
}

fn discover_packs_in_dirs(search_dirs: Vec<(PathBuf, bool)>) -> Vec<SpritePackInfo> {
    let mut packs = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for (dir, is_builtin) in search_dirs {
        if !dir.exists() || !dir.is_dir() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if !entry_path.is_dir() {
                    continue;
                }

                if let Ok(info) = SpritePackManager::load_pack_from_dir(&entry_path, is_builtin) {
                    if seen_ids.insert(info.manifest.id.clone()) {
                        packs.push(info);
                    }
                }
            }
        }
    }

    packs
}

fn asset_revision(pack_dir: &Path, manifest: &SpritePackManifest) -> String {
    let mut paths = vec![pack_dir.join("pack.json"), pack_dir.join("manifest.json")];
    paths.extend(manifest.states.values().map(|file| pack_dir.join(file)));
    if let Some(preview) = &manifest.preview {
        paths.push(pack_dir.join(preview));
    }

    let newest_modified = paths
        .into_iter()
        .filter_map(|path| fs::metadata(path).ok()?.modified().ok())
        .filter_map(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .max()
        .unwrap_or_default();

    format!(
        "{}-{newest_modified}",
        manifest.sha256.as_deref().unwrap_or(&manifest.version)
    )
}

fn find_archive_pack_dir(extracted_root: &Path) -> Result<PathBuf> {
    let mut pending = vec![extracted_root.to_path_buf()];
    let mut candidates = Vec::new();

    while let Some(dir) = pending.pop() {
        if dir.join("pack.json").is_file() || dir.join("manifest.json").is_file() {
            candidates.push(dir);
            continue;
        }

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == "__MACOSX" || name.starts_with('.') {
                continue;
            }
            pending.push(entry.path());
        }
    }

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(anyhow!(
            "Invalid sprite pack: missing pack.json manifest file"
        )),
        count => Err(anyhow!(
            "Invalid sprite pack: archive contains {count} manifest directories"
        )),
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        asset_revision, delete_pack_from_dir, discover_packs_in_dirs, find_archive_pack_dir,
        SpritePackManifest,
    };
    use std::collections::HashMap;

    #[test]
    fn archive_pack_dir_ignores_macos_metadata() {
        let root =
            std::env::temp_dir().join(format!("familiar-fpack-root-{}", uuid::Uuid::new_v4()));
        let pack_dir = root.join("huajuan-cat");
        std::fs::create_dir_all(root.join("__MACOSX/huajuan-cat")).unwrap();
        std::fs::create_dir_all(&pack_dir).unwrap();
        std::fs::write(pack_dir.join("pack.json"), b"{}").unwrap();

        let found = find_archive_pack_dir(&root).unwrap();

        assert_eq!(found, pack_dir);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn archive_pack_dir_rejects_multiple_manifests() {
        let root =
            std::env::temp_dir().join(format!("familiar-fpack-ambiguous-{}", uuid::Uuid::new_v4()));
        for name in ["first", "second"] {
            let dir = root.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("pack.json"), b"{}").unwrap();
        }

        let error = find_archive_pack_dir(&root).unwrap_err().to_string();

        assert!(error.contains("2 manifest directories"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn asset_revision_includes_declared_digest() {
        let root =
            std::env::temp_dir().join(format!("familiar-fpack-revision-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("idle.png"), b"sprite").unwrap();
        let manifest = SpritePackManifest {
            format_version: 1,
            id: "test".into(),
            name: "Test".into(),
            author: String::new(),
            created_at: String::new(),
            email: String::new(),
            version: "1.0.0".into(),
            license: None,
            source: None,
            description: String::new(),
            preview: Some("idle.png".into()),
            states: HashMap::from([("idle".into(), "idle.png".into())]),
            sha256: Some("new-content-digest".into()),
        };

        let revision = asset_revision(&root, &manifest);

        assert!(revision.starts_with("new-content-digest-"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn custom_pack_wins_when_builtin_has_same_id() {
        let root = std::env::temp_dir().join(format!(
            "familiar-fpack-precedence-{}",
            uuid::Uuid::new_v4()
        ));
        let custom_root = root.join("custom");
        let builtin_root = root.join("builtin");

        for (pack_root, digest) in [(&custom_root, "custom"), (&builtin_root, "builtin")] {
            let pack_dir = pack_root.join("same-id");
            std::fs::create_dir_all(&pack_dir).unwrap();
            std::fs::write(pack_dir.join("idle.png"), digest.as_bytes()).unwrap();
            std::fs::write(
                pack_dir.join("pack.json"),
                format!(
                    r#"{{"id":"same-id","name":"Test","author":"","created_at":"","email":"","version":"1","states":{{"idle":"idle.png"}},"sha256":"{digest}"}}"#
                ),
            )
            .unwrap();
        }

        let packs = discover_packs_in_dirs(vec![(custom_root, false), (builtin_root, true)]);

        assert_eq!(packs.len(), 1);
        assert!(!packs[0].is_builtin);
        assert_eq!(packs[0].manifest.sha256.as_deref(), Some("custom"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn delete_pack_removes_only_named_child() {
        let root =
            std::env::temp_dir().join(format!("familiar-fpack-delete-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("delete-me")).unwrap();
        std::fs::create_dir_all(root.join("keep-me")).unwrap();

        delete_pack_from_dir(&root, "delete-me").unwrap();

        assert!(!root.join("delete-me").exists());
        assert!(root.join("keep-me").is_dir());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn delete_pack_rejects_path_traversal() {
        let root = std::env::temp_dir().join(format!(
            "familiar-fpack-delete-guard-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();

        let error = delete_pack_from_dir(&root, "../outside").unwrap_err();

        assert!(error.to_string().contains("Invalid sprite pack ID"));
        let _ = std::fs::remove_dir_all(root);
    }
}
