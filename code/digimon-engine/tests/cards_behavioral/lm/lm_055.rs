//! LM-055 Sprint Dash Training - Option, Cost 2, Green/Red.
//!
//! Printed text:
//! - While you don't have [Sprint Dash Training] in the battle area, you can
//!   ignore this card's color requirements.
//! - [Main] Reveal the top 2 cards of your deck. Add 1 green or red card among
//!   them to the hand. Return the rest to the bottom of deck. Then, place this
//!   card in the battle area.
//! - [Main] <Delay> 1 of your Digimon may digivolve into a green or red
//!   Digimon card in your hand for its digivolution cost. When it would
//!   digivolve by this effect, reduce the cost by 2.
//! - Security: same reveal/add/remainder flow, then place this card in the
//!   battle area.

#![allow(dead_code)]

// Audit: official observation 71671fa55340ba8944d21b8e7f20fa89f26e7ef0ce621239fb97891031b4c2d4,
// actual LM-055 image, and base-repository DCGO LM_055.cs.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledPredicate,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/lm/LM-055.yaml");

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-055 YAML must parse and compile")
        .memory(10)
        .start()
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn colored_card(id: &str, color: CardColor, kind: CardKind) -> CardData {
    let mut card = make_test_card(id, id);
    card.colors = vec![color];
    card.card_kind = kind;
    if kind == CardKind::Digimon {
        card.level = Some(4);
        card.dp = Some(5000);
    }
    card
}

fn green_option(id: &str) -> CardData {
    colored_card(id, CardColor::Green, CardKind::Option)
}

fn red_option(id: &str) -> CardData {
    colored_card(id, CardColor::Red, CardKind::Option)
}

fn green_digimon(id: &str, level: u8) -> CardData {
    let mut card = colored_card(id, CardColor::Green, CardKind::Digimon);
    card.level = Some(level);
    card
}

fn red_digimon(id: &str, level: u8) -> CardData {
    let mut card = colored_card(id, CardColor::Red, CardKind::Digimon);
    card.level = Some(level);
    card
}

fn attacker(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(6000);
    card.colors = vec![CardColor::Red];
    card
}

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    runner()
        .compiled_card("LM-055")
        .expect("LM-055 compiled card present")
        .clone()
}

fn triggered_clause(when: CompiledTiming) -> digimon_dsl::compiled::CompiledTriggeredClause {
    compiled()
        .effects
        .into_iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) if triggered.when.contains(&when) => {
                Some(triggered)
            }
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("LM-055 must have triggered clause {when:?}"))
}

fn has_red_or_green_filter(filter: &CompiledPredicate) -> bool {
    let colors: Vec<_> = filter.any_of.iter().filter_map(|p| p.color_is).collect();
    colors.contains(&CompiledColor::Green) && colors.contains(&CompiledColor::Red)
}

fn drain_first_choices(runner: &mut DebugRunner) {
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
    assert!(
        steps < 20,
        "selection drain exceeded guard; pending={:?}",
        runner.game.pending_selection
    );
}

#[test]
fn lm_055_yaml_parses_and_compiles() {
    let card = compiled();
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.color, vec![CompiledColor::Green, CompiledColor::Red]);
}

#[test]
fn lm_055_has_color_bypass_main_delay_and_security_clauses() {
    let card = compiled();
    assert_eq!(
        card.effects.len(),
        3,
        "expected main_from_hand, delay, and inherited security"
    );

    assert!(card
        .use_requirement
        .as_ref()
        .unwrap()
        .no_permanent
        .is_some());
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay { .. })
        )),
        "LM-055 must compile a Delay declarative clause"
    );
}

#[test]
fn lm_055_main_clause_has_reveal_add_remainder_and_place_steps() {
    let main = triggered_clause(CompiledTiming::MainFromHand);

    assert!(!main.optional, "the reveal [Main] effect is mandatory");
    assert_eq!(main.scope, CompiledScope::FaceUp);
    assert_eq!(main.process.len(), 5, "main process shape drifted");
    assert!(matches!(
        main.process[0],
        CompiledStep::RevealTopDeck { count: 2, .. }
    ));
    let CompiledStep::SelectReveal {
        filter, optional, ..
    } = &main.process[1]
    else {
        panic!("main step 1 must select from reveal");
    };
    assert!(
        !*optional,
        "printed add 1 is mandatory when an eligible card exists"
    );
    assert!(has_red_or_green_filter(filter));
    assert!(matches!(
        main.process[2],
        CompiledStep::AddToHandFromReveal { .. }
    ));
    assert!(matches!(
        main.process[3],
        CompiledStep::PlaceRemainderOnDeck { .. }
    ));
    assert_eq!(main.process[4], CompiledStep::PlaceSelfAsDelayOption);
}

#[test]
fn lm_055_delay_clause_structure_matches_effect_digivolve_cost_reduce_2() {
    let card = compiled();
    let delay = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                process,
                ..
            }) => Some((*trigger, process)),
            _ => None,
        })
        .expect("Delay clause must exist");

    // TRIGGER CORRECTION (2026-08-24): was `OnPlay`, which no delay arm named,
    // so the old lowering's `_` catch-all silently turned it into a
    // turn-scheduled auto-fire at the end of the controller's next turn.
    // LM-055 prints "[Main] <Delay>", so it is the standard player-activated
    // Delay: `Delayed` -> DelayTrigger::MainPhaseActivated, where NOT
    // activating is the §16-17-2 decline.
    assert_eq!(delay.0, CompiledTiming::Delayed);
    assert_eq!(delay.1.len(), 3, "delay process shape drifted");
    let CompiledStep::SelectOwnPermanent {
        filter, optional, ..
    } = &delay.1[0]
    else {
        panic!("delay step 0 must choose one of your Digimon");
    };
    assert!(
        *optional,
        "printed 'may digivolve' requires optional target selection"
    );
    assert_eq!(filter.kind, Some(CompiledCardKind::Digimon));

    let CompiledStep::SelectHand {
        filter, optional, ..
    } = &delay.1[1]
    else {
        panic!("delay step 1 must choose a Digimon card in hand");
    };
    assert!(
        *optional,
        "DCGO DigivolveIntoHandOrTrashCard permits declining the hand result independently"
    );
    assert!(filter
        .all_of
        .iter()
        .any(|p| p.kind == Some(CompiledCardKind::Digimon)));
    assert!(filter.all_of.iter().any(has_red_or_green_filter));

    assert!(matches!(
        delay.1[2],
        CompiledStep::EffectInitiatedDigivolve {
            cost: digimon_dsl::compiled::CompiledCostDelta::Reduce(2),
            ignore_requirements: false,
            ..
        }
    ));
}

#[test]
fn lm_055_security_clause_has_reveal_add_remainder_and_place_steps() {
    let security = triggered_clause(CompiledTiming::OnSecurity);

    assert_eq!(security.scope, CompiledScope::Inherited);
    assert_eq!(security.process.len(), 5, "security process shape drifted");
    assert!(matches!(
        security.process[0],
        CompiledStep::RevealTopDeck { count: 2, .. }
    ));
    let CompiledStep::SelectReveal { filter, .. } = &security.process[1] else {
        panic!("security step 1 must select from reveal");
    };
    assert!(has_red_or_green_filter(filter));
    assert!(matches!(
        security.process[2],
        CompiledStep::AddToHandFromReveal { .. }
    ));
    assert!(matches!(
        security.process[3],
        CompiledStep::PlaceRemainderOnDeck { .. }
    ));
    assert_eq!(security.process[4], CompiledStep::PlaceSelfAsDelayOption);
}

#[test]
fn lm_055_main_reveals_green_or_red_card_adds_to_hand_and_bottoms_remainder() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-055 YAML parses")
        .add_card(green_option("GREEN-PICK"))
        .add_card(red_option("RED-REMAINDER"))
        .add_card(filler("FILL"))
        .hand(0, &["LM-055"])
        .hand(1, &["FILL"])
        .deck(0, &["GREEN-PICK", "RED-REMAINDER", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);
    let deck_before = runner.game.players[0].deck.len();

    assert!(runner.game.activate_hand_main(0, 0));
    drain_first_choices(&mut runner);

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "LM-055 leaves hand for battle area while one revealed green/red card is added"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before - 1,
        "two revealed cards minus one hand pick leaves a net deck loss of one"
    );
    let bottom_id = runner.game.players[0]
        .deck
        .last()
        .expect("deck still has bottom card")
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        bottom_id, "RED-REMAINDER",
        "unpicked revealed card should return to the bottom of deck"
    );
    let placed = runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "LM-055")
        .expect("LM-055 must become a battle-area Delay permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed { owner: 0, .. }
    ));
}

#[test]
fn lm_055_delay_body_installs_digimon_selection_when_conditions_met() {
    let mut evo = red_digimon("RED-EVO", 4);
    evo.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: CardColor::Green as u8,
        level: 3,
        memory_cost: 3,
    }];
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-055 YAML parses")
        .add_card(green_digimon("BASE", 3))
        .add_card(evo)
        .add_card(filler("FILL"))
        .hand(0, &["RED-EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "LM-055", Some(0));
    // This body fixture starts after the placing turn with a genuine Delay Option.
    // A raw Standard Option is rule-trashed and cannot stand in for this state.
    let placed_on_turn = runner.game.turn_count;
    runner.game.turn_count = placed_on_turn + 2;
    runner.game.player_mut(0).battle_area[delay_perm.index as usize].option_state =
        digimon_engine::permanent::OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: digimon_engine::enums::DelayTrigger::MainPhaseActivated,
            placed_on_turn,
        };

    runner.place_on_field(0, "BASE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_some(),
        "Delay body should ask for one of your Digimon when a base is present"
    );
}

#[test]
fn lm_055_security_reveals_adds_and_places_self_as_delay_option() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-055 YAML parses")
        .add_card(attacker("ATTACKER"))
        .add_card(green_option("SEC-GREEN-PICK"))
        .add_card(red_option("SEC-RED-REMAINDER"))
        .add_card(filler("FILL"))
        .security(1, &["LM-055"])
        .deck(1, &["SEC-GREEN-PICK", "SEC-RED-REMAINDER", "FILL"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);
    assert_eq!(
        result,
        AttackResult::InProgress,
        "security reveal selection should park combat until auto_resolve drives it"
    );
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(runner.security_count(1), 0, "LM-055 left security");
    assert_eq!(
        runner.hand_size(1),
        1,
        "security reveal should add one green/red card to hand"
    );
    assert_eq!(
        runner.trash_size(1),
        0,
        "LM-055 should not be trashed after placing itself"
    );
    let placed = runner.game.players[1]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "LM-055")
        .expect("LM-055 must become a battle-area Delay permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed { owner: 1, .. }
    ));
}

fn audit_delay(two_costs: bool) -> DebugRunner {
    audit_delay_candidates(two_costs, false, true)
}

fn audit_delay_candidates(two_costs: bool, decoys: bool, compatible: bool) -> DebugRunner {
    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
    };
    use digimon_engine::card_data::EvoCost;
    let mut base = green_digimon("AUDIT-BASE", 3);
    base.colors.push(CardColor::Red);
    let mut evo = red_digimon("AUDIT-EVO", 4);
    evo.evo_costs = vec![EvoCost {
        card_color: CardColor::Green as u8,
        level: 3,
        memory_cost: 3,
    }];
    if two_costs {
        evo.evo_costs.push(EvoCost {
            card_color: CardColor::Red as u8,
            level: 3,
            memory_cost: 4,
        });
    }
    if !compatible {
        evo.evo_costs.clear();
    }
    let mut wrong_level = red_digimon("WRONG-LEVEL", 6);
    wrong_level.evo_costs = vec![EvoCost {
        card_color: CardColor::Green as u8,
        level: 5,
        memory_cost: 3,
    }];
    let mut wrong_color = colored_card("WRONG-COLOR", CardColor::Blue, CardKind::Digimon);
    wrong_color.evo_costs = evo.evo_costs.clone();
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .add_card(base)
        .add_card(green_digimon("WRONG-BASE", 6))
        .add_card(wrong_level)
        .add_card(wrong_color)
        .add_card(red_option("WRONG-KIND"))
        .add_card(evo)
        .add_card(filler("AUDIT-FILL"))
        .hand(
            0,
            if decoys {
                &[
                    "LM-055",
                    "AUDIT-EVO",
                    "WRONG-LEVEL",
                    "WRONG-COLOR",
                    "WRONG-KIND",
                ]
            } else {
                &["LM-055", "AUDIT-EVO"]
            },
        )
        .deck(0, &["AUDIT-FILL"; 12])
        .deck(1, &["AUDIT-FILL"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    r.game.play_option_from_hand(0, 0);
    r.auto_resolve().unwrap();
    r.place_on_field(0, "AUDIT-BASE", Some(0));
    if decoys {
        r.place_on_field(0, "WRONG-BASE", Some(0));
    }
    let index = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "LM-055")
        .unwrap();
    let bit =
        FIELD_EFFECT_START + index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[bit as usize],
        0.0
    );
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.game.set_memory(5);
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[bit as usize],
        1.0
    );
    r.game.decode_action(bit, 0);
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "LM-055"));
    r
}

#[test]
fn lm_055_actual_delay_pays_reduced_cost_and_clone_can_decline() {
    use digimon_engine::action::space::PASS;
    let mut r = audit_delay(false);
    let saved = r.game.clone();
    let v = r.pending_selection_view().expect("optional base choice");
    assert!(r.pending_is_optional());
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    let v = r.pending_selection_view().expect("hand evolution choice");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    assert_eq!(r.memory(), 4);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "AUDIT-EVO"
    );
    r.game = saved;
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.memory(), 5);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "AUDIT-BASE"
    );
}

#[test]
fn lm_055_reveal_is_mandatory_and_clone_can_choose_either_color() {
    use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .add_card(green_option("AUDIT-GREEN"))
        .add_card(colored_card("AUDIT-RED", CardColor::Red, CardKind::Tamer))
        .hand(0, &["LM-055"])
        .deck(0, &["AUDIT-GREEN", "AUDIT-RED"])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    r.game.play_option_from_hand(0, 0);
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    assert_eq!(mask[PASS as usize], 0.0);
    assert_eq!(mask[SEL_REVEAL_START as usize], 1.0);
    assert_eq!(mask[SEL_REVEAL_START as usize + 1], 1.0);
    let saved = r.game.clone();
    for (action, expected) in [
        (SEL_REVEAL_START, "AUDIT-RED"),
        (SEL_REVEAL_START + 1, "AUDIT-GREEN"),
    ] {
        r.game = saved.clone();
        r.execute_action(0, action).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(
            r.game.players[0].hand[0].card_id(&r.game.card_data),
            expected
        );
        assert_eq!(r.memory(), 8);
        assert_eq!(r.game.players[0].battle_area.len(), 1);
    }
}

#[test]
fn lm_055_color_bypass_checks_exact_name_controller_and_battle_area() {
    use digimon_engine::action::space::PLAY_HAND_START;
    for (name, owner, allowed) in [
        ("Sprint Dash Training", 0, false),
        ("Sprint Dash Training Plus", 0, true),
        ("Sprint Dash Training", 1, true),
    ] {
        let mut other = colored_card("AUDIT-NAME", CardColor::Blue, CardKind::Option);
        other.card_name = name.into();
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(YAML)
            .unwrap()
            .add_card(other)
            .hand(0, &["LM-055"])
            .memory(10)
            .start();
        r.place_on_field(owner, "AUDIT-NAME", Some(0));
        r.game.enter_main_phase();
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
            f32::from(allowed),
            "name={name}, owner={owner}"
        );
    }
}

#[test]
fn lm_055_delay_masks_exclude_incompatible_bases_and_hand_cards() {
    use digimon_engine::action::space::PASS;
    let mut r = audit_delay_candidates(false, true, true);
    let v = r.pending_selection_view().expect("legal base choice");
    assert_eq!(
        v.valid_action_ids.len(),
        1,
        "only compatible base is selectable"
    );
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
        1.0
    );
    let action = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, action).unwrap();
    let v = r.pending_selection_view().expect("compatible hand choice");
    assert_eq!(
        v.valid_action_ids.len(),
        1,
        "only compatible hand result; exclude wrong level, color, and kind"
    );
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
        1.0
    );
    let action = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, action).unwrap();
    assert_eq!(r.memory(), 4);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "AUDIT-EVO"
    );
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        "WRONG-BASE"
    );
}

#[test]
fn lm_055_delay_without_compatible_route_finishes_without_false_choices() {
    let r = audit_delay_candidates(false, false, false);
    assert!(
        r.game.pending_selection.is_none(),
        "a Digimon without a legal hand evolution is not a target"
    );
    assert_eq!(r.memory(), 5);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "AUDIT-BASE"
    );
}

#[test]
fn lm_055_delay_decline_stops_before_hand_prompt() {
    use digimon_engine::action::space::PASS;
    let mut r = audit_delay(true);
    r.execute_action(0, PASS).unwrap();
    assert!(
        r.game.pending_selection.is_none(),
        "declining the base must not install a hand prompt"
    );
    assert_eq!(r.memory(), 5);
}

#[test]
fn lm_055_delay_cloned_route_choice_pays_both_printed_costs_minus_two() {
    use digimon_engine::action::space::PASS;
    let mut r = audit_delay(true);
    for _ in 0..2 {
        let v = r.pending_selection_view().expect("base then hand choice");
        let action = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        r.execute_action(0, action).unwrap();
    }
    let v = r
        .pending_selection_view()
        .expect("both evolution routes must remain player choices");
    assert_eq!(v.valid_action_ids.len(), 2);
    assert!(!v.valid_action_ids.contains(&PASS));
    let actions = v.valid_action_ids.clone();
    let saved = r.game.clone();
    let mut memories = Vec::new();
    for action in actions {
        r.game = saved.clone();
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[action as usize],
            1.0
        );
        r.execute_action(0, action).unwrap();
        assert!(r.game.pending_selection.is_none());
        memories.push(r.memory());
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "AUDIT-EVO"
        );
    }
    memories.sort();
    assert_eq!(
        memories,
        vec![3, 4],
        "printed 3/4 routes must pay reduced 1/2"
    );
}

// DCGO LM_055.cs calls DigivolveIntoHandOrTrashCard without overriding
// isOptional=true (CardEffectCommons.cs:834); its hand picker uses
// canNoSelect: isOptional (:1055), independently of the earlier base choice.
#[test]
fn lm_055_delay_cloned_hand_choice_can_decline_after_selecting_base() {
    use digimon_engine::action::space::PASS;
    let mut r = audit_delay(false);
    let v = r.pending_selection_view().expect("optional base choice");
    let base_action = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, base_action).unwrap();
    let v = r.pending_selection_view().expect("optional hand result");
    assert!(r.pending_is_optional());
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
        1.0
    );
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
        1.0
    );
    let accept = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    let saved = r.game.clone();
    r.execute_action(0, PASS).unwrap();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 5);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "AUDIT-BASE"
    );
    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "AUDIT-EVO"));
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "LM-055"));
    r.game = saved;
    r.execute_action(0, accept).unwrap();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 4);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "AUDIT-EVO"
    );
}

// ─────── Task 7 re-audit (2026-10-21): printed BT20 route choice ───────
//
// The LM-055 extension of G-ENGINE-EFFECT-DIGIVOLVE-COST-CHOICE observed the
// real Delay auto-paying 1 for routes costing 3 or 4. Real pair from the
// Examon deck: Wingdramon (BT20-025, blue/red — red qualifies) on Coredramon
// (BT20-023): named route 3 and level-4 colour routes 4, reduced by 2 → the
// player chooses between paying 1 and 2 (duplicate colour routes collapse).
#[test]
fn lm_055_real_delay_wingdramon_on_coredramon_offers_reduced_1_and_2() {
    use digimon_engine::action::build_action_mask;
    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
        PLAY_HAND_START,
    };
    use digimon_engine::selection::SelectionKind;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .dsl_card("BT20-023")
        .expect("Coredramon YAML")
        .dsl_card("BT20-025")
        .expect("Wingdramon YAML")
        .add_card(filler("AUDIT-FILL"))
        .hand(0, &["LM-055", "BT20-025"])
        .deck(0, &["AUDIT-FILL"; 12])
        .deck(1, &["AUDIT-FILL"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    r.auto_resolve().unwrap();
    r.place_on_field(0, "BT20-023", Some(0));
    let index = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "LM-055")
        .expect("LM-055 placed by its own [Main]");
    let bit =
        FIELD_EFFECT_START + index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN;
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.game.set_memory(5);
    assert_eq!(build_action_mask(&r.game, 0)[bit as usize], 1.0);
    r.game.decode_action(bit, 0);
    for kind in [SelectionKind::OwnField, SelectionKind::Hand] {
        assert_eq!(r.pending_kind(), Some(kind));
        let v = r.pending_selection_view().unwrap();
        let pick = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        assert_eq!(build_action_mask(&r.game, 0)[pick as usize], 1.0);
        r.execute_action(0, pick).unwrap();
    }
    let routes = r
        .pending_selection_view()
        .expect("named 3 and colour 4 routes stay a player choice, not an auto-paid 1");
    assert!(!routes.is_optional);
    assert_eq!(routes.valid_action_ids.len(), 2);
    let saved = r.game.clone();
    let mut memories = Vec::new();
    for a in routes.valid_action_ids.clone() {
        r.game = saved.clone();
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.execute_action(0, a).unwrap();
        r.auto_resolve().unwrap();
        assert!(r.game.players[0].battle_area.iter().any(|p| p
            .top_card()
            .card_id(&r.game.card_data)
            == "BT20-025"
            && p.stack_size() == 2));
        memories.push(r.memory());
    }
    memories.sort();
    assert_eq!(memories, vec![3, 4], "pay 4-2=2 or 3-2=1 from 5 memory");
}
