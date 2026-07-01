//! Integration tests for `Game::n_digivolutions` / `Game::n_dna_digivolutions`
//! counter instrumentation. These counters back the digivolve reward-shaping
//! signal in `DigimonEnv._compute_reward`; see
//! `docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md`.

use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card_with_level, make_test_dna_card, DebugRunner};
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

    let ok = runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    assert!(ok, "regular digivolve must succeed in this setup");

    assert_eq!(runner.game.n_digivolutions, [1u32, 0u32]);
    assert_eq!(runner.game.n_dna_digivolutions, [0u32, 0u32]);
}

/// Drive a successful DNA digivolve via `Game::initiate_dna_digivolve` and
/// the two selection stages. Assert that **both** counters incremented
/// for the active player (DNA stacks on regular per spec decision 5),
/// and the opponent's counters did not move.
#[test]
fn dna_digivolve_increments_both_active_player_counters_once() {
    let lv5 = make_test_card_with_level("TST-LV5", "FiveDigi", 5);
    let lv6 = make_test_card_with_level("TST-LV6", "SixDigi", 6);
    let dna = make_test_dna_card("TST-DNA", "DnaDigi", 5, 6, 0);

    let mut runner = DebugRunner::builder()
        .add_card(lv5)
        .add_card(lv6)
        .add_card(dna)
        .hand(0, &["TST-DNA"])
        .memory(5)
        .start();

    let handle_lv5 = runner.place_on_field(0, "TST-LV5", None);
    let handle_lv6 = runner.place_on_field(0, "TST-LV6", None);
    runner.game.current_phase = GamePhase::Main;

    assert!(runner.game.initiate_dna_digivolve(0, 0));
    runner
        .game
        .resolve_selection(0, handle_lv5.index as u16)
        .expect("stage 1");
    runner
        .game
        .resolve_selection(0, handle_lv6.index as u16)
        .expect("stage 2");

    assert_eq!(runner.game.n_digivolutions, [1u32, 0u32]);
    assert_eq!(runner.game.n_dna_digivolutions, [1u32, 0u32]);
}

#[test]
fn dna_digivolve_material_prompts_clone_faithfully() {
    let lv5 = make_test_card_with_level("TST-LV5", "FiveDigi", 5);
    let lv6 = make_test_card_with_level("TST-LV6", "SixDigi", 6);
    let dna = make_test_dna_card("TST-DNA", "DnaDigi", 5, 6, 0);

    let mut runner = DebugRunner::builder()
        .add_card(lv5)
        .add_card(lv6)
        .add_card(dna)
        .hand(0, &["TST-DNA"])
        .memory(5)
        .start();

    let handle_lv5 = runner.place_on_field(0, "TST-LV5", None);
    let handle_lv6 = runner.place_on_field(0, "TST-LV6", None);
    runner.game.current_phase = GamePhase::Main;

    assert!(runner.game.initiate_dna_digivolve(0, 0));
    assert!(
        runner.game.pending_selection_resume.is_some(),
        "DNA first-material prompt must park a data frame"
    );

    let mut clone = runner.game.clone();
    clone
        .resolve_selection(0, handle_lv5.index as u16)
        .expect("clone picks first DNA material");
    assert!(
        clone.pending_selection_resume.is_some(),
        "DNA second-material prompt must also park a data frame"
    );
    assert!(
        runner.game.pending_selection.is_some(),
        "resolving the clone must leave the original at the first DNA prompt"
    );

    clone
        .resolve_selection(0, handle_lv6.index as u16)
        .expect("clone picks second DNA material");
    assert!(clone.pending_selection.is_none());
    assert_eq!(clone.player(0).hand.len(), 0);
    assert_eq!(clone.player(0).battle_area.len(), 1);
    assert_eq!(clone.n_digivolutions, [1u32, 0u32]);
    assert_eq!(clone.n_dna_digivolutions, [1u32, 0u32]);

    runner
        .game
        .resolve_selection(0, handle_lv5.index as u16)
        .expect("original picks first DNA material");
    runner
        .game
        .resolve_selection(0, handle_lv6.index as u16)
        .expect("original picks second DNA material");

    assert_eq!(
        runner.game.player(0).hand.len(),
        clone.player(0).hand.len(),
        "original and clone consume the DNA result card identically"
    );
    assert_eq!(
        runner.game.player(0).battle_area.len(),
        clone.player(0).battle_area.len(),
        "original and clone leave the same number of permanents"
    );
    assert_eq!(
        runner.game.n_dna_digivolutions, clone.n_dna_digivolutions,
        "original and clone increment DNA counters identically"
    );
}

/// If a DNA digivolve is rejected for a phase-illegality reason (here:
/// invoking it outside the Main phase), the counters must stay at zero.
/// This locks the implementation choice "increment after legality
/// validation, before state mutation" — any refactor that moves the
/// bump earlier in the function will fail this test.
#[test]
fn rejected_dna_digivolve_does_not_increment_counters() {
    let dna = make_test_dna_card("TST-DNA", "DnaDigi", 5, 6, 0);
    let mut runner = DebugRunner::builder()
        .add_card(dna)
        .hand(0, &["TST-DNA"])
        .start();

    // Non-Main phase: initiate_dna_digivolve should reject up-front.
    runner.game.current_phase = GamePhase::EndTurn;

    let ok = runner.game.initiate_dna_digivolve(0, 0);
    assert!(!ok, "non-Main phase must reject the DNA digivolve");

    assert_eq!(runner.game.n_digivolutions, [0u32, 0u32]);
    assert_eq!(runner.game.n_dna_digivolutions, [0u32, 0u32]);
}
