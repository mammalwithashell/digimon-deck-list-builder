//! BT7-071 Loweemon — Digimon, Lv.4, Purple, DP 5000, Cost 5.
//! Traits: Hybrid / Variable / Warrior.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT7-071.md`;
//! card image confirms)
//!
//! You may digivolve this card from your hand onto one of your purple Tamers
//! as if the Tamer is a level 3 purple Digimon.
//!
//! Official Q&A: "Yes, it also treats the Tamer as a Digimon when it
//! digivolves. It treats it as a Digimon that digivolves, therefore 'when a
//! Digimon would digivolve' effects and 'when a Digimon digivolves' effects
//! will trigger. In addition, if a 'Digimon can't digivolve' effect
//! activates, digivolution won't be possible using this effect."
//!
//! Digivolve: Purple Lv.3 / cost 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT7/Purple/BT7_071.cs —
//! `AddSelfDigivolutionRequirementStaticEffect(permanentCondition: TopCard is
//! Purple && IsTamer, digivolutionCost: 2, ignoreDigivolutionRequirement:
//! false, condition: card in owner's hand)`. The "as if Lv.3 purple Digimon"
//! reading means the Tamer satisfies the card's own Purple Lv.3 / cost 2
//! circle — the alt-path cost is that printed 2.
//!
//! # Patterns this test covers
//! - Hybrid Tamer digivolve: `alt_paths` `from: { kind: tamer, color_is:
//!   purple }` + `source_treated_as: level_3_purple_digimon`
//!   (`tests/dsl/hybrid_tamer_digivolve.rs` idiom, AD1-002 sister).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathDirection, CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};

const CARD_ID: &str = "BT7-071";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn tamer(id: &str, name: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c.colors = vec![color];
    c
}

fn rookie(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(3);
    c.dp = Some(2000);
    c.colors = vec![color];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT7-071 YAML parses, compiles and is in the embedded pack")
        .add_card(tamer("TAMER-PU", "Purple Tamer", CardColor::Purple))
        .add_card(tamer("TAMER-RD", "Red Tamer", CardColor::Red))
        .add_card(rookie("ROOKIE-PU", CardColor::Purple))
        .add_card(rookie("ROOKIE-RD", CardColor::Red))
}

fn stack_ids(runner: &DebugRunner, player: u8, index: u8) -> Vec<String> {
    runner.game.players[player as usize].battle_area[index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn bt7_071_metadata_matches_printed_card() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.name, "Loweemon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.color, vec![CompiledColor::Purple]);
    for t in ["Hybrid", "Variable", "Warrior"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t} printed");
    }
    assert!(
        card.effects.is_empty(),
        "Loweemon prints no timed/inherited effect — only the Tamer digivolve route"
    );
}

#[test]
fn bt7_071_has_printed_circle_and_purple_tamer_hybrid_route() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.alt_paths.len(), 2, "Purple Lv.3 circle + purple-Tamer route");

    let circle = &card.alt_paths[0];
    assert_eq!(circle.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(circle.cost, Some(CompiledCost::Literal(2)));
    assert!(circle.source_treated_as.is_none());
    assert_eq!(circle.from.as_ref().and_then(|f| f.level_eq), Some(3));

    let hybrid = &card.alt_paths[1];
    assert_eq!(hybrid.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(hybrid.direction, CompiledAltPathDirection::From);
    assert_eq!(hybrid.cost, Some(CompiledCost::Literal(2)));
    assert_eq!(
        hybrid.source_treated_as.as_deref(),
        Some("level_3_purple_digimon"),
        "'as if the Tamer is a level 3 purple Digimon'"
    );
    assert!(!hybrid.ignore_requirements);
    let from = hybrid.from.as_ref().expect("Tamer gate");
    assert_eq!(from.kind, Some(CompiledCardKind::Tamer));
}

// ─── Section 2 / 3 — Behavioral ──────────────────────────────────────────────

#[test]
fn bt7_071_mask_exposes_purple_tamer_as_a_digivolve_base() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    let tamer = runner.place_on_field(0, "TAMER-PU", Some(0));
    runner.game.current_phase = GamePhase::Main;

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[encode_digivolve(0, tamer.index as u16) as usize],
        1.0,
        "the purple Tamer must be offered as a legal base for Loweemon"
    );
    assert_eq!(
        runner.game.card_data[runner.game.players[0].battle_area[tamer.index as usize]
            .top_card()
            .data_index]
            .card_kind,
        CardKind::Tamer,
        "legality must not mutate the printed Tamer kind"
    );
}

#[test]
fn bt7_071_digivolves_onto_purple_tamer_for_2() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["ROOKIE-RD"])
        .memory(5)
        .start();
    let tamer = runner.place_on_field(0, "TAMER-PU", Some(0));
    runner.game.current_phase = GamePhase::Main;
    let memory_before = runner.game.memory;

    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, tamer.index as usize, PlaySource::ByHand),
        "Loweemon digivolves from hand onto the purple Tamer"
    );
    let stack = stack_ids(&runner, 0, tamer.index);
    assert_eq!(stack.first().map(String::as_str), Some("TAMER-PU"), "Tamer is the bottom source");
    assert_eq!(stack.last().map(String::as_str), Some(CARD_ID), "Loweemon is the top card");
    assert_eq!(memory_before - runner.game.memory, 2, "memory cost of 2");
    assert_eq!(runner.hand_size(0), 1, "digivolving draws 1 (hand: Loweemon out, draw in)");
}

#[test]
fn bt7_071_cannot_digivolve_onto_a_red_tamer() {
    // NEGATIVE: "one of your PURPLE Tamers".
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    let tamer = runner.place_on_field(0, "TAMER-RD", Some(0));
    runner.game.current_phase = GamePhase::Main;

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[encode_digivolve(0, tamer.index as u16) as usize], 0.0);
    assert!(!runner
        .game
        .digivolve_from_hand(0, 0, tamer.index as usize, PlaySource::ByHand));
    assert_eq!(runner.hand_size(0), 1);
}

#[test]
fn bt7_071_cannot_digivolve_onto_opponents_purple_tamer() {
    // NEGATIVE: "one of YOUR purple Tamers".
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    let opp_tamer = runner.place_on_field(1, "TAMER-PU", Some(0));
    runner.game.current_phase = GamePhase::Main;

    assert!(!runner
        .game
        .digivolve_from_hand(0, 0, opp_tamer.index as usize, PlaySource::ByHand));
}

#[test]
fn bt7_071_standard_circle_still_digivolves_from_a_purple_lv3_digimon() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    let rookie = runner.place_on_field(0, "ROOKIE-PU", Some(0));
    runner.game.current_phase = GamePhase::Main;
    let memory_before = runner.game.memory;

    assert!(runner
        .game
        .digivolve_from_hand(0, 0, rookie.index as usize, PlaySource::ByHand));
    assert_eq!(
        stack_ids(&runner, 0, rookie.index).last().map(String::as_str),
        Some(CARD_ID)
    );
    assert_eq!(memory_before - runner.game.memory, 2);
}

#[test]
fn bt7_071_cannot_digivolve_from_a_red_lv3_digimon() {
    // NEGATIVE: the printed circle is Purple Lv.3 only.
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    let rookie = runner.place_on_field(0, "ROOKIE-RD", Some(0));
    runner.game.current_phase = GamePhase::Main;

    assert!(!runner
        .game
        .digivolve_from_hand(0, 0, rookie.index as usize, PlaySource::ByHand));
}
