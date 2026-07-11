//! Desktop-side model manager: talks to the hosted API's public manifest,
//! downloads `.onnx` files into a per-app cache directory, verifies SHA256
//! + size against the manifest, and reports what's available locally.
//!
//! Cache layout:
//!
//! ```text
//! <app_data_dir>/models/
//!   <model_id>/
//!     policy.onnx
//!     meta.json
//! ```
//!
//! `meta.json` mirrors the manifest entry (id, name, type, shapes, SHA,
//! etc.) plus a `downloaded_at` epoch timestamp. The frontend uses it to
//! render the local library without re-fetching the manifest.
//!
//! Compatibility gate: download and `load_cached` both reject models whose
//! `tensor_size` / `action_space_size` don't match the Rust engine's
//! compiled-in constants. That's the safety rail from Phase C of the plan —
//! cheap insurance against shipping a model trained against a different
//! engine tensor layout.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use digimon_engine::action::space::ACTION_SPACE_SIZE;
use digimon_engine::tensor::TENSOR_SIZE;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::engine_commands::EngineHandle;
use crate::inference_state::InferenceState;

/// Shape of a single entry in `GET /models/manifest.json`. Must stay in
/// sync with `digimon_gym/db/schemas.py::ManifestModel`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestModel {
    pub id: String,
    pub name: String,
    pub model_type: String,
    pub tensor_size: usize,
    pub action_space_size: usize,
    pub engine_commit: Option<String>,
    pub trained_at: Option<String>,
    pub file_sha256: String,
    pub file_size_bytes: u64,
    pub url: String,
    pub deck_id: Option<String>,
    pub deck_name: Option<String>,
    /// Free-form slug linking this model to a *built-in* (non-DB) deck — e.g.
    /// an AI-Starter built-in deck id like "starter_st3_heavens_yellow". Lets
    /// the AI-Starter mode route each starter deck to its trained specialist.
    /// `None` for non-starter models (older servers omit the field entirely).
    #[serde(default)]
    pub starter_deck: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestResponse {
    generated_at: Option<String>,
    models: Vec<ManifestModel>,
    #[serde(default)]
    starter_ai_model_id: Option<String>,
}

/// What we persist alongside each downloaded `.onnx` in the cache. Extends
/// `ManifestModel` with the local download timestamp so the UI doesn't
/// need to stat the file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelMeta {
    #[serde(flatten)]
    pub manifest: ManifestModel,
    /// Unix epoch seconds when this file finished downloading.
    pub downloaded_at: u64,
}

/// Engine-side shape contract — what the running build's observation and
/// action-mask lengths are. The UI uses this to grey out incompatible
/// manifest entries before the user clicks "download."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineContract {
    pub tensor_size: usize,
    pub action_space_size: usize,
    /// Git commit of the engine crate if baked in at build time. `None`
    /// today — future builds can populate this via a `build.rs` env var.
    pub engine_commit: Option<String>,
}

impl EngineContract {
    pub fn current() -> Self {
        Self {
            tensor_size: TENSOR_SIZE,
            action_space_size: ACTION_SPACE_SIZE,
            engine_commit: option_env!("DIGIMON_ENGINE_COMMIT").map(|s| s.to_string()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ModelManagerError {
    #[error("incompatible model: {0}")]
    Incompatible(String),
    #[error("integrity check failed: {0}")]
    Integrity(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("model not found locally: {0}")]
    NotFound(String),
    #[error("{0}")]
    Other(String),
}

impl From<ModelManagerError> for String {
    fn from(e: ModelManagerError) -> Self {
        e.to_string()
    }
}

/// Tauri-managed state — knows where the cache lives on disk. Shares a
/// single `reqwest::Client` across downloads so connection pools / TLS
/// configs are reused.
pub struct ModelsManager {
    cache_root: PathBuf,
    http: reqwest::Client,
}

impl ModelsManager {
    pub fn new(cache_root: PathBuf) -> Self {
        Self {
            cache_root,
            http: reqwest::Client::builder()
                .user_agent(concat!("digimon-tcg-desktop/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("failed to build HTTP client (rustls init?)"),
        }
    }

    pub fn models_dir(&self) -> PathBuf {
        self.cache_root.join("models")
    }

    pub fn model_dir(&self, id: &str) -> PathBuf {
        self.models_dir().join(sanitize_id(id))
    }

    pub fn model_path(&self, id: &str) -> PathBuf {
        self.model_dir(id).join("policy.onnx")
    }

    pub fn meta_path(&self, id: &str) -> PathBuf {
        self.model_dir(id).join("meta.json")
    }

    pub fn engine_contract(&self) -> EngineContract {
        EngineContract::current()
    }

    /// Enumerate every cached model, newest-first. Silently skips entries
    /// with a missing/broken `meta.json` — a previous download that never
    /// finished shouldn't crash the UI.
    pub fn list_local(&self) -> Result<Vec<LocalModelMeta>, ModelManagerError> {
        let dir = self.models_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut out: Vec<LocalModelMeta> = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let meta_path = entry.path().join("meta.json");
            if !meta_path.exists() {
                continue;
            }
            match std::fs::read_to_string(&meta_path)
                .ok()
                .and_then(|s| serde_json::from_str::<LocalModelMeta>(&s).ok())
            {
                Some(meta) => out.push(meta),
                None => {
                    // Skip broken entries — don't let one corrupt meta file
                    // take down the whole listing.
                }
            }
        }
        out.sort_by_key(|m| std::cmp::Reverse(m.downloaded_at));
        Ok(out)
    }

    pub fn local_meta(&self, id: &str) -> Result<Option<LocalModelMeta>, ModelManagerError> {
        let meta_path = self.meta_path(id);
        if !meta_path.exists() {
            return Ok(None);
        }
        let s = std::fs::read_to_string(&meta_path)?;
        Ok(Some(serde_json::from_str(&s)?))
    }

    pub async fn fetch_manifest_full(
        &self,
        base_url: &str,
    ) -> Result<(Vec<ManifestModel>, Option<String>), ModelManagerError> {
        let url = format!("{}/models/manifest.json", base_url.trim_end_matches('/'));
        let resp = self.http.get(&url).send().await?.error_for_status()?;
        let parsed: ManifestResponse = resp.json().await?;
        Ok((parsed.models, parsed.starter_ai_model_id))
    }

    pub async fn fetch_manifest(
        &self,
        base_url: &str,
    ) -> Result<Vec<ManifestModel>, ModelManagerError> {
        Ok(self.fetch_manifest_full(base_url).await?.0)
    }

    /// Download a manifest entry. Rejects upfront on shape mismatch, streams
    /// the body to `<model_dir>/policy.onnx.tmp`, hashes as it goes, and
    /// atomically renames to `policy.onnx` only after SHA + size verify.
    pub async fn download(
        &self,
        model: &ManifestModel,
    ) -> Result<LocalModelMeta, ModelManagerError> {
        self.assert_compatible(model)?;

        let model_dir = self.model_dir(&model.id);
        tokio::fs::create_dir_all(&model_dir).await?;

        let final_path = self.model_path(&model.id);
        let tmp_path = model_dir.join("policy.onnx.tmp");

        // If a previous failed run left a .tmp behind, toss it — partial
        // contents are useless.
        let _ = tokio::fs::remove_file(&tmp_path).await;

        let resp = self.http.get(&model.url).send().await?.error_for_status()?;
        let mut stream = resp.bytes_stream();

        let mut file = tokio::fs::File::create(&tmp_path).await?;
        let mut hasher = Sha256::new();
        let mut total: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            hasher.update(&bytes);
            total += bytes.len() as u64;
            file.write_all(&bytes).await?;
        }
        file.flush().await?;
        drop(file);

        if total != model.file_size_bytes {
            tokio::fs::remove_file(&tmp_path).await.ok();
            return Err(ModelManagerError::Integrity(format!(
                "expected {} bytes, got {total}",
                model.file_size_bytes
            )));
        }

        let got_sha = hex_lower(&hasher.finalize());
        let expected_sha = model.file_sha256.to_lowercase();
        if got_sha != expected_sha {
            tokio::fs::remove_file(&tmp_path).await.ok();
            return Err(ModelManagerError::Integrity(format!(
                "sha256 mismatch: expected {expected_sha}, got {got_sha}"
            )));
        }

        tokio::fs::rename(&tmp_path, &final_path).await?;

        let meta = LocalModelMeta {
            manifest: model.clone(),
            downloaded_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        tokio::fs::write(
            self.meta_path(&model.id),
            serde_json::to_string_pretty(&meta)?,
        )
        .await?;
        Ok(meta)
    }

    pub fn delete(&self, id: &str) -> Result<bool, ModelManagerError> {
        let dir = self.model_dir(id);
        if !dir.exists() {
            return Ok(false);
        }
        std::fs::remove_dir_all(&dir)?;
        Ok(true)
    }

    /// Hand an already-downloaded model off to the inference cache. The
    /// Phase-B `InferenceState::load` does the ONNX session build; this just
    /// wires up the path and re-checks shape compatibility as a defence in
    /// depth against tampered meta.json files.
    pub fn load_cached(
        &self,
        id: &str,
        inference: &InferenceState,
    ) -> Result<LocalModelMeta, ModelManagerError> {
        let meta = self
            .local_meta(id)?
            .ok_or_else(|| ModelManagerError::NotFound(id.to_string()))?;
        self.assert_compatible(&meta.manifest)?;
        let path = self.model_path(id);
        if !path.exists() {
            return Err(ModelManagerError::NotFound(format!(
                "meta present but onnx missing: {}",
                path.display()
            )));
        }
        inference
            .load(id.to_string(), &path)
            .map_err(ModelManagerError::Other)?;
        Ok(meta)
    }

    fn assert_compatible(&self, model: &ManifestModel) -> Result<(), ModelManagerError> {
        if model.tensor_size != TENSOR_SIZE {
            return Err(ModelManagerError::Incompatible(format!(
                "model '{}' tensor_size {} != engine TENSOR_SIZE {}",
                model.id, model.tensor_size, TENSOR_SIZE
            )));
        }
        if model.action_space_size != ACTION_SPACE_SIZE {
            return Err(ModelManagerError::Incompatible(format!(
                "model '{}' action_space_size {} != engine ACTION_SPACE_SIZE {}",
                model.id, model.action_space_size, ACTION_SPACE_SIZE
            )));
        }
        Ok(())
    }
}

/// Keep filesystem paths sane when a manifest hands us a weird model id.
/// Backend ids are UUIDs today but this guards against future ids that
/// contain path separators or `..` segments.
fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            _ => '_',
        })
        .collect()
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

// ─── Tauri commands ───────────────────────────────────────────────────

#[tauri::command]
pub fn models_engine_contract(
    manager: tauri::State<'_, Arc<ModelsManager>>,
) -> Result<EngineContract, String> {
    Ok(manager.engine_contract())
}

#[tauri::command]
pub async fn models_fetch_manifest(
    manager: tauri::State<'_, Arc<ModelsManager>>,
    base_url: String,
) -> Result<Vec<ManifestModel>, String> {
    manager
        .fetch_manifest(&base_url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn models_list_local(
    manager: tauri::State<'_, Arc<ModelsManager>>,
) -> Result<Vec<LocalModelMeta>, String> {
    manager.list_local().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn models_download(
    manager: tauri::State<'_, Arc<ModelsManager>>,
    model: ManifestModel,
) -> Result<LocalModelMeta, String> {
    manager.download(&model).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn models_delete(
    manager: tauri::State<'_, Arc<ModelsManager>>,
    engine: tauri::State<'_, EngineHandle>,
    model_id: String,
) -> Result<bool, String> {
    // Evict from inference cache (owned by the engine worker) if loaded —
    // otherwise the session would hold an open handle to a file we're about
    // to remove.
    let evict_id = model_id.clone();
    engine
        .run(move |world| {
            let _ = world.inference.unload(&evict_id);
        })
        .await?;
    manager.delete(&model_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn models_load_cached(
    manager: tauri::State<'_, Arc<ModelsManager>>,
    engine: tauri::State<'_, EngineHandle>,
    model_id: String,
) -> Result<LocalModelMeta, String> {
    // The ONNX session build (`InferenceState::load`) must run on the engine
    // worker that owns the inference cache, so the session never crosses
    // threads. `ModelsManager` is `Send + Sync`, so we clone the `Arc` into
    // the worker closure.
    let manager: Arc<ModelsManager> = Arc::clone(&manager);
    engine
        .run(move |world| -> Result<LocalModelMeta, String> {
            manager
                .load_cached(&model_id, &world.inference)
                .map_err(|e| e.to_string())
        })
        .await?
}

/// Resolve the released "starter AI" model the hosted manifest points at via
/// `starter_ai_model_id`: download it (if not cached) and load it into the
/// inference cache, returning its id. Returns `Ok(None)` — so the caller falls
/// back to the greedy CPU — when there's no pointer, the model is missing /
/// incompatible, or the manifest can't be fetched (offline).
#[tauri::command]
pub async fn models_resolve_starter(
    manager: tauri::State<'_, Arc<ModelsManager>>,
    engine: tauri::State<'_, EngineHandle>,
    base_url: String,
) -> Result<Option<String>, String> {
    let (models, starter_id) = match manager.fetch_manifest_full(&base_url).await {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let Some(starter_id) = starter_id else {
        return Ok(None);
    };
    let Some(model) = models.into_iter().find(|m| m.id == starter_id) else {
        return Ok(None);
    };
    // Shape gate: skip incompatible models rather than erroring the game start.
    if model.tensor_size != TENSOR_SIZE || model.action_space_size != ACTION_SPACE_SIZE {
        return Ok(None);
    }
    // Download if not already cached.
    if manager
        .local_meta(&model.id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        if manager.download(&model).await.is_err() {
            return Ok(None);
        }
    }
    // Load into the inference cache on the engine worker (owns the session).
    let manager_arc: Arc<ModelsManager> = Arc::clone(&manager);
    let id = model.id.clone();
    let load: Result<(), String> = engine
        .run(move |world| -> Result<(), String> {
            manager_arc
                .load_cached(&id, &world.inference)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await?;
    match load {
        Ok(()) => Ok(Some(model.id)),
        Err(_) => Ok(None),
    }
}

/// Resolve the trained specialist for a specific built-in starter deck: find
/// the published, shape-compatible manifest model whose `starter_deck` slug
/// equals `deck_id`, download it (if not cached), load it into the inference
/// cache, and return its id. Returns `Ok(None)` — so the caller falls back to
/// the greedy CPU — when no matching/compatible model is published or the
/// manifest can't be fetched (offline). The manifest is ordered newest-first,
/// so the most recently trained specialist for the deck wins.
#[tauri::command]
pub async fn models_resolve_for_deck(
    manager: tauri::State<'_, Arc<ModelsManager>>,
    engine: tauri::State<'_, EngineHandle>,
    base_url: String,
    deck_id: String,
) -> Result<Option<String>, String> {
    let models = match manager.fetch_manifest(&base_url).await {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let Some(model) = models.into_iter().find(|m| {
        m.starter_deck.as_deref() == Some(deck_id.as_str())
            && m.tensor_size == TENSOR_SIZE
            && m.action_space_size == ACTION_SPACE_SIZE
    }) else {
        return Ok(None);
    };
    // Download if not already cached.
    if manager
        .local_meta(&model.id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        if manager.download(&model).await.is_err() {
            return Ok(None);
        }
    }
    // Load into the inference cache on the engine worker (owns the session).
    let manager_arc: Arc<ModelsManager> = Arc::clone(&manager);
    let id = model.id.clone();
    let load: Result<(), String> = engine
        .run(move |world| -> Result<(), String> {
            manager_arc
                .load_cached(&id, &world.inference)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await?;
    match load {
        Ok(()) => Ok(Some(model.id)),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_manifest_entry(id: &str, sha: &str, size: u64) -> ManifestModel {
        ManifestModel {
            id: id.into(),
            name: format!("test-{id}"),
            model_type: "mlp".into(),
            tensor_size: TENSOR_SIZE,
            action_space_size: ACTION_SPACE_SIZE,
            engine_commit: None,
            trained_at: None,
            file_sha256: sha.into(),
            file_size_bytes: size,
            url: "http://example.invalid/".into(),
            deck_id: None,
            deck_name: None,
            starter_deck: None,
            notes: None,
        }
    }

    /// A manifest entry tagged with a starter-deck slug (the per-deck
    /// specialist routing key), shape-compatible with the current engine.
    fn dummy_specialist(id: &str, starter_deck: &str) -> ManifestModel {
        let mut m = dummy_manifest_entry(id, "deadbeef", 10);
        m.starter_deck = Some(starter_deck.into());
        m
    }

    /// Mirror of `models_resolve_for_deck`'s selection predicate (the async
    /// command needs Tauri state, so the routing logic is unit-tested here).
    fn pick_specialist<'a>(
        models: &'a [ManifestModel],
        deck_id: &str,
    ) -> Option<&'a ManifestModel> {
        models.iter().find(|m| {
            m.starter_deck.as_deref() == Some(deck_id)
                && m.tensor_size == TENSOR_SIZE
                && m.action_space_size == ACTION_SPACE_SIZE
        })
    }

    #[test]
    fn manifest_model_parses_starter_deck_and_defaults_none() {
        let with: ManifestModel = serde_json::from_value(serde_json::json!({
            "id": "m1", "name": "ST-3 Specialist", "model_type": "mlp",
            "tensor_size": TENSOR_SIZE, "action_space_size": ACTION_SPACE_SIZE,
            "engine_commit": null, "trained_at": null,
            "file_sha256": "ab", "file_size_bytes": 1, "url": "x",
            "deck_id": null, "deck_name": null,
            "starter_deck": "starter_st3_heavens_yellow", "notes": null,
        }))
        .unwrap();
        assert_eq!(
            with.starter_deck.as_deref(),
            Some("starter_st3_heavens_yellow")
        );

        // Older servers omit the field entirely.
        let without: ManifestModel = serde_json::from_value(serde_json::json!({
            "id": "m2", "name": "generalist", "model_type": "lstm",
            "tensor_size": TENSOR_SIZE, "action_space_size": ACTION_SPACE_SIZE,
            "engine_commit": null, "trained_at": null,
            "file_sha256": "ab", "file_size_bytes": 1, "url": "x",
            "deck_id": null, "deck_name": null, "notes": null,
        }))
        .unwrap();
        assert_eq!(without.starter_deck, None);
    }

    #[test]
    fn resolve_for_deck_matches_slug_and_skips_incompatible() {
        let mut wrong_shape = dummy_specialist("bad", "starter_st1_gaia_red");
        wrong_shape.tensor_size = TENSOR_SIZE + 1;
        let models = vec![
            wrong_shape,
            dummy_specialist("st1", "starter_st1_gaia_red"),
            dummy_specialist("st2", "starter_st2_cocytus_blue"),
        ];
        // Picks the compatible st1 specialist, not the shape-mismatched one.
        assert_eq!(
            pick_specialist(&models, "starter_st1_gaia_red").unwrap().id,
            "st1"
        );
        assert_eq!(
            pick_specialist(&models, "starter_st2_cocytus_blue")
                .unwrap()
                .id,
            "st2"
        );
        // No specialist published for this deck -> greedy fallback (None).
        assert!(pick_specialist(&models, "starter_st6_venomous_violet").is_none());
    }

    #[test]
    fn engine_contract_matches_compiled_constants() {
        let c = EngineContract::current();
        assert_eq!(c.tensor_size, TENSOR_SIZE);
        assert_eq!(c.action_space_size, ACTION_SPACE_SIZE);
    }

    #[test]
    fn assert_compatible_rejects_shape_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ModelsManager::new(tmp.path().to_path_buf());

        let mut bad_tensor = dummy_manifest_entry("a", "deadbeef", 10);
        bad_tensor.tensor_size = TENSOR_SIZE + 1;
        assert!(matches!(
            mgr.assert_compatible(&bad_tensor),
            Err(ModelManagerError::Incompatible(_))
        ));

        let mut bad_actions = dummy_manifest_entry("b", "deadbeef", 10);
        bad_actions.action_space_size = ACTION_SPACE_SIZE + 1;
        assert!(matches!(
            mgr.assert_compatible(&bad_actions),
            Err(ModelManagerError::Incompatible(_))
        ));
    }

    #[test]
    fn assert_compatible_accepts_matching_shapes() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ModelsManager::new(tmp.path().to_path_buf());
        let good = dummy_manifest_entry("c", "deadbeef", 10);
        mgr.assert_compatible(&good).expect("matching shapes ok");
    }

    #[test]
    fn list_local_is_empty_on_fresh_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ModelsManager::new(tmp.path().to_path_buf());
        let listing = mgr.list_local().unwrap();
        assert!(listing.is_empty());
    }

    #[test]
    fn list_local_returns_written_meta_and_ignores_broken_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ModelsManager::new(tmp.path().to_path_buf());
        std::fs::create_dir_all(mgr.model_dir("good")).unwrap();
        let meta = LocalModelMeta {
            manifest: dummy_manifest_entry("good", "deadbeef", 10),
            downloaded_at: 42,
        };
        std::fs::write(mgr.meta_path("good"), serde_json::to_string(&meta).unwrap()).unwrap();

        // A sibling directory with a corrupt meta.json — should be silently
        // skipped, not fail the whole listing.
        std::fs::create_dir_all(mgr.model_dir("bad")).unwrap();
        std::fs::write(mgr.meta_path("bad"), "{ this isn't JSON").unwrap();

        let listing = mgr.list_local().unwrap();
        assert_eq!(listing.len(), 1);
        assert_eq!(listing[0].manifest.id, "good");
    }

    #[test]
    fn delete_removes_cache_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ModelsManager::new(tmp.path().to_path_buf());
        std::fs::create_dir_all(mgr.model_dir("x")).unwrap();
        std::fs::write(mgr.meta_path("x"), "{}").unwrap();
        assert!(mgr.delete("x").unwrap());
        assert!(!mgr.model_dir("x").exists());
        // Second delete is a no-op (returns false).
        assert!(!mgr.delete("x").unwrap());
    }

    #[test]
    fn manifest_response_parses_backend_schema() {
        // Shape matches digimon_gym/db/schemas.py::ManifestResponse. If the
        // backend drifts this test fails loudly rather than at runtime in
        // the hands of a user.
        let raw = serde_json::json!({
            "generated_at": "2026-04-17T00:00:00Z",
            "models": [{
                "id": "abc-123",
                "name": "Test Bot",
                "model_type": "lstm",
                "tensor_size": TENSOR_SIZE,
                "action_space_size": ACTION_SPACE_SIZE,
                "engine_commit": "abcd1234",
                "trained_at": "2026-04-01T12:00:00Z",
                "file_sha256": "ab".repeat(32),
                "file_size_bytes": 1024,
                "url": "https://spaces.example/models/abc-123/policy.onnx",
                "deck_id": null,
                "deck_name": null,
                "notes": "hello"
            }]
        });
        let parsed: ManifestResponse = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed.models.len(), 1);
        assert_eq!(parsed.models[0].model_type, "lstm");
        assert_eq!(parsed.models[0].tensor_size, TENSOR_SIZE);
    }

    #[test]
    fn manifest_response_parses_starter_pointer_and_defaults_none() {
        let with = serde_json::json!({
            "generated_at": "2026-06-16T00:00:00Z",
            "models": [],
            "starter_ai_model_id": "abc-123"
        });
        let parsed: ManifestResponse = serde_json::from_value(with).unwrap();
        assert_eq!(parsed.starter_ai_model_id.as_deref(), Some("abc-123"));

        let without = serde_json::json!({ "generated_at": "x", "models": [] });
        let parsed: ManifestResponse = serde_json::from_value(without).unwrap();
        assert_eq!(parsed.starter_ai_model_id, None);
    }

    #[test]
    fn sanitize_id_drops_path_traversal_chars() {
        assert_eq!(sanitize_id("abc-123_def"), "abc-123_def");
        assert_eq!(sanitize_id("../../evil"), "______evil");
        assert_eq!(sanitize_id("weird/\\chars"), "weird__chars");
    }

    #[tokio::test]
    async fn download_rejects_incompatible_tensor_size_before_hitting_network() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ModelsManager::new(tmp.path().to_path_buf());
        let mut bad = dummy_manifest_entry("z", "deadbeef", 1);
        bad.tensor_size = TENSOR_SIZE + 1;
        bad.url = "http://example.invalid/nowhere".into();
        let err = mgr.download(&bad).await.expect_err("should reject upfront");
        assert!(matches!(err, ModelManagerError::Incompatible(_)));
        // No directory should have been created.
        assert!(!mgr.model_dir("z").exists());
    }
}
