//! ST21-14 Believe in Our Friendship — Option, Purple, Cost 3, traits: [ADVENTURE].
//!
//! # Card text (official bundle `data/card_bundles/ST21-14.md`)
//!
//! ```text
//! While you have a Digimon or Tamer with the [ADVENTURE] trait on the field,
//! you can ignore this card's color requirements.
//! [Main] Reveal the top 3 cards of your deck. Add 1 card with the [ADVENTURE]
//! trait among them to the hand. Return the rest to the bottom of the deck.
//! Then, place this card in the battle area.
//! [Main] <Delay> (By trashing this card after the placing turn, activate the
//! effect.) ・Gain 2 memory.
//!
//! Security Effect: [Security] Place this card in the battle area.
//! ```
//! Official Q&A: "on the field" = the battle area or the breeding area.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Purple/ST21_14.cs` —
//! `IgnoreColorConditionClass(self)` while an own ADVENTURE Digimon/Tamer
//! exists; OptionSkill `RevealDeckTopCardsAndSelect(3, [ADVENTURE AddHand max
//! 1, canNoSelect false], DeckBottom)` then `PlaceDelayOptionCards`;
//! OnDeclaration Delay gain 2 memory; `PlaceSelfDelayOptionSecurityEffect`.
//! Same shape as BT22-099 Kuremi Detective Agency.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

const CARD_ID: &str = "ST21-14";

fn card(id: &str, kind: CardKind, color: CardColor, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![color];
    if kind != CardKind::Digimon {
        c.level = None;
        c.dp = None;
    }
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-14 YAML parses, compiles and is in the embedded pack")
        .add_card(card("ADV-RED-DIGI", CardKind::Digimon, CardColor::Red, true))
        .add_card(card("ADV-RED-TAMER", CardKind::Tamer, CardColor::Red, true))
        .add_card(card("ADV-OPPONENT", CardKind::Digimon, CardColor::Red, true))
        .add_card(card("PLAIN-RED", CardKind::Digimon, CardColor::Red, false))
        .add_card(card("PURPLE", CardKind::Digimon, CardColor::Purple, false))
        .add_card(card("ADV-OPTION", CardKind::Option, CardColor::Blue, true))
        .add_card(make_test_card("FILL", "Fill"))
}

fn stack_deck_top(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered"));
        let card_index = runner.game.next_card_index();
        runner.game.players[0].deck.push(CardSource::new(data_idx, 0, card_index));
    }
}

fn ids(runner: &DebugRunner, zone: &[CardSource]) -> Vec<String> {
    zone.iter().map(|c| c.card_id(&runner.game.card_data).to_string()).collect()
}

fn ready(board: &[&str], opp_board: &[&str]) -> DebugRunner {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    for b in board {
        runner.place_on_field(0, b, Some(0));
    }
    for b in opp_board {
        runner.place_on_field(1, b, Some(0));
    }
    runner.game.enter_main_phase();
    runner
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st21_14_clause_shape() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert!(card.traits.iter().any(|t| t == "ADVENTURE"));
    assert_eq!(card.effects.len(), 3, "[Main], <Delay>, [Security]");
    assert!(card.use_requirement.is_some(), "colour bypass is the use_requirement");
    assert!(!card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { .. })
    )));
    match &card.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, process, .. }) => {
            assert_eq!(*trigger, CompiledTiming::Delayed, "[Main] <Delay> is player-activated");
            assert!(process.iter().any(|s| matches!(s, CompiledStep::GainMemory(2))));
        }
        other => panic!("clause 1 must be the Delay; got {other:?}"),
    }
    match &card.effects[2] {
        CompiledClause::Triggered(t) => {
            assert_eq!(t.scope, CompiledScope::Inherited);
            assert!(t.when.contains(&CompiledTiming::OnSecurity));
            assert!(!t.optional);
            assert!(t.process.iter().any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)));
        }
        other => panic!("clause 2 must be the [Security] clause; got {other:?}"),
    }
}

// ─── Section 2 — colour requirement bypass ───────────────────────────────────

#[test]
fn st21_14_blocked_without_purple_or_adventure() {
    let mut runner = ready(&["PLAIN-RED"], &[]);
    assert_eq!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
    assert_eq!(ids(&runner, &runner.game.players[0].hand), vec![CARD_ID]);
}

#[test]
fn st21_14_opponents_adventure_digimon_does_not_count() {
    let mut runner = ready(&["PLAIN-RED"], &["ADV-OPPONENT"]);
    assert_eq!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
}

#[test]
fn st21_14_adventure_digimon_bypasses_color() {
    let mut runner = ready(&["ADV-RED-DIGI"], &[]);
    stack_deck_top(&mut runner, &["FILL", "FILL", "FILL"]);
    assert_ne!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
}

#[test]
fn st21_14_adventure_tamer_bypasses_color() {
    let mut runner = ready(&["ADV-RED-TAMER"], &[]);
    stack_deck_top(&mut runner, &["FILL", "FILL", "FILL"]);
    assert_ne!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
}

#[test]
fn st21_14_adventure_digimon_in_breeding_bypasses_color() {
    let mut runner = ready(&[], &[]);
    runner.place_in_breeding(0, "ADV-RED-DIGI");
    stack_deck_top(&mut runner, &["FILL", "FILL", "FILL"]);
    assert_ne!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "Official Q&A: the breeding area is 'on the field'"
    );
}

#[test]
fn st21_14_purple_source_meets_color_normally() {
    let mut runner = ready(&["PURPLE"], &[]);
    stack_deck_top(&mut runner, &["FILL", "FILL", "FILL"]);
    assert_ne!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
}

// ─── Section 3 — [Main] reveal + place ───────────────────────────────────────

#[test]
fn st21_14_main_adds_one_adventure_card_bottoms_rest_and_parks_as_delay() {
    let mut runner = ready(&["ADV-RED-TAMER"], &[]);
    stack_deck_top(&mut runner, &["PLAIN-RED", "ADV-OPTION", "ADV-RED-DIGI"]);
    assert_eq!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let view = runner.pending_selection_view().expect("reveal pick");
    let action_for = |r: &DebugRunner, id: &str| {
        r.game
            .revealed_cards
            .iter()
            .enumerate()
            .find_map(|(i, c)| (c.card_id(&r.game.card_data) == id).then_some(SEL_REVEAL_START + i as u16))
            .unwrap()
    };
    assert!(view.valid_action_ids.contains(&action_for(&runner, "ADV-OPTION")));
    assert!(!view.valid_action_ids.contains(&action_for(&runner, "PLAIN-RED")));
    assert_eq!(view.valid_action_ids.len(), 2, "two [ADVENTURE] cards revealed, no PASS");
    let pick = action_for(&runner, "ADV-OPTION");
    runner.execute_action(view.selecting_player, pick).expect("add");
    let _ = runner.auto_resolve();
    assert_eq!(ids(&runner, &runner.game.players[0].hand), vec!["ADV-OPTION"]);
    let deck = ids(&runner, &runner.game.players[0].deck);
    assert!(deck[..2].contains(&"PLAIN-RED".to_string()) && deck[..2].contains(&"ADV-RED-DIGI".to_string()));
    let placed = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("placed in the battle area");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed { trigger: DelayTrigger::MainPhaseActivated, .. }
    ));
}

// ─── Section 4 — <Delay> gain 2 memory ───────────────────────────────────────

fn seat_delay(runner: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    let placing_turn = runner.game.turn_count;
    runner.game.player_mut(0).battle_area[handle.index as usize].option_state = OptionState::Delayed {
        owner: 0,
        trash_on_turn: u16::MAX,
        trigger: DelayTrigger::MainPhaseActivated,
        placed_on_turn: placing_turn,
    };
    handle
}

#[test]
fn st21_14_delay_after_placing_turn_gains_two_memory() {
    let mut runner = builder()
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(0)
        .start();
    let handle = seat_delay(&mut runner);
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(3);
    assert!(runner.game.activate_field_main(0, handle.index as usize));
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 5, "gain 2 memory");
    assert!(ids(&runner, &runner.game.players[0].trash).contains(&CARD_ID.to_string()));
}

#[test]
fn st21_14_delay_not_activatable_on_placing_turn() {
    let mut runner = builder().deck(0, &["FILL"; 6]).deck(1, &["FILL"; 6]).memory(3).start();
    let handle = seat_delay(&mut runner);
    runner.game.enter_main_phase();
    assert!(!runner.game.activate_field_main(0, handle.index as usize));
    assert_eq!(runner.memory(), 3);
}
