//! P-037 Yellow Memory Boost! - Option, Cost 3, Yellow.
//!
//! # Card text (cards.json / printed)
//!
//! [Main] Reveal the top 4 cards of your deck. Add 1 yellow Digimon card among
//! them to the hand. Place the remaining cards at the bottom of your deck in any
//! order. Then, place this card in your battle area.
//!
//! [Main] <Delay> (By trashing this card after the placing turn, activate the
//! effect below.)
//! Gain 2 memory.
//!
//! Inherited: Security Effect [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Yellow/P_037.cs
//!
//! # Patterns this test covers
//! - A1 Reveal top-N + select-by-color/kind + add to hand
//! - A2 place_remainder_on_deck(bottom) with ordered remainder placement
//! - Standard <Delay> (PUPPETS-G009): gain_memory via player-visible
//!   [Main]-phase activation action
//! - Inherited security placement through place_self_as_delay_option

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledPredicate,
    CompiledScope, CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

const YAML: &str = include_str!("../../../cards/p/P-037.yaml");

fn yellow_digimon(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card
}

fn blue_digimon(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card
}

fn filler(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

fn predicate_has_kind_and_yellow(predicate: &CompiledPredicate) -> bool {
    let has_kind = predicate.kind == Some(CompiledCardKind::Digimon)
        || predicate
            .all_of
            .iter()
            .any(|part| part.kind == Some(CompiledCardKind::Digimon));
    let has_yellow = predicate.color_is == Some(CompiledColor::Yellow)
        || predicate
            .all_of
            .iter()
            .any(|part| part.color_is == Some(CompiledColor::Yellow));

    has_kind && has_yellow
}

#[test]
fn p_037_yaml_parses_and_compiles() {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-037 YAML must parse and compile");
}

#[test]
fn p_037_is_yellow_option_cost_3() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-037"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-037").expect("P-037 compiled");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(3));
}

#[test]
fn p_037_has_main_delay_and_inherited_security_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-037"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-037").expect("P-037 compiled");
    assert_eq!(compiled.effects.len(), 3);

    match &compiled.effects[0] {
        CompiledClause::Triggered(triggered) => {
            assert!(triggered.when.contains(&CompiledTiming::MainFromHand));
            assert_eq!(triggered.scope, CompiledScope::FaceUp);
            assert!(!triggered.optional);
        }
        other => panic!("clause 0 must be main_from_hand; got {other:?}"),
    }

    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. }) => {
            assert_eq!(*trigger, CompiledTiming::Delayed);
        }
        other => panic!("clause 1 must be delay; got {other:?}"),
    }

    match &compiled.effects[2] {
        CompiledClause::Triggered(triggered) => {
            assert_eq!(triggered.scope, CompiledScope::Inherited);
            assert!(triggered.when.contains(&CompiledTiming::OnSecurity));
        }
        other => panic!("clause 2 must be inherited on_security; got {other:?}"),
    }
}

#[test]
fn p_037_main_clause_reveals_four_selects_yellow_digimon_and_bottoms_remainder() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-037"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-037").expect("P-037 compiled");
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("clause 0 must be triggered");
    };

    assert!(matches!(
        triggered.process.first(),
        Some(CompiledStep::RevealTopDeck { count: 4, .. })
    ));

    let select_reveal = triggered
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectReveal {
                filter, optional, ..
            } => Some((filter, optional)),
            _ => None,
        })
        .expect("main process must select from reveal");
    assert!(
        !*select_reveal.1,
        "printed text says add 1 matching card; PASS must not be legal when a candidate exists"
    );
    assert!(
        predicate_has_kind_and_yellow(select_reveal.0),
        "select_reveal must filter to yellow Digimon"
    );

    assert!(triggered
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::AddToHandFromReveal { .. })));
    assert!(triggered.process.iter().any(|step| {
        matches!(
            step,
            CompiledStep::PlaceRemainderOnDeck {
                position: CompiledStackPosition::Bottom,
                ..
            }
        )
    }));
}

#[test]
fn p_037_main_adds_selected_yellow_digimon_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(yellow_digimon("YELLOW-D1"))
        .add_card(blue_digimon("BLUE-D1"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        .add_card(filler("FILL-C"))
        .hand(0, &["P-037"])
        .deck(
            0,
            &[
                "FILL-A",
                "FILL-B",
                "FILL-C",
                "BLUE-D1",
                "FILL-A",
                "FILL-B",
                "FILL-C",
                "YELLOW-D1",
            ],
        )
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .auto_resolve()
        .expect("resolve reveal and bottom ordering");

    let hand_ids: Vec<_> = runner.game.players[0]
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.iter().any(|id| id == "YELLOW-D1"),
        "selected yellow Digimon must be added to hand; hand={hand_ids:?}"
    );
    assert_eq!(runner.hand_size(0), hand_before + 1);
    assert_eq!(
        deck_before - runner.deck_size(0),
        1,
        "one revealed card moves to hand and the other three return to deck"
    );
}

#[test]
fn p_037_delay_clause_gains_two_memory() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-037"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-037").expect("P-037 compiled");
    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { process, .. }) => {
            assert!(
                process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::GainMemory(2))),
                "Delay process must contain gain_memory: 2; got {process:?}"
            );
        }
        other => panic!("clause 1 must be delay; got {other:?}"),
    }
}

#[test]
fn p_037_inherited_security_places_self_as_delay_option_structurally() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-037"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-037").expect("P-037 compiled");
    match &compiled.effects[2] {
        CompiledClause::Triggered(triggered) => {
            assert_eq!(triggered.scope, CompiledScope::Inherited);
            assert!(triggered.when.contains(&CompiledTiming::OnSecurity));
            assert_eq!(
                triggered.process,
                vec![CompiledStep::PlaceSelfAsDelayOption],
                "security placement must use the audited DSL placement primitive"
            );
        }
        other => panic!("clause 2 must be inherited on_security; got {other:?}"),
    }
}

#[test]
fn p_037_security_check_places_self_in_battle_area_as_delay_option() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(yellow_digimon("ATTACKER"))
        .add_card(filler("FILL"))
        .security(1, &["P-037"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0, "P-037 should leave security");
    assert_eq!(
        runner.trash_size(1),
        0,
        "P-037 should be placed in battle area instead of trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "P-037")
        .expect("P-037 should be placed as a battle-area Option permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

/// PUPPETS-G009 — P-037's standard `<Delay>` is a player-visible
/// `[Main]`-phase activation. Playing it parks it as a `MainPhaseActivated`
/// delayed Option; on a later main phase the controller takes the
/// `FIELD_EFFECT` activation, which trashes the Option as the cost and runs
/// the `<Delay>` body (gain 2 memory).
#[test]
fn p_037_delay_activation_gains_2_memory_via_main_phase_action() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(yellow_digimon("YELLOW-DIGI"))
        .add_card(filler("FILL"))
        .hand(0, &["P-037"])
        .deck(0, &["FILL", "FILL", "FILL", "YELLOW-DIGI", "FILL", "FILL"])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "YELLOW-DIGI", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve P-037 reveal selection and delay placement");

    let delay_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|permanent| {
            matches!(
                permanent.option_state,
                OptionState::Delayed {
                    trigger: DelayTrigger::MainPhaseActivated,
                    ..
                }
            )
        })
        .expect("playing P-037 should park it as a MainPhaseActivated delayed option");

    // Same turn it was placed: the Delay activation is NOT legal (16-16-3).
    let bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        0.0,
        "P-037 <Delay> must not be activatable on the placing turn"
    );

    // Advance to the controller's next main phase.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(0);

    // The Delay activation is now a legal [Main]-phase action; PASS too.
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[bit], 1.0, "P-037 <Delay> activation is a legal action");
    assert_eq!(mask[PASS as usize], 1.0, "declining stays legal");

    // Take the activation: trash P-037 as cost, run the body.
    runner.game.decode_action(bit as u16, 0);

    assert_eq!(
        runner.memory(),
        2,
        "P-037 <Delay> body gains 2 memory when the player activates it"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|permanent| matches!(permanent.option_state, OptionState::Delayed { .. })),
        "P-037 is trashed as the <Delay> activation cost"
    );
}
