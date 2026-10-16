//! LM-036 Jade Memory Boost! — Option, Cost 3, Green.
//!
//! Printed text:
//! - Blue also meets this card's color requirements.
//! - [Main] Reveal the top 3 cards of your deck. Add 1 green or blue
//!   Digimon card among them to the hand. Return the rest to the bottom of deck.
//!   Then, place this card in the battle area.
//! - [Main] <Delay> Gain 2 memory.
//! - Security Effect [Security] Place this card in the battle area.
//! Audit: official observation 072c5e2161bb61838b0de691a588f3895629da95d55098ac41f6953a86a9310c,
//! actual LM-036 image, Q4069/Q4070, and base DCGO LM_036.cs.
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

#[test]
fn lm_036_main_reveals_3_and_adds_blue_digimon_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-036 YAML parses")
        .add_card(digimon("BLUE-DIGI", CardColor::Blue))
        .add_card(digimon("YELLOW-DIGI", CardColor::Yellow))
        .add_card(option("GREEN-OPTION", CardColor::Green))
        .add_card(filler("FILL"))
        .hand(0, &["LM-036"])
        .deck(0, &["FILL", "GREEN-OPTION", "YELLOW-DIGI", "BLUE-DIGI"])
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

    let saved = runner.game.clone();
    runner.game.decode_action(bit as u16, 0);
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "LM-036"));
    runner.game = saved;
    assert_eq!(runner.memory(), 0, "cloning preserves unactivated Delay");
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

#[test]
fn lm_036_color_access_owner_kind_and_breeding_matrix() {
    for (kind, color, owner, breeding, allowed) in [
        (CardKind::Digimon, CardColor::Blue, 0, true, true),
        (CardKind::Digimon, CardColor::Blue, 0, false, true),
        (CardKind::Tamer, CardColor::Green, 0, false, true),
        (CardKind::Digimon, CardColor::Green, 0, true, true),
        (CardKind::Option, CardColor::Blue, 0, false, false),
        (CardKind::Digimon, CardColor::Blue, 1, false, false),
        (CardKind::Tamer, CardColor::Red, 0, false, false),
    ] {
        let mut c = digimon("ACCESS", color);
        c.card_kind = kind;
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(YAML)
            .unwrap()
            .add_card(c)
            .hand(0, &["LM-036"])
            .memory(10)
            .start();
        r.skip_mulligan();
        r.place_on_field(owner, "ACCESS", None);
        if breeding {
            let p = r.game.players[owner as usize].battle_area.remove(0);
            r.game.players[owner as usize].breeding_area = Some(p);
        }
        assert_eq!(
            build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.0,
            allowed,
            "{kind:?} {color:?} owner={owner} breeding={breeding}"
        );
    }
}

fn audit_reveal(top: &[&str]) -> DebugRunner {
    use digimon_engine::card_source::CardSource;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .add_card(digimon("B", CardColor::Blue))
        .add_card(digimon("G", CardColor::Green))
        .add_card(digimon("R", CardColor::Red))
        .add_card(tamer("T", CardColor::Green))
        .add_card(option("O", CardColor::Blue))
        .hand(0, &["LM-036"])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.place_on_field(0, "B", None);
    for id in top.iter().rev() {
        let index = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap();
        let serial = r.game.next_card_index();
        r.game.players[0]
            .deck
            .push(CardSource::new(index, 0, serial));
    }
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    r
}

fn audit_order(r: &mut DebugRunner, reverse: bool) {
    use digimon_engine::selection::SelectionKind;
    for _ in 0..4 {
        let Some(v) = r.pending_selection_view() else {
            return;
        };
        assert!(matches!(v.kind, SelectionKind::OrderedPermutation { .. }));
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
        let a = if reverse {
            *v.valid_action_ids.last().unwrap()
        } else {
            v.valid_action_ids[0]
        };
        r.execute_action(0, a).unwrap();
    }
    panic!("remainder did not resolve");
}

#[test]
fn lm_036_mandatory_search_offers_both_colors_and_clone_preserves_order_choices() {
    use digimon_engine::action::space::SEL_REVEAL_START;
    let mut r = audit_reveal(&["B", "G", "T", "R"]);
    let m = build_action_mask(&r.game, 0);
    assert_eq!(m[PASS as usize], 0.0);
    assert_eq!(m[SEL_REVEAL_START as usize], 1.0);
    assert_eq!(m[SEL_REVEAL_START as usize + 1], 1.0);
    assert_eq!(m[SEL_REVEAL_START as usize + 2], 0.0);
    let saved = r.game.clone();
    r.execute_action(0, SEL_REVEAL_START).unwrap();
    let order = r.pending_selection_view().unwrap();
    assert_eq!(order.valid_action_ids.len(), 2);
    audit_order(&mut r, false);
    let first: Vec<_> = r.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_owned())
        .collect();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.game.players[0].hand[0].card_id(&r.game.card_data), "B");
    assert_eq!(r.memory(), 7);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    r.game = saved;
    r.execute_action(0, SEL_REVEAL_START).unwrap();
    audit_order(&mut r, true);
    let second: Vec<_> = r.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_owned())
        .collect();
    assert_ne!(
        first, second,
        "both legal bottom orders must be player choices"
    );
    assert_eq!(first.last().map(String::as_str), Some("R"));
    assert_eq!(second.last().map(String::as_str), Some("R"));
}

#[test]
fn lm_036_no_eligible_and_short_deck_still_place_delay() {
    for top in [&["O", "T", "R"][..], &["G"][..], &[][..]] {
        let mut r = audit_reveal(top);
        if top == ["G"] {
            let v = r.pending_selection_view().unwrap();
            assert_eq!(v.valid_action_ids.len(), 1);
            r.execute_action(0, v.valid_action_ids[0]).unwrap();
        }
        audit_order(&mut r, false);
        assert_eq!(r.hand_size(0), usize::from(top == ["G"]));
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "LM-036"));
        assert_eq!(r.memory(), 7);
    }
}

#[test]
fn lm_036_native_and_alternate_breeding_digiegg_allow_actual_paid_option_action() {
    for color in [CardColor::Green, CardColor::Blue] {
        let mut egg = digimon("ACCESS-EGG", color);
        egg.card_kind = CardKind::DigiEgg;
        egg.level = Some(2);
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(YAML).unwrap()
            .add_card(egg)
            .hand(0, &["LM-036"])
            .memory(10).start();
        r.skip_mulligan();
        r.place_on_field(0, "ACCESS-EGG", None);
        let egg = r.game.players[0].battle_area.remove(0);
        r.game.players[0].breeding_area = Some(egg);
        assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0,
            "{color:?} breeding Digi-Egg meets printed option access");
        r.game.decode_action(PLAY_HAND_START, 0);
        assert_eq!(r.memory(), 7, "actual action pays printed cost 3");
        assert_eq!(r.hand_size(0), 0);
        assert!(r.game.players[0].battle_area.iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "LM-036"));
        assert!(r.game.players[0].breeding_area.is_some());
    }
}

#[test]
fn lm_036_no_eligible_reveal_exposes_all_six_bottom_orders() {
    let mut r = audit_reveal(&["O", "T", "R", "B"]);
    let saved = r.game.clone();
    let mut outcomes = std::collections::BTreeSet::new();
    for first in 0..3 {
        for second in 0..2 {
            r.game = saved.clone();
            let choices = r.pending_selection_view().unwrap();
            assert_eq!(choices.valid_action_ids.len(), 3);
            assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
            r.execute_action(0, choices.valid_action_ids[first]).unwrap();
            let choices = r.pending_selection_view().unwrap();
            assert_eq!(choices.valid_action_ids.len(), 2);
            r.execute_action(0, choices.valid_action_ids[second]).unwrap();
            audit_order(&mut r, false);
            let order: Vec<_> = r.game.players[0].deck.iter()
                .map(|c| c.card_id(&r.game.card_data).to_owned()).collect();
            assert_eq!(order.len(), 4);
            assert_eq!(order.last().map(String::as_str), Some("B"));
            assert_eq!(r.hand_size(0), 0, "colored Options and Tamers are ineligible");
            assert_eq!(r.memory(), 7);
            assert_eq!(r.game.players[0].battle_area.len(), 2);
            outcomes.insert(order);
        }
    }
    assert_eq!(outcomes.len(), 6, "every bottom order is a player choice");
}
