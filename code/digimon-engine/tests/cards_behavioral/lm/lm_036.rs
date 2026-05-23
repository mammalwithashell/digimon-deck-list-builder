//! LM-036 Jade Memory Boost! — Option, Cost 3, Green.
//!
//! Printed text:
//! - Blue also meets this card's color requirements.
//! - [Main] Reveal the top 3 cards of your deck. Add 1 green or blue
//!   Digimon card among them to the hand. Return the rest to the bottom of deck.
//!   Then, place this card in the battle area.
//! - [Main] <Delay> (By trashing this card after the placing turn, activate
//!   the effect below.)
//! - ・Gain 2 memory.
//! - Security Effect [Security] Place this card in the battle area.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/LM/Green/LM_036.cs
//!
//! Color-swap parity: LM-035 (yellow + purple alternate) is the primary
//! template. LM-036 is green + blue alternate — identical structure, only
//! colors swapped. DCGO LM_036.cs vs LM_035.cs: identical except
//! CardColor.Blue (036) vs CardColor.Purple (035) in CanUseCondition, and
//! Green/Blue (036) vs Yellow/Purple (035) in CanSelectCardCondition.
//!
//! Patterns:
//! - Color flex / use_requirement (alternate color satisfies Option color gate)
//! - A1 Reveal top-N, add by color filter
//! - Delay option (PUPPETS-G009, MainPhaseActivated)
//! - Security placement as Delay permanent
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

const YAML: &str = include_str!("../../../cards/lm/LM-036.yaml");

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

fn lm_036_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-036 YAML must parse and compile")
        .memory(10)
        .start()
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn lm_036_yaml_parses_and_compiles() {
    let _runner = lm_036_runner();
}

#[test]
fn lm_036_is_green_option_cost_3_with_blue_use_requirement() {
    let runner = lm_036_runner();
    let compiled = runner.compiled_card("LM-036").expect("LM-036 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(3));
    assert!(
        compiled.use_requirement.is_some(),
        "Blue also meets this card's color requirements must compile as use_requirement"
    );
}

#[test]
fn lm_036_has_main_delay_and_inherited_security_clauses() {
    let runner = lm_036_runner();
    let compiled = runner.compiled_card("LM-036").expect("LM-036 compiled");

    assert_eq!(
        compiled.effects.len(),
        3,
        "LM-036 should have Main, Delay, and inherited Security clauses"
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
fn lm_036_delay_clause_is_standard_gain_2_memory() {
    let runner = lm_036_runner();
    let compiled = runner.compiled_card("LM-036").expect("LM-036 compiled");

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
        .expect("LM-036 must have a declarative Delay clause");

    assert_eq!(*delay.0, CompiledTiming::Delayed);
    assert!(
        delay
            .1
            .iter()
            .any(|step| matches!(step, CompiledStep::GainMemory(2))),
        "Delay body must gain 2 memory"
    );
}

// ─── Section 2: Condition gating — color flex ─────────────────────────────────

/// Positive: a blue Tamer on the field satisfies the alternate color requirement,
/// making LM-036 playable even though the controller has no green permanent.
#[test]
fn lm_036_blue_permanent_satisfies_option_color_requirement_in_mask() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-036 YAML parses")
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .hand(0, &["LM-036"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "blue Digimon/Tamer should satisfy Jade Memory Boost!'s printed color access"
    );
}

/// Negative: a red-only board (neither green nor blue) must not allow LM-036.
#[test]
fn lm_036_does_not_bypass_color_requirement_without_green_or_blue_access() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-036 YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .hand(0, &["LM-036"])
        .memory(10)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "red-only board must not satisfy green option color or blue alternate access"
    );
}

// ─── Section 3: Behavioral outcomes ───────────────────────────────────────────

/// [Main] reveals 3 and adds the blue Digimon (matching alternate color) to hand.
#[test]
fn lm_036_main_reveals_3_and_adds_blue_digimon_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-036 YAML parses")
        .add_card(digimon("BLUE-DIGI", CardColor::Blue))
        .add_card(digimon("RED-DIGI", CardColor::Red))
        .add_card(option("GREEN-OPTION", CardColor::Green))
        .add_card(filler("FILL"))
        .hand(0, &["LM-036"])
        .deck(0, &["FILL", "GREEN-OPTION", "RED-DIGI", "BLUE-DIGI"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only the blue Digimon among the top 3 should be selectable"
    );
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BLUE-DIGI"),
        "selected blue Digimon should be added to hand"
    );
    assert_eq!(
        runner.deck_size(0),
        3,
        "top 3 reveal should return the two non-picked cards to the deck bottom"
    );
}

/// [Main] reveals 3 and adds the green Digimon (primary color) to hand.
#[test]
fn lm_036_main_reveals_3_and_adds_green_digimon_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-036 YAML parses")
        .add_card(digimon("GREEN-DIGI", CardColor::Green))
        .add_card(digimon("RED-DIGI", CardColor::Red))
        .add_card(option("BLUE-OPTION", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(0, &["LM-036"])
        .deck(0, &["FILL", "BLUE-OPTION", "RED-DIGI", "GREEN-DIGI"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "GREEN-DIGI", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only the green Digimon among the top 3 should be selectable"
    );
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "GREEN-DIGI"),
        "selected green Digimon should be added to hand"
    );
}

// ─── Section 4: Delay trash activation ───────────────────────────────────────

/// PUPPETS-G009 — LM-036's standard `<Delay>` is a player-visible
/// `[Main]`-phase activation. Playing it parks a `MainPhaseActivated`
/// delayed Option; on a later main phase the controller activates it,
/// trashing the Option as cost and running the body (gain 2 memory).
#[test]
fn lm_036_delay_activation_gains_2_memory_via_main_phase_action() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-036 YAML parses")
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(0, &["LM-036"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve LM-036 reveal selection and delay placement");

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
        .expect("playing LM-036 should park it as a MainPhaseActivated delayed option");

    let bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        0.0,
        "LM-036 <Delay> must not be activatable on the placing turn"
    );

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(0);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[bit], 1.0,
        "LM-036 <Delay> activation is a legal action"
    );
    assert_eq!(mask[PASS as usize], 1.0, "declining stays legal");

    runner.game.decode_action(bit as u16, 0);

    assert_eq!(
        runner.memory(),
        2,
        "LM-036 <Delay> body gains 2 memory when the player activates it"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|permanent| matches!(permanent.option_state, OptionState::Delayed { .. })),
        "LM-036 is trashed as the <Delay> activation cost"
    );
}

// ─── Section 3 (security): Security effect places card in battle area ─────────

#[test]
fn lm_036_inherited_security_places_self_in_battle_area() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-036 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Red))
        .add_card(filler("FILL"))
        .security(1, &["LM-036"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0, "LM-036 should leave security");
    assert_eq!(
        runner.trash_size(1),
        0,
        "LM-036 should be placed in battle area instead of trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "LM-036")
        .expect("LM-036 should be placed as a battle-area Option permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}
