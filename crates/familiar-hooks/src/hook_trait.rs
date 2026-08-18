use anyhow::Result;
use async_trait::async_trait;
use familiar_core::event::{AgentCategory, AgentEvent};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// Build a distinctive backup path for an agent config file before an inject
/// (`marker = "bak"`) or uninstall (`marker = "bak.uninstall"`). The
/// `familiar-` prefix marks familiar's backups unambiguously, so the cleanup
/// scanner can recognize them without ever touching a backup another tool
/// created in the same directory.
pub fn backup_path(path: &Path, marker: &str) -> Result<PathBuf> {
    let stem = path
        .file_stem()
        .ok_or_else(|| anyhow::anyhow!("config path has no file name: {}", path.display()))?;
    let ts = chrono::Utc::now().timestamp();
    Ok(path.with_file_name(format!("familiar-{}.{marker}.{ts}", stem.to_string_lossy())))
}

#[async_trait]
pub trait AgentHook: Send + Sync {
    fn name(&self) -> &str;
    fn category(&self) -> AgentCategory;
    async fn start(&self, sender: mpsc::Sender<AgentEvent>) -> Result<()>;
    async fn stop(&self) -> Result<()>;

    // Hook injection management
    fn config_path(&self) -> Option<std::path::PathBuf> {
        None
    }
    fn is_injected(&self) -> bool {
        false
    }
    fn get_injection_payload(&self) -> Option<serde_json::Value> {
        None
    }
    fn inject(&self) -> Result<()> {
        Err(anyhow::anyhow!("Not implemented for this agent"))
    }
    fn uninstall(&self) -> Result<()> {
        Err(anyhow::anyhow!("Not implemented for this agent"))
    }

    // Returns (before_content, after_content)
    fn preview_inject(&self) -> Result<(String, String)> {
        Err(anyhow::anyhow!("Not implemented for this agent"))
    }
    fn preview_uninstall(&self) -> Result<(String, String)> {
        Err(anyhow::anyhow!("Not implemented for this agent"))
    }
}

#[cfg(test)]
mod tests {
    use super::backup_path;
    use std::path::PathBuf;

    #[test]
    fn backup_path_marks_with_familiar_prefix() {
        let path = PathBuf::from("/home/u/.claude/settings.json");
        let bak = backup_path(&path, "bak").expect("build inject backup path");
        let name = bak.file_name().unwrap().to_string_lossy();
        assert!(
            name.starts_with("familiar-settings.bak."),
            "unexpected inject backup name: {name}"
        );
        let epoch: u64 = name
            .rsplit('.')
            .next()
            .unwrap()
            .parse()
            .expect("epoch tail");
        assert!(epoch > 0);

        let uninstall = backup_path(&path, "bak.uninstall").expect("build uninstall backup path");
        let name = uninstall.file_name().unwrap().to_string_lossy();
        assert!(
            name.starts_with("familiar-settings.bak.uninstall."),
            "unexpected uninstall backup name: {name}"
        );
    }

    #[test]
    fn backup_path_keeps_parent_dir_and_stem() {
        let path = PathBuf::from("/home/u/.gemini/config/hooks.json");
        let bak = backup_path(&path, "bak").expect("build backup path");
        assert_eq!(bak.parent(), path.parent());
        assert!(bak
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("familiar-hooks.bak."));
    }

    #[test]
    fn backup_path_rejects_path_without_file_name() {
        // An empty path has no file-name component on any platform.
        assert!(backup_path(&PathBuf::new(), "bak").is_err());
    }
}
