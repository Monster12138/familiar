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
        if let Some(release_cli) = release_sibling_cli(&exe) {
            return release_cli.to_string_lossy().to_string();
        }

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

/// Build the command written into an Agent's Hook configuration. When the
/// management CLI was invoked with an explicit `--config`, the path is
/// carried into the injected command so the later Agent process reports to
/// the same server configuration even when that file is outside the normal
/// user config search paths.
pub fn hook_command(source: &str, event: &str, quote_cli_path: bool) -> String {
    let cli = resolve_cli_bin_path();
    let config = std::env::var("FAMILIAR_HOOK_CONFIG")
        .ok()
        .filter(|path| !path.trim().is_empty());
    hook_command_with_config(source, event, quote_cli_path, config.as_deref(), &cli)
}

fn hook_command_with_config(
    source: &str,
    event: &str,
    quote_cli_path: bool,
    config_path: Option<&str>,
    cli: &str,
) -> String {
    let executable = if quote_cli_path {
        format!("\"{cli}\"")
    } else {
        cli.to_string()
    };
    let config_arg = config_path
        .filter(|path| !path.trim().is_empty())
        .map(|path| format!(" --config {}", quote_hook_arg(path)))
        .unwrap_or_default();
    format!("{executable} hook --source {source} --event {event}{config_arg}")
}

fn quote_hook_arg(value: &str) -> String {
    if cfg!(windows) {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

/// When the running executable lives under a cargo `target/<profile>` dir
/// (a dev build of familiar-app or a test binary), prefer the `release`
/// sibling of familiar-cli. The debug binary carries MSVC debug-runtime
/// dependencies that can fail to initialize when a GUI host such as DeepSeek
/// Harness spawns the hook command, so injected hooks must not point at it
/// while a release build exists.
fn release_sibling_cli(exe: &std::path::Path) -> Option<PathBuf> {
    let profile_dir = exe.parent()?;
    let target_dir = profile_dir.parent()?;
    if target_dir.file_name().and_then(|s| s.to_str()) != Some("target") {
        return None;
    }
    let release_cli = target_dir.join("release").join(cli_exe_name());
    release_cli.exists().then_some(release_cli)
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

    #[test]
    fn release_sibling_ignores_non_target_layout() {
        let exe = PathBuf::from("/opt/familiar/bin/familiar-app");
        assert_eq!(release_sibling_cli(&exe), None);
    }

    #[test]
    fn release_sibling_prefers_release_cli_under_target_dir() {
        let root = std::env::temp_dir().join(format!("fam-bin-test-{}", std::process::id()));
        let target = root.join("target");
        let debug_dir = target.join("debug");
        let release_dir = target.join("release");
        let exe_name = cli_exe_name();
        let exe = debug_dir.join(format!("familiar-app{exe_name}"));
        let release_cli = release_dir.join(&exe_name);

        std::fs::create_dir_all(&debug_dir).expect("create debug dir");
        std::fs::create_dir_all(&release_dir).expect("create release dir");
        // The sibling only resolves when the release CLI actually exists.
        std::fs::write(&release_cli, b"").expect("write release cli");
        std::fs::write(&exe, b"").expect("write app exe");

        assert_eq!(release_sibling_cli(&exe), Some(release_cli.clone()));

        std::fs::remove_file(&release_cli).ok();
        assert_eq!(release_sibling_cli(&exe), None);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn hook_command_carries_explicit_config_path() {
        let command = hook_command_with_config(
            "codex",
            "SessionStart",
            true,
            Some("/srv/familiar/server.toml"),
            "/opt/familiar-cli",
        );
        assert!(command.contains("--config"));
        assert!(command.contains("server.toml"));
    }
}
