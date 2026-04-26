//! Integration tests for `Game::initiate_dna_digivolve` resolving through
//! both selection stages into `Game::dna_digivolve_inner`. Companion to
//! `tests/effect_context/effect_initiated_dna_digivolve.rs` which covers
//! the engine-effect path.

use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::GamePhase;

fn empty_req() -> DnaRequirement {
    DnaRequirement {
        level: 0,
        card_colors: Vec::new(),
        name_contains: String::new(),
        text_contains: String::new(),
    }
}

fn lvl_req(level: u8) -> DnaRequirement {
    DnaRequirement {
        level,
        ..empty_req()
    }
}

fn dna_card(card_id: &str, name: &str, req1_lv: u8, req2_lv: u8, mem: i16) -> CardData {
    let mut d = make_test_card(card_id, name);
    d.dna_costs = vec![DnaCost {
        memory_cost: mem,
        requirement1: lvl_req(req1_lv),
        requirement2: lvl_req(req2_lv),
    }];
    d
}

fn lv_card(card_id: &str, name: &str, level: u8) -> CardData {
    let mut d = make_test_card(card_id, name);
    d.level = Some(level);
    d
}

#[test]
fn user_action_dna_digivolve_two_stage_resolution_merges_permanents() {
    let mut runner = DebugRunner::builder()
        .add_card(lv_card("TST-LV5", "FiveDigi", 5))
        .add_card(lv_card("TST-LV6", "SixDigi", 6))
        .add_card(dna_card("TST-DNA", "DnaDigi", 5, 6, 0))
        .hand(0, &["TST-DNA"])
        .memory(5)
        .start();

    let handle_lv5 = runner.place_on_field(0, "TST-LV5", None);
    let handle_lv6 = runner.place_on_field(0, "TST-LV6", None);
    runner.game.current_phase = GamePhase::Main;
    // Initiate: phase flips to SelectMaterial, first-stage selection installed.
    let ok = runner.game.initiate_dna_digivolve(0, 0);
    assert!(ok, "initiate must accept valid hand index");
    assert_eq!(runner.game.current_phase, GamePhase::SelectMaterial);
    assert!(runner.game.pending_selection.is_some());

    // Resolve first stage: pick handle_lv5 (idx 0).
    runner
        .game
        .resolve_selection(0, handle_lv5.index as u16)
        .expect("first-stage resolution must succeed");
    // Second-stage selection now installed (still in SelectMaterial phase).
    assert_eq!(runner.game.current_phase, GamePhase::SelectMaterial);
    assert!(runner.game.pending_selection.is_some());

    // Resolve second stage: pick handle_lv6 (idx 1).
    runner
        .game
        .resolve_selection(0, handle_lv6.index as u16)
        .expect("second-stage resolution must succeed");

    // Phase restored to Main.
    assert_eq!(runner.game.current_phase, GamePhase::Main);
    assert!(runner.game.pending_selection.is_none());

    // One merged permanent with 3 stacked sources.
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    let merged = &runner.game.players[0].battle_area[0];
    assert_eq!(merged.card_sources.len(), 3);
    // Hand consumed.
    assert_eq!(runner.game.players[0].hand.len(), 0);
}

#[test]
fn user_action_dna_digivolve_pays_memory_cost() {
    let mut runner = DebugRunner::builder()
        .add_card(lv_card("TST-LV5", "FiveDigi", 5))
        .add_card(lv_card("TST-LV6", "SixDigi", 6))
        .add_card(dna_card("TST-DNA-3", "DnaCost3", 5, 6, 3))
        .hand(0, &["TST-DNA-3"])
        .memory(5)
        .start();

    runner.place_on_field(0, "TST-LV5", None);
    runner.place_on_field(0, "TST-LV6", None);
    runner.game.current_phase = GamePhase::Main;

    runner.game.initiate_dna_digivolve(0, 0);
    runner.game.resolve_selection(0, 0).expect("stage 1");
    runner.game.resolve_selection(0, 1).expect("stage 2");

    // memory: 5 - 3 = 2
    assert_eq!(runner.game.memory, 2);
}

#[test]
fn user_action_dna_digivolve_grants_draw_bonus() {
    let mut runner = DebugRunner::builder()
        .add_card(lv_card("TST-LV5", "FiveDigi", 5))
        .add_card(lv_card("TST-LV6", "SixDigi", 6))
        .add_card(dna_card("TST-DNA", "DnaDigi", 5, 6, 0))
        .add_card(make_test_card("TST-DECK", "DeckCard"))
        .hand(0, &["TST-DNA"])
        .deck(0, &["TST-DECK"])
        .memory(5)
        .start();

    runner.place_on_field(0, "TST-LV5", None);
    runner.place_on_field(0, "TST-LV6", None);
    runner.game.current_phase = GamePhase::Main;

    let pre_hand_size = runner.game.players[0].hand.len();
    runner.game.initiate_dna_digivolve(0, 0);
    runner.game.resolve_selection(0, 0).expect("stage 1");
    runner.game.resolve_selection(0, 1).expect("stage 2");

    // Hand: -1 (DNA card consumed), +1 (digivolution bonus draw) = same size.
    // But deck shrank by 1.
    assert_eq!(runner.game.players[0].hand.len(), pre_hand_size);
    assert_eq!(runner.game.players[0].deck.len(), 0);
}

#[test]
fn user_action_dna_digivolve_rejects_when_phase_is_not_main() {
    let mut runner = DebugRunner::builder()
        .add_card(dna_card("TST-DNA", "DnaDigi", 5, 6, 0))
        .hand(0, &["TST-DNA"])
        .start();
    // Force a non-Main phase. (`Battle` is not a GamePhase variant; use
    // `EndTurn` to cover the "not Main" branch.)
    runner.game.current_phase = GamePhase::EndTurn;

    let ok = runner.game.initiate_dna_digivolve(0, 0);
    assert!(!ok, "non-Main phase must reject");
    assert!(runner.game.pending_selection.is_none());
}
