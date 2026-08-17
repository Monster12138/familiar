use std::fs;
use std::path::PathBuf;

/// Whether a filename tail is a non-empty sequence of ASCII digits (the Unix
/// epoch-seconds suffix familiar's backups carry).
fn is_epoch_tail(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// True for the exact filename pattern familiar's inject/uninstall backups
/// produce: `familiar-<stem>.bak.<epoch>` or
/// `familiar-<stem>.bak.uninstall.<epoch>` where `<stem>` is `settings` or
/// `hooks` (see [`crate::hook_trait::backup_path`]). The `familiar-` prefix
/// is required, so backups created by other tools — including the pre-marker
/// `settings.bak.<epoch>` format — are never touched.
pub fn is_familiar_backup(file_name: &str) -> bool {
    for stem in ["settings", "hooks"] {
        for marker in ["bak", "bak.uninstall"] {
            if let Some(rest) = file_name.strip_prefix(&format!("familiar-{stem}.{marker}.")) {
                if is_epoch_tail(rest) {
                    return true;
                }
            }
        }
    }
    false
}

/// The agent config directories familiar writes backups into. Keep in sync
/// when a new agent hook integration is added.
pub fn backup_dirs() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    vec![
        home.join(".claude"),
        home.join(".codex"),
        home.join(".qoder"),
        home.join(".gemini").join("config"),
        home.join(".dsh"),
    ]
}

/// Scan a set of directories (non-recursive) for familiar backup files.
/// Skips non-existent dirs and unreadable entries.
pub fn scan_backups_in(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut backups = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_file() && is_familiar_backup(&entry.file_name().to_string_lossy()) {
                backups.push(entry.path());
            }
        }
    }
    backups
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn familiar_backup_name_matches_marked_inject_and_uninstall() {
        assert!(is_familiar_backup("familiar-settings.bak.1755273600"));
        assert!(is_familiar_backup("familiar-hooks.bak.1755273600"));
        assert!(is_familiar_backup(
            "familiar-settings.bak.uninstall.1755273600"
        ));
        assert!(is_familiar_backup("familiar-hooks.bak.uninstall.0"));
    }

    #[test]
    fn familiar_backup_name_rejects_unmarked_and_non_familiar_files() {
        for name in [
            "settings.json",
            "hooks.json",
            "config.json",
            // Pre-marker backups must not be matched (another tool's backup
            // could share that name).
            "settings.bak.1755273600",
            "hooks.bak.uninstall.1755273600",
            "settings.bak",
            "hooks.bak.uninstall",
            "familiar-settings.bak",
            "familiar-hooks.bak.uninstall",
            "familiar-settings.bak.",
            "familiar-settings.bak.abc",
            "familiar-settings.bak.12a",
            "familiar-settings.json.bak.123",
            "familiar-hooks.json.bak.uninstall.123",
            "familiar-other.bak.123",
            "familiar-settings.bak.uninstall.",
        ] {
            assert!(!is_familiar_backup(name), "should reject {name}");
        }
    }

    #[test]
    fn scans_only_marked_familiar_backups_in_dirs() {
        let dir =
            std::env::temp_dir().join(format!("familiar-hooks-backup-scan-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        fs::write(dir.join("familiar-settings.bak.1755273600"), "a").expect("write backup");
        fs::write(dir.join("familiar-hooks.bak.uninstall.1755273600"), "b").expect("write backup");
        fs::write(dir.join("settings.json"), "c").expect("write config");
        fs::write(dir.join("settings.json.bak.123"), "d").expect("write editor backup");
        fs::write(dir.join("settings.bak.1755273600"), "e").expect("write legacy backup");
        fs::write(dir.join("unrelated.txt"), "f").expect("write unrelated");

        let found = scan_backups_in(std::slice::from_ref(&dir));
        let names: Vec<String> = found
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        assert_eq!(names.len(), 2, "found: {names:?}");
        assert!(names.contains(&"familiar-settings.bak.1755273600".to_string()));
        assert!(names.contains(&"familiar-hooks.bak.uninstall.1755273600".to_string()));

        fs::remove_dir_all(&dir).expect("remove temp dir");
    }

    #[test]
    fn missing_dir_scans_to_empty() {
        let missing = std::env::temp_dir().join(format!(
            "familiar-hooks-backup-missing-{}",
            std::process::id()
        ));
        assert!(scan_backups_in(&[missing]).is_empty());
    }

    #[test]
    fn backup_dirs_lists_five_agent_dirs() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        assert_eq!(
            backup_dirs(),
            vec![
                home.join(".claude"),
                home.join(".codex"),
                home.join(".qoder"),
                home.join(".gemini").join("config"),
                home.join(".dsh"),
            ]
        );
    }
}
