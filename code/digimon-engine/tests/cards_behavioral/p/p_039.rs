//! P-039 Black Memory Boost! — Option, Black, Cost 3.
//!
//! Printed card text:
//!   [Main] Reveal the top 4 cards of your deck. Add 1 black Digimon card
//!   among them to the hand. Place the remaining cards at the bottom of your
//!   deck in any order. Then, place this card in your battle area.
//!   [Main] <Delay> (By trashing this card after the placing turn, activate
//!   the effect below.) · Gain 2 memory.
//!   [Security] Place this card in the battle area.
//!
//! # Patterns covered
//! - reveal_top_deck + select_reveal (black Digimon) + add_to_hand_from_reveal
//! - place_remainder_on_deck (bottom)
//! - Standard <Delay>: engine auto-parks as MainPhaseActivated via kind: delay clause
//! - Inherited security placement: place_self_as_delay_option (G-PLACE-SELF-AS-OPTION-PERMANENT resolved 2026-05-02)

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

fn black_digimon(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card
}

fn red_digimon(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card
}

fn filler(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

#[test]
fn p_039_has_main_from_hand_and_delay_gain_two() {
    let runner = DebugRunner::builder()
        .dsl_card("P-039")
        .expect("P-039 YAML parses and compiles")
        .build();
    let card = runner.compiled_card("P-039").expect("P-039 compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::MainFromHand)
    )));

    let delay = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { process, .. }) => {
            Some(process)
        }
        _ => None,
    });

    let process = delay.expect("P-039 must compile a Delay clause");
    assert!(
        process
            .iter()
            .any(|step| matches!(step, CompiledStep::GainMemory(2))),
        "P-039 Delay must gain 2 memory"
    );
}

#[test]
fn p_039_is_black_option_cost_3() {
    let runner = DebugRunner::builder()
        .dsl_card("P-039")
        .expect("parses")
        .build();
    let compiled = runner.compiled_card("P-039").expect("P-039 compiled");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Black]);
    assert_eq!(compiled.cost, Some(3));
}

#[test]
fn p_039_has_three_clauses_main_delay_inherited_security() {
    let runner = DebugRunner::builder()
        .dsl_card("P-039")
        .expect("parses")
        .build();
    let compiled = runner.compiled_card("P-039").expect("P-039 compiled");
    assert_eq!(
        compiled.effects.len(),
        3,
        "P-039 must have exactly 3 clauses"
    );

    // Clause 0: main_from_hand
    match &compiled.effects[0] {
        CompiledClause::Triggered(triggered) => {
            assert!(triggered.when.contains(&CompiledTiming::MainFromHand));
            assert_eq!(triggered.scope, CompiledScope::FaceUp);
        }
        other => panic!("clause 0 must be main_from_hand Triggered; got {other:?}"),
    }

    // Clause 1: Delay
    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { .. }) => {}
        other => panic!("clause 1 must be Delay; got {other:?}"),
    }

    // Clause 2: inherited on_security
    match &compiled.effects[2] {
        CompiledClause::Triggered(triggered) => {
            assert_eq!(triggered.scope, CompiledScope::Inherited);
            assert!(triggered.when.contains(&CompiledTiming::OnSecurity));
        }
        other => panic!("clause 2 must be inherited on_security; got {other:?}"),
    }
}

/// TDD: this test FAILS until the security `process:` body is authored.
/// The security clause must contain exactly `place_self_as_delay_option`.
#[test]
fn p_039_inherited_security_clause_places_self_as_delay_option_structurally() {
    let runner = DebugRunner::builder()
        .dsl_card("P-039")
        .expect("parses")
        .build();
    let compiled = runner.compiled_card("P-039").expect("P-039 compiled");
    match &compiled.effects[2] {
        CompiledClause::Triggered(triggered) => {
            assert_eq!(triggered.scope, CompiledScope::Inherited);
            assert!(triggered.when.contains(&CompiledTiming::OnSecurity));
            assert_eq!(
                triggered.process,
                vec![CompiledStep::PlaceSelfAsDelayOption],
                "security clause process must be [place_self_as_delay_option]; got {:?}",
                triggered.process
            );
        }
        other => panic!("clause 2 must be inherited on_security; got {other:?}"),
    }
}

/// TDD: this test FAILS until the security `process:` body is authored.
/// When P-039 is hit as a security card, it must be placed in the controller's
/// battle area as a Delay-Option permanent (not trashed).
#[test]
fn p_039_security_check_places_self_in_battle_area() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-039")
        .expect("parses")
        .add_card(red_digimon("ATTACKER"))
        .add_card(filler("FILL"))
        .security(1, &["P-039"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0, "P-039 must leave security");
    assert_eq!(
        runner.trash_size(1),
        0,
        "P-039 must not be trashed; it goes to the battle area"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "P-039")
        .expect("P-039 must be placed as a battle-area Option permanent");
    assert!(
        matches!(
            placed.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::MainPhaseActivated,
                ..
            }
        ),
        "P-039 in battle area must be Delayed(MainPhaseActivated); got {:?}",
        placed.option_state
    );
}

/// P-039 main play: reveal top 4, add 1 black Digimon to hand, bottom the rest.
/// The engine auto-parks the card as Delayed(MainPhaseActivated) because the
/// compiled card has a `kind: delay` clause — no explicit place_self step needed.
/// A black Digimon on the field is required to satisfy the color requirement.
#[test]
fn p_039_main_play_adds_black_digimon_to_hand_and_parks_as_delay() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-039")
        .expect("parses")
        .add_card(black_digimon("BLACK-FIELD"))
        .add_card(black_digimon("BLACK-D1"))
        .add_card(red_digimon("RED-D1"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        .add_card(filler("FILL-C"))
        .hand(0, &["P-039"])
        .deck(
            0,
            &[
                "FILL-A", "FILL-B", "FILL-C", "RED-D1", "FILL-A", "FILL-B", "FILL-C", "BLACK-D1",
            ],
        )
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    // Place a black Digimon on the field to satisfy the black color requirement.
    runner.place_on_field(0, "BLACK-FIELD", Some(0));
    runner.game.enter_main_phase();

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve reveal selection and deck ordering");

    // The selected black Digimon goes to hand.
    let hand_ids: Vec<_> = runner.game.players[0]
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.iter().any(|id| id == "BLACK-D1"),
        "selected black Digimon must be in hand; hand={hand_ids:?}"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "net 0 change to hand: P-039 consumed by play_option_from_hand, BLACK-D1 added from reveal"
    );
    assert_eq!(
        deck_before - runner.deck_size(0),
        1,
        "one card leaves deck (added to hand); rest return to bottom"
    );

    // P-039 is parked as a MainPhaseActivated delayed Option by the engine.
    assert!(
        runner.game.player(0).battle_area.iter().any(|p| matches!(
            p.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::MainPhaseActivated,
                ..
            }
        )),
        "P-039 must be parked in battle area as Delayed(MainPhaseActivated) after play"
    );
}
