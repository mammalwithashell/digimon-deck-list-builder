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
    /// Archetypes this opponent pilots (deck-name prefix before `:sha256:`).
    /// Only meaningful with `learner_decks`; None = any non-learner archetype.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decks: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpponentManifest {
    pub schema: String,
    pub selfplay_probability: f64,
    pub opponents: Vec<OpponentEntry>,
    /// Specialist mode: the learner seat always pilots one of these archetypes
    /// and the other seat never does (no mirrors). Self-play games then put the
    /// current model on both seats but train only on the learner seat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub learner_decks: Option<Vec<String>>,
}

/// Archetype of a deck-pool name: the part before `:sha256:`.
pub fn deck_archetype(name: &str) -> &str {
    name.split(":sha256:").next().unwrap_or(name)
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
        if let Some(learner) = &self.learner_decks {
            if learner.is_empty() || learner.iter().any(|a| a.trim().is_empty()) {
                return Err("learner_decks must name nonempty archetypes".into());
            }
            for entry in &self.opponents {
                if let Some(decks) = &entry.decks {
                    if decks.is_empty() || decks.iter().any(|a| learner.contains(a)) {
                        return Err(format!(
                            "opponent {} decks must be nonempty and exclude the learner archetypes",
                            entry.id
                        ));
                    }
                }
            }
        } else if self.opponents.iter().any(|e| e.decks.is_some()) {
            return Err("opponent deck restrictions require learner_decks".into());
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

    /// Specialist deck indices: (learner candidates, per-opponent candidates,
    /// self-play opponent candidates). None when not in specialist mode.
    pub fn specialist_decks(
        &self,
        deck_names: &[String],
    ) -> Result<Option<SpecialistDecks>, String> {
        let Some(learner) = &self.learner_decks else {
            return Ok(None);
        };
        let pick = |keep: &dyn Fn(&str) -> bool| -> Vec<usize> {
            (0..deck_names.len()).filter(|&i| keep(deck_archetype(&deck_names[i]))).collect()
        };
        let learner_idx = pick(&|a| learner.iter().any(|l| l == a));
        let others = pick(&|a| !learner.iter().any(|l| l == a));
        if learner_idx.is_empty() || others.is_empty() {
            return Err("specialist mode needs learner and non-learner decks in the pool".into());
        }
        let mut per_opponent = Vec::new();
        for entry in &self.opponents {
            let idx = match &entry.decks {
                None => others.clone(),
                Some(decks) => pick(&|a| decks.iter().any(|d| d == a)),
            };
            if idx.is_empty() {
                return Err(format!("opponent {} has no decks in the pool", entry.id));
            }
            per_opponent.push(idx);
        }
        Ok(Some(SpecialistDecks { learner: learner_idx, per_opponent, selfplay: others }))
    }
}

/// Deck-pool indices each seat may pilot in specialist mode.
#[derive(Debug, Clone)]
pub struct SpecialistDecks {
    pub learner: Vec<usize>,
    pub per_opponent: Vec<Vec<usize>>,
    pub selfplay: Vec<usize>,
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
