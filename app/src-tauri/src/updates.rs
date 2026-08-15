//! In-app update orchestration: fetches the latest GitHub release, runs the
//! domain decision logic from `familiar_core::update`, persists `last_check_at`,
//! and stashes the result so a freshly-opened settings window can show it even
//! if it missed the `update_available` event.

use familiar_core::update::{
    parse, parse_latest_release, should_check, should_notify, CheckUpdateResult, NotifyDecision,
};
use std::sync::{Arc, RwLock};

/// Latest stable release of familiar, published via `.github/workflows/release.yml`.
const RELEASES_URL: &str = "https://github.com/Monster12138/familiar/releases";
const LATEST_API_URL: &str = "https://api.github.com/repos/Monster12138/familiar/releases/latest";

/// Holds the most recent check result so a settings window created *after* the
/// check ran can still show the update prompt (the `update_available` event
/// emitted at check time would otherwise be missed).
pub struct PendingUpdateState(pub RwLock<Option<CheckUpdateResult>>);

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

async fn fetch_latest_release_json() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;
    let resp = client
        .get(LATEST_API_URL)
        // GitHub rejects requests without a User-Agent with a 403.
        .header(
            "User-Agent",
            concat!("familiar-app/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .map_err(|e| format!("failed to reach GitHub Releases API: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub Releases API returned {}", resp.status()));
    }
    resp.text()
        .await
        .map_err(|e| format!("failed to read GitHub release response: {e}"))
}

fn no_update_result(current_version: String, reason: &str) -> CheckUpdateResult {
    CheckUpdateResult {
        current_version,
        latest_version: None,
        has_update: false,
        release_notes: None,
        download_url: None,
        release_url: RELEASES_URL.to_string(),
        published_at: None,
        suppressed_reason: Some(reason.to_string()),
    }
}

/// Run one update check. `force=true` (manual check or tray) always hits the
/// network; `force=false` (startup auto-check) is gated by `check_on_startup`
/// and the `last_check_at` interval. `last_check_at` is only persisted after a
/// successful request+parse, so a failed check is retried on the next launch.
pub async fn run_check(
    app_handle: &tauri::AppHandle,
    config_state: &Arc<crate::commands::AppConfigState>,
    pending: &Arc<PendingUpdateState>,
    force: bool,
) -> Result<CheckUpdateResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let cfg = config_state.get_config();
    let interval = cfg.update.interval;
    if !force {
        if !cfg.update.check_on_startup {
            return Ok(no_update_result(current_version, "check_disabled"));
        }
        if !should_check(cfg.update.last_check_at, interval, now_secs()) {
            return Ok(no_update_result(current_version, "interval"));
        }
    }

    let json = fetch_latest_release_json().await?;
    let release = parse_latest_release(&json, std::env::consts::OS, std::env::consts::ARCH)
        .ok_or_else(|| "Could not parse the latest release".to_string())?;

    // Only a successful check counts toward the auto-check interval.
    let mut new_config = config_state.get_config();
    new_config.update.last_check_at = Some(now_secs());
    crate::commands::save_config_internal(app_handle, config_state, new_config)?;

    let latest_version = release.tag_name.clone();
    let decision = should_notify(
        parse(&current_version).as_ref(),
        release.version.as_ref(),
        cfg.update.skipped_version.as_deref(),
        &cfg.update.ignored_versions,
    );

    let (has_update, suppressed_reason) = match decision {
        NotifyDecision::Notify => (true, None),
        NotifyDecision::NoUpdate => (false, None),
        NotifyDecision::SuppressSkipped => (false, Some("skipped".to_string())),
        NotifyDecision::SuppressIgnored => (false, Some("ignored".to_string())),
    };

    let result = CheckUpdateResult {
        current_version,
        latest_version: Some(latest_version),
        has_update,
        release_notes: Some(release.body),
        download_url: release.download_url,
        release_url: release.html_url,
        published_at: release.published_at,
        suppressed_reason,
    };

    if has_update {
        if let Ok(mut guard) = pending.0.write() {
            *guard = Some(result.clone());
        }
    }
    Ok(result)
}
