//! In-memory game and ONNX model registries for the MCP server.
//!
//! A stdio MCP server is per-client; the registries live in the server
//! process and die when the client disconnects. No persistence, no
//! cross-client sharing.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use digimon_engine::inference::{
    load_inspector_evaluator, InferenceError, LoadedModelInfo,
};
use digimon_engine::live_game::LiveGame;
use digimon_engine::observation::ObservationProfileId;
use digimon_engine::search::PolicyValueFn;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Opaque short identifier — 8 alphanumeric chars. Easier to read in
/// logs than UUIDs while still globally unique within a single server
/// session.
pub fn new_game_id() -> String {
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| rng.sample(Alphanumeric) as char)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Auto-opponent policy bound to a `SessionMode::Play` game session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpponentDriver {
    Greedy,
    Onnx {
        model_id: String,
    },
    Mcts {
        #[serde(default)]
        model_id: Option<String>,
        #[serde(default = "default_mcts_sims")]
        simulations: u32,
        #[serde(default = "default_mcts_worlds")]
        worlds: u32,
        #[serde(default)]
        seed: u64,
    },
}

fn default_mcts_sims() -> u32 {
    64
}

fn default_mcts_worlds() -> u32 {
    2
}

impl OpponentDriver {
    pub fn from_json(val: Option<&Value>) -> Result<Option<Self>, String> {
        let Some(v) = val else {
            return Ok(None);
        };
        if v.is_null() {
            return Ok(None);
        }
        if let Some(s) = v.as_str() {
            return match s.trim() {
                "" | "none" | "human" => Ok(None),
                "greedy" => Ok(Some(Self::Greedy)),
                "mcts" => Ok(Some(Self::Mcts {
                    model_id: None,
                    simulations: default_mcts_sims(),
                    worlds: default_mcts_worlds(),
                    seed: 0,
                })),
                other if other.starts_with("onnx:") => Ok(Some(Self::Onnx {
                    model_id: other["onnx:".len()..].to_string(),
                })),
                other => Err(format!(
                    "unknown opponent_driver string {:?} — expected \"greedy\", \"mcts\", \"onnx:<model_id>\", or an object",
                    other
                )),
            };
        }
        if let Some(obj) = v.as_object() {
            let kind = obj
                .get("kind")
                .or_else(|| obj.get("type"))
                .and_then(Value::as_str)
                .ok_or_else(|| "opponent_driver object missing 'kind'".to_string())?;
            return match kind {
                "none" | "human" => Ok(None),
                "greedy" => Ok(Some(Self::Greedy)),
                "onnx" | "model" => {
                    let mid = obj
                        .get("model_id")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "opponent_driver kind='onnx' requires 'model_id'".to_string())?;
                    Ok(Some(Self::Onnx {
                        model_id: mid.to_string(),
                    }))
                }
                "mcts" => {
                    let model_id = obj
                        .get("model_id")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(String::from);
                    let simulations = obj
                        .get("simulations")
                        .and_then(Value::as_u64)
                        .map(|n| n as u32)
                        .unwrap_or_else(default_mcts_sims);
                    let worlds = obj
                        .get("worlds")
                        .and_then(Value::as_u64)
                        .map(|n| n as u32)
                        .unwrap_or_else(default_mcts_worlds);
                    let seed = obj.get("seed").and_then(Value::as_u64).unwrap_or(0);
                    Ok(Some(Self::Mcts {
                        model_id,
                        simulations,
                        worlds,
                        seed,
                    }))
                }
                other => Err(format!("unknown opponent_driver kind {:?}", other)),
            };
        }
        Err("opponent_driver must be a string, object, or null".to_string())
    }
}

/// Access-control & auto-opponent mode for an open game session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMode {
    /// Full God-view authoring & debugging access (default).
    Author,
    /// Seat-locked play session: clamps all state views to `Perspective::Player(seat)`,
    /// blocks `view="god"` and staging/export mutators, and optionally auto-steps
    /// opponent decisions via `opponent_driver`.
    Play {
        seat: u8,
        opponent_driver: Option<OpponentDriver>,
    },
}

impl SessionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Author => "author",
            Self::Play { .. } => "play",
        }
    }
}

pub struct GameEntry {
    pub game: LiveGame,
    pub mode: SessionMode,
    pub puzzle_id: Option<String>,
}

impl GameEntry {
    pub fn author(game: LiveGame) -> Self {
        Self {
            game,
            mode: SessionMode::Author,
            puzzle_id: None,
        }
    }
}

/// Cached ONNX policy+value evaluator (MLP or LSTM + `.value.onnx` sidecar).
pub struct CachedModel {
    pub model_id: String,
    pub onnx_path: String,
    pub info: LoadedModelInfo,
    pub evaluator: Box<dyn PolicyValueFn>,
}

#[derive(Default)]
pub struct ModelRegistry {
    models: BTreeMap<String, CachedModel>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: BTreeMap::new(),
        }
    }

    pub fn load(
        &mut self,
        model_id: String,
        onnx_path: String,
        profile_override: Option<ObservationProfileId>,
    ) -> Result<&LoadedModelInfo, InferenceError> {
        let (mut evaluator, info) =
            load_inspector_evaluator(Path::new(&onnx_path), profile_override)?;
        evaluator.reset_episode();
        self.models.insert(
            model_id.clone(),
            CachedModel {
                model_id: model_id.clone(),
                onnx_path,
                info,
                evaluator,
            },
        );
        Ok(&self.models.get(&model_id).expect("just inserted").info)
    }

    pub fn unload(&mut self, model_id: &str) -> bool {
        self.models.remove(model_id).is_some()
    }

    pub fn get_mut(&mut self, model_id: &str) -> Option<&mut CachedModel> {
        self.models.get_mut(model_id)
    }

    #[allow(dead_code)]
    pub fn get(&self, model_id: &str) -> Option<&CachedModel> {
        self.models.get(model_id)
    }

    pub fn list_json(&self) -> Vec<Value> {
        self.models
            .values()
            .map(|m| {
                json!({
                    "model_id": m.model_id,
                    "onnx_path": m.onnx_path,
                    "kind": m.info.kind,
                    "profile": m.info.profile.as_str(),
                    "value_semantics": m.info.value_semantics,
                })
            })
            .collect()
    }
}

pub struct GameRegistry {
    games: HashMap<String, GameEntry>,
    limit: usize,
    pub models: ModelRegistry,
    card_registry: Option<digimon_engine::card_registry::CardRegistry>,
}

impl GameRegistry {
    pub fn new(limit: usize) -> Self {
        Self {
            games: HashMap::new(),
            limit,
            models: ModelRegistry::new(),
            card_registry: None,
        }
    }

    pub fn ensure_card_registry(
        &mut self,
        card_data: &HashMap<String, digimon_engine::card_data::CardData>,
    ) -> &digimon_engine::card_registry::CardRegistry {
        self.card_registry
            .get_or_insert_with(|| digimon_engine::card_registry::CardRegistry::from_cards(card_data))
    }

    pub fn insert(&mut self, game: LiveGame) -> Result<String, RegistryError> {
        self.insert_entry(GameEntry::author(game))
    }

    pub fn insert_entry(&mut self, entry: GameEntry) -> Result<String, RegistryError> {
        if self.games.len() >= self.limit {
            return Err(RegistryError::AtCapacity(self.limit));
        }
        for _ in 0..4 {
            let id = new_game_id();
            if !self.games.contains_key(&id) {
                self.games.insert(id.clone(), entry);
                return Ok(id);
            }
        }
        Err(RegistryError::IdCollision)
    }

    pub fn get(&self, id: &str) -> Result<&LiveGame, RegistryError> {
        self.games
            .get(id)
            .map(|e| &e.game)
            .ok_or_else(|| RegistryError::UnknownGame(id.to_string()))
    }

    #[allow(dead_code)]
    pub fn get_mut(&mut self, id: &str) -> Result<&mut LiveGame, RegistryError> {
        self.games
            .get_mut(id)
            .map(|e| &mut e.game)
            .ok_or_else(|| RegistryError::UnknownGame(id.to_string()))
    }

    pub fn get_entry(&self, id: &str) -> Result<&GameEntry, RegistryError> {
        self.games
            .get(id)
            .ok_or_else(|| RegistryError::UnknownGame(id.to_string()))
    }

    #[allow(dead_code)]
    pub fn get_entry_mut(&mut self, id: &str) -> Result<&mut GameEntry, RegistryError> {
        self.games
            .get_mut(id)
            .ok_or_else(|| RegistryError::UnknownGame(id.to_string()))
    }

    pub fn split_mut_with_cards(
        &mut self,
        id: &str,
        card_data: &HashMap<String, digimon_engine::card_data::CardData>,
    ) -> Result<
        (
            &mut GameEntry,
            &mut ModelRegistry,
            &digimon_engine::card_registry::CardRegistry,
        ),
        RegistryError,
    > {
        if self.card_registry.is_none() {
            self.card_registry = Some(digimon_engine::card_registry::CardRegistry::from_cards(
                card_data,
            ));
        }
        let entry = self
            .games
            .get_mut(id)
            .ok_or_else(|| RegistryError::UnknownGame(id.to_string()))?;
        let creg = self.card_registry.as_ref().expect("initialized above");
        Ok((entry, &mut self.models, creg))
    }

    pub fn remove(&mut self, id: &str) -> Result<(), RegistryError> {
        match self.games.remove(id) {
            Some(_) => Ok(()),
            None => Err(RegistryError::UnknownGame(id.to_string())),
        }
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = (&String, &LiveGame)> {
        self.games.iter().map(|(k, v)| (k, &v.game))
    }

    pub fn iter_entries(&self) -> impl Iterator<Item = (&String, &GameEntry)> {
        self.games.iter()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.games.len()
    }

    pub fn limit(&self) -> usize {
        self.limit
    }
}

#[derive(Debug)]
pub enum RegistryError {
    AtCapacity(usize),
    IdCollision,
    UnknownGame(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AtCapacity(n) => write!(
                f,
                "registry at capacity ({} games) — close one with close_game before opening another",
                n
            ),
            Self::IdCollision => write!(f, "failed to allocate a unique game_id"),
            Self::UnknownGame(id) => write!(f, "unknown game_id: {}", id),
        }
    }
}
