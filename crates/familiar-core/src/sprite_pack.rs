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
    pub preview_url: String,
    pub state_urls: HashMap<String, String>,
}

pub struct SpritePackManager;

impl SpritePackManager {
    pub fn get_search_dirs() -> Vec<(PathBuf, bool)> {
        let mut dirs = Vec::new();

        // Custom user sprite pack directory (~/.config/familiar/sprites) FIRST
        if let Some(home) = dirs::home_dir() {
            dirs.push((home.join(".config").join("familiar").join("sprites"), false));
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
        if let Some(home) = dirs::home_dir() {
            home.join(".config").join("familiar").join("sprites")
        } else {
            PathBuf::from("sprites")
        }
    }

    pub fn discover_packs() -> Vec<SpritePackInfo> {
        Self::discover_packs_with_extra(None)
    }

    pub fn discover_packs_with_extra(extra_dir: Option<&Path>) -> Vec<SpritePackInfo> {
        let mut packs = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        let mut search_dirs = Self::get_search_dirs();
        if let Some(extra) = extra_dir {
            search_dirs.insert(0, (extra.to_path_buf(), true));
            search_dirs.insert(0, (extra.join("sprites"), true));
        }

        for (dir, is_builtin) in search_dirs {
            if !dir.exists() || !dir.is_dir() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        if let Ok(info) = Self::load_pack_from_dir(&entry_path, is_builtin) {
                            if !seen_ids.contains(&info.manifest.id) {
                                seen_ids.insert(info.manifest.id.clone());
                                packs.push(info);
                            }
                        }
                    }
                }
            }
        }

        packs
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

            // If the zip contained a single nested folder, use that folder
            let entries: Vec<_> = fs::read_dir(&temp_dir)?.filter_map(|e| e.ok()).collect();
            if entries.len() == 1 && entries[0].path().is_dir() {
                entries[0].path()
            } else {
                temp_dir.clone()
            }
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
