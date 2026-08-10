//! Platform-conventional locations for Familiar's per-user data.
//!
//! Historically everything lived under `~/.config/familiar`. That is the
//! convention on Unix systems, but on Windows user data belongs under the
//! per-user config directory (`%APPDATA%\familiar`). The helpers here prefer
//! the platform convention while keeping the legacy location as a fallback so
//! existing installs and hand-written configs keep working.

use std::path::PathBuf;

/// Preferred directory for Familiar's per-user files on this platform.
///
/// Windows prefers the platform config directory (`%APPDATA%\familiar`);
/// other platforms keep the conventional `~/.config/familiar`.
pub fn user_config_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(dir) = dirs::config_dir() {
        return dir.join("familiar");
    }
    legacy_config_dir().unwrap_or_else(|| PathBuf::from(".config").join("familiar"))
}

/// Legacy config directory `~/.config/familiar`, if the home dir resolves.
pub fn legacy_config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".config").join("familiar"))
}

/// All candidate locations of the user `config.toml`, highest priority first.
///
/// On Windows the platform config directory takes priority; the legacy
/// `~/.config/familiar` path remains a fallback for existing installs. On
/// Unix only the legacy path is returned (it is the convention there).
pub fn user_config_file_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    #[cfg(windows)]
    if let Some(dir) = dirs::config_dir() {
        candidates.push(dir.join("familiar").join("config.toml"));
    }
    if let Some(legacy) = legacy_config_dir() {
        candidates.push(legacy.join("config.toml"));
    }
    candidates
}

/// Directory where user-imported sprite packs are stored.
pub fn user_sprite_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(dir) = dirs::config_dir() {
        return dir.join("familiar").join("sprites");
    }
    legacy_config_dir()
        .map(|d| d.join("sprites"))
        .unwrap_or_else(|| PathBuf::from("sprites"))
}

/// Candidate directories for user sprite packs, highest priority first.
pub fn user_sprite_dir_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    #[cfg(windows)]
    if let Some(dir) = dirs::config_dir() {
        candidates.push(dir.join("familiar").join("sprites"));
    }
    if let Some(legacy) = legacy_config_dir() {
        let legacy_sprites = legacy.join("sprites");
        if !candidates.contains(&legacy_sprites) {
            candidates.push(legacy_sprites);
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_config_dir_is_absolute() {
        assert!(user_config_dir().is_absolute());
    }

    #[test]
    fn config_candidates_include_legacy_path() {
        let candidates = user_config_file_candidates();
        assert!(!candidates.is_empty());
        let legacy = legacy_config_dir().expect("home dir should resolve in tests");
        assert!(candidates.contains(&legacy.join("config.toml")));
    }

    #[test]
    fn config_candidates_prefer_platform_dir_on_windows() {
        let candidates = user_config_file_candidates();
        #[cfg(windows)]
        {
            let expected = dirs::config_dir()
                .expect("config dir should resolve on Windows")
                .join("familiar")
                .join("config.toml");
            assert_eq!(candidates.first(), Some(&expected));
        }
        #[cfg(not(windows))]
        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn sprite_dir_candidates_include_legacy_path() {
        let candidates = user_sprite_dir_candidates();
        assert!(!candidates.is_empty());
        let legacy = legacy_config_dir()
            .expect("home dir should resolve in tests")
            .join("sprites");
        assert!(candidates.contains(&legacy));
    }
}
