//! Shared fixtures for the search integration tests.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::game::Game;

/// Run a test body on a big-stack thread (registry construction and deep
/// effect recursion overflow the default test-thread stack).
pub fn run_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .expect("spawn big-stack test thread")
        .join()
        .expect("test thread panicked");
}

/// Vanilla effectless Digimon for synthetic perfect-information boards.
pub fn make_digimon(id: &str, color: CardColor, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 5,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// Full card pool from `data/cards.json` (for real-deck mid-game states).
pub fn load_card_data() -> HashMap<String, CardData> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json");
    let text = std::fs::read_to_string(path).expect("read data/cards.json from repo root");
    CardData::load_from_str(&text).expect("parse cards.json")
}

/// ST-1 starter decklist (mirrors `tests/clone_fuzz/main.rs`).
pub const ST1_DECK: &[&str] = &[
    "ST1-01", "ST1-01", "ST1-01", "ST1-01", "ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-03",
    "ST1-03", "ST1-03", "ST1-03", "ST1-04", "ST1-04", "ST1-04", "ST1-04", "ST1-05", "ST1-05",
    "ST1-05", "ST1-05", "ST1-06", "ST1-06", "ST1-06", "ST1-06", "ST1-07", "ST1-07", "ST1-07",
    "ST1-07", "ST1-08", "ST1-08", "ST1-09", "ST1-09", "ST1-09", "ST1-09", "ST1-10", "ST1-10",
    "ST1-10", "ST1-10", "ST1-11", "ST1-11", "ST1-12", "ST1-12", "ST1-13", "ST1-13", "ST1-14",
    "ST1-14", "ST1-14", "ST1-14", "ST1-15", "ST1-15", "ST1-16", "ST1-16", "ST1-16", "ST1-16",
];

/// ST-5 starter decklist (mirrors `tests/clone_fuzz/main.rs`).
pub const ST5_DECK: &[&str] = &[
    "ST5-01", "ST5-01", "ST5-01", "ST5-01", "ST5-02", "ST5-02", "ST5-02", "ST5-02", "ST5-03",
    "ST5-03", "ST5-03", "ST5-03", "ST5-04", "ST5-04", "ST5-04", "ST5-04", "ST5-05", "ST5-05",
    "ST5-05", "ST5-05", "ST5-06", "ST5-06", "ST5-06", "ST5-06", "ST5-07", "ST5-07", "ST5-07",
    "ST5-07", "ST5-08", "ST5-08", "ST5-09", "ST5-09", "ST5-09", "ST5-09", "ST5-10", "ST5-10",
    "ST5-10", "ST5-10", "ST5-11", "ST5-11", "ST5-12", "ST5-12", "ST5-13", "ST5-13", "ST5-14",
    "ST5-14", "ST5-14", "ST5-14", "ST5-15", "ST5-15",
];

/// Deterministic LCG for reproducible random playouts (no thread rng).
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    pub fn pick(&mut self, mask: &[f32]) -> Option<u16> {
        let legal: Vec<u16> = mask
            .iter()
            .enumerate()
            .filter(|(_, &v)| v > 0.5)
            .map(|(i, _)| i as u16)
            .collect();
        if legal.is_empty() {
            return None;
        }
        Some(legal[(self.next() as usize) % legal.len()])
    }
}

/// The player whose decision the current state is waiting on (mirrors the
/// resolution rule in `tests/clone_fuzz/main.rs`).
pub fn decision_player(game: &Game) -> PlayerId {
    if let Some(p) = game.mulligan_current_player() {
        return p;
    }
    if let Some(sel) = game.pending_selection.as_ref() {
        return sel.selecting_player;
    }
    game.turn_player()
}

/// Advance a game by up to `decisions` uniformly-random legal actions.
pub fn advance_random(game: &mut Game, lcg: &mut Lcg, decisions: u32) {
    let mut done = 0u32;
    while !game.game_over && done < decisions {
        let pid = decision_player(game);
        let mask = digimon_engine::build_action_mask(game, pid);
        let Some(action) = lcg.pick(&mask) else {
            panic!("empty action mask during random playout (decision {done})");
        };
        game.decode_action(action, pid);
        done += 1;
    }
}

/// Legal action ids in a mask.
pub fn legal_actions(mask: &[f32]) -> Vec<u16> {
    mask.iter()
        .enumerate()
        .filter(|(_, &v)| v > 0.5)
        .map(|(i, _)| i as u16)
        .collect()
}
