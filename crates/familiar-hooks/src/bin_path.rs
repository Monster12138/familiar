//! Shared resolution of the `familiar-cli` binary path used by hook injection.

use std::path::{Path, PathBuf};

/// File name of the CLI executable on the current platform
/// (`familiar-cli.exe` on Windows, `familiar-cli` elsewhere).
pub fn cli_exe_name() -> String {
    format!("familiar-cli{}", std::env::consts::EXE_SUFFIX)
}

/// Ordered candidate locations relative to the currently running executable.
/// Covers side-by-side builds, the Tauri resource layout (`bin/`, used by
/// Windows installers), and the macOS `.app` bundle layout (`Resources/`).
fn candidates_for_exe(exe: &Path) -> Vec<PathBuf> {
    let exe_name = cli_exe_name();
    let mut candidates = Vec::new();

    if exe.file_stem().and_then(|s| s.to_str()) == Some("familiar-cli") {
        candidates.push(exe.to_path_buf());
    }

    if let Some(parent) = exe.parent() {
        candidates.push(parent.join(&exe_name));
        candidates.push(parent.join("bin").join(&exe_name));
        if let Some(grandparent) = parent.parent() {
            candidates.push(grandparent.join("Resources").join("bin").join(&exe_name));
            candidates.push(grandparent.join("Resources").join(&exe_name));
        }
        candidates.push(parent.join("Resources").join(&exe_name));
    }

    candidates
}

/// Resolve the path to the `familiar-cli` binary, falling back to the bare
/// command name (resolved via PATH at hook execution time) when no known
/// candidate exists on disk.
pub fn resolve_cli_bin_path() -> String {
    if let Ok(exe) = std::env::current_exe() {
        for candidate in candidates_for_exe(&exe) {
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let exe_name = cli_exe_name();
        for profile in ["release", "debug"] {
            let dev_cli = PathBuf::from(&manifest_dir)
                .join("../../target")
                .join(profile)
                .join(&exe_name);
            if dev_cli.exists() {
                return dev_cli.to_string_lossy().to_string();
            }
        }
    }

    if let Some(home) = dirs::home_dir() {
        let cargo_cli = home.join(".cargo").join("bin").join(cli_exe_name());
        if cargo_cli.exists() {
            return cargo_cli.to_string_lossy().to_string();
        }
    }

    "familiar-cli".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_name_uses_platform_suffix() {
        #[cfg(windows)]
        assert_eq!(cli_exe_name(), "familiar-cli.exe");
        #[cfg(not(windows))]
        assert_eq!(cli_exe_name(), "familiar-cli");
    }

    #[test]
    fn candidates_match_cli_regardless_of_extension() {
        let exe = PathBuf::from(format!(
            "/opt/familiar/familiar-cli{}",
            std::env::consts::EXE_SUFFIX
        ));
        let candidates = candidates_for_exe(&exe);
        assert_eq!(candidates.first(), Some(&exe));
    }

    #[test]
    fn candidates_cover_sibling_bin_and_bundle_dirs() {
        let exe = PathBuf::from("/opt/familiar/familiar-app");
        let candidates = candidates_for_exe(&exe);
        let exe_name = cli_exe_name();
        assert!(candidates
            .iter()
            .any(|p| p == &PathBuf::from(format!("/opt/familiar/{exe_name}"))));
        assert!(candidates
            .iter()
            .any(|p| p == &PathBuf::from(format!("/opt/familiar/bin/{exe_name}"))));
        assert!(candidates
            .iter()
            .any(|p| p == &PathBuf::from(format!("/opt/Resources/bin/{exe_name}"))));
        assert!(candidates
            .iter()
            .any(|p| p == &PathBuf::from(format!("/opt/Resources/{exe_name}"))));
        assert!(candidates
            .iter()
            .any(|p| p == &PathBuf::from(format!("/opt/familiar/Resources/{exe_name}"))));
    }
}
