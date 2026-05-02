//! Integration tests for `Game::initiate_dna_digivolve` resolving through
//! both selection stages into `Game::dna_digivolve_inner`. Companion to
//! `tests/effect_context/effect_initiated_dna_digivolve.rs` which covers
//! the engine-effect path.

use digimon_engine::action::{build_action_mask, DNA_DIGIVOLVE_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, make_test_dna_card, DebugRunner,
};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};

fn colored_digimon_level(id: &str, color: CardColor, level: u8) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(level),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        dual: None,
        digixros_aliases: Vec::new(),
        ace_overflow: None,
    }
}

#[test]
fn digixros_matching_accepts_scoped_alias_but_generic_name_checks_do_not() {
    let mut omni = make_test_card_with_level("BT21-021", "OmniShoutmon", 5);
    omni.digixros_aliases = vec!["Shoutmon".to_string()];

    let runner = DebugRunner::builder()
        .add_card(omni)
        .add_card(make_test_card_with_level("BT10-009", "Shoutmon X4", 4))
        .hand(0, &["BT21-021", "BT10-009"])
        .build();

    assert!(runner
        .game
        .card_can_satisfy_digixros_name("BT21-021", "Shoutmon"));
    assert!(!runner
        .game
        .card_matches_generic_name("BT21-021", "Shoutmon"));
}

#[test]
fn digixros_matching_helper_uses_production_name_match_semantics() {
    let runner = DebugRunner::builder()
        .add_card(make_test_card_with_level("BT10-009", "Shoutmon X4", 4))
        .hand(0, &["BT10-009"])
        .build();

    assert!(runner
        .game
        .card_can_satisfy_digixros_name("BT10-009", "Shoutmon"));
}

#[test]
fn user_action_dna_digivolve_two_stage_resolution_merges_permanents() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card_with_level("TST-LV5", "FiveDigi", 5))
        .add_card(make_test_card_with_level("TST-LV6", "SixDigi", 6))
        .add_card(make_test_dna_card("TST-DNA", "DnaDigi", 5, 6, 0))
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
        .add_card(make_test_card_with_level("TST-LV5", "FiveDigi", 5))
        .add_card(make_test_card_with_level("TST-LV6", "SixDigi", 6))
        .add_card(make_test_dna_card("TST-DNA-3", "DnaCost3", 5, 6, 3))
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
        .add_card(make_test_card_with_level("TST-LV5", "FiveDigi", 5))
        .add_card(make_test_card_with_level("TST-LV6", "SixDigi", 6))
        .add_card(make_test_dna_card("TST-DNA", "DnaDigi", 5, 6, 0))
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
        .add_card(make_test_dna_card("TST-DNA", "DnaDigi", 5, 6, 0))
        .hand(0, &["TST-DNA"])
        .start();
    // Force a non-Main phase. (`Battle` is not a GamePhase variant; use
    // `EndTurn` to cover the "not Main" branch.)
    runner.game.current_phase = GamePhase::EndTurn;

    let ok = runner.game.initiate_dna_digivolve(0, 0);
    assert!(!ok, "non-Main phase must reject");
    assert!(runner.game.pending_selection.is_none());
}

#[test]
fn authored_dna_alt_path_makes_dna_action_legal_for_bt20_016() {
    let yaml = r#"
card: BT20-016
name: Paildramon
kind: digimon
level: 5
color: [red, purple]
cost: 8
dp: 8000
traits: [Dragonkin]
alt_paths:
  - kind: dna_digivolve
    materials:
      - { level_eq: 4, color: red }
      - { level_eq: 4, color: purple }
    cost: 0
effects: []
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DNA alt-path DSL compiles")
        .add_card(colored_digimon_level("RED-LV4", CardColor::Red, 4))
        .add_card(colored_digimon_level("PURPLE-LV4", CardColor::Purple, 4))
        .hand(0, &["BT20-016"])
        .memory(3)
        .start();

    runner.game.current_phase = GamePhase::Main;
    runner.place_on_field(0, "RED-LV4", Some(0));
    runner.place_on_field(0, "PURPLE-LV4", Some(0));

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[DNA_DIGIVOLVE_START as usize], 1.0,
        "BT20-016's authored DNA alt_path must populate CardData.dna_costs for the user action mask",
    );
}
