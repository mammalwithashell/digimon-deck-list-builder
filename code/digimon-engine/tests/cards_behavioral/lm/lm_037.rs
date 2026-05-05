//! LM-037 Sepia Memory Boost! - Option, Cost 3, Black.
//!
//! Printed text:
//! Yellow also meets this card's color requirements.
//! [Main] Reveal the top 3 cards of your deck. Add 1 black or yellow Digimon
//! card among them to the hand. Return the rest to the bottom of deck. Then,
//! place this card in the battle area.
//! [Main] <Delay> Gain 2 memory.
//!
//! Inherited: Security Effect [Security] Place this card in the battle area.
//!
//! Security placement is covered by the native `place_self_as_delay_option`
//! step used by the inherited on-security clause.

#![allow(dead_code)]

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

fn lm_037_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/lm/LM-037.yaml"))
        .expect("LM-037 YAML must exist at cards/lm/LM-037.yaml")
}

fn lm_037_runner() -> DebugRunner {
    let yaml = lm_037_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-037 YAML must parse and compile")
        .memory(10)
        .start()
}

fn make_digimon(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card
}

fn make_tamer(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card
}

fn make_option(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![color];
    card
}

fn black_digimon(id: &str) -> CardData {
    make_digimon(id, CardColor::Black)
}

fn yellow_digimon(id: &str) -> CardData {
    make_digimon(id, CardColor::Yellow)
}

fn red_digimon(id: &str) -> CardData {
    make_digimon(id, CardColor::Red)
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

#[test]
fn lm_037_yaml_parses_without_error() {
    let _runner = lm_037_runner();
}

#[test]
fn lm_037_is_black_option_cost_3() {
    let runner = lm_037_runner();
    let compiled = runner
        .compiled_card("LM-037")
        .expect("LM-037 must be registered");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Black]);
    assert_eq!(compiled.cost, Some(3));
}

#[test]
fn lm_037_use_requirement_allows_yellow_to_meet_option_color_requirement() {
    let runner = lm_037_runner();
    let compiled = runner
        .compiled_card("LM-037")
        .expect("LM-037 must be registered");

    let use_requirement = compiled
        .use_requirement
        .as_ref()
        .expect("yellow color-requirement clause must be represented");
    let field_req = use_requirement
        .any_field_permanent
        .as_ref()
        .expect("use_requirement should scan your field");

    assert_eq!(field_req.of, digimon_dsl::compiled::CompiledPlayerRef::You);
    assert_eq!(
        field_req.predicate.color_is,
        Some(CompiledColor::Yellow),
        "yellow permanents must satisfy LM-037 option use requirements"
    );
    assert!(
        field_req
            .predicate
            .any_of
            .iter()
            .any(|p| p.kind == Some(CompiledCardKind::Digimon)),
        "yellow Digimon must satisfy the alternate color requirement"
    );
    assert!(
        field_req
            .predicate
            .any_of
            .iter()
            .any(|p| p.kind == Some(CompiledCardKind::Tamer)),
        "yellow Tamers must satisfy the alternate color requirement"
    );
}

#[test]
fn lm_037_action_mask_allows_use_with_yellow_tamer_but_not_unrelated_color() {
    let yaml = lm_037_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-037 YAML parses")
        .add_card(make_tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(make_tamer("RED-TAMER", CardColor::Red))
        .add_card(filler("FILL"))
        .hand(0, &["LM-037"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "LM-037 should not be usable with no black or yellow Digimon/Tamer"
    );

    runner.place_on_field(0, "RED-TAMER", Some(0));
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "red permanents must not satisfy LM-037 color requirements"
    );

    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        1.0,
        "yellow permanents must satisfy LM-037 color requirements"
    );
    assert_ne!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "execution legality must match the action mask"
    );
}

#[test]
fn lm_037_has_main_delay_and_inherited_security_clauses() {
    let runner = lm_037_runner();
    let compiled = runner
        .compiled_card("LM-037")
        .expect("LM-037 must be registered");

    assert_eq!(
        compiled.effects.len(),
        3,
        "LM-037 should have Main, Delay, and inherited Security clauses"
    );

    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => {
            assert!(t.when.contains(&CompiledTiming::MainFromHand));
            assert_eq!(t.scope, CompiledScope::FaceUp);
            assert!(!t.optional, "[Main] search is mandatory");
        }
        other => panic!("clause 0 must be main_from_hand triggered; got {other:?}"),
    }

    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
            trigger, process, ..
        }) => {
            assert_eq!(*trigger, CompiledTiming::EndOfYourNextTurn);
            assert!(
                process
                    .iter()
                    .any(|s| matches!(s, CompiledStep::GainMemory(2))),
                "Delay process must gain 2 memory; got {process:?}"
            );
        }
        other => panic!("clause 1 must be a standard Delay; got {other:?}"),
    }

    match &compiled.effects[2] {
        CompiledClause::Triggered(t) => {
            assert!(t.when.contains(&CompiledTiming::OnSecurity));
            assert_eq!(t.scope, CompiledScope::Inherited);
            assert_eq!(
                t.process,
                vec![CompiledStep::PlaceSelfAsDelayOption],
                "inherited Security placement must use the native delay-option placement step"
            );
        }
        other => panic!("clause 2 must be inherited on_security; got {other:?}"),
    }
}

#[test]
fn lm_037_main_reveals_top_3_and_filters_black_or_yellow_digimon() {
    let runner = lm_037_runner();
    let compiled = runner
        .compiled_card("LM-037")
        .expect("LM-037 must be registered");

    let main = match &compiled.effects[0] {
        CompiledClause::Triggered(t) => t,
        other => panic!("clause 0 must be triggered; got {other:?}"),
    };

    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. })),
        "Main process must reveal top 3 cards; got {:?}",
        main.process
    );

    let select_filter = main
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::SelectReveal {
                filter, optional, ..
            } => {
                assert!(
                    !*optional,
                    "printed text says add 1 matching card; PASS must not be legal when a candidate exists"
                );
                Some(filter)
            }
            _ => None,
        })
        .expect("Main process must select from reveal");

    assert_eq!(select_filter.kind, Some(CompiledCardKind::Digimon));
    assert!(
        select_filter
            .any_of
            .iter()
            .any(|p| p.color_is == Some(CompiledColor::Black)),
        "select_reveal must allow black Digimon"
    );
    assert!(
        select_filter
            .any_of
            .iter()
            .any(|p| p.color_is == Some(CompiledColor::Yellow)),
        "select_reveal must allow yellow Digimon"
    );
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddToHandFromReveal { .. })),
        "Main process must add the selected revealed card to hand"
    );
    assert!(
        main.process.iter().any(|s| matches!(
            s,
            CompiledStep::PlaceRemainderOnDeck {
                position: CompiledStackPosition::Bottom,
                ..
            }
        )),
        "Main process must return the rest to deck bottom"
    );
}

#[test]
fn lm_037_main_adds_yellow_digimon_from_top_3_to_hand() {
    let yaml = lm_037_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-037 YAML parses")
        .add_card(yellow_digimon("YELLOW-DIGI"))
        .add_card(black_digimon("BLACK-DIGI"))
        .add_card(red_digimon("RED-DIGI"))
        .add_card(make_option("BLACK-OPT", CardColor::Black))
        .add_card(filler("FILL"))
        .hand(0, &["LM-037"])
        .deck(
            0,
            &[
                "FILL",
                "FILL",
                "FILL",
                "BLACK-OPT",
                "RED-DIGI",
                "BLACK-DIGI",
                "YELLOW-DIGI",
            ],
        )
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.deck_size(0);

    assert!(runner.game.activate_hand_main(0, 0));
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "LM-037 Main should add exactly one eligible Digimon from reveal"
    );
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "YELLOW-DIGI"),
        "the isolated eligible yellow Digimon should be selected and added to hand"
    );
    assert_eq!(
        deck_before - runner.deck_size(0),
        1,
        "Reveal 3, add 1, and return 2 should shrink deck by exactly 1"
    );
}

#[test]
fn lm_037_security_check_places_self_in_battle_area_as_delay_option() {
    let yaml = lm_037_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-037 YAML parses")
        .add_card(red_digimon("ATTACKER"))
        .add_card(filler("FILL"))
        .security(1, &["LM-037"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0, "LM-037 should leave security");
    assert_eq!(
        runner.trash_size(1),
        0,
        "LM-037 should be placed in battle area instead of trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "LM-037")
        .expect("LM-037 should be placed as a battle-area Option permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::EndOfYourNextTurn,
            ..
        }
    ));
}

#[test]
fn lm_037_delay_activation_gains_2_memory_when_played_as_delay_option() {
    let yaml = lm_037_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-037 YAML parses")
        .add_card(make_tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(red_digimon("FILL"))
        .hand(0, &["LM-037"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    runner.game.enter_main_phase();
    let start_turn = runner.game.turn_count;

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Trashed
    );
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|permanent| matches!(
                permanent.option_state,
                OptionState::Delayed {
                    trigger: DelayTrigger::EndOfYourNextTurn,
                    ..
                }
            )),
        "playing LM-037 should park it as a delayed option"
    );

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_count, start_turn + 2);
    assert_eq!(runner.game.turn_player(), 0);

    runner.game.enter_main_phase();
    runner.game.set_memory(0);
    runner.end_turn();

    let expected_memory = if runner.game.turn_player() == 0 {
        2
    } else {
        -2
    };
    assert_eq!(
        runner.memory(),
        expected_memory,
        "LM-037 Delay body should give its owner 2 memory at end of owner's next turn"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|permanent| matches!(permanent.option_state, OptionState::Delayed { .. })),
        "delayed LM-037 should be trashed after the Delay body resolves"
    );
}
