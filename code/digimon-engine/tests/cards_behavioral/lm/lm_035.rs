//! LM-035 Amber Memory Boost! — Option, Cost 3, Yellow.
//!
//! Printed text:
//! - Purple also meets this card's color requirements.
//! - [Main] Reveal the top 3 cards of your deck. Add 1 yellow or purple
//!   Digimon card among them to the hand. Return the rest to the bottom of deck.
//!   Then, place this card in the battle area.
//! - [Main] <Delay> Gain 2 memory.
//! - Security Effect [Security] Place this card in the battle area.
//!
#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, PLAY_HAND_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

const YAML: &str = include_str!("../../../cards/lm/LM-035.yaml");

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card
}

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card
}

fn option(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![color];
    card
}

fn lm_035_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-035 YAML must parse and compile")
        .memory(10)
        .start()
}

#[test]
fn lm_035_yaml_parses_and_compiles() {
    let _runner = lm_035_runner();
}

#[test]
fn lm_035_is_yellow_option_cost_3_with_purple_use_requirement() {
    let runner = lm_035_runner();
    let compiled = runner.compiled_card("LM-035").expect("LM-035 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(3));
    assert!(
        compiled.use_requirement.is_some(),
        "Purple also meets this card's color requirements must compile as use_requirement"
    );
}

#[test]
fn lm_035_has_main_delay_and_inherited_security_clauses() {
    let runner = lm_035_runner();
    let compiled = runner.compiled_card("LM-035").expect("LM-035 compiled");

    assert_eq!(
        compiled.effects.len(),
        3,
        "LM-035 should have Main, Delay, and inherited Security clauses"
    );

    assert!(
        compiled.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::MainFromHand) && !t.optional
        )),
        "Main effect must be a mandatory main_from_hand triggered clause"
    );

    let security = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnSecurity)
                    && t.scope == digimon_dsl::compiled::CompiledScope::Inherited =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("Security placement must be represented as inherited on_security");

    assert_eq!(
        security.process,
        vec![CompiledStep::PlaceSelfAsDelayOption],
        "Security placement must use the native delay-option placement step"
    );
}

#[test]
fn lm_035_delay_clause_is_standard_gain_2_memory() {
    let runner = lm_035_runner();
    let compiled = runner.compiled_card("LM-035").expect("LM-035 compiled");

    let delay = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                process,
                ..
            }) => Some((trigger, process)),
            _ => None,
        })
        .expect("LM-035 must have a declarative Delay clause");

    assert_eq!(*delay.0, CompiledTiming::Delayed);
    assert!(
        delay
            .1
            .iter()
            .any(|step| matches!(step, CompiledStep::GainMemory(2))),
        "Delay body must gain 2 memory"
    );
}

#[test]
fn lm_035_purple_permanent_satisfies_option_color_requirement_in_mask() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-035 YAML parses")
        .add_card(tamer("PURPLE-TAMER", CardColor::Purple))
        .hand(0, &["LM-035"])
        .memory(10)
        .start();

    runner.place_on_field(0, "PURPLE-TAMER", Some(0));
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "purple Digimon/Tamer should satisfy Amber Memory Boost!'s printed color access"
    );
}

#[test]
fn lm_035_does_not_bypass_color_requirement_without_yellow_or_purple_access() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-035 YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .hand(0, &["LM-035"])
        .memory(10)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "red-only board must not satisfy yellow option color or purple alternate access"
    );
}

#[test]
fn lm_035_main_reveals_3_and_adds_purple_digimon_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-035 YAML parses")
        .add_card(digimon("PURPLE-DIGI", CardColor::Purple))
        .add_card(digimon("BLUE-DIGI", CardColor::Blue))
        .add_card(option("YELLOW-OPTION", CardColor::Yellow))
        .add_card(filler("FILL"))
        .hand(0, &["LM-035"])
        .deck(0, &["FILL", "YELLOW-OPTION", "BLUE-DIGI", "PURPLE-DIGI"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only the purple Digimon among the top 3 should be selectable"
    );
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "PURPLE-DIGI"),
        "selected purple Digimon should be added to hand"
    );
    assert_eq!(
        runner.deck_size(0),
        3,
        "top 3 reveal should return the two non-picked cards to the deck bottom"
    );
}

#[test]
fn lm_035_main_reveals_3_and_adds_yellow_digimon_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-035 YAML parses")
        .add_card(digimon("YELLOW-DIGI", CardColor::Yellow))
        .add_card(digimon("GREEN-DIGI", CardColor::Green))
        .add_card(option("PURPLE-OPTION", CardColor::Purple))
        .add_card(filler("FILL"))
        .hand(0, &["LM-035"])
        .deck(0, &["FILL", "PURPLE-OPTION", "GREEN-DIGI", "YELLOW-DIGI"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "YELLOW-DIGI", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only the yellow Digimon among the top 3 should be selectable"
    );
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "YELLOW-DIGI"),
        "selected yellow Digimon should be added to hand"
    );
}

/// PUPPETS-G009 — LM-035's standard `<Delay>` is a player-visible
/// `[Main]`-phase activation. Playing it parks a `MainPhaseActivated`
/// delayed Option; on a later main phase the controller activates it,
/// trashing the Option as cost and running the body (gain 2 memory).
#[test]
fn lm_035_delay_activation_gains_2_memory_via_main_phase_action() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-035 YAML parses")
        .add_card(tamer("PURPLE-TAMER", CardColor::Purple))
        .add_card(filler("FILL"))
        .hand(0, &["LM-035"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "PURPLE-TAMER", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve LM-035 reveal selection and delay placement");

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
        .expect("playing LM-035 should park it as a MainPhaseActivated delayed option");

    let bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        0.0,
        "LM-035 <Delay> must not be activatable on the placing turn"
    );

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(0);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[bit], 1.0, "LM-035 <Delay> activation is a legal action");
    assert_eq!(mask[PASS as usize], 1.0, "declining stays legal");

    runner.game.decode_action(bit as u16, 0);

    assert_eq!(
        runner.memory(),
        2,
        "LM-035 <Delay> body gains 2 memory when the player activates it"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|permanent| matches!(permanent.option_state, OptionState::Delayed { .. })),
        "LM-035 is trashed as the <Delay> activation cost"
    );
}

#[test]
fn lm_035_inherited_security_places_self_in_battle_area() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-035 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Red))
        .add_card(filler("FILL"))
        .security(1, &["LM-035"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0, "LM-035 should leave security");
    assert_eq!(
        runner.trash_size(1),
        0,
        "LM-035 should be placed in battle area instead of trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "LM-035")
        .expect("LM-035 should be placed as a battle-area Option permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}
