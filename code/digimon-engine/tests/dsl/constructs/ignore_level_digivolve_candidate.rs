//! `can_digivolve_onto_with_cost { target, cost, ignore_level: true }` -- the
//! candidate leaf for "may digivolve into [X] ... ignoring level"
//! (G-DSL-CAN-DIGIVOLVE-ONTO-IGNORE-LEVEL).
//!
//! DCGO offers such a card only when `CanPlayCardTargetFrame(target, true, ...,
//! IgnoreRequirement.Level)` holds (EX13_071.cs:357-361): the card must be able
//! to digivolve onto THAT target with only the level waived -- a printed circle
//! of the target's colour (any level) or a level-stripped alt-path. Official
//! ruling Q7449 (Richard Sampson (EX13-071)): only a card that meets the
//! digivolution requirements, its Lv. being the sole exception, may be chosen.
//! A card that fails would be refused at commit (CR 4.3 8-1-2-6), and a card
//! whose cost can't be paid can't be declared (CR 4.3 1-3-11-1), so offering
//! either is a dead choice. (DCGO's filter checks no payment for a digivolve
//! onto a permanent, CardSource.cs:1150-1160; the rules manual governs.)
//!
//! The compile pass (`digimon_dsl::digivolve_candidates`) adds the leaf to the
//! pick that binds an `effect_initiated_digivolve { ignore_level: true }` card,
//! carrying the step's own cost delta, so the payment is checked at the cost
//! the digivolve actually charges.
use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_engine::action::{build_action_mask, space};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;

/// A Lv.3 yellow Digimon: "[On Play] This Digimon may digivolve into a
/// Digimon card in your hand, ignoring level and with the cost reduced by 1."
/// The pick authors no candidate leaf; the compile pass supplies it.
const IMPLICIT: &str = r#"
card: TEST-IGNORE-LEVEL-HOST
name: Ignore level host
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 3000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter: {kind: digimon}
          prompt: This Digimon may digivolve into a card in your hand, ignoring level, cost -1
      - effect_initiated_digivolve:
          target: source
          from_hand: result
          cost: {reduce: 1}
          ignore_level: true
"#;

/// Same effect with the leaf authored explicitly.
const EXPLICIT: &str = r#"
card: TEST-IGNORE-LEVEL-HOST
name: Ignore level host
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 3000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter:
            all_of:
              - kind: digimon
              - can_digivolve_onto_with_cost: {target: source, cost: {reduce: 1}, ignore_level: true}
          prompt: This Digimon may digivolve into a card in your hand, ignoring level, cost -1
      - effect_initiated_digivolve:
          target: source
          from_hand: result
          cost: {reduce: 1}
          ignore_level: true
"#;

/// Lv.6 result whose only printed circle is `color` Lv.5 at `cost`.
fn result(id: &str, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(12000);
    c.colors = vec![CardColor::Yellow];
    c.play_cost = 13;
    c.evo_costs = vec![EvoCost {
        card_color: color as u8,
        level: 5,
        memory_cost: cost,
    }];
    c
}

fn runner(yaml: &str, hand: &[&str], memory: i16) -> DebugRunner {
    let mut cards = vec!["TEST-IGNORE-LEVEL-HOST"];
    cards.extend_from_slice(hand);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("ignore-level candidate syntax must compile")
        // Level waived, yellow circle kept: digivolves onto the yellow host.
        .add_card(result("RES-YELLOW", CardColor::Yellow, 3))
        // Level waived, but the only circle is red: the yellow host can't
        // satisfy it.
        .add_card(result("RES-RED", CardColor::Red, 3))
        // Printed 11, reduced to 10: payable from memory 0, but only with the
        // effect's reduction applied.
        .add_card(result("RES-COST-11", CardColor::Yellow, 11))
        // Printed 12, reduced to 11: not payable from memory 0.
        .add_card(result("RES-COST-12", CardColor::Yellow, 12))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &cards)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(memory);
    r
}

/// Play the host (hand[0], cost 0) so its [On Play] runs.
fn play_host(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[space::PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(space::PLAY_HAND_START, 0);
}

/// The card ids the pending hand pick offers.
fn offered(r: &DebugRunner) -> Vec<String> {
    let Some(v) = r.pending_selection_view() else { return Vec::new() };
    assert!(v.is_optional, "\"may digivolve\" is optional");
    v.valid_action_ids
        .iter()
        .filter(|a| **a != space::PASS)
        .map(|a| r.game.players[0].hand[*a as usize].card_id(&r.game.card_data).to_string())
        .collect()
}

fn host_top(r: &DebugRunner) -> String {
    r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data).to_string()
}

#[test]
fn ignore_level_candidate_waives_only_the_level_not_the_colour() {
    for yaml in [IMPLICIT, EXPLICIT] {
        let mut r = runner(yaml, &["RES-YELLOW", "RES-RED"], 5);
        play_host(&mut r);
        assert_eq!(offered(&r), vec!["RES-YELLOW"], "the red-circle card can't digivolve onto a yellow Digimon");
        let pick = r.pending_selection_view().unwrap().valid_action_ids[0];
        r.execute_action(0, pick).unwrap();
        assert_eq!(host_top(&r), "RES-YELLOW", "Lv.6 onto Lv.3: level ignored");
        assert_eq!(r.memory(), 3, "printed 3 reduced by 1");
    }
}

#[test]
fn ignore_level_candidate_checks_payment_at_the_reduced_cost() {
    for yaml in [IMPLICIT, EXPLICIT] {
        let mut r = runner(yaml, &["RES-COST-11", "RES-COST-12"], 0);
        play_host(&mut r);
        assert_eq!(
            offered(&r),
            vec!["RES-COST-11"],
            "11-1=10 is payable from memory 0; 12-1=11 is not (CR 4.3 1-3-11-1)"
        );
        let pick = r.pending_selection_view().unwrap().valid_action_ids[0];
        r.execute_action(0, pick).unwrap();
        assert_eq!(host_top(&r), "RES-COST-11");
        // 10 memory paid from 0: the gauge sits at the opponent's 10.
        let mine = if r.game.turn_player() == 0 { r.memory() } else { -r.memory() };
        assert_eq!(mine, -10);
    }
}

#[test]
fn ignore_level_candidate_respects_the_effect_digivolve_prohibition() {
    // Waiving the level never waives a prohibiting effect.
    let mut r = runner(IMPLICIT, &["RES-YELLOW"], 5);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotDigivolveDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    play_host(&mut r);
    assert!(offered(&r).is_empty(), "no card can digivolve: nothing to choose");
    assert!(r.pending_selection().is_none());
    assert_eq!(host_top(&r), "TEST-IGNORE-LEVEL-HOST");
}

#[test]
fn ignore_level_compile_pass_emits_the_level_waiving_cost_leaf() {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(IMPLICIT).expect("parses");
    let card = digimon_dsl::compile::compile(&spec).expect("compiles");
    let CompiledClause::Triggered(t) = &card.effects[0] else { panic!("triggered clause") };
    let CompiledStep::SelectHand { filter, .. } = &t.process[0] else { panic!("select_hand") };
    let filter = serde_json::to_value(filter).unwrap();
    assert!(filter["can_digivolve_onto"].is_null(), "the route-only leaf honours the level: {filter}");
    let leaf = &filter["can_digivolve_onto_with_cost"];
    assert_eq!(leaf["target"], "source", "{filter}");
    assert_eq!(leaf["ignore_level"], true, "{filter}");
    assert_eq!(leaf["cost"], serde_json::to_value(match &t.process[1] {
        CompiledStep::EffectInitiatedDigivolve { cost, .. } => cost,
        other => panic!("digivolve step, got {other:?}"),
    }).unwrap(), "the leaf carries the step's own cost delta");
}

// ---- review follow-ups (2026-11-14) ------------------------------------------------------------------------------
//
// 1. A candidate leaf authored on the pick must agree with the digivolve's waiver. The compile pass defers to an
//    authored leaf, so a level-honouring leaf on an `ignore_level` pick (or a level-waiving leaf on a normal pick)
//    would silently change the candidate set. It is a compile error instead. An `ignore_requirements` step is not
//    checked: the compiled flag also comes from `digivolve_without_cost` sugar whose route leaf is the printed
//    requirement check (BT17-097).
// 2. Inside a `cost_reduction` `pay_cost` body the payment probe is forbidden, so an `ignore_level` pick gets the
//    route-only level-waiving leaf `can_digivolve_onto_ignoring_level` there.
// 3. `has_digivolve_candidate { ignore_level: true }`: the host-pick sibling ("your level 4 Digimon can digivolve
//    into ... ignoring level", the BT12-089 shape) waives only the level too.

/// The `IMPLICIT` card with `filter` on its hand pick and `flags` on the digivolve step.
fn with_pick(filter: &str, flags: &str) -> String {
    format!(
        r#"
card: TEST-IGNORE-LEVEL-HOST
name: Ignore level host
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 3000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter: {filter}
          prompt: This Digimon may digivolve into a card in your hand
      - effect_initiated_digivolve:
          target: source
          from_hand: result
          cost: {{reduce: 1}}
{flags}
"#
    )
}

fn compile_errors(yaml: &str) -> Vec<String> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parses");
    match digimon_dsl::compile::compile(&spec) {
        Ok(_) => Vec::new(),
        Err(errors) => errors.into_iter().map(|e| format!("{}: {}", e.path, e.message)).collect(),
    }
}

const IGNORE_LEVEL: &str = "          ignore_level: true";
const IGNORE_REQUIREMENTS: &str = "          ignore_requirements: true";

#[test]
fn ignore_level_pick_with_a_level_honouring_leaf_is_a_compile_error() {
    for filter in [
        "{all_of: [{kind: digimon}, {can_digivolve_onto: source}]}",
        "{can_digivolve_onto_with_cost: {target: source, cost: {reduce: 1}}}",
    ] {
        let errors = compile_errors(&with_pick(filter, IGNORE_LEVEL));
        assert!(
            errors.iter().any(|e| e.contains("ignore_level")),
            "a level-honouring leaf re-imposes the level the digivolve waives ({filter}): {errors:?}"
        );
    }
    // The agreeing leaves compile.
    for filter in [
        "{can_digivolve_onto_with_cost: {target: source, cost: {reduce: 1}, ignore_level: true}}",
        "{all_of: [{kind: digimon}, {can_digivolve_onto_ignoring_level: source}]}",
    ] {
        assert_eq!(compile_errors(&with_pick(filter, IGNORE_LEVEL)), Vec::<String>::new(), "{filter}");
    }
}

#[test]
fn level_honouring_pick_with_a_level_waiving_leaf_is_a_compile_error() {
    for filter in [
        "{can_digivolve_onto_with_cost: {target: source, cost: {reduce: 1}, ignore_level: true}}",
        "{all_of: [{kind: digimon}, {can_digivolve_onto_ignoring_level: source}]}",
    ] {
        let errors = compile_errors(&with_pick(filter, ""));
        assert!(
            errors.iter().any(|e| e.contains("ignore_level")),
            "a level-waiving leaf offers cards the digivolve refuses ({filter}): {errors:?}"
        );
    }
}

#[test]
fn ignore_requirements_pick_keeps_its_authored_route_leaf() {
    // `digivolve_without_cost` sugar compiles to an `ignore_requirements` step while the printed text keeps the
    // requirements (BT17-097's <Delay>); the authored route leaf is then the requirement check and must compile.
    // The sugar always compiles to `cost: free` (compile.rs `digivolve_without_cost`), so that is its shape here.
    let free = with_pick("{can_digivolve_onto: source}", IGNORE_REQUIREMENTS).replace("cost: {reduce: 1}", "cost: free");
    assert_eq!(compile_errors(&free), Vec::<String>::new());
    // A PAID requirement-ignoring step is always printed "ignoring digivolution requirements" (EX13-074, P-092): a
    // route leaf on its pick would hide cards it accepts, so it is a compile error (G-DSL-PAID-DIGIVOLVE-PICK-PAYMENT).
    let errors = compile_errors(&with_pick("{can_digivolve_onto: source}", IGNORE_REQUIREMENTS));
    assert!(errors.iter().any(|e| e.contains("ignore_requirements")), "{errors:?}");
}

/// "Digivolve ignoring level" as a `cost_reduction` clause's payment.
const IN_PAY_COST: &str = r#"
card: TEST-IGNORE-LEVEL-HOST
name: Ignore level host
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 3000
effects:
  - kind: cost_reduction
    active_when: {your_turn: true}
    when_any_ally_played: {kind: digimon}
    optional: true
    amount: 1
    pay_cost:
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter: {kind: digimon}
          prompt: This Digimon may digivolve into a card in your hand, ignoring level
      - effect_initiated_digivolve:
          target: source
          from_hand: result
          cost: {reduce: 1}
          ignore_level: true
"#;

#[test]
fn ignore_level_compile_pass_inside_pay_cost_emits_the_route_only_level_waiving_leaf() {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(IN_PAY_COST).expect("parses");
    let card = digimon_dsl::compile::compile(&spec).expect("compiles");
    let filter = serde_json::to_value(&card.effects[0]).unwrap();
    let text = filter.to_string();
    assert!(
        text.contains("\"can_digivolve_onto_ignoring_level\":\"source\""),
        "the pick inside pay_cost must still be filtered, with the level waived: {text}"
    );
    assert!(!text.contains("\"can_digivolve_onto_with_cost\":{"), "no payment probe inside a reducer: {text}");
}

#[test]
fn can_digivolve_onto_ignoring_level_waives_the_level_and_checks_no_payment() {
    let yaml = with_pick("{all_of: [{kind: digimon}, {can_digivolve_onto_ignoring_level: source}]}", IGNORE_LEVEL);
    let mut r = runner(&yaml, &["RES-YELLOW", "RES-RED"], 5);
    play_host(&mut r);
    assert_eq!(offered(&r), vec!["RES-YELLOW"], "level waived, yellow circle still required");
    // Route-only: a route whose cost can't be paid is still a route. Shown on a pick that feeds no digivolve: a pick
    // that feeds a PAID digivolve also gets the payment leaf from the compile pass
    // (G-DSL-PAID-DIGIVOLVE-PICK-PAYMENT, paid_digivolve_candidate_payment.rs).
    let lone = yaml.split("      - effect_initiated_digivolve:").next().unwrap().to_string();
    let mut r = runner(&lone, &["RES-COST-12"], 0);
    play_host(&mut r);
    assert_eq!(offered(&r), vec!["RES-COST-12"], "the route-only leaf checks no payment");
    let mut r = runner(&yaml, &["RES-COST-12"], 0);
    play_host(&mut r);
    assert!(offered(&r).is_empty(), "the paid digivolve's pick checks the payment: 12-1=11 > 10");
}

/// A host pick filtered by `has_digivolve_candidate`, then the result pick, then an `ignore_level` digivolve.
fn host_pick(candidate: &str) -> String {
    format!(
        r#"
card: TEST-IGNORE-LEVEL-HOST
name: Ignore level host
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 3000
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: host
          optional: true
          filter:
            all_of:
              - kind: digimon
              - has_digivolve_candidate: {candidate}
          prompt: 1 of your Digimon may digivolve into a card in your hand, ignoring level
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter: {{kind: digimon}}
          prompt: Choose the card to digivolve into
      - effect_initiated_digivolve:
          target: host
          from_hand: result
          cost: {{reduce: 1}}
          ignore_level: true
"#
    )
}

#[test]
fn has_digivolve_candidate_ignore_level_waives_only_the_level() {
    let yaml = host_pick("{zone: [hand], ignore_level: true}");
    // A yellow circle (Lv.5) onto the yellow Lv.3 host: a candidate once the level is waived.
    let mut r = runner(&yaml, &["RES-YELLOW"], 5);
    play_host(&mut r);
    let host = r.pending_selection_view().expect("the host has a level-waived candidate");
    let pick = *host.valid_action_ids.iter().find(|a| **a != space::PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    let card = r.pending_selection_view().expect("result pick");
    let pick = *card.valid_action_ids.iter().find(|a| **a != space::PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    assert_eq!(host_top(&r), "RES-YELLOW");
    // A red circle: no candidate even with the level waived, so no host to choose.
    let mut r = runner(&yaml, &["RES-RED"], 5);
    play_host(&mut r);
    assert!(r.pending_selection().is_none(), "the red-circle card can't digivolve onto a yellow Digimon");
}

#[test]
fn has_digivolve_candidate_ignore_level_with_cost_checks_the_reduced_payment() {
    let yaml = host_pick("{zone: [hand], cost: {reduce: 1}, ignore_level: true}");
    let mut r = runner(&yaml, &["RES-COST-11"], 0);
    play_host(&mut r);
    assert!(r.pending_selection().is_some(), "11-1=10 is payable from memory 0");
    let mut r = runner(&yaml, &["RES-COST-12"], 0);
    play_host(&mut r);
    assert!(r.pending_selection().is_none(), "12-1=11 is not payable from memory 0 (CR 4.3 1-3-11-1)");
}

#[test]
fn ignore_level_host_pick_with_a_level_honouring_candidate_leaf_is_a_compile_error() {
    let errors = compile_errors(&host_pick("{zone: [hand]}"));
    assert!(
        errors.iter().any(|e| e.contains("ignore_level")),
        "the host pick's candidate leaf must waive the level the digivolve waives: {errors:?}"
    );
    assert_eq!(compile_errors(&host_pick("{zone: [hand], ignore_level: true}")), Vec::<String>::new());
}
