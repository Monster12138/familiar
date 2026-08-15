//! In-app update domain logic: version parsing/comparison, GitHub Releases
//! payload parsing, and the notify/suppress decision. Pure functions only —
//! no networking here. The Tauri app layer fetches the GitHub API and feeds
//! the raw JSON string into [`parse_latest_release`].

use crate::config::UpdateInterval;
use serde_json::Value;

/// Release prerelease kind, ordered by precedence (alpha < beta < rc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrereleaseKind {
    Alpha,
    Beta,
    Rc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prerelease {
    pub kind: PrereleaseKind,
    pub number: u32,
}

/// A lightweight semver-style version parsed from a release tag such as
/// `v0.5.4` or `v0.1.0-alpha.2`. A prerelease is always lower than the plain
/// release of the same `major.minor.patch` (matching semver precedence).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub prerelease: Option<Prerelease>,
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.prerelease, &other.prerelease) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(a), Some(b)) => (a.kind, a.number).cmp(&(b.kind, b.number)),
            })
    }
}

/// Parse a release tag. Accepts an optional `v`/`V` prefix, exactly three
/// numeric `major.minor.patch` segments, and an optional `alpha`/`beta`/`rc`
/// prerelease suffix (with optional `.N` number, defaulting to 0). Malformed
/// input returns `None` and callers treat that conservatively.
pub fn parse(tag: &str) -> Option<Version> {
    let s = tag
        .strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag);

    let (core, prerelease) = match s.find('-') {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (s, None),
    };

    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        // More than three numeric segments is not a valid release tag.
        return None;
    }

    let prerelease = match prerelease {
        Some(s) => Some(parse_prerelease(s)?),
        None => None,
    };
    Some(Version {
        major,
        minor,
        patch,
        prerelease,
    })
}

fn parse_prerelease(s: &str) -> Option<Prerelease> {
    let (kind, number) = match s.split_once('.') {
        Some((k, n)) => (k, n.parse().ok()?),
        None => (s, 0u32),
    };
    let kind = match kind {
        "alpha" => PrereleaseKind::Alpha,
        "beta" => PrereleaseKind::Beta,
        "rc" => PrereleaseKind::Rc,
        _ => return None,
    };
    Some(Prerelease { kind, number })
}

/// A parsed `GET /repos/{owner}/{repo}/releases/latest` response, with the
/// download URL of the asset matching the current platform (if any).
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub version: Option<Version>,
    pub html_url: String,
    pub published_at: Option<String>,
    pub body: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
}

/// Parse a GitHub Releases API JSON payload. `os`/`arch` come from
/// `std::env::consts::{OS, ARCH}` (passed in so tests can drive any platform).
/// Asset names are matched against familiar's post-rename release artifacts:
/// `Familiar_X.Y.Z_windows_x64-setup.exe`, `..._macos_aarch64.dmg`,
/// `..._macos_x64.dmg`, `familiar_X.Y.Z_linux_amd64.deb` (or `.AppImage`).
pub fn parse_latest_release(json: &str, os: &str, arch: &str) -> Option<ReleaseInfo> {
    let root: Value = serde_json::from_str(json).ok()?;
    let tag_name = root.get("tag_name")?.as_str()?.to_string();
    let html_url = root
        .get("html_url")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let published_at = root
        .get("published_at")
        .and_then(Value::as_str)
        .map(str::to_string);
    let body = root
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let assets: Vec<(String, String)> = root
        .get("assets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let name = a.get("name").and_then(Value::as_str)?;
            let url = a.get("browser_download_url").and_then(Value::as_str)?;
            Some((name.to_string(), url.to_string()))
        })
        .collect();

    let (download_url, asset_name) = find_platform_asset(&assets, os, arch);
    let version = parse(&tag_name);

    Some(ReleaseInfo {
        tag_name,
        version,
        html_url,
        published_at,
        body,
        download_url,
        asset_name,
    })
}

fn find_asset(
    assets: &[(String, String)],
    pred: impl Fn(&str) -> bool,
) -> Option<&(String, String)> {
    assets.iter().find(|(name, _)| pred(&name.to_lowercase()))
}

fn find_platform_asset(
    assets: &[(String, String)],
    os: &str,
    arch: &str,
) -> (Option<String>, Option<String>) {
    let matched = match (os, arch) {
        ("windows", _) => find_asset(assets, |n| n.contains("x64-setup.exe")),
        ("macos", "aarch64") => find_asset(assets, |n| n.contains("aarch64.dmg")),
        ("macos", "x86_64") => find_asset(assets, |n| n.contains("x64.dmg")),
        ("linux", "x86_64") => find_asset(assets, |n| n.contains("amd64.deb"))
            .or_else(|| find_asset(assets, |n| n.contains("amd64.appimage"))),
        _ => None,
    };
    match matched {
        Some((name, url)) => (Some(url.clone()), Some(name.clone())),
        None => (None, None),
    }
}

/// Decision the UI should act on for a given release comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyDecision {
    Notify,
    NoUpdate,
    SuppressSkipped,
    SuppressIgnored,
}

/// Decide whether the latest release warrants an update prompt. A latest or
/// current version that fails to parse is treated conservatively as no
/// update. Skipped/ignored entries that fail to parse simply never match.
pub fn should_notify(
    current: Option<&Version>,
    latest: Option<&Version>,
    skipped_version: Option<&str>,
    ignored_versions: &[String],
) -> NotifyDecision {
    let (Some(current), Some(latest)) = (current, latest) else {
        return NotifyDecision::NoUpdate;
    };
    if latest <= current {
        return NotifyDecision::NoUpdate;
    }
    if let Some(skipped) = skipped_version.and_then(parse) {
        if skipped == *latest {
            return NotifyDecision::SuppressSkipped;
        }
    }
    if ignored_versions
        .iter()
        .any(|s| parse(s).as_ref() == Some(latest))
    {
        return NotifyDecision::SuppressIgnored;
    }
    NotifyDecision::Notify
}

/// Seconds in one auto-check interval.
pub fn interval_secs(interval: UpdateInterval) -> u64 {
    match interval {
        UpdateInterval::Daily => 86_400,
        UpdateInterval::Weekly => 7 * 86_400,
    }
}

/// Whether an auto-check should run now, given the last successful check time.
/// `None` means never checked, so always check.
pub fn should_check(last_check_at: Option<u64>, interval: UpdateInterval, now_secs: u64) -> bool {
    match last_check_at {
        None => true,
        Some(last) => now_secs.saturating_sub(last) >= interval_secs(interval),
    }
}

/// Contract returned to the frontend after a check. `has_update` is the
/// single flag the UI keys on; `suppressed_reason` explains a negative result
/// (`"skipped" | "ignored" | "interval" | "check_disabled"`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckUpdateResult {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub release_url: String,
    pub published_at: Option<String>,
    pub suppressed_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major,
            minor,
            patch,
            prerelease: None,
        }
    }

    fn vpre(major: u64, minor: u64, patch: u64, kind: PrereleaseKind, number: u32) -> Version {
        Version {
            major,
            minor,
            patch,
            prerelease: Some(Prerelease { kind, number }),
        }
    }

    #[test]
    fn parses_valid_tags() {
        assert_eq!(
            parse("v0.5.4"),
            Some(Version {
                major: 0,
                minor: 5,
                patch: 4,
                prerelease: None,
            })
        );
        assert_eq!(parse("0.5.4"), parse("v0.5.4"));
        assert_eq!(parse("V0.5.4"), parse("v0.5.4"));
        assert_eq!(
            parse("v0.1.0-alpha.2"),
            Some(Version {
                major: 0,
                minor: 1,
                patch: 0,
                prerelease: Some(Prerelease {
                    kind: PrereleaseKind::Alpha,
                    number: 2,
                }),
            })
        );
        assert_eq!(
            parse("v1.2.3-beta.1"),
            Some(Version {
                major: 1,
                minor: 2,
                patch: 3,
                prerelease: Some(Prerelease {
                    kind: PrereleaseKind::Beta,
                    number: 1,
                }),
            })
        );
        // Missing prerelease number defaults to 0.
        assert_eq!(
            parse("v1.2.3-alpha"),
            Some(Version {
                major: 1,
                minor: 2,
                patch: 3,
                prerelease: Some(Prerelease {
                    kind: PrereleaseKind::Alpha,
                    number: 0,
                }),
            })
        );
        assert_eq!(
            parse("v1.2.3-rc"),
            Some(vpre(1, 2, 3, PrereleaseKind::Rc, 0))
        );
    }

    #[test]
    fn rejects_malformed_tags() {
        assert_eq!(parse("abc"), None);
        assert_eq!(parse("v1.2"), None);
        assert_eq!(parse("v1.2.3.4"), None);
        assert_eq!(parse("v1.2.3-x.y"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("v"), None);
    }

    #[test]
    fn orders_versions() {
        assert!(v(0, 5, 4) > v(0, 5, 3));
        assert!(v(0, 5, 4) == v(0, 5, 4));
        assert!(v(1, 0, 0) > v(0, 9, 9));
        // Prerelease is lower than the plain release of the same triple.
        assert!(v(0, 5, 4) > vpre(0, 5, 4, PrereleaseKind::Alpha, 2));
        // Precedence order alpha < beta < rc, and number breaks ties.
        assert!(vpre(0, 5, 4, PrereleaseKind::Alpha, 2) > vpre(0, 5, 4, PrereleaseKind::Alpha, 1));
        assert!(vpre(0, 5, 4, PrereleaseKind::Beta, 0) > vpre(0, 5, 4, PrereleaseKind::Alpha, 9));
        assert!(vpre(0, 5, 4, PrereleaseKind::Rc, 0) > vpre(0, 5, 4, PrereleaseKind::Beta, 9));
        // Missing number equals an explicit 0.
        assert_eq!(
            vpre(1, 2, 3, PrereleaseKind::Alpha, 0),
            vpre(1, 2, 3, PrereleaseKind::Alpha, 0)
        );
        assert!(parse("v1.0.0-alpha").unwrap() > parse("v0.5.4").unwrap());
    }

    fn sample_release_json() -> String {
        r#"{
            "tag_name": "v0.6.0",
            "html_url": "https://github.com/Monster12138/familiar/releases/tag/v0.6.0",
            "published_at": "2026-08-16T00:00:00Z",
            "body": "Highlights: a new version.",
            "assets": [
                {"name": "Familiar_0.6.0_windows_x64-setup.exe", "browser_download_url": "https://example.com/win.exe"},
                {"name": "Familiar_0.6.0_macos_aarch64.dmg", "browser_download_url": "https://example.com/mac-arm.dmg"},
                {"name": "Familiar_0.6.0_macos_x64.dmg", "browser_download_url": "https://example.com/mac-intel.dmg"},
                {"name": "familiar_0.6.0_linux_amd64.deb", "browser_download_url": "https://example.com/linux.deb"}
            ]
        }"#
        .to_string()
    }

    #[test]
    fn parses_release_for_current_platform() {
        let json = sample_release_json();
        let windows = parse_latest_release(&json, "windows", "x86_64").unwrap();
        assert_eq!(
            windows.asset_name.as_deref(),
            Some("Familiar_0.6.0_windows_x64-setup.exe")
        );
        assert_eq!(
            windows.download_url.as_deref(),
            Some("https://example.com/win.exe")
        );
        assert_eq!(windows.version, parse("v0.6.0"));
        assert_eq!(
            windows.published_at.as_deref(),
            Some("2026-08-16T00:00:00Z")
        );
        assert!(windows.body.contains("new version"));

        let mac_arm = parse_latest_release(&json, "macos", "aarch64").unwrap();
        assert_eq!(
            mac_arm.asset_name.as_deref(),
            Some("Familiar_0.6.0_macos_aarch64.dmg")
        );

        let mac_intel = parse_latest_release(&json, "macos", "x86_64").unwrap();
        assert_eq!(
            mac_intel.asset_name.as_deref(),
            Some("Familiar_0.6.0_macos_x64.dmg")
        );

        let linux = parse_latest_release(&json, "linux", "x86_64").unwrap();
        assert_eq!(
            linux.asset_name.as_deref(),
            Some("familiar_0.6.0_linux_amd64.deb")
        );
    }

    #[test]
    fn release_asset_falls_back_to_appimage_and_none() {
        let json = r#"{"tag_name":"v0.6.0","html_url":"u","assets":[
            {"name":"familiar_0.6.0_linux_amd64.AppImage","browser_download_url":"https://example.com/app"}]}"#;
        let linux = parse_latest_release(json, "linux", "x86_64").unwrap();
        assert_eq!(
            linux.download_url.as_deref(),
            Some("https://example.com/app")
        );

        let no_match = parse_latest_release(json, "linux", "aarch64").unwrap();
        assert_eq!(no_match.download_url, None);
        assert_eq!(no_match.asset_name, None);
    }

    #[test]
    fn malformed_release_json_returns_none() {
        assert!(parse_latest_release("not json", "windows", "x86_64").is_none());
        assert!(parse_latest_release(r#"{"foo":1}"#, "windows", "x86_64").is_none());
    }

    #[test]
    fn decides_whether_to_notify() {
        let current = v(0, 5, 4);
        let latest = v(0, 6, 0);

        assert_eq!(
            should_notify(Some(&current), Some(&latest), None, &[]),
            NotifyDecision::Notify
        );
        assert_eq!(
            should_notify(Some(&latest), Some(&current), None, &[]),
            NotifyDecision::NoUpdate
        );
        assert_eq!(
            should_notify(Some(&current), Some(&latest), Some("v0.6.0"), &[]),
            NotifyDecision::SuppressSkipped
        );
        assert_eq!(
            should_notify(Some(&current), Some(&latest), None, &["v0.6.0".to_string()]),
            NotifyDecision::SuppressIgnored
        );
        // Skipped version that does not match does not suppress.
        assert_eq!(
            should_notify(Some(&current), Some(&latest), Some("v0.5.4"), &[]),
            NotifyDecision::Notify
        );
        // Unparseable current/latest suppress conservatively.
        assert_eq!(
            should_notify(None, Some(&latest), None, &[]),
            NotifyDecision::NoUpdate
        );
        assert_eq!(
            should_notify(Some(&current), None, None, &[]),
            NotifyDecision::NoUpdate
        );
        // Garbage in skipped/ignored lists never matches.
        assert_eq!(
            should_notify(
                Some(&current),
                Some(&latest),
                Some("garbage"),
                &["oops".to_string()]
            ),
            NotifyDecision::Notify
        );
    }

    #[test]
    fn gates_auto_check_by_interval() {
        assert!(should_check(None, UpdateInterval::Daily, 1_000));
        assert!(!should_check(
            Some(1_000),
            UpdateInterval::Daily,
            1_000 + 86_399
        ));
        assert!(should_check(
            Some(1_000),
            UpdateInterval::Daily,
            1_000 + 86_400
        ));
        assert!(!should_check(
            Some(1_000),
            UpdateInterval::Weekly,
            1_000 + 7 * 86_399
        ));
        assert!(should_check(
            Some(1_000),
            UpdateInterval::Weekly,
            1_000 + 7 * 86_400
        ));
        assert_eq!(interval_secs(UpdateInterval::Daily), 86_400);
        assert_eq!(interval_secs(UpdateInterval::Weekly), 7 * 86_400);
    }
}
