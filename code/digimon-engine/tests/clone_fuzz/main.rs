//! Clone-fuzz spike (add-determinized-search, Phase 2 opener).
//!
//! Empirical check that `Game: Clone` is faithful at EVERY production
//! decision point, not just the per-installer clone-guard sites: random
//! legal playouts over real starter decks where each decision
//!   1. clones the game,
//!   2. applies the SAME action to original and clone,
//!   3. asserts both land in an identical state (observation tensors for
//!      both seats + both action masks + scalar game state), and
//!   4. periodically CONTINUES FROM THE CLONE and discards the original
//!      (ship-of-Theseus: a fork must be a fully viable game, not merely
//!      one-step-safe).
//!
//! A surviving closure-based `pending_selection` park would panic loudly
//! at step 2 (the deliberate panic-stub in `impl Clone for
//! PendingSelection`); a shallow-shared mutable structure would diverge
//! the fingerprints. Random play reaches combat interrupts (Blocker /
//! Counter / Alliance windows), digivolves, options, and trigger windows
//! that scripted tests may not.
//!
//! Budget: `CLONE_FUZZ_GAMES` env overrides the default game count.
//! Run: RUST_MIN_STACK=268435456 cargo test --test clone_fuzz

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::enums::PlayerId;
use digimon_engine::game::Game;
use digimon_engine::runners::HeadlessRunner;

const D1: &[&str] = &[
    "ST1-01", "ST1-01", "ST1-01", "ST1-01", "ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-03",
    "ST1-03", "ST1-03", "ST1-03", "ST1-04", "ST1-04", "ST1-04", "ST1-04", "ST1-05", "ST1-05",
    "ST1-05", "ST1-05", "ST1-06", "ST1-06", "ST1-06", "ST1-06", "ST1-07", "ST1-07", "ST1-07",
    "ST1-07", "ST1-08", "ST1-08", "ST1-09", "ST1-09", "ST1-09", "ST1-09", "ST1-10", "ST1-10",
    "ST1-10", "ST1-10", "ST1-11", "ST1-11", "ST1-12", "ST1-12", "ST1-13", "ST1-13", "ST1-14",
    "ST1-14", "ST1-14", "ST1-14", "ST1-15", "ST1-15", "ST1-16", "ST1-16", "ST1-16", "ST1-16",
];
const D2: &[&str] = &[
    "ST5-01", "ST5-01", "ST5-01", "ST5-01", "ST5-02", "ST5-02", "ST5-02", "ST5-02", "ST5-03",
    "ST5-03", "ST5-03", "ST5-03", "ST5-04", "ST5-04", "ST5-04", "ST5-04", "ST5-05", "ST5-05",
    "ST5-05", "ST5-05", "ST5-06", "ST5-06", "ST5-06", "ST5-06", "ST5-07", "ST5-07", "ST5-07",
    "ST5-07", "ST5-08", "ST5-08", "ST5-09", "ST5-09", "ST5-09", "ST5-09", "ST5-10", "ST5-10",
    "ST5-10", "ST5-10", "ST5-11", "ST5-11", "ST5-12", "ST5-12", "ST5-13", "ST5-13", "ST5-14",
    "ST5-14", "ST5-14", "ST5-14", "ST5-15", "ST5-15",
];
// Second pairing to vary the mechanics mix (ST-2 blue bounce vs ST-6 red).
const D3: &[&str] = &[
    "ST2-01", "ST2-01", "ST2-01", "ST2-01", "ST2-02", "ST2-02", "ST2-02", "ST2-02", "ST2-03",
    "ST2-03", "ST2-03", "ST2-03", "ST2-04", "ST2-04", "ST2-04", "ST2-04", "ST2-05", "ST2-05",
    "ST2-05", "ST2-05", "ST2-06", "ST2-06", "ST2-06", "ST2-06", "ST2-07", "ST2-07", "ST2-07",
    "ST2-07", "ST2-08", "ST2-08", "ST2-09", "ST2-09", "ST2-09", "ST2-09", "ST2-10", "ST2-10",
    "ST2-10", "ST2-10", "ST2-11", "ST2-11", "ST2-12", "ST2-12", "ST2-13", "ST2-13", "ST2-14",
    "ST2-14", "ST2-14", "ST2-14", "ST2-15", "ST2-15", "ST2-16", "ST2-16", "ST2-16", "ST2-16",
];
const D4: &[&str] = &[
    "ST6-01", "ST6-01", "ST6-01", "ST6-01", "ST6-02", "ST6-02", "ST6-02", "ST6-02", "ST6-03",
    "ST6-03", "ST6-03", "ST6-03", "ST6-04", "ST6-04", "ST6-04", "ST6-04", "ST6-05", "ST6-05",
    "ST6-05", "ST6-05", "ST6-06", "ST6-06", "ST6-06", "ST6-06", "ST6-07", "ST6-07", "ST6-07",
    "ST6-07", "ST6-08", "ST6-08", "ST6-09", "ST6-09", "ST6-09", "ST6-09", "ST6-10", "ST6-10",
    "ST6-10", "ST6-10", "ST6-11", "ST6-11", "ST6-12", "ST6-12", "ST6-13", "ST6-13", "ST6-14",
    "ST6-14", "ST6-14", "ST6-14", "ST6-15", "ST6-15", "ST6-16", "ST6-16", "ST6-16", "ST6-16",
];

fn load_card_data() -> HashMap<String, CardData> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json");
    let text = std::fs::read_to_string(path).expect("read data/cards.json from repo root");
    CardData::load_from_str(&text).expect("parse cards.json")
}

/// Load a real meta decklist from `data/deck_library.json` (first decklist of
/// the named archetype). Used to reach mechanics the starter decks don't —
/// e.g. the Appmon pair exercises DigiLink (`LinkPickStep`), plug-in options,
/// and App Fusion, whose selection machinery starters never touch. Loud
/// failure on schema drift is intentional.
fn load_library_deck(archetype: &str) -> Vec<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/deck_library.json");
    let text = std::fs::read_to_string(path).expect("read data/deck_library.json from repo root");
    let v: serde_json::Value = serde_json::from_str(&text).expect("parse deck_library.json");
    let dl = v["archetypes"][archetype]["decklists"][0]["decklist"]
        .as_str()
        .unwrap_or_else(|| panic!("deck_library archetype {archetype:?} missing decklist[0]"));
    serde_json::from_str::<Vec<String>>(dl)
        .unwrap_or_else(|e| panic!("decklist for {archetype:?} is not a JSON string array: {e}"))
}

/// Deterministic LCG so the fuzz is reproducible per seed (no Date/thread rng).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick(&mut self, mask: &[f32]) -> Option<u16> {
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

fn decision_player(game: &Game) -> PlayerId {
    if let Some(p) = game.mulligan_current_player() {
        return p;
    }
    if let Some(sel) = game.pending_selection.as_ref() {
        return sel.selecting_player;
    }
    game.turn_player()
}

/// State fingerprint strong enough to catch shared-mutation or divergent
/// resolution: both seats' full observation tensors, both seats' action
/// masks, and the scalar game state.
fn fingerprint(game: &Game, registry: &CardRegistry) -> (Vec<f32>, Vec<f32>, String) {
    let mut floats = Vec::new();
    for pid in 0..2u8 {
        floats.extend(digimon_engine::tensor::build_tensor(game, pid, registry));
    }
    let mut masks = Vec::new();
    for pid in 0..2u8 {
        masks.extend(digimon_engine::build_action_mask(game, pid));
    }
    let scalars = format!(
        "phase={:?} turn={} memory={} over={} winner={:?} pending={} resume={}",
        game.current_phase,
        game.turn_count,
        game.memory,
        game.game_over,
        game.winner,
        game.pending_selection.is_some(),
        game.pending_selection_resume.is_some(),
    );
    (floats, masks, scalars)
}

fn assert_fingerprints_equal(
    a: &(Vec<f32>, Vec<f32>, String),
    b: &(Vec<f32>, Vec<f32>, String),
    ctx: &str,
) {
    assert_eq!(a.2, b.2, "scalar state diverged {ctx}");
    assert_eq!(a.0.len(), b.0.len(), "tensor length diverged {ctx}");
    if let Some(i) = a.0.iter().zip(b.0.iter()).position(|(x, y)| x != y) {
        panic!(
            "observation tensor diverged {ctx} at float {i}: {} vs {}",
            a.0[i], b.0[i]
        );
    }
    if let Some(i) = a.1.iter().zip(b.1.iter()).position(|(x, y)| x != y) {
        panic!(
            "action mask diverged {ctx} at bit {i}: {} vs {}",
            a.1[i], b.1[i]
        );
    }
}

fn fuzz_one_game(
    seed: u64,
    d1: &[String],
    d2: &[String],
    card_data: &HashMap<String, CardData>,
    registry: &CardRegistry,
    closure_parks: &mut std::collections::BTreeMap<String, u32>,
) -> (u32, u32) {
    let runner = HeadlessRunner::new(
        d1.to_vec(),
        d2.to_vec(),
        card_data,
        false,
        false,
        false,
        Some(seed),
    )
    .expect("build");
    // Drive the Game directly (symmetric application to both copies).
    let mut game = runner.game;
    let mut lcg = Lcg(seed ^ 0x9E3779B97F4A7C15);
    let mut decisions = 0u32;
    let mut swaps = 0u32;
    const DECISION_CAP: u32 = 600;

    while !game.game_over && decisions < DECISION_CAP {
        let pid = decision_player(&game);
        let mask = digimon_engine::build_action_mask(&game, pid);
        let action = match lcg.pick(&mask) {
            Some(a) => a,
            None => panic!(
                "seed {seed}: empty action mask for decision player {pid} at decision {decisions} (phase {:?})",
                game.current_phase
            ),
        };

        // A pending selection with no resume stack is a CLOSURE-ONLY park —
        // the exact clone-unsafety this spike hunts. Catalog it (kind +
        // prompt + source card) and skip the clone-compare for this decision
        // (the clone would panic on resolve); keep scanning for more.
        let closure_only =
            game.pending_selection.is_some() && game.pending_selection_resume.is_none();
        if closure_only {
            let sel = game.pending_selection.as_ref().unwrap();
            let key = format!(
                "kind={:?} source={:?} prompt={:?}",
                sel.kind,
                sel.source_card,
                sel.prompt.chars().take(80).collect::<String>()
            );
            *closure_parks.entry(key).or_insert(0) += 1;
            game.decode_action(action, pid);
            decisions += 1;
            continue;
        }

        // Fork BEFORE the action; resolve both copies identically.
        let mut clone = game.clone();
        game.decode_action(action, pid);
        clone.decode_action(action, pid);

        let fa = fingerprint(&game, registry);
        let fb = fingerprint(&clone, registry);
        assert_fingerprints_equal(
            &fa,
            &fb,
            &format!("(seed {seed}, decision {decisions}, action {action}, player {pid})"),
        );

        // Ship of Theseus: periodically continue from the CLONE.
        if decisions % 5 == 0 {
            game = clone;
            swaps += 1;
        }
        decisions += 1;
    }
    (decisions, swaps)
}

#[test]
fn clone_is_faithful_at_every_decision_of_random_playouts() {
    // Registry construction recurses deeply — run on a big-stack thread
    // (same mitigation as digimon-engine-cli main / RUST_MIN_STACK docs).
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            let games: u64 = std::env::var("CLONE_FUZZ_GAMES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(12);
            let card_data = load_card_data();
            let registry = CardRegistry::from_cards(&card_data);
            let owned = |d: &[&str]| -> Vec<String> { d.iter().map(|s| s.to_string()).collect() };
            // Pair 3 reaches the link/plug-in/App-Fusion selection machinery.
            let pairs: Vec<(Vec<String>, Vec<String>)> = vec![
                (owned(D1), owned(D2)),
                (owned(D3), owned(D4)),
                (load_library_deck("Appmon"), load_library_deck("Hero Appmon")),
            ];
            let (mut total_decisions, mut total_swaps) = (0u32, 0u32);
            let mut closure_parks = std::collections::BTreeMap::new();
            for seed in 0..games {
                let (d1, d2) = &pairs[(seed % pairs.len() as u64) as usize];
                let (n, s) =
                    fuzz_one_game(1000 + seed, d1, d2, &card_data, &registry, &mut closure_parks);
                total_decisions += n;
                total_swaps += s;
            }
            eprintln!(
                "clone_fuzz: {games} games, {total_decisions} decisions cloned+verified, {total_swaps} ship-of-theseus swaps"
            );
            assert!(
                total_decisions > 100,
                "fuzz did not reach a meaningful decision count ({total_decisions})"
            );
            if !closure_parks.is_empty() {
                let mut report = String::from(
                    "closure-only pending_selection parks found in production playouts \
                     (clone-UNSAFE — these must move to the resumable VM):\n",
                );
                for (site, count) in &closure_parks {
                    report.push_str(&format!("  x{count}  {site}\n"));
                }
                panic!("{report}");
            }
        })
        .expect("spawn")
        .join()
        .expect("fuzz thread panicked");
}
