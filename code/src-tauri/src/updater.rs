//! Min-version guard: fetch the channel manifest on startup and, if the
//! running app's version is below the manifest's `min_version`, emit an
//! `updater:force-update` event so the frontend can render a blocking modal.
//!
//! This is separate from Tauri's own updater check — we want the min-version
//! decision made *before* any normal update prompt, because the whole point
//! is "the user cannot continue on this version."

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

/// Hardcoded to the alpha channel for now. When beta/stable ship, split by
/// build feature or by reading a compile-time env var.
const MANIFEST_URL: &str =
    "https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com/updates/alpha/latest.json";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MinVersionPeek {
    pub min_version: String,
    pub version: String,
}

/// Spawn the min-version check in a background Tokio task. Failure modes
/// (network error, bad JSON, missing file) are logged and ignored — we
/// never block the user on a transient failure.
pub fn spawn_min_version_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = check_min_version(&app).await {
            log::warn!("min-version check failed (ignoring): {e}");
        }
    });
}

async fn check_min_version(
    app: &AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()?;
    let resp = client.get(MANIFEST_URL).send().await?;
    if !resp.status().is_success() {
        return Err(format!("manifest HTTP {}", resp.status()).into());
    }
    let peek: MinVersionPeek = resp.json().await?;

    let running = app.package_info().version.to_string();
    if version_lt(&running, &peek.min_version) {
        log::warn!(
            "running version {} is below manifest min_version {} — forcing update to {}",
            running,
            peek.min_version,
            peek.version
        );
        app.emit("updater:force-update", &peek)?;
    }
    Ok(())
}

/// SemVer comparison. Manifest versions may have prerelease suffixes
/// (e.g. `-alpha.3`), so we delegate to the `semver` crate.
fn version_lt(a: &str, b: &str) -> bool {
    match (semver::Version::parse(a), semver::Version::parse(b)) {
        (Ok(va), Ok(vb)) => va < vb,
        _ => false, // if either side is unparseable, don't force-update
    }
}

#[cfg(test)]
mod tests {
    use super::version_lt;

    #[test]
    fn prerelease_ordering() {
        assert!(version_lt("0.2.0-alpha.2", "0.2.0-alpha.3"));
        assert!(version_lt("0.2.0-alpha.3", "0.2.0"));
        assert!(!version_lt("0.2.0", "0.2.0"));
        assert!(!version_lt("0.3.0", "0.2.0"));
    }

    #[test]
    fn unparseable_does_not_force() {
        assert!(!version_lt("not-a-version", "0.2.0"));
        assert!(!version_lt("0.2.0", "not-a-version"));
    }
}
