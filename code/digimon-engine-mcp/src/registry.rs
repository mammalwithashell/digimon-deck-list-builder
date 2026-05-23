//! In-memory game registry keyed by short opaque token IDs.
//!
//! A stdio MCP server is per-client; the registry lives in the server
//! process and dies when the client disconnects. No persistence, no
//! cross-client sharing.

use std::collections::HashMap;

use digimon_engine::live_game::LiveGame;
use rand::{distributions::Alphanumeric, Rng};

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

pub struct GameRegistry {
    games: HashMap<String, LiveGame>,
    limit: usize,
}

impl GameRegistry {
    pub fn new(limit: usize) -> Self {
        Self {
            games: HashMap::new(),
            limit,
        }
    }

    pub fn insert(&mut self, game: LiveGame) -> Result<String, RegistryError> {
        if self.games.len() >= self.limit {
            return Err(RegistryError::AtCapacity(self.limit));
        }
        // Retry on the (vanishingly unlikely) collision.
        for _ in 0..4 {
            let id = new_game_id();
            if !self.games.contains_key(&id) {
                self.games.insert(id.clone(), game);
                return Ok(id);
            }
        }
        Err(RegistryError::IdCollision)
    }

    pub fn get(&self, id: &str) -> Result<&LiveGame, RegistryError> {
        self.games
            .get(id)
            .ok_or_else(|| RegistryError::UnknownGame(id.to_string()))
    }

    pub fn get_mut(&mut self, id: &str) -> Result<&mut LiveGame, RegistryError> {
        self.games
            .get_mut(id)
            .ok_or_else(|| RegistryError::UnknownGame(id.to_string()))
    }

    pub fn remove(&mut self, id: &str) -> Result<(), RegistryError> {
        match self.games.remove(id) {
            Some(_) => Ok(()),
            None => Err(RegistryError::UnknownGame(id.to_string())),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &LiveGame)> {
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
