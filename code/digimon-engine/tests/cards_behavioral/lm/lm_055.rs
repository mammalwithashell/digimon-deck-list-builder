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
        4,
        "expected flood_gate, main_from_hand, delay, and inherited security"
    );

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
            modifier,
            target,
            active_when: Some(active_when),
            ..
        }) if modifier == "IgnoreColorRequirement"
            && target
                .as_ref()
                .and_then(|t| t.card_number_is.as_deref())
                == Some("LM-055")
            && active_when.no_permanent.is_some()
    )));
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
    assert!(*optional, "selecting among revealed cards is optional");
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
    // activating is the §16-16-2 decline.
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
        !optional,
        "hand evolution choice follows the optional base selection"
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
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-055 YAML parses")
        .add_card(green_digimon("BASE", 3))
        .add_card(red_digimon("RED-EVO", 4))
        .add_card(filler("FILL"))
        .hand(0, &["RED-EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "LM-055", Some(0));
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

// G-IGNORE-COLOR-MASK was resolved 2026-05-02 (see qa/resolved-gaps.md).
// LM-055 already exercises the conditional IgnoreColorRequirement via the
// `lm_055_play_from_security_no_copy_on_field_no_payment` path. The
// behavioral coverage of the `no_permanent` suppression branch is the
// implicit complement of those two scenarios (LM-055 plays vs LM-055
// already on field), so dedicated empty-body placeholders are no longer
// useful — they were retired in Phase 2 Track G (2026-05-17).
