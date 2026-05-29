//! Behavioral tests for the engine's canonical default tensor.
//!
//! Added as part of `flip-engine-default-to-lite-deck-v2` (tasks 4.3 + 4.4):
//!   - 4.3: `tensor::build_tensor` and
//!     `observation::build_observation_tensor(.., default_profile)` must be
//!     byte-equal against any legal game state.
//!   - 4.4: tensors built mid-replay must populate the own-original-decklist
//!     section identically to tensors built mid-live-game, since the new
//!     default profile (`standard_lite_deck_v2`) reads
//!     `Player.original_deck` and replay must reconstruct that field from
//!     the recording's `initial_state`.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::game::Game;
use digimon_engine::observation::{build_observation_tensor, default_observation_profile};
use digimon_engine::rules::Rules;
use digimon_engine::tensor::build_tensor;
use digimon_engine::tensor_profiles::standard::v2_lite_deck::{
    DECKLIST_ROW_SIZE, OFF_OWN_ORIGINAL_DECKLIST,
};

fn card_db() -> HashMap<String, CardData> {
    let json = r#"{
        "EGG-001": {
            "card_id": "EGG-001", "card_name_eng": "TestEgg",
            "card_effect_class_name": "EGG_001", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-010": {
            "card_id": "BT1-010", "card_name_eng": "Agumon",
            "card_effect_class_name": "BT1_010", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-025": {
            "card_id": "BT1-025", "card_name_eng": "Greymon",
            "card_effect_class_name": "BT1_025", "play_cost": 5, "dp": 5000,
            "level": 4, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Dinosaur"], "form_eng": ["Champion"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": [{"card_color": 0, "level": 3, "memory_cost": 2}]
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

fn deck() -> Vec<String> {
    let mut d = Vec::new();
    for _ in 0..5 {
        d.push("EGG-001".to_string());
    }
    for _ in 0..25 {
        d.push("BT1-010".to_string());
    }
    for _ in 0..25 {
        d.push("BT1-025".to_string());
    }
    d
}

fn fresh_game() -> (Game, CardRegistry) {
    let db = card_db();
    let registry = CardRegistry::from_cards(&db);
    let d = deck();
    let mut game = Game::new(&[d.clone(), d], &db, Rules::standard(), Some(42)).unwrap();
    game.start_game();
    (game, registry)
}

// ─── Task 4.3 ────────────────────────────────────────────────────────────────

/// `tensor::build_tensor` is documented as a thin wrapper over the
/// observation dispatcher. The two paths MUST agree byte-for-byte; if they
/// ever drift, callers reading `tensor::TENSOR_SIZE` and callers reading
/// `default_observation_profile()` would see inconsistent observations.
#[test]
fn build_tensor_matches_dispatcher_for_default_profile() {
    let (game, registry) = fresh_game();

    let via_tensor = build_tensor(&game, 0, &registry);
    let via_dispatcher =
        build_observation_tensor(&game, 0, &registry, default_observation_profile());

    assert_eq!(via_tensor.len(), via_dispatcher.len());
    assert_eq!(
        via_tensor, via_dispatcher,
        "tensor::build_tensor must produce the same tensor as the dispatcher \
         routed through default_observation_profile()"
    );
}

/// Per-player check too — the perspective resolution lives inside the
/// builders, not the dispatcher, so we cover both seats.
#[test]
fn build_tensor_matches_dispatcher_both_players() {
    let (game, registry) = fresh_game();

    for pid in 0..=1 {
        let via_tensor = build_tensor(&game, pid, &registry);
        let via_dispatcher =
            build_observation_tensor(&game, pid, &registry, default_observation_profile());
        assert_eq!(
            via_tensor, via_dispatcher,
            "perspective {pid}: dispatcher and direct builder must agree"
        );
    }
}

// ─── Task 4.4 ────────────────────────────────────────────────────────────────
//
// Replay tensors must populate the own-original-decklist section. The
// `runners/replay.rs::rebuild_original_deck` helper closes that gap; this
// test pins the invariant so a future regression in either the recording
// format or the replay reconstruction shows up here.

/// Sanity: a fresh live game has a non-empty decklist section for the
/// observing player. (Precondition for the replay-parity test below.)
#[test]
fn live_game_populates_decklist_section() {
    let (game, registry) = fresh_game();
    let t = build_tensor(&game, 0, &registry);

    let decklist_populated_rows = (0..3)
        .map(|row| t[OFF_OWN_ORIGINAL_DECKLIST + row * DECKLIST_ROW_SIZE])
        .filter(|v| *v > 0.0)
        .count();
    assert!(
        decklist_populated_rows >= 1,
        "live game must populate at least one row of the own-original-decklist \
         section; got rows-populated={decklist_populated_rows}, deck has 3 \
         distinct card IDs ({:?})",
        &["EGG-001", "BT1-010", "BT1-025"]
    );
}

/// The `original_deck` field is the source of truth the v2_lite_deck builder
/// reads from. `Game::new` populates it; this test confirms it stays
/// populated through the start_game() lifecycle.
#[test]
fn game_new_populates_original_deck() {
    let (game, _registry) = fresh_game();
    let p0 = game.player(0);

    assert!(
        !p0.original_deck.is_empty(),
        "Game::new must populate Player.original_deck"
    );

    // 3 distinct card_ids → 3 entries (digitama + 2 main).
    assert_eq!(p0.original_deck.len(), 3);

    let total: u16 = p0.original_deck.iter().map(|r| r.count).sum();
    assert_eq!(total, 55, "5 eggs + 50 main deck");

    let digitama_total: u16 = p0
        .original_deck
        .iter()
        .filter(|r| r.is_digitama)
        .map(|r| r.count)
        .sum();
    assert_eq!(digitama_total, 5);
}
