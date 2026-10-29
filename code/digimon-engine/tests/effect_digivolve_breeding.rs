//! Effect evolution onto breeding keeps a zone-pinned declaration through
//! player choices. CR 4.3 3-4-7-3..8 / 8-1-3-3; official Q4300 / Q4389.
use digimon_engine::{
    action::{build_action_mask, space::{encode_breeding_select, BREEDING_TARGET, HAND_EFFECT_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START}},
    card_data::EvoCost,
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardSourceRef, CostDelta, EffectTiming, GamePhase, Keyword, PlaySource},
    permanent::PermanentHandle,
    selection::{SelectionKind, TriggerSource},
};

const SOURCE: &str = r#"
card: BREED-EFFECT
name: Breeding evolution source
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - select_own_breeding_permanent:
          bind_as: host
          prompt: Choose the breeding host
      - select_union_zone:
          of: you
          zones: [hand, trash]
          bind_as: result
          optional: true
          cost: false
          filter: {name_is: Breeding result}
          prompt: Choose a result from hand or trash
      - effect_initiated_digivolve: {target: host, source: result, cost: free}
      - if:
          condition: {effect_digivolved_any_digimon: true}
          then: [{gain_memory: 2}]
      - draw: {of: you, count: 2}
"#;

const RESULT: &str = r#"
card: BREED-RESULT
name: Breeding result
kind: digimon
color: [red]
level: 5
cost: 6
dp: 6000
alt_paths:
  - kind: digivolve
    from: {level_eq: 4, color_is: red}
    cost: 3
effects:
  - when: when_digivolving
    process: [{gain_memory: 7}]
"#;

const OBSERVER: &str = r#"
card: BREED-OBSERVER
name: Ordinary evolution observer
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_digivolve
    process: [{gain_memory: 5}]
"#;

const PAYMENT_OBSERVER: &str = r#"
card: BREED-COST-OBSERVER
name: Ordinary before evolution observers
kind: tamer
color: [red]
cost: 0
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    when_any_ally_digivolves_into: {name_is: Breeding result}
    amount: 1
  - when: before_pay_cost_observe
    active_when: {cost_target: {name_is: Breeding result}}
    process: [{gain_memory: 4}]
"#;

fn fixture(owner: u8, two_routes: bool) -> (DebugRunner, PermanentHandle) {
    let mut host = make_test_card("BREED-HOST", "Breeding host");
    host.colors = vec![CardColor::Red, CardColor::Green];
    host.level = Some(4);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SOURCE).unwrap()
        .from_dsl_yaml(RESULT).unwrap()
        .from_dsl_yaml(OBSERVER).unwrap()
        .from_dsl_yaml(PAYMENT_OBSERVER).unwrap()
        .dsl_card("BT3-103").unwrap()
        .add_card(host)
        .add_card(make_test_card("BREED-FILL", "Filler"))
        .deck(0, &["BREED-FILL"; 20])
        .deck(1, &["BREED-FILL"; 20])
        .start();
    r.skip_mulligan();
    r.set_first_player(owner);
    r.set_phase(GamePhase::Main);
    r.game.set_memory(6);
    r.game.players[0].hand.clear();
    r.game.players[1].hand.clear();
    if two_routes {
        let card = r.game.card_data.iter_mut().find(|c| c.card_id == "BREED-RESULT").unwrap();
        card.evo_costs = [3, 4].into_iter().map(|memory_cost| EvoCost {card_color: 0, level: 4, memory_cost}).collect();
    }
    r.place_in_breeding(owner, "BREED-HOST");
    r.place_on_field(owner, "BREED-OBSERVER", None);
    let source = r.place_on_field(owner, "BREED-EFFECT", None);
    (r, source)
}

fn sentinel(owner: u8) -> PermanentHandle {
    PermanentHandle {player: owner, index: BREEDING_TARGET as u8}
}

fn top(r: &DebugRunner, owner: u8) -> String {
    r.game.players[owner as usize].breeding_area.as_ref().unwrap().top_card().card_id(&r.game.card_data).to_owned()
}

fn park_result(r: &mut DebugRunner, source: PermanentHandle) {
    r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    r.game.drain_effect_queue();
    let id = encode_breeding_select(source.player).unwrap();
    assert_eq!(build_action_mask(&r.game, source.player)[id as usize], 1.0);
    r.execute_action(source.player, id).unwrap();
    assert_eq!(r.pending_selection().unwrap().selecting_player, source.player);
}

#[test]
fn breeding_effect_hand_and_trash_both_owners_draw_and_suppress_ordinary_triggers() {
    for owner in [0, 1] {
        for from_trash in [false, true] {
            let (mut r, source) = fixture(owner, false);
            r.add_to_hand(owner, "BREED-RESULT");
            if from_trash {
                let card = r.game.players[owner as usize].hand.remove(0);
                r.game.players[owner as usize].trash.push(card);
            }
            park_result(&mut r, source);
            let action = if from_trash {TRASH_EFFECT_START} else {PLAY_HAND_START};
            assert_eq!(build_action_mask(&r.game, owner)[action as usize], 1.0);
            r.execute_action(owner, action).unwrap();
            assert_eq!(top(&r, owner), "BREED-RESULT");
            assert_eq!(r.memory(), 8, "only the initiating result tail gains memory; breeding WD/ally observers do not trigger");
            assert_eq!(r.game.players[owner as usize].hand.len(), 3, "one procedure draw, then two unconditional outer-tail draws");
            assert!(r.game.events.iter().any(|event| matches!(event,
                digimon_engine::events::GameEvent::Digivolve {player, field_index, top_card_id, memory_paid, ..}
                    if *player == owner && *field_index == BREEDING_TARGET as u8 && top_card_id == "BREED-RESULT" && *memory_paid == 0)),
                "breeding evolution is visible in the ordinary Digivolve event stream");
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn breeding_effect_decline_runs_only_outer_tail() {
    let (mut r, source) = fixture(0, false);
    r.add_to_hand(0, "BREED-RESULT");
    park_result(&mut r, source);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    r.execute_action(0, PASS).unwrap();
    assert_eq!(top(&r, 0), "BREED-HOST");
    assert_eq!(r.memory(), 6);
    assert_eq!(r.game.players[0].hand.len(), 3);
}

#[test]
fn breeding_selected_host_does_not_retarget_replaced_or_moved_stack() {
    for moved in [false, true] {
        let (mut r, source) = fixture(0, false);
        r.add_to_hand(0, "BREED-RESULT");
        park_result(&mut r, source);
        let original = r.game.players[0].breeding_area.take().unwrap();
        if moved {r.game.players[0].battle_area.push(original);}
        r.place_in_breeding(0, "BREED-HOST");
        let mut clone = r.game.clone();
        r.execute_action(0, PLAY_HAND_START).unwrap();
        clone.decode_action(PLAY_HAND_START, 0);
        for game in [&r.game, &clone] {
            assert_eq!(game.players[0].breeding_area.as_ref().unwrap().top_card().card_id(&game.card_data), "BREED-HOST");
            assert_eq!(game.memory, 6);
            assert_eq!(game.players[0].hand.len(), 3, "only unconditional tail draws after stale target");
        }
    }
}

fn park_route(r: &mut DebugRunner, owner: u8) {
    assert!(!r.game.effect_initiated_digivolve_from_source(owner, CardSourceRef::Hand(owner, 0), sentinel(owner), CostDelta::Reduce(1), false, PlaySource::ByEffect));
    let p = r.pending_selection().expect("distinct breeding routes require a visible choice");
    assert_eq!(p.kind, SelectionKind::EffectChoice);
    assert_eq!(p.valid_action_ids, vec![HAND_EFFECT_START, HAND_EFFECT_START + 1]);
    assert_eq!(build_action_mask(&r.game, owner)[PASS as usize], 0.0);
}

#[test]
fn breeding_cost_route_choice_clones_preserve_distinct_costs_and_draw() {
    let (mut r, _) = fixture(0, true);
    r.add_to_hand(0, "BREED-RESULT");
    park_route(&mut r, 0);
    let mut clone = r.game.clone();
    r.execute_action(0, HAND_EFFECT_START).unwrap();
    clone.decode_action(HAND_EFFECT_START + 1, 0);
    for (game, memory) in [(&r.game, 4), (&clone, 3)] {
        assert_eq!(game.memory, memory);
        assert_eq!(game.players[0].hand.len(), 1);
        assert_eq!(game.players[0].breeding_area.as_ref().unwrap().top_card().card_id(&game.card_data), "BREED-RESULT");
        assert!(game.pending_selection.is_none());
    }
}

#[test]
fn breeding_route_resume_rejects_zone_change_and_replaced_top() {
    for changed_top in [false, true] {
        let (mut r, _) = fixture(0, true);
        r.add_to_hand(0, "BREED-RESULT");
        park_route(&mut r, 0);
        if changed_top {
            r.add_to_hand(0, "BREED-FILL");
            let card = r.game.players[0].hand.pop().unwrap();
            r.game.players[0].breeding_area.as_mut().unwrap().digivolve(card, 1);
        } else {
            let old = r.game.players[0].breeding_area.take().unwrap();
            r.game.players[0].battle_area.push(old);
            r.place_in_breeding(0, "BREED-HOST");
        }
        let mut clone = r.game.clone();
        r.execute_action(0, HAND_EFFECT_START).unwrap();
        clone.decode_action(HAND_EFFECT_START, 0);
        for game in [&r.game, &clone] {
            assert_eq!(game.memory, 6);
            assert_eq!(game.players[0].hand.len(), 1);
            assert_ne!(game.players[0].breeding_area.as_ref().unwrap().top_card().card_id(&game.card_data), "BREED-RESULT");
            assert!(game.pending_selection.is_none());
        }
    }
}

#[test]
fn breeding_route_tracks_physical_result_within_original_source_zone() {
    for changes_zone in [false, true] {
        let (mut r, _) = fixture(0, true);
        r.add_to_hand(0, "BREED-RESULT");
        r.add_to_hand(0, "BREED-FILL");
        park_route(&mut r, 0);
        let physical = r.game.players[0].hand[0].handle();
        if changes_zone {
            let card = r.game.players[0].hand.remove(0);
            r.game.players[0].trash.push(card);
            r.add_to_hand(0, "BREED-RESULT");
        } else {
            r.game.players[0].hand.swap(0, 1);
        }
        r.execute_action(0, HAND_EFFECT_START).unwrap();
        if changes_zone {
            assert_eq!(top(&r, 0), "BREED-HOST");
            assert_eq!(r.memory(), 6);
            assert!(r.game.players[0].trash.iter().any(|c| c.handle() == physical));
        } else {
            assert_eq!(r.game.players[0].breeding_area.as_ref().unwrap().top_card().handle(), physical);
            assert_eq!(r.memory(), 4);
            assert_eq!(r.game.players[0].hand.len(), 2);
        }
    }
}

#[test]
fn breeding_evolution_does_not_trigger_or_consume_hpd_digisorption_or_field_cost_effects() {
    // Digisorption's evolving-host condition is battle-area-only (CR 3-4-7-6;
    // DCGO BT2_045 BeforePayCost); the same applies to ordinary HPD and
    // "when an ally would digivolve" field effects. The initiating -1 remains.
    let (mut r, _) = fixture(0, false);
    r.place_on_field(0, "BREED-COST-OBSERVER", None);
    r.place_on_field(0, "BREED-HOST", None); // a legal suspend payer exists
    r.add_to_hand(0, "BT3-103");
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
    assert!(r.pending_selection().is_none());
    r.game.card_data.iter_mut().find(|c| c.card_id == "BREED-RESULT").unwrap().keywords.push(Keyword::Digisorption(2));
    r.add_to_hand(0, "BREED-RESULT");
    assert!(r.game.effect_initiated_digivolve_from_source(0, CardSourceRef::Hand(0, 0), sentinel(0), CostDelta::Reduce(1), false, PlaySource::ByEffect));
    assert_eq!(r.memory(), 4, "only the printed 3 minus initiating 1 is paid");
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1, "breeding is not HPD's next qualifying evolution");
    assert!(r.game.players[0].battle_area.iter().all(|p| !p.is_suspended));
    assert!(r.pending_selection().is_none());
    assert_eq!(top(&r, 0), "BREED-RESULT");
}

#[test]
fn breeding_evolution_rejects_unsupported_security_source_without_payment_or_movement() {
    use digimon_dsl::compiled::{CompiledCostDelta, CompiledDigivolveCostPredicate, CompiledPredicate};
    use digimon_engine::{
        dsl_cards::{bindings::Bindings, predicate::{eval_predicate_with_bindings, PredicateSubject}},
        effect_context::EffectReadContext,
    };

    let (mut r, _) = fixture(0, false);
    r.add_to_hand(0, "BREED-RESULT");
    let result = r.game.players[0].hand.pop().unwrap();
    let result_handle = result.handle();
    r.game.players[0].security.clear();
    r.game.players[0].security.push(result);
    let before = r.game.verification_digest();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("host", sentinel(0));
    let source = r.game.players[0].breeding_area.as_ref().unwrap().top_card().handle();
    let ctx = EffectReadContext::new(&r.game, source, Some(sentinel(0)), 0);
    for predicate in [
        CompiledPredicate { can_digivolve_onto: Some("host".into()), ..Default::default() },
        CompiledPredicate { can_digivolve_onto_with_cost: Some(CompiledDigivolveCostPredicate {
            target: "host".into(), cost: CompiledCostDelta::Literal(2),
        }), ..Default::default() },
    ] {
        assert!(!eval_predicate_with_bindings(&predicate, &ctx,
            PredicateSubject::Card(result_handle), Some(&bindings)),
            "both route-only and cost-aware candidates must exclude the unsupported source");
    }
    assert!(!r.game.effect_initiated_digivolve_from_source(
        0, CardSourceRef::Security(0, 0), sentinel(0), CostDelta::Fixed(2),
        false, PlaySource::ByEffect,
    ), "security removal has no breeding-aware commit lifecycle yet");
    assert_eq!(r.game.verification_digest(), before,
        "unsupported source must not pay, remove security, evolve another host, or install a pending operation");
}
