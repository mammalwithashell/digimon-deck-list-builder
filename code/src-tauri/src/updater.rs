//! Update detection + manual-update support for the desktop launcher.
//!
//! Two historical bugs motivated this module's current shape:
//!
//!  * **Emit-before-ready race.** The previous implementation fetched the
//!    channel manifest in a `setup()` background task and *pushed* an
//!    `updater:force-update` event to the webview. Because the CDN fetch
//!    completes faster than the WebView2 cold-start + React mount, the
//!    event was emitted before the frontend registered its listener, and
//!    Tauri does not buffer events for late listeners — so the blocking
//!    "update required" modal was silently dropped. Detection is now
//!    **pull-based**: the frontend `invoke`s [`updater_check_info`] on
//!    mount and reads the result directly, so there is no race.
//!
//!  * **Silent failure.** Update-check failures were swallowed with no log
//!    sink wired, so a stranded install produced zero diagnostics. The
//!    commands here log every outcome (via `tauri-plugin-log`, configured
//!    in `main.rs`), and the frontend surfaces errors in the UI with a
//!    manual-download fallback.
//!
//! Detection (version comparison) lives here in Rust and is reliable.
//! The *in-app install* still goes through the `tauri-plugin-updater`
//! JS API on the frontend; if that path fails, the manual browser
//! download ([`updater_open_download_page`]) is always available.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// Hardcoded to the alpha channel for now. When beta/stable ship, split by
/// build feature or by reading a compile-time env var.
const MANIFEST_URL: &str =
    "https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com/updates/alpha/latest.json";

/// Canonical user-facing download page. Resolves the latest GitHub release
/// client-side, so it never needs updating per release. This is the target
/// of the "download manually" action — a fallback that works even when the
/// in-app updater path is broken.
const DOWNLOAD_PAGE_URL: &str = "https://mammalwithashell.github.io/digimon-deck-list-builder/";

/// One platform entry of the channel manifest (`signature` is unused for
/// detection — it's verified by the updater plugin at install time).
#[derive(Debug, Deserialize, Clone)]
struct PlatformEntry {
    url: String,
}

/// The channel manifest. Extra fields (`channel`, `engine_commit`,
/// `release_id`, `pub_date`, `signature`) are intentionally ignored.
#[derive(Debug, Deserialize, Clone)]
struct Manifest {
    version: String,
    min_version: String,
    #[serde(default)]
    notes: Option<String>,
    platforms: HashMap<String, PlatformEntry>,
}

/// Update status returned to the frontend. Drives both the auto modals
/// (on mount) and the always-visible manual-update card.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    /// The running app version.
    pub current_version: String,
    /// Latest version advertised by the manifest.
    pub latest_version: String,
    /// Hard floor below which the app must update to keep running.
    pub min_version: String,
    /// Release notes for the latest version, if any.
    pub notes: Option<String>,
    /// `current < latest` — a newer version exists.
    pub update_available: bool,
    /// `current < min_version` — the running version is unsupported and
    /// must be updated (drives the blocking modal).
    pub update_required: bool,
    /// Direct installer URL for the current platform, if the manifest has
    /// an entry for it.
    pub download_url: Option<String>,
    /// Browser download page (manual fallback).
    pub download_page_url: String,
}

/// The Tauri updater target key for the current platform, e.g.
/// `windows-x86_64`. Matches the keys used in the channel manifest's
/// `platforms` map and the `tauri-plugin-updater` plugin.
pub fn current_platform_key() -> String {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        std::env::consts::OS
    };
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        std::env::consts::ARCH
    };
    format!("{os}-{arch}")
}

/// SemVer comparison. Manifest versions may carry prerelease suffixes
/// (e.g. `-alpha.3`), so we delegate to the `semver` crate. Returns
/// `false` if either side is unparseable (never force/offer on garbage).
fn version_lt(a: &str, b: &str) -> bool {
    match (semver::Version::parse(a), semver::Version::parse(b)) {
        (Ok(va), Ok(vb)) => va < vb,
        _ => false,
    }
}

/// Pure version/platform comparison — the testable core of detection.
fn compute_update_info(
    current: &str,
    manifest: &Manifest,
    platform_key: &str,
    download_page_url: &str,
) -> UpdateInfo {
    UpdateInfo {
        current_version: current.to_string(),
        latest_version: manifest.version.clone(),
        min_version: manifest.min_version.clone(),
        notes: manifest.notes.clone(),
        update_available: version_lt(current, &manifest.version),
        update_required: version_lt(current, &manifest.min_version),
        download_url: manifest.platforms.get(platform_key).map(|p| p.url.clone()),
        download_page_url: download_page_url.to_string(),
    }
}

/// Fetch + parse the channel manifest from an arbitrary URL (testable).
async fn fetch_manifest_from(url: &str) -> Result<Manifest, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| format!("http client build failed: {e}"))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("manifest request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("manifest HTTP {}", resp.status()));
    }
    resp.json::<Manifest>()
        .await
        .map_err(|e| format!("manifest parse failed: {e}"))
}

/// Resolve the live update status for the running app. The single source
/// of truth for the frontend's detection (pull-based; no event race).
#[tauri::command]
pub async fn updater_check_info(app: AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    let manifest = fetch_manifest_from(MANIFEST_URL).await.map_err(|e| {
        log::warn!("updater_check_info: {e}");
        e
    })?;
    let info = compute_update_info(
        &current,
        &manifest,
        &current_platform_key(),
        DOWNLOAD_PAGE_URL,
    );
    log::info!(
        "updater_check_info: current={} latest={} min={} available={} required={}",
        info.current_version,
        info.latest_version,
        info.min_version,
        info.update_available,
        info.update_required
    );
    Ok(info)
}

/// Open the canonical download page in the user's default browser. The
/// manual-update fallback — works regardless of the in-app updater path.
#[tauri::command]
pub fn updater_open_download_page() -> Result<(), String> {
    log::info!("updater_open_download_page: opening {DOWNLOAD_PAGE_URL}");
    opener::open_browser(DOWNLOAD_PAGE_URL).map_err(|e| {
        let msg = format!("failed to open download page: {e}");
        log::warn!("{msg}");
        msg
    })
}

/// Best-effort startup probe. No longer pushes an event to the webview
/// (the frontend pulls on mount); this only emits a diagnostic log line
/// so a failed/stranded update is visible in the on-disk log.
pub fn spawn_min_version_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        match fetch_manifest_from(MANIFEST_URL).await {
            Ok(manifest) => {
                let current = app.package_info().version.to_string();
                let info = compute_update_info(
                    &current,
                    &manifest,
                    &current_platform_key(),
                    DOWNLOAD_PAGE_URL,
                );
                if info.update_required {
                    log::warn!(
                        "startup update probe: running {} is below min_version {} (latest {})",
                        info.current_version,
                        info.min_version,
                        info.latest_version
                    );
                } else if info.update_available {
                    log::info!(
                        "startup update probe: update available {} -> {}",
                        info.current_version,
                        info.latest_version
                    );
                } else {
                    log::info!(
                        "startup update probe: up to date ({})",
                        info.current_version
                    );
                }
            }
            Err(e) => log::warn!("startup update probe failed (ignoring): {e}"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str, min: &str) -> Manifest {
        let mut platforms = HashMap::new();
        platforms.insert(
            "windows-x86_64".to_string(),
            PlatformEntry {
                url: "https://example/win-setup.exe".to_string(),
            },
        );
        platforms.insert(
            "linux-x86_64".to_string(),
            PlatformEntry {
                url: "https://example/app.AppImage".to_string(),
            },
        );
        Manifest {
            version: version.to_string(),
            min_version: min.to_string(),
            notes: Some("notes here".to_string()),
            platforms,
        }
    }

    #[test]
    fn version_lt_prerelease_ordering() {
        assert!(version_lt("0.2.0-alpha.2", "0.2.0-alpha.3"));
        assert!(version_lt("0.2.0-alpha.3", "0.2.0"));
        assert!(!version_lt("0.2.0", "0.2.0"));
        assert!(!version_lt("0.3.0", "0.2.0"));
    }

    #[test]
    fn version_lt_unparseable_is_false() {
        assert!(!version_lt("not-a-version", "0.2.0"));
        assert!(!version_lt("0.2.0", "not-a-version"));
    }

    #[test]
    fn below_min_is_available_and_required() {
        let m = manifest("0.2.2", "0.2.0");
        let info = compute_update_info("0.1.0", &m, "windows-x86_64", DOWNLOAD_PAGE_URL);
        assert!(info.update_available);
        assert!(info.update_required);
        assert_eq!(info.latest_version, "0.2.2");
        assert_eq!(info.min_version, "0.2.0");
        assert_eq!(
            info.download_url.as_deref(),
            Some("https://example/win-setup.exe")
        );
        assert_eq!(info.download_page_url, DOWNLOAD_PAGE_URL);
    }

    #[test]
    fn between_min_and_latest_is_available_not_required() {
        let m = manifest("0.2.2", "0.2.0");
        let info = compute_update_info("0.2.1", &m, "windows-x86_64", DOWNLOAD_PAGE_URL);
        assert!(info.update_available);
        assert!(!info.update_required);
    }

    #[test]
    fn current_equals_latest_is_neither() {
        let m = manifest("0.2.2", "0.2.0");
        let info = compute_update_info("0.2.2", &m, "windows-x86_64", DOWNLOAD_PAGE_URL);
        assert!(!info.update_available);
        assert!(!info.update_required);
    }

    #[test]
    fn unknown_platform_yields_no_download_url() {
        let m = manifest("0.2.2", "0.2.0");
        let info = compute_update_info("0.1.0", &m, "solaris-sparc", DOWNLOAD_PAGE_URL);
        assert!(info.update_available);
        assert!(info.download_url.is_none());
    }

    #[test]
    fn platform_key_nonempty_and_dashed() {
        let key = current_platform_key();
        assert!(key.contains('-'), "platform key should be os-arch: {key}");
    }

    #[tokio::test]
    async fn fetch_manifest_parses_real_shaped_json() {
        let server = wiremock::MockServer::start().await;
        let body = serde_json::json!({
            "channel": "alpha",
            "engine_commit": "deadbeef",
            "min_version": "0.2.0",
            "notes": "Game: fixes.",
            "platforms": {
                "windows-x86_64": {
                    "signature": "sig-blob",
                    "url": "https://example/win-setup.exe"
                },
                "linux-x86_64": {
                    "signature": "sig-blob",
                    "url": "https://example/app.AppImage"
                }
            },
            "pub_date": "2026-06-12T21:29:10.516880Z",
            "release_id": "abc",
            "version": "0.2.2"
        });
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server)
            .await;

        let manifest = fetch_manifest_from(&server.uri()).await.expect("parse ok");
        assert_eq!(manifest.version, "0.2.2");
        assert_eq!(manifest.min_version, "0.2.0");
        assert_eq!(manifest.notes.as_deref(), Some("Game: fixes."));
        assert_eq!(
            manifest
                .platforms
                .get("windows-x86_64")
                .map(|p| p.url.as_str()),
            Some("https://example/win-setup.exe")
        );
    }

    #[tokio::test]
    async fn fetch_manifest_errors_on_http_500() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let err = fetch_manifest_from(&server.uri()).await.unwrap_err();
        assert!(err.contains("500"), "expected HTTP 500 in error: {err}");
    }
}
