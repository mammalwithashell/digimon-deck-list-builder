//! Exact saved-root reconstruction and paired intervention; no policy or search.
//! This module is for a new diagnostic harness, never archived-run recovery.
use digimon_engine::{
    action::build_action_mask, card_data::CardData, card_registry::CardRegistry,
    game::Game, observation::{build_observation_tensor, ObservationProfileId},
    rules::Rules, search::decision_player,
};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const PROFILE: ObservationProfileId = ObservationProfileId::StandardLiteV2;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SavedRoot {
    pub id: String,
    pub deck_indices: Vec<usize>,
    pub game_seed: u64,
    pub actions: Vec<u16>,
    pub mover: u8,
    pub hand: Vec<String>,
    pub observation_sha256: String,
}

pub fn observation_hash(game: &Game, registry: &CardRegistry, actor: u8) -> String {
    let mut hash = Sha256::new();
    for value in build_observation_tensor(game, actor, registry, PROFILE) {
        hash.update(value.to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

pub fn verify_root(game: &Game, registry: &CardRegistry, root: &SavedRoot) -> Result<(), String> {
    if root.mover > 1 || game.game_over || game.mulligan_current_player() != Some(root.mover)
        || decision_player(game) != root.mover {
        return Err(format!("{}: saved mulligan actor mismatch", root.id));
    }
    let legal: Vec<_> = build_action_mask(game, root.mover).iter().enumerate()
        .filter_map(|(id, p)| (*p > 0.5).then_some(id)).collect();
    if legal != [0, 1] { return Err(format!("{}: expected exact keep/redraw mask", root.id)); }
    let hand: Vec<_> = game.players[root.mover as usize].hand.iter()
        .map(|card| card.card_id(&game.card_data).to_string()).collect();
    if hand != root.hand { return Err(format!("{}: saved hand mismatch", root.id)); }
    if observation_hash(game, registry, root.mover) != root.observation_sha256 {
        return Err(format!("{}: saved observation byte hash mismatch", root.id));
    }
    Ok(())
}

pub fn reconstruct(root: &SavedRoot, decks: &[Vec<String>], data: &HashMap<String, CardData>,
                   registry: &CardRegistry) -> Result<Game, String> {
    if root.deck_indices.len() != 2 || root.deck_indices.iter().any(|id| *id >= decks.len()) {
        return Err(format!("{}: invalid deck indices", root.id));
    }
    let pair: Vec<_> = root.deck_indices.iter().map(|i| decks[*i].clone()).collect();
    let mut game = Game::new(&pair, data, Rules::standard(), Some(root.game_seed))
        .map_err(|e| format!("{}: {e:?}", root.id))?;
    game.turn_count = 1; // Matches the original recorded campaign harness.
    for (index, &action) in root.actions.iter().enumerate() {
        if game.game_over { return Err(format!("{}: terminal prefix {index}", root.id)); }
        let actor = decision_player(&game);
        if !build_action_mask(&game, actor).get(action as usize).is_some_and(|v| *v > 0.5) {
            return Err(format!("{}: illegal prefix {index} action {action}", root.id));
        }
        game.decode_action(action, actor);
    }
    verify_root(&game, registry, root)?;
    Ok(game)
}

/// Clone the exact root, replacing only future real-game RNG before forcing
/// keep(0)/redraw(1). Search RNG is intentionally absent from this interface.
pub fn intervene(game: &Game, registry: &CardRegistry, root: &SavedRoot,
                 branch: u16, outcome_seed: u64) -> Result<Game, String> {
    if branch > 1 { return Err(format!("{}: invalid forced branch", root.id)); }
    verify_root(game, registry, root)?;
    let mut fork = game.clone();
    fork.rng = rand::rngs::StdRng::seed_from_u64(outcome_seed);
    if fork.verification_digest() != game.verification_digest() {
        return Err(format!("{}: reseeding changed game state", root.id));
    }
    verify_root(&fork, registry, root)?;
    let before_pending = fork.mulligan_pending.len();
    fork.decode_action(branch, root.mover);
    if fork.mulligan_pending.len() + 1 != before_pending
        || fork.mulligan_used[root.mover as usize] != (branch == 1) {
        return Err(format!("{}: forced mulligan did not commit", root.id));
    }
    Ok(fork)
}
