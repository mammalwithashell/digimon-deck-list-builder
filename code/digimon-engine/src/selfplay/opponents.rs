//! Frozen historical opponents. Assignment uses no game/search RNG state.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::search::PolicyValueFn;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpponentEntry {
    pub id: String,
    pub path: PathBuf,
    pub sha256: String,
    /// Canonical sidecar is `<model>.onnx.meta.json`.
    pub metadata_sha256: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpponentManifest {
    pub schema: String,
    pub selfplay_probability: f64,
    pub opponents: Vec<OpponentEntry>,
}

pub fn file_sha256(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut digest = Sha256::new();
    std::io::copy(&mut file, &mut digest).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", digest.finalize()))
}

impl OpponentManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "search-opponent-pool-v1" {
            return Err("unsupported opponent pool schema".into());
        }
        if !self.selfplay_probability.is_finite()
            || !(0.0..=1.0).contains(&self.selfplay_probability)
        {
            return Err("selfplay_probability must be finite and in [0, 1]".into());
        }
        if self.opponents.is_empty() {
            return Err("requested opponent pool is empty".into());
        }
        let mut ids = HashSet::new();
        let mut hashes = HashSet::new();
        for entry in &self.opponents {
            if entry.id.trim().is_empty() || entry.id == "current" || !ids.insert(&entry.id) {
                return Err("opponent IDs must be unique, nonempty and not 'current'".into());
            }
            if !hashes.insert(&entry.sha256) {
                return Err("duplicate opponent model hash".into());
            }
            for hash in [&entry.sha256, &entry.metadata_sha256] {
                if hash.len() != 64
                    || !hash
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err("opponent hashes must be lowercase SHA256 hex".into());
                }
            }
            if !entry.weight.is_finite() || entry.weight <= 0.0 {
                return Err("opponent weights must be finite and positive".into());
            }
        }
        if !self
            .opponents
            .iter()
            .map(|e| e.weight)
            .sum::<f64>()
            .is_finite()
        {
            return Err("opponent weight sum must be finite".into());
        }
        Ok(())
    }

    /// Resolve paths against the manifest, validating every asset before workers play.
    pub fn load(path: &Path, current_model: &Path) -> Result<Self, String> {
        let raw = std::fs::read(path).map_err(|e| e.to_string())?;
        let mut manifest: Self = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        manifest.validate()?;
        let current_hash = file_sha256(current_model)?;
        for entry in &mut manifest.opponents {
            entry.path = path.parent().unwrap_or(Path::new(".")).join(&entry.path);
            if entry.sha256 == current_hash {
                return Err(format!("opponent {} aliases the current model", entry.id));
            }
            let meta = PathBuf::from(format!("{}.meta.json", entry.path.display()));
            if file_sha256(&entry.path)? != entry.sha256
                || file_sha256(&meta)? != entry.metadata_sha256
            {
                return Err(format!("opponent {} artifact hash mismatch", entry.id));
            }
        }
        Ok(manifest)
    }

    /// None means current-current self-play. Paired indices share an opponent;
    /// the learner opens on even games and plays second on odd games.
    pub fn assignment(&self, seed: u64, game_index: u32) -> Option<usize> {
        let mut rng = StdRng::seed_from_u64(super::driver::derive_seed(
            seed,
            0x48495354,
            (game_index / 2) as u64,
        ));
        if rng.gen::<f64>() < self.selfplay_probability {
            return None;
        }
        let mut ticket = rng.gen::<f64>() * self.opponents.iter().map(|e| e.weight).sum::<f64>();
        for (index, entry) in self.opponents.iter().enumerate() {
            ticket -= entry.weight;
            if ticket < 0.0 {
                return Some(index);
            }
        }
        Some(self.opponents.len() - 1)
    }
}

pub struct HistoricalOpponents {
    pub manifest: OpponentManifest,
    pub evaluators: Vec<Box<dyn PolicyValueFn>>,
}

impl HistoricalOpponents {
    pub fn validate(&self) -> Result<(), String> {
        self.manifest.validate()?;
        if self.evaluators.len() != self.manifest.opponents.len() {
            return Err("opponent evaluator count mismatch".into());
        }
        Ok(())
    }
}
