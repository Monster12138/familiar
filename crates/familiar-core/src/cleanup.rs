use std::fs;
use std::path::{Path, PathBuf};

use crate::logger::default_log_dir;

/// UI-facing result of a data-cleanup dry-run or execution. Lives in
/// familiar-core so familiar-hooks (backup scanning) and the Tauri app can
/// share it.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DataCleanupSummary {
    pub backup_count: usize,
    pub log_count: usize,
    pub freed_bytes: u64,
    /// Per-file deletion errors (e.g. a log held open on Windows). The
    /// command keeps going and surfaces these instead of aborting.
    pub failures: Vec<String>,
}

/// Whether a filename looks like one of familiar's own log files
/// (`familiar_tauri.log`, `familiar_daemon.log`, and any future `familiar_*.log`).
pub fn is_familiar_log(file_name: &str) -> bool {
    file_name.starts_with("familiar_") && file_name.ends_with(".log")
}

/// Scan a flat directory for familiar log files (non-recursive; logs are
/// written directly to `default_log_dir`). Unreadable entries are skipped.
pub fn familiar_log_files_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut logs = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file() && is_familiar_log(&entry.file_name().to_string_lossy()) {
            logs.push(entry.path());
        }
    }
    logs
}

/// Scan the platform default log directory for familiar log files.
pub fn default_log_files_in() -> Vec<PathBuf> {
    familiar_log_files_in(&default_log_dir())
}

/// Whether a file is older than `age_days`. `0` means "always eligible".
/// Files that cannot be stat-ed are conservatively treated as NOT eligible.
pub fn is_older_than(path: &Path, age_days: u32) -> bool {
    if age_days == 0 {
        return true;
    }
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    modified
        .elapsed()
        .map(|age| age.as_secs() > u64::from(age_days) * 86_400)
        .unwrap_or(false)
}

/// Keep only files older than `age_days` (see [`is_older_than`]).
pub fn filter_by_age(files: Vec<PathBuf>, age_days: u32) -> Vec<PathBuf> {
    files
        .into_iter()
        .filter(|file| is_older_than(file, age_days))
        .collect()
}

/// Total size in bytes of the given files (used for the dry-run preview).
pub fn bytes_of(files: &[PathBuf]) -> u64 {
    files
        .iter()
        .filter_map(|file| fs::metadata(file).ok())
        .map(|meta| meta.len())
        .sum()
}

/// Delete files, tolerating already-missing files, and collect per-file
/// failures (e.g. the active log held open on Windows). Returns the total
/// freed bytes and the list of failures.
pub fn delete_files(files: &[PathBuf]) -> (u64, Vec<String>) {
    let mut freed = 0u64;
    let mut failures = Vec::new();
    for file in files {
        // Read the size before deleting so freed accounting works on
        // platforms where the path can no longer be stat-ed afterwards.
        let size = fs::metadata(file).map(|meta| meta.len()).unwrap_or(0);
        match fs::remove_file(file) {
            Ok(()) => freed += size,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => failures.push(format!("{}: {e}", file.display())),
        }
    }
    (freed, failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::time::{Duration, SystemTime};

    fn temp_file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        fs::write(&path, contents).expect("write temp file");
        path
    }

    #[test]
    fn familiar_log_name_matches_prefix() {
        assert!(is_familiar_log("familiar_tauri.log"));
        assert!(is_familiar_log("familiar_daemon.log"));
        assert!(is_familiar_log("familiar_anything.log"));
        assert!(!is_familiar_log("tauri.log"));
        assert!(!is_familiar_log("familiar_tauri.txt"));
        assert!(!is_familiar_log("notfamiliar.log"));
    }

    #[test]
    fn scans_only_familiar_logs_in_dir() {
        let dir =
            std::env::temp_dir().join(format!("familiar-cleanup-scan-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        fs::write(dir.join("familiar_tauri.log"), "a").expect("write log");
        fs::write(dir.join("familiar_daemon.log"), "bb").expect("write log");
        fs::write(dir.join("other.log"), "c").expect("write other");
        fs::write(dir.join("familiar_tauri.txt"), "d").expect("write txt");

        let found = familiar_log_files_in(&dir);
        let names: Vec<String> = found
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"familiar_tauri.log".to_string()));
        assert!(names.contains(&"familiar_daemon.log".to_string()));

        fs::remove_dir_all(&dir).expect("remove temp dir");
    }

    #[test]
    fn missing_dir_scans_to_empty() {
        let missing =
            std::env::temp_dir().join(format!("familiar-cleanup-missing-{}", std::process::id()));
        assert!(familiar_log_files_in(&missing).is_empty());
    }

    #[test]
    fn age_zero_always_eligible() {
        let file = temp_file("familiar-cleanup-fresh", "x");
        assert!(is_older_than(&file, 0));
        fs::remove_file(file).expect("remove temp file");
    }

    #[test]
    fn old_file_eligible_at_retention() {
        let file = temp_file("familiar-cleanup-old", "x");
        let old = SystemTime::now()
            .checked_sub(Duration::from_secs(200 * 86_400))
            .expect("200 days before now");
        OpenOptions::new()
            .write(true)
            .open(&file)
            .expect("open temp file for writing")
            .set_modified(old)
            .expect("set modified");

        assert!(is_older_than(&file, 90));
        assert!(!is_older_than(&file, 365));

        fs::remove_file(file).expect("remove temp file");
    }

    #[test]
    fn fresh_file_not_eligible_at_retention() {
        let file = temp_file("familiar-cleanup-fresh2", "x");
        assert!(!is_older_than(&file, 90));
        assert!(is_older_than(&file, 0));
        fs::remove_file(file).expect("remove temp file");
    }

    #[test]
    fn filter_keeps_only_old_files() {
        let dir =
            std::env::temp_dir().join(format!("familiar-cleanup-filter-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        let old_path = dir.join("old.log");
        let fresh_path = dir.join("fresh.log");
        fs::write(&old_path, "x").expect("write old");
        fs::write(&fresh_path, "x").expect("write fresh");
        let old = SystemTime::now()
            .checked_sub(Duration::from_secs(200 * 86_400))
            .expect("200 days before now");
        OpenOptions::new()
            .write(true)
            .open(&old_path)
            .expect("open old for writing")
            .set_modified(old)
            .expect("set old modified");

        let kept = filter_by_age(vec![old_path.clone(), fresh_path.clone()], 90);
        assert_eq!(kept, vec![old_path]);

        fs::remove_dir_all(&dir).expect("remove temp dir");
    }

    #[test]
    fn bytes_of_sums_file_sizes() {
        let dir =
            std::env::temp_dir().join(format!("familiar-cleanup-bytes-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        let a = dir.join("a.log");
        let b = dir.join("b.log");
        fs::write(&a, "aaaa").expect("write a");
        fs::write(&b, "bb").expect("write b");

        assert_eq!(bytes_of(&[a.clone(), b.clone()]), 6);
        // Missing files contribute zero.
        assert_eq!(bytes_of(&[a, dir.join("missing.log")]), 4);

        fs::remove_dir_all(&dir).expect("remove temp dir");
    }

    #[test]
    fn delete_removes_tolerates_missing_and_records_failures() {
        let dir = std::env::temp_dir().join(format!("familiar-cleanup-del-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        let file = dir.join("gone.log");
        fs::write(&file, "hello").expect("write file");

        let missing = dir.join("missing.log");
        let (freed, failures) = delete_files(&[file.clone(), missing.clone()]);
        assert_eq!(freed, 5);
        assert!(failures.is_empty(), "failures: {failures:?}");
        assert!(!file.exists());

        // A directory path cannot be removed with remove_file -> failure recorded.
        let (_, failures) = delete_files(std::slice::from_ref(&dir));
        assert_eq!(failures.len(), 1);

        fs::remove_dir_all(&dir).expect("remove temp dir");
    }
}
