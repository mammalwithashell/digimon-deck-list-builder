//! Integration tests for `Game::n_digivolutions` / `Game::n_dna_digivolutions`
//! counter instrumentation. These counters back the digivolve reward-shaping
//! signal in `DigimonEnv._compute_reward`; see
//! `docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md`.

use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{GamePhase, PlaySource};

#[test]
fn new_game_starts_with_zero_digivolution_counters() {
    let runner = DebugRunner::builder().start();
    assert_eq!(runner.game.n_digivolutions, [0u32, 0u32]);
    assert_eq!(runner.game.n_dna_digivolutions, [0u32, 0u32]);
}

/// Drive a successful regular digivolve via `Game::digivolve_from_hand`
/// (which dispatches through `digivolve_from_hand_inner`) and assert the
/// regular counter incremented exactly once for the acting player, with
/// the DNA counter and the opponent's counters unchanged.
#[test]
fn regular_digivolve_from_hand_increments_only_active_player_regular_counter() {
    let base = make_test_card_with_level("BASE-LV4", "BaseLv4", 4);
    let mut evo = make_test_card_with_level("EVO-LV5", "EvoLv5", 5);
    // Standard evo-cost: from a Lv.4 of the same color (Red = default), 0 memory.
    evo.evo_costs = vec![EvoCost {
        card_color: 0, // Red — matches default color of make_test_card_with_level
        level: 4,
        memory_cost: 0,
    }];

    let mut runner = DebugRunner::builder()
        .add_card(base)
        .add_card(evo)
        .hand(0, &["EVO-LV5"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE-LV4", Some(0));
    runner.game.current_phase = GamePhase::Main;

    let ok = runner
        .game
        .digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    assert!(ok, "regular digivolve must succeed in this setup");

    assert_eq!(runner.game.n_digivolutions, [1u32, 0u32]);
    assert_eq!(runner.game.n_dna_digivolutions, [0u32, 0u32]);
}
