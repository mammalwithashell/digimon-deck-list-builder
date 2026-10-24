//! BT20-095's reusable prerequisite: a paid breeding move identifies the
//! permanent that actually moved, for a subsequent optional evolution.
//! CR 4.3 §§15-7-1/2/4/5: the cost must be possible, must succeed before its
//! benefit, and may be paid even when no evolution can follow.

use std::sync::{Arc, Mutex};

use digimon_dsl::compiled::{CompiledClause, CompiledPredicate, CompiledStep};
use digimon_engine::action::{
    build_action_mask,
    space::{encode_breeding_select, BREEDING_TARGET, PASS},
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::{
    bindings::Bindings,
    predicate::{eval_predicate, PredicateSubject},
    step::{run_steps, RunOutcome},
};
use digimon_engine::effect::{CardEffect, Effect, EffectBuilder};
use digimon_engine::effect_context::{EffectContext, EffectReadContext};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::{Permanent, PermanentHandle};
use digimon_engine::{CardKind, EffectTiming, Expiry, GamePhase, ModifierType, TriggerSource};

const MOVE_COST: &str = r#"      - select_own_breeding_permanent:
          bind_as: breeding
          optional: true
          filter: {kind: digimon, level_gte: 3, trait_has: Chronicle, can_move_from_breeding_by_effect: true}
          prompt: Pay by moving the Chronicle Digimon
          then:
            - move_from_breeding: {of: you, bind_as: moved}
            - if:
                condition: {binding_present: moved}
                then:
                  - select_union_zone:
                      of: you
                      zones: [hand, trash]
                      bind_as: evolution
                      optional: true
                      cost: false
                      filter: {kind: digimon, trait_has: Chronicle, can_digivolve_onto: moved}
                      prompt: Evolve exactly the moved Digimon
                  - effect_initiated_digivolve: {target: moved, source: evolution, cost: free}
"#;

const MOVE_AND_SUSPEND: &str = r#"      - move_from_breeding: {of: you, bind_as: moved}
      - suspend: {target: moved}
"#;

const OBSERVER: &str = r#"
card: MOVE-OBSERVER
name: Move Observer
kind: tamer
color: [red]
cost: 3
effects:
  - when: on_move
    process:
      - select_own_permanent:
          bind_as: front
          filter: {card_number_is: MOVE-FRONT}
          prompt: Remove the front slot during the move observer
      - delete_permanent: {target: front}
"#;

fn yaml(body: &str) -> String {
    format!("card: MOVE-SOURCE\nname: Move Source\nkind: tamer\ncolor: [red]\ncost: 3\neffects:\n  - when: on_play\n    process:\n{body}")
}

fn compiled_steps(body: &str) -> Vec<CompiledStep> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml(body)).expect("move syntax parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("move syntax compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("triggered clause")
    };
    clause.process.clone()
}

fn move_predicate(want: bool) -> CompiledPredicate {
    let body = format!(
        "      - if:\n          condition: {{can_move_from_breeding_by_effect: {want}}}\n          then: [{{gain_memory: 1}}]\n"
    );
    let steps = compiled_steps(&body);
    let CompiledStep::If { condition, .. } = &steps[0] else {
        panic!("predicate probe")
    };
    condition.clone()
}

fn validate(body: &str) -> Result<(), Vec<digimon_dsl::errors::ValidationError>> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml(body)).unwrap();
    let raw = digimon_dsl::raw_rust_registry::StubRegistry::empty();
    digimon_dsl::validator::validate(
        &spec,
        &digimon_dsl::validator::ValidationContext { raw_rust: &raw },
    )
}

fn runner(body: &str, owner: u8, observer: bool) -> DebugRunner {
    let mut host = make_test_card("MOVE-HOST", "Chronicle Rookie");
    host.traits = vec!["Chronicle".into()];
    let mut egg = make_test_card("MOVE-EGG", "Chronicle Egg");
    egg.card_kind = CardKind::DigiEgg;
    egg.level = Some(2);
    egg.dp = None;
    egg.traits = vec!["Chronicle".into()];
    let mut result = make_test_card("MOVE-EVOLUTION", "Chronicle Evolution");
    result.level = Some(4);
    result.dp = Some(4000);
    result.traits = vec!["Chronicle".into()];
    result.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 3,
    }];
    let mut wrong = result.clone();
    wrong.card_id = "MOVE-WRONG".into();
    wrong.card_name = "Ineligible Chronicle Evolution".into();
    wrong.evo_costs[0].level = 5;
    let mut front = make_test_card("MOVE-FRONT", "Front Tamer");
    front.card_kind = CardKind::Tamer;
    front.level = None;
    front.dp = None;
    let mut builder = DebugRunner::builder()
        .from_dsl_yaml(&yaml(body))
        .expect("inline move card loads")
        .add_card(host)
        .add_card(egg)
        .add_card(result)
        .add_card(wrong)
        .add_card(front)
        .add_card(make_test_card("MOVE-OTHER", "Non-Chronicle Rookie"));
    if observer {
        builder = builder.from_dsl_yaml(OBSERVER).unwrap();
    }
    let mut r = builder
        .deck(0, &["MOVE-OTHER"; 20])
        .deck(1, &["MOVE-OTHER"; 20])
        .start();
    r.skip_mulligan();
    r.set_first_player(owner);
    r.set_phase(GamePhase::Main);
    r.game.set_memory(5);
    r.game.players[0].hand.clear();
    r.game.players[1].hand.clear();
    r
}

fn start(r: &mut DebugRunner, source: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    r.game.drain_effect_queue();
}

fn choices(r: &DebugRunner, owner: u8, count: usize, optional: bool) -> Vec<u16> {
    let pending = r.game.pending_selection.as_ref().expect("visible choice");
    assert_eq!(pending.selecting_player, owner);
    assert_eq!(pending.is_optional, optional);
    let ids: Vec<_> = pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|id| *id != PASS)
        .collect();
    assert_eq!(ids.len(), count, "{}", pending.prompt);
    for (id, value) in build_action_mask(&r.game, owner).iter().enumerate() {
        assert_eq!(
            *value,
            if ids.contains(&(id as u16)) || (optional && id == PASS as usize) {
                1.0
            } else {
                0.0
            },
            "mask {id}"
        );
    }
    assert!(build_action_mask(&r.game, 1 - owner)
        .iter()
        .all(|v| *v == 0.0));
    ids
}

fn choose(r: &mut DebugRunner, owner: u8, count: usize, optional: bool, index: usize) {
    let action = choices(r, owner, count, optional)[index];
    r.execute_action(owner, action).unwrap();
}

fn locate(r: &DebugRunner, owner: u8, card: CardHandle) -> PermanentHandle {
    let index = r.game.players[owner as usize]
        .battle_area
        .iter()
        .position(|p| p.card_sources.iter().any(|c| c.handle() == card))
        .expect("physical host survives");
    PermanentHandle {
        player: owner,
        index: index as u8,
    }
}

fn top(r: &DebugRunner, host: PermanentHandle) -> &str {
    r.game.players[host.player as usize].battle_area[host.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
}

fn block_move(r: &mut DebugRunner, owner: u8) {
    r.game.modifiers.add(
        PermanentHandle {
            player: owner,
            index: BREEDING_TARGET as u8,
        },
        ModifierEntry::simple(ModifierType::CannotMove, 0, Expiry::Permanent, owner),
    );
}

fn fill_field(r: &mut DebugRunner, owner: u8) {
    while r.game.players[owner as usize].battle_area.len() < r.game.rules.field_slots as usize {
        r.place_on_field(owner, "MOVE-OTHER", None);
    }
}

#[test]
fn move_result_is_declared_for_later_semantic_binding_checks() {
    // Inline fixtures compile without invoking the separate semantic validator.
    // Exercise it explicitly so production YAML can use the result in both a
    // presence guard and a subsequent evolution-candidate filter.
    for body in [MOVE_COST, MOVE_AND_SUSPEND] {
        let validation = validate(body);
        assert!(validation.is_ok(), "{validation:?}");
    }
    for (body, path) in [
        (
            MOVE_COST.replace("binding_present: moved", "binding_present: missing"),
            "binding_present",
        ),
        (
            MOVE_COST.replace("can_digivolve_onto: moved", "can_digivolve_onto: missing"),
            "can_digivolve_onto",
        ),
    ] {
        let errors = validate(&body).expect_err("undeclared result must be rejected");
        assert!(
            errors
                .iter()
                .any(|error| { error.path.contains(path) && error.message.contains("missing") }),
            "{errors:?}"
        );
    }
}

#[test]
fn move_eligibility_requires_the_actual_breeding_subject_for_either_boolean() {
    for owner in [0, 1] {
        let mut r = runner(MOVE_AND_SUSPEND, owner, false);
        let source = r.place_on_field(owner, "MOVE-SOURCE", None);
        let field = r.place_on_field(owner, "MOVE-HOST", None);
        let breeding_card = r.place_in_breeding(owner, "MOVE-HOST").card;
        let hand_index = r.add_to_hand(owner, "MOVE-HOST");
        let hand_card = r.game.players[owner as usize].hand[hand_index].handle();
        let source_card = r.top_card(source);
        let rctx = EffectReadContext::new(&r.game, source_card, Some(source), owner);
        for want in [false, true] {
            let predicate = move_predicate(want);
            for subject in [
                PredicateSubject::Permanent(field),
                PredicateSubject::Card(hand_card),
                PredicateSubject::RevealedCard(hand_card),
                PredicateSubject::Card(breeding_card),
                PredicateSubject::None,
                PredicateSubject::BreedingPermanent(1 - owner),
                PredicateSubject::Permanent(PermanentHandle {
                    player: 1 - owner,
                    index: BREEDING_TARGET as u8,
                }),
            ] {
                assert!(
                    !eval_predicate(&predicate, &rctx, subject),
                    "{subject:?} is not an existing breeding permanent, want {want}, owner {owner}"
                );
            }
        }
    }
}

#[test]
fn move_eligibility_false_means_an_existing_breeder_whose_move_is_unavailable() {
    for owner in [0, 1] {
        for mode in ["legal", "cannot_move", "full_field", "egg"] {
            let mut r = runner(MOVE_AND_SUSPEND, owner, false);
            let source = r.place_on_field(owner, "MOVE-SOURCE", None);
            r.place_in_breeding(
                owner,
                if mode == "egg" {
                    "MOVE-EGG"
                } else {
                    "MOVE-HOST"
                },
            );
            // The opposite side remains legal. A probe must consult the
            // subject's owner rather than the effect controller's breeding slot.
            r.place_in_breeding(1 - owner, "MOVE-HOST");
            if mode == "cannot_move" {
                block_move(&mut r, owner);
            } else if mode == "full_field" {
                fill_field(&mut r, owner);
            }
            let source_card = r.top_card(source);
            let rctx = EffectReadContext::new(&r.game, source_card, Some(source), owner);
            for subject_owner in [owner, 1 - owner] {
                let can_move = subject_owner != owner || mode == "legal";
                for subject in [
                    PredicateSubject::BreedingPermanent(subject_owner),
                    PredicateSubject::Permanent(PermanentHandle {
                        player: subject_owner,
                        index: BREEDING_TARGET as u8,
                    }),
                ] {
                    for want in [false, true] {
                        assert_eq!(
                            eval_predicate(&move_predicate(want), &rctx, subject),
                            can_move == want,
                            "mode {mode}, {subject:?}, want {want}, controller {owner}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn breeding_move_cost_filter_exposes_only_legal_own_lv3_chronicle_hosts() {
    for owner in [0, 1] {
        for mode in [
            "legal",
            "egg",
            "wrong_trait",
            "empty",
            "opponent_only",
            "cannot_move",
            "full_field",
        ] {
            let mut r = runner(MOVE_COST, owner, false);
            let source = r.place_on_field(owner, "MOVE-SOURCE", None);
            match mode {
                "empty" => {}
                "opponent_only" => {
                    r.place_in_breeding(1 - owner, "MOVE-HOST");
                }
                "egg" => {
                    r.place_in_breeding(owner, "MOVE-EGG");
                }
                "wrong_trait" => {
                    r.place_in_breeding(owner, "MOVE-OTHER");
                }
                _ => {
                    r.place_in_breeding(owner, "MOVE-HOST");
                }
            }
            if mode == "cannot_move" {
                block_move(&mut r, owner);
            }
            if mode == "full_field" {
                fill_field(&mut r, owner);
            }
            start(&mut r, source);
            if mode == "legal" {
                assert_eq!(
                    choices(&r, owner, 1, true),
                    vec![encode_breeding_select(owner).unwrap()]
                );
                r.execute_action(owner, PASS).unwrap();
                assert!(r.game.players[owner as usize].breeding_area.is_some());
            } else {
                assert!(
                    r.game.pending_selection.is_none(),
                    "unpayable {mode}, owner {owner}"
                );
            }
            assert_eq!(r.memory(), 5);
        }
    }
}

#[test]
fn move_result_targets_only_the_actual_promoted_stack() {
    for owner in [0, 1] {
        let mut r = runner(MOVE_AND_SUSPEND, owner, false);
        let decoy = r.place_on_field(owner, "MOVE-HOST", None);
        let source = r.place_on_field(owner, "MOVE-SOURCE", None);
        let moved = r.place_in_breeding(owner, "MOVE-HOST").card;
        let source_card = r.top_card(source);
        let steps = compiled_steps(MOVE_AND_SUSPEND);
        let mut bindings = Bindings::new();
        let outcome = run_steps(
            &steps,
            &mut EffectContext::new(&mut r.game, source_card, Some(source), owner),
            &mut bindings,
        );
        assert_eq!(outcome, RunOutcome::Synchronous);
        assert!(r.game.players[owner as usize].breeding_area.is_none());
        assert!(
            r.game.players[owner as usize].battle_area[locate(&r, owner, moved).index as usize]
                .is_suspended
        );
        assert!(!r.game.players[owner as usize].battle_area[decoy.index as usize].is_suspended);
        assert!(
            bindings.get("moved").is_some(),
            "successful promotion creates its result"
        );
    }
}

#[test]
fn move_result_free_hand_or_trash_evolution_keeps_other_eligible_hosts_unchanged() {
    for owner in [0, 1] {
        for trash in [false, true] {
            let mut r = runner(MOVE_COST, owner, false);
            let decoy = r.place_on_field(owner, "MOVE-HOST", None);
            let source = r.place_on_field(owner, "MOVE-SOURCE", None);
            let moved = r.place_in_breeding(owner, "MOVE-HOST").card;
            if trash {
                r.inject_trash(owner, "MOVE-EVOLUTION");
            } else {
                r.add_to_hand(owner, "MOVE-EVOLUTION");
            }
            r.add_to_hand(owner, "MOVE-WRONG");
            start(&mut r, source);
            choose(&mut r, owner, 1, true, 0);
            assert!(r.game.players[owner as usize].breeding_area.is_none());
            choose(&mut r, owner, 1, true, 0);
            assert_eq!(top(&r, locate(&r, owner, moved)), "MOVE-EVOLUTION");
            assert_eq!(top(&r, decoy), "MOVE-HOST");
            assert_eq!(
                r.memory(),
                5,
                "free effect evolution, owner {owner}, trash {trash}"
            );
            assert!(r.game.players[owner as usize]
                .hand
                .iter()
                .any(|c| c.card_id(&r.game.card_data) == "MOVE-WRONG"));
            assert!(r.game.pending_selection.is_none());
        }
    }
}

#[test]
fn moving_is_payable_without_an_evolution_and_evolution_can_be_declined() {
    for have_result in [false, true] {
        let mut r = runner(MOVE_COST, 0, false);
        let source = r.place_on_field(0, "MOVE-SOURCE", None);
        let moved = r.place_in_breeding(0, "MOVE-HOST").card;
        if have_result {
            r.add_to_hand(0, "MOVE-EVOLUTION");
        }
        start(&mut r, source);
        choose(&mut r, 0, 1, true, 0);
        if have_result {
            choices(&r, 0, 1, true);
            r.execute_action(0, PASS).unwrap();
        }
        assert!(r.game.players[0].breeding_area.is_none());
        assert_eq!(top(&r, locate(&r, 0, moved)), "MOVE-HOST");
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.memory(), 5);
    }
}

#[test]
fn move_rechecks_payment_after_a_parked_choice_before_binding_its_benefit() {
    for mode in ["cannot_move", "full_field"] {
        let mut r = runner(MOVE_COST, 0, false);
        let source = r.place_on_field(0, "MOVE-SOURCE", None);
        let moved = r.place_in_breeding(0, "MOVE-HOST").card;
        r.add_to_hand(0, "MOVE-EVOLUTION");
        start(&mut r, source);
        let action = choices(&r, 0, 1, true)[0];
        if mode == "cannot_move" {
            block_move(&mut r, 0);
        } else {
            fill_field(&mut r, 0);
        }
        r.execute_action(0, action).unwrap();
        assert_eq!(
            r.game.players[0]
                .breeding_area
                .as_ref()
                .unwrap()
                .top_card()
                .handle(),
            moved
        );
        assert!(
            r.game.pending_selection.is_none(),
            "unpaid move cannot offer evolution: {mode}"
        );
        assert_eq!(r.game.players[0].hand.len(), 1);
        assert_eq!(r.memory(), 5);
    }
}

#[test]
fn failed_second_move_clears_a_previous_binding_instead_of_reusing_success() {
    let body = r#"      - move_from_breeding: {of: you, bind_as: moved}
      - move_from_breeding: {of: you, bind_as: moved}
      - if:
          condition: {binding_present: moved}
          then: [{gain_memory: 3}]
"#;
    let mut r = runner(body, 0, false);
    let source = r.place_on_field(0, "MOVE-SOURCE", None);
    r.place_in_breeding(0, "MOVE-HOST");
    let card = r.top_card(source);
    let mut bindings = Bindings::new();
    run_steps(
        &compiled_steps(body),
        &mut EffectContext::new(&mut r.game, card, Some(source), 0),
        &mut bindings,
    );
    assert!(r.game.players[0].breeding_area.is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert!(
        bindings.get("moved").is_none(),
        "failed call publishes an absent result"
    );
    assert_eq!(
        r.memory(),
        5,
        "an earlier successful move cannot pay a later failed condition"
    );
}

#[test]
fn moved_identity_survives_compaction_and_clone_during_evolution_choice() {
    let mut r = runner(MOVE_COST, 0, false);
    r.place_on_field(0, "MOVE-FRONT", None);
    let source = r.place_on_field(0, "MOVE-SOURCE", None);
    let moved = r.place_in_breeding(0, "MOVE-HOST").card;
    r.add_to_hand(0, "MOVE-EVOLUTION");
    start(&mut r, source);
    choose(&mut r, 0, 1, true, 0);
    let action = choices(&r, 0, 1, true)[0];
    let front = r.game.players[0].battle_area.remove(0);
    r.game.players[0].trash.extend(front.card_sources);
    let parked = r.game.clone();
    let mut digest = None;
    for cloned in [false, true] {
        if cloned {
            r.game = parked.clone();
        }
        r.execute_action(0, action).unwrap();
        assert_eq!(top(&r, locate(&r, 0, moved)), "MOVE-EVOLUTION");
        assert!(r.game.pending_selection.is_none());
        let actual = r.game.verification_digest();
        if let Some(expected) = digest {
            assert_eq!(actual, expected);
        }
        digest = Some(actual);
    }
}

#[test]
fn departed_moved_host_never_retargets_a_replacement_in_its_slot() {
    let mut r = runner(MOVE_COST, 0, false);
    let source = r.place_on_field(0, "MOVE-SOURCE", None);
    let moved = r.place_in_breeding(0, "MOVE-HOST").card;
    r.add_to_hand(0, "MOVE-EVOLUTION");
    start(&mut r, source);
    choose(&mut r, 0, 1, true, 0);
    let action = choices(&r, 0, 1, true)[0];
    let h = locate(&r, 0, moved);
    let gone = r.game.players[0].battle_area.remove(h.index as usize);
    r.game.players[0].trash.extend(gone.card_sources);
    let replacement = r.place_on_field(0, "MOVE-HOST", None);
    assert_eq!(replacement, h);
    let parked = r.game.clone();
    for cloned in [false, true] {
        if cloned {
            r.game = parked.clone();
        }
        r.execute_action(0, action).unwrap();
        assert_eq!(top(&r, replacement), "MOVE-HOST");
        assert_eq!(
            r.game.players[0].hand.len(),
            1,
            "the evolution card remains in hand"
        );
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.memory(), 5);
    }
}

#[test]
fn moved_binding_survives_a_parked_move_observer_then_compaction_and_clone() {
    let mut r = runner(MOVE_AND_SUSPEND, 0, true);
    r.place_on_field(0, "MOVE-FRONT", None);
    let source = r.place_on_field(0, "MOVE-SOURCE", None);
    r.place_on_field(0, "MOVE-OBSERVER", None);
    let moved = r.place_in_breeding(0, "MOVE-HOST").card;
    let source_card = r.top_card(source);
    let mut bindings = Bindings::new();
    // Exercise the reusable operation without a surrounding deferred-drain
    // scope: its OnMove observer really parks before the operation returns.
    let outcome = run_steps(
        &compiled_steps(MOVE_AND_SUSPEND),
        &mut EffectContext::new(&mut r.game, source_card, Some(source), 0),
        &mut bindings,
    );
    assert_eq!(outcome, RunOutcome::Parked);
    assert!(r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .prompt
        .contains("front slot"));
    let action = choices(&r, 0, 1, false)[0];
    let parked = r.game.clone();
    let mut digest = None;
    for cloned in [false, true] {
        if cloned {
            r.game = parked.clone();
        }
        r.execute_action(0, action).unwrap();
        assert!(r.game.players[0].battle_area[locate(&r, 0, moved).index as usize].is_suspended);
        assert!(r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "MOVE-FRONT"));
        assert!(r.game.pending_selection.is_none());
        let actual = r.game.verification_digest();
        if let Some(expected) = digest {
            assert_eq!(actual, expected);
        }
        digest = Some(actual);
    }
}

#[test]
fn moved_route_only_evolution_pick_follows_compaction_after_a_parked_observer() {
    let body = r#"      - move_from_breeding: {of: you, bind_as: moved}
      - select_union_zone:
          of: you
          zones: [hand, trash]
          bind_as: evolution
          optional: true
          cost: false
          filter: {kind: digimon, can_digivolve_onto: moved}
          prompt: Evolve the moved host after the observer
      - effect_initiated_digivolve: {target: moved, source: evolution, cost: free}
"#;
    for owner in [0, 1] {
        let mut r = runner(body, owner, true);
        r.place_on_field(owner, "MOVE-FRONT", None);
        let source = r.place_on_field(owner, "MOVE-SOURCE", None);
        r.place_on_field(owner, "MOVE-OBSERVER", None);
        let moved = r.place_in_breeding(owner, "MOVE-HOST").card;
        r.add_to_hand(owner, "MOVE-EVOLUTION");
        let source_card = r.top_card(source);
        let outcome = run_steps(
            &compiled_steps(body),
            &mut EffectContext::new(&mut r.game, source_card, Some(source), owner),
            &mut Bindings::new(),
        );
        assert_eq!(outcome, RunOutcome::Parked);
        assert!(r.pending_selection().unwrap().prompt.contains("front slot"));
        let remove_front = choices(&r, owner, 1, false)[0];
        let parked = r.game.clone();
        let mut digest = None;
        for cloned in [false, true] {
            if cloned {
                r.game = parked.clone();
            }
            // The result selector has not been installed yet: its predicate
            // must resolve the moved binding after this observer compacts it.
            r.execute_action(owner, remove_front).unwrap();
            assert_eq!(locate(&r, owner, moved).index, 2);
            assert_eq!(top(&r, locate(&r, owner, moved)), "MOVE-HOST");
            let evolution = choices(&r, owner, 1, true)[0];
            assert!(r.pending_selection().unwrap().prompt.contains("after the observer"));
            r.execute_action(owner, evolution).unwrap();
            assert_eq!(top(&r, locate(&r, owner, moved)), "MOVE-EVOLUTION");
            assert_eq!(r.memory(), 5, "the route-only gate leaves the free cost to the effect");
            assert!(r.pending_selection().is_none());
            let actual = r.game.verification_digest();
            if let Some(expected) = digest {
                assert_eq!(actual, expected);
            }
            digest = Some(actual);
        }
    }
}

struct ReplayMovedHostBeforeMoveReturns(Arc<Mutex<bool>>);
impl CardEffect for ReplayMovedHostBeforeMoveReturns {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![EffectBuilder::new(card, EffectTiming::OnMove)
            .process(move |ctx| {
                let moved = ctx
                    .game
                    .current_trigger_context
                    .as_ref()
                    .unwrap()
                    .event_permanent
                    .unwrap();
                // Model a departure followed by replay of that same physical card:
                // matching the card or its old field slot must not match this new
                // permanent incarnation. The observer runs inside the move call.
                let departed = ctx.game.players[moved.player as usize]
                    .battle_area
                    .remove(moved.index as usize);
                let replayed = Permanent::new(departed.top_card().clone(), ctx.game.turn_count);
                ctx.game.players[moved.player as usize]
                    .battle_area
                    .insert(moved.index as usize, replayed);
                *seen.lock().unwrap() = true;
            })
            .build()]
    }
}

#[test]
fn promoted_identity_is_captured_before_an_observer_replays_the_same_physical_card() {
    let body = MOVE_AND_SUSPEND;
    let mut r = runner(body, 0, true);
    let source = r.place_on_field(0, "MOVE-SOURCE", None);
    r.place_on_field(0, "MOVE-OBSERVER", None);
    let moved = r.place_in_breeding(0, "MOVE-HOST").card;
    let seen = Arc::new(Mutex::new(false));
    r.register_effect(
        "MOVE-OBSERVER",
        Arc::new(ReplayMovedHostBeforeMoveReturns(seen.clone())),
    );
    let card = r.top_card(source);
    run_steps(
        &compiled_steps(body),
        &mut EffectContext::new(&mut r.game, card, Some(source), 0),
        &mut Bindings::new(),
    );
    assert!(
        *seen.lock().unwrap(),
        "the observer ran before move_from_breeding returned"
    );
    assert!(
        !r.game.players[0].battle_area[locate(&r, 0, moved).index as usize].is_suspended,
        "the binding is the moved incarnation, not the physical card replayed by the observer"
    );
}

#[test]
fn battle_compaction_preserves_breeding_restrictions_and_source_attribution() {
    for owner in [0, 1] {
        let mut r = runner(MOVE_AND_SUSPEND, owner, false);
        r.place_in_breeding(owner, "MOVE-HOST");
        r.place_on_field(owner, "MOVE-FRONT", None);
        r.place_on_field(owner, "MOVE-FRONT", None);
        let survivor = r.place_on_field(owner, "MOVE-OTHER", None);
        let other = r.place_on_field(1 - owner, "MOVE-OTHER", None);
        let breeding = PermanentHandle {
            player: owner,
            index: BREEDING_TARGET as u8,
        };
        let mut restriction =
            ModifierEntry::simple(ModifierType::CannotMove, 0, Expiry::Permanent, owner);
        restriction.source_permanent = Some(breeding);
        r.game.modifiers.add(breeding, restriction);
        let mut bonus =
            ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::Permanent, owner);
        bonus.source_permanent = Some(breeding);
        r.game.modifiers.add(survivor, bonus.clone());
        r.game.modifiers.add(other, bonus);

        for expected_index in [1, 0] {
            r.game.delete_permanent_with_effects(PermanentHandle {
                player: owner,
                index: 0,
            });
            r.game.drain_effect_queue();
            assert!(!r.game.can_move_from_breeding_by_effect(owner));
            let restriction = r.game.modifiers.get(breeding, ModifierType::CannotMove);
            assert_eq!(restriction.len(), 1);
            assert_eq!(restriction[0].source_permanent, Some(breeding));
            for target in [
                PermanentHandle {
                    player: owner,
                    index: expected_index,
                },
                other,
            ] {
                let bonus = r.game.modifiers.get(target, ModifierType::ChangeDp);
                assert_eq!(
                    bonus.len(),
                    1,
                    "battle modifiers follow their surviving hosts"
                );
                assert_eq!(bonus[0].source_permanent, Some(breeding));
            }
        }
        assert!(!r.game.move_from_breeding_by_effect(owner));
        assert!(r.game.players[owner as usize].breeding_area.is_some());
    }
}
