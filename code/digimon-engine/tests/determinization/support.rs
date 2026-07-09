//! Shared fixtures for the determinization tests: real starter decks, card
//! data loading, a deterministic playout driver, and multiset helpers.
//!
//! Deck lists + playout pattern mirror `tests/clone_fuzz/main.rs` (test
//! crates cannot import from each other, so the constants are duplicated).

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::determinize::{check_world_invariants, materialize};
use digimon_engine::enums::PlayerId;
use digimon_engine::game::Game;
use digimon_engine::rules::Rules;

pub const D1: &[&str] = &[
    "ST1-01", "ST1-01", "ST1-01", "ST1-01", "ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-03",
    "ST1-03", "ST1-03", "ST1-03", "ST1-04", "ST1-04", "ST1-04", "ST1-04", "ST1-05", "ST1-05",
    "ST1-05", "ST1-05", "ST1-06", "ST1-06", "ST1-06", "ST1-06", "ST1-07", "ST1-07", "ST1-07",
    "ST1-07", "ST1-08", "ST1-08", "ST1-09", "ST1-09", "ST1-09", "ST1-09", "ST1-10", "ST1-10",
    "ST1-10", "ST1-10", "ST1-11", "ST1-11", "ST1-12", "ST1-12", "ST1-13", "ST1-13", "ST1-14",
    "ST1-14", "ST1-14", "ST1-14", "ST1-15", "ST1-15", "ST1-16", "ST1-16", "ST1-16", "ST1-16",
];
pub const D2: &[&str] = &[
    "ST5-01", "ST5-01", "ST5-01", "ST5-01", "ST5-02", "ST5-02", "ST5-02", "ST5-02", "ST5-03",
    "ST5-03", "ST5-03", "ST5-03", "ST5-04", "ST5-04", "ST5-04", "ST5-04", "ST5-05", "ST5-05",
    "ST5-05", "ST5-05", "ST5-06", "ST5-06", "ST5-06", "ST5-06", "ST5-07", "ST5-07", "ST5-07",
    "ST5-07", "ST5-08", "ST5-08", "ST5-09", "ST5-09", "ST5-09", "ST5-09", "ST5-10", "ST5-10",
    "ST5-10", "ST5-10", "ST5-11", "ST5-11", "ST5-12", "ST5-12", "ST5-13", "ST5-13", "ST5-14",
    "ST5-14", "ST5-14", "ST5-14", "ST5-15", "ST5-15",
];
pub const D3: &[&str] = &[
    "ST2-01", "ST2-01", "ST2-01", "ST2-01", "ST2-02", "ST2-02", "ST2-02", "ST2-02", "ST2-03",
    "ST2-03", "ST2-03", "ST2-03", "ST2-04", "ST2-04", "ST2-04", "ST2-04", "ST2-05", "ST2-05",
    "ST2-05", "ST2-05", "ST2-06", "ST2-06", "ST2-06", "ST2-06", "ST2-07", "ST2-07", "ST2-07",
    "ST2-07", "ST2-08", "ST2-08", "ST2-09", "ST2-09", "ST2-09", "ST2-09", "ST2-10", "ST2-10",
    "ST2-10", "ST2-10", "ST2-11", "ST2-11", "ST2-12", "ST2-12", "ST2-13", "ST2-13", "ST2-14",
    "ST2-14", "ST2-14", "ST2-14", "ST2-15", "ST2-15", "ST2-16", "ST2-16", "ST2-16", "ST2-16",
];
pub const D4: &[&str] = &[
    "ST6-01", "ST6-01", "ST6-01", "ST6-01", "ST6-02", "ST6-02", "ST6-02", "ST6-02", "ST6-03",
    "ST6-03", "ST6-03", "ST6-03", "ST6-04", "ST6-04", "ST6-04", "ST6-04", "ST6-05", "ST6-05",
    "ST6-05", "ST6-05", "ST6-06", "ST6-06", "ST6-06", "ST6-06", "ST6-07", "ST6-07", "ST6-07",
    "ST6-07", "ST6-08", "ST6-08", "ST6-09", "ST6-09", "ST6-09", "ST6-09", "ST6-10", "ST6-10",
    "ST6-10", "ST6-10", "ST6-11", "ST6-11", "ST6-12", "ST6-12", "ST6-13", "ST6-13", "ST6-14",
    "ST6-14", "ST6-14", "ST6-14", "ST6-15", "ST6-15", "ST6-16", "ST6-16", "ST6-16", "ST6-16",
];

pub fn owned(d: &[&str]) -> Vec<String> {
    d.iter().map(|s| s.to_string()).collect()
}

pub fn card_data() -> &'static HashMap<String, CardData> {
    static DATA: OnceLock<HashMap<String, CardData>> = OnceLock::new();
    DATA.get_or_init(|| {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json");
        let text = std::fs::read_to_string(path).expect("read data/cards.json from repo root");
        CardData::load_from_str(&text).expect("parse cards.json")
    })
}

/// Build a fresh 2-player standard game on ST1 vs ST5 (or the given decks).
pub fn new_game(seed: u64) -> Game {
    new_game_with(seed, D1, D2)
}

pub fn new_game_with(seed: u64, d1: &[&str], d2: &[&str]) -> Game {
    Game::new(
        &[owned(d1), owned(d2)],
        card_data(),
        Rules::standard(),
        Some(seed),
    )
    .expect("build game")
}

/// Registry construction / DSL lowering recurses deeply — run test bodies on
/// a big-stack thread (same mitigation as clone_fuzz / RUST_MIN_STACK docs).
pub fn with_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .expect("spawn big-stack test thread")
        .join()
        .expect("test thread panicked");
}

/// Deterministic LCG so playouts are reproducible per seed.
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

pub fn decision_player(game: &Game) -> PlayerId {
    if let Some(p) = game.mulligan_current_player() {
        return p;
    }
    if let Some(sel) = game.pending_selection.as_ref() {
        return sel.selecting_player;
    }
    game.turn_player()
}

/// Apply up to `decisions` random legal actions. Returns actual decisions made
/// (stops early on game over).
pub fn advance_random(game: &mut Game, lcg: &mut Lcg, decisions: u32) -> u32 {
    let mut done = 0;
    while !game.game_over && done < decisions {
        let pid = decision_player(game);
        let mask = digimon_engine::build_action_mask(game, pid);
        let action = lcg
            .pick(&mask)
            .unwrap_or_else(|| panic!("empty action mask for player {pid} after {done} decisions"));
        game.decode_action(action, pid);
        done += 1;
    }
    done
}

// ── zone helpers (tests may X-ray the true game — they're the oracle) ──

pub fn zone_ids(cards: &[CardSource], game: &Game) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&game.card_data).to_string())
        .collect()
}

pub fn zone_multiset(cards: &[CardSource], game: &Game) -> BTreeMap<String, usize> {
    let mut ms = BTreeMap::new();
    for c in cards {
        *ms.entry(c.card_id(&game.card_data).to_string())
            .or_insert(0) += 1;
    }
    ms
}

/// Ordered id dump of every zone of both players — a full identity
/// fingerprint used by the determinism tests.
pub fn full_zone_dump(game: &Game) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for p in &game.players {
        out.push(zone_ids(&p.hand, game));
        out.push(zone_ids(&p.deck, game));
        out.push(zone_ids(&p.security, game));
        out.push(zone_ids(&p.digitama_deck, game));
        out.push(zone_ids(&p.trash, game));
    }
    out
}

/// Materialize a world AND run the invariant checker — every test goes
/// through this so the checker runs on every sampled world.
pub fn materialize_checked(game: &Game, viewer: PlayerId, seed: u64) -> Game {
    let world = materialize(game, viewer, seed);
    check_world_invariants(game, viewer, &world).unwrap_or_else(|e| {
        panic!("world invariants violated (viewer {viewer}, seed {seed}): {e}")
    });
    world
}
