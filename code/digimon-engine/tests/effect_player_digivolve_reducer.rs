//! BT3-103 official Q1140: its next-evolution reducer applies to effect evolution.
//! Sources: official observation 013e0883fe99052b207ae80d4ae1faef3112328c3c2448ecebf34b5d9927a6d4,
//! actual BT3-103 image, general_rule 8-1-2-1/4 and 8-1-3-1/3;
//! DCGO BT3_103.cs player BeforePayCost observer and AfterPayCost removal.
//! This is the player-scoped reducer, not a field BeforePayCost fixture.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS, PLAY_HAND_START},
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CostDelta, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn fixture(routes: &[u16]) -> (DebugRunner, PermanentHandle, CardHandle) {
    fixture_with_nested_payment(routes, false)
}

fn fixture_with_nested_payment(
    routes: &[u16],
    nested: bool,
) -> (DebugRunner, PermanentHandle, CardHandle) {
    let mut host = make_test_card("HPD-HOST", "Green Host");
    host.level = Some(4);
    host.colors = vec![CardColor::Green];
    let mut result = make_test_card("HPD-RESULT", "Effect Result");
    result.level = Some(5);
    result.colors = vec![CardColor::Red];
    result.evo_costs = routes
        .iter()
        .map(|&memory_cost| EvoCost {
            card_color: 3,
            level: 4,
            memory_cost,
        })
        .collect();
    let mut inner_result = make_test_card("HPD-INNER", "Inner Result");
    inner_result.level = Some(4);
    inner_result.colors = vec![CardColor::Red];
    inner_result.evo_costs = [1, 2]
        .iter()
        .map(|&memory_cost| EvoCost {
            card_color: 0,
            level: 3,
            memory_cost,
        })
        .collect();
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-103")
        .unwrap()
        .from_dsl_yaml(if nested {
            r#"
card: HPD-FIELD-REDUCER
name: Nested field payment
kind: tamer
color: [white]
cost: 1
effects:
  - kind: cost_reduction
    summary: Nested field payment
    reduction_timing: before_pay_cost
    when_any_ally_digivolves_into: { name_is: Effect Result }
    amount: 1
    pay_cost:
      - select_own_permanent:
          filter: { name_is: Red spare, is_suspended: false }
          bind_as: fee
          prompt: Choose the nested evolution host
      - select_hand:
          of: you
          filter: { name_is: Inner Result }
          bind_as: inner
          prompt: Choose the nested evolution result
      - effect_initiated_digivolve:
          target: fee
          from_hand: inner
          cost: { reduce: 0 }
      - if:
          condition: { effect_digivolved_any_digimon: true }
          then:
            - draw: { of: you, count: 4 }
"#
        } else {
            r#"
card: HPD-FIELD-REDUCER
name: Nested field payment
kind: tamer
color: [white]
cost: 1
effects:
  - kind: cost_reduction
    summary: Nested field payment
    reduction_timing: before_pay_cost
    when_any_ally_digivolves_into: { name_is: Effect Result }
    amount: 1
    pay_cost:
      - select_own_permanent:
          filter: { name_is: Red spare, is_suspended: false }
          bind_as: fee
          prompt: Choose the separate field reducer payment
      - suspend: { target: fee }
"#
        })
        .unwrap()
        .from_dsl_yaml(
            r#"
card: HPD-HOST
name: Green Host
kind: digimon
level: 4
color: [green]
cost: 5
dp: 5000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: evo
          filter: { name_is: Effect Result }
          prompt: Choose the effect evolution result
      - if:
          condition: { binding_present: evo }
          then:
            - effect_initiated_digivolve:
                target: source
                from_hand: evo
                cost: { reduce: 3 }
            - if:
                condition: { effect_digivolved_any_digimon: true }
                then:
                  - draw: { of: you, count: 2 }
      - draw: { of: you, count: 3 }
"#,
        )
        .unwrap()
        .from_dsl_yaml(
            r#"
card: HPD-OBSERVER
name: Evolution provenance witness
kind: tamer
color: [white]
cost: 1
effects:
  - when: on_digivolve
    condition:
      all_of:
        - event_card_name_is: Effect Result
        - event_is_effect_initiated: true
        - event_target_owner: you
    process:
      - draw: { of: you, count: 1 }
"#,
        )
        .unwrap()
        .add_card(host)
        .add_card(result)
        .add_card(inner_result)
        // The shared test store fingerprints evo_costs by length. Distinct
        // fixture configurations need distinct store keys, even when their
        // synthetic card IDs and number of routes are identical.
        .add_card(make_test_card(
            &format!("HPD-FIXTURE-{routes:?}-{nested}"),
            "Unused fixture key",
        ))
        .add_card(make_test_card("HPD-SPARE", "Red spare"))
        .hand(0, &["BT3-103", "HPD-SPARE", "HPD-RESULT"])
        .deck(0, &["HPD-SPARE"; 20])
        .deck(1, &["HPD-SPARE"; 20])
        .memory(10)
        .start();
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "HPD-OBSERVER", Some(0));
    let host = r.place_on_field(0, "HPD-HOST", Some(0));
    r.place_on_field(0, "HPD-HOST", Some(0));
    r.place_on_field(0, "HPD-SPARE", Some(0));
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "BT3-103"));
    let result = r.game.players[0]
        .hand
        .iter()
        .find(|c| c.card_id(&r.game.card_data) == "HPD-RESULT")
        .unwrap()
        .handle();
    let actual = r.game.players[0]
        .hand
        .iter()
        .find(|c| c.handle() == result)
        .unwrap();
    assert_eq!(
        actual
            .digivolution_costs(&r.game.card_data)
            .iter()
            .map(|c| c.memory_cost)
            .collect::<Vec<_>>(),
        routes
    );
    (r, host, result)
}

fn start(r: &mut DebugRunner, host: PermanentHandle, result: CardHandle) {
    let index = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.handle() == result)
        .unwrap();
    r.game.effect_initiated_digivolve(
        0,
        index,
        host,
        CostDelta::Reduce(3),
        false,
        PlaySource::ByEffect,
    );
}
fn act(r: &mut DebugRunner, action: u16) {
    let view = r
        .pending_selection_view()
        .expect("legal player-visible reducer choice");
    assert_eq!(view.selecting_player, 0);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.execute_action(0, action).unwrap();
}
fn reducer_accept(r: &DebugRunner) -> u16 {
    let v = r
        .pending_selection_view()
        .expect("Q1140 effect evolution must offer armed reducer");
    assert_eq!(v.kind, SelectionKind::EffectChoice);
    assert!(v.is_optional, "the suspension payment is optional");
    assert_eq!(v.selecting_player, 0);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    assert_eq!(
        r.memory(),
        10,
        "cost cannot be paid before the reducer choice"
    );
    *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap()
}
fn suspend(r: &mut DebugRunner, index: u16) {
    let v = r
        .pending_selection_view()
        .expect("choose one own Digimon to suspend");
    assert_eq!(v.kind, SelectionKind::OwnField);
    for slot in [1, 2, 3] {
        assert!(v.valid_action_ids.contains(&encode_attack(0, slot)));
    }
    assert!(
        !v.valid_action_ids.contains(&encode_attack(0, 0)),
        "the witness Tamer cannot pay"
    );
    act(r, encode_attack(0, index));
}

#[test]
fn effect_evolution_accept_decline_and_both_parked_clones_consume_next_reducer() {
    let (mut base, host, result) = fixture(&[6]);
    let hand_before = base.hand_size(0);
    start(&mut base, host, result);
    let yes = reducer_accept(&base);
    for accepted in [false, true] {
        let mut r = DebugRunner::wrap(base.game.clone());
        act(&mut r, if accepted { yes } else { PASS });
        if accepted {
            let mut parked_clone = DebugRunner::wrap(r.game.clone());
            suspend(&mut parked_clone, 1);
            assert!(
                parked_clone.game.players[0].battle_area[1].is_suspended,
                "evolved host may pay its own cost and retains suspension"
            );
            assert_eq!(parked_clone.memory(), 10);
            suspend(&mut r, 3);
        }
        assert_eq!(r.memory(), if accepted { 10 } else { 7 });
        assert_eq!(r.game.players[0].battle_area[1].top_card().handle(), result);
        assert_eq!(r.game.players[0].battle_area[3].is_suspended, accepted);
        assert_eq!(
            r.hand_size(0),
            hand_before + 1,
            "one evolution draw plus one effect-origin witness draw"
        );
        assert!(
            r.game.player_digivolve_cost_reducers.is_empty(),
            "next-event reducer expires after accept OR decline"
        );
        assert!(r.pending_selection().is_none());
        let next = r.add_to_hand(0, "HPD-RESULT");
        let next = r.game.players[0].hand[next].handle();
        start(
            &mut r,
            PermanentHandle {
                player: 0,
                index: 2,
            },
            next,
        );
        assert!(
            r.pending_selection().is_none(),
            "no second reducer use this turn"
        );
        assert_eq!(r.memory(), if accepted { 7 } else { 4 });
    }
}

#[test]
fn effect_route_choice_keeps_selected_printed_cost_through_player_reducer() {
    let (mut base, host, result) = fixture(&[11, 12]);
    start(&mut base, host, result);
    let v = base
        .pending_selection_view()
        .expect("choose legal printed requirement first");
    assert_eq!(v.kind, SelectionKind::EffectChoice);
    assert!(!v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 2);
    assert_eq!(build_action_mask(&base.game, 0)[PASS as usize], 0.0);
    for (i, action) in v.valid_action_ids.clone().into_iter().enumerate() {
        let mut r = DebugRunner::wrap(base.game.clone());
        act(&mut r, action);
        let yes = reducer_accept(&r);
        let mut branch = DebugRunner::wrap(r.game.clone());
        act(&mut branch, yes);
        suspend(&mut branch, 3);
        assert_eq!(
            branch.memory(),
            7 - i as i16,
            "printed 11/12, effect -3, HPD -5"
        );
        assert_eq!(
            branch.game.players[0].battle_area[1].top_card().handle(),
            result
        );
        assert!(branch.game.player_digivolve_cost_reducers.is_empty());
    }
}

#[test]
fn parked_effect_reducer_tracks_host_and_hand_identity_through_index_shifts() {
    let (mut r, host, result) = fixture(&[6]);
    start(&mut r, host, result);
    let yes = reducer_accept(&r);
    // Remove an earlier unrelated hand card and swap two same-name hosts:
    // neither change may redirect the declared evolution to a different copy.
    let removed = r.game.players[0].hand.remove(0);
    assert_ne!(removed.handle(), result);
    r.game.players[0].trash.push(removed);
    r.game.players[0].battle_area.swap(1, 2);
    act(&mut r, yes);
    suspend(&mut r, 3);
    assert_eq!(r.game.players[0].battle_area[2].top_card().handle(), result);
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        "HPD-HOST"
    );
    assert_eq!(r.memory(), 10);
}

#[test]
fn parked_effect_reducer_cannot_substitute_same_id_card_after_declared_card_leaves() {
    let (mut r, host, result) = fixture(&[6]);
    start(&mut r, host, result);
    let yes = reducer_accept(&r);
    let index = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.handle() == result)
        .unwrap();
    let removed = r.game.players[0].hand.remove(index);
    r.game.players[0].trash.push(removed);
    let replacement = r.add_to_hand(0, "HPD-RESULT");
    let replacement = r.game.players[0].hand[replacement].handle();
    act(&mut r, yes);
    if r.pending_selection().is_some() {
        suspend(&mut r, 3);
    }
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        "HPD-HOST"
    );
    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|c| c.handle() == replacement));
    assert_eq!(r.memory(), 10, "stale declared evolution cannot pay memory");
}

fn start_dsl(r: &mut DebugRunner, host: PermanentHandle) {
    r.fire_play_event_triggers(0, host.index as usize, true, false);
    let v = r
        .pending_selection_view()
        .expect("DSL chooses its bound evolution card");
    assert_eq!(v.kind, SelectionKind::Hand);
    assert_eq!(v.valid_action_ids.len(), 1);
    act(r, v.valid_action_ids[0]);
}

fn assert_dsl_success(r: &DebugRunner, result: CardHandle, before_hand: usize, before_deck: usize) {
    assert_eq!(r.game.players[0].battle_area[1].top_card().handle(), result);
    assert_eq!(r.hand_size(0), before_hand + 6,
        "result leaves hand; procedure +1, provenance witness +1, success-only tail +2, unconditional tail +3");
    assert_eq!(
        r.game.players[0].deck.len(),
        before_deck - 7,
        "all three tails and the rule draw execute exactly once"
    );
    assert!(r.pending_selection().is_none());
}

#[test]
fn dsl_evolution_tail_survives_optional_and_suspend_parking_on_original_and_clone() {
    let (mut base, host, result) = fixture(&[6]);
    let hand = base.hand_size(0);
    let deck = base.game.players[0].deck.len();
    start_dsl(&mut base, host);
    let yes = reducer_accept(&base);
    assert_eq!(
        base.hand_size(0),
        hand,
        "no initiating DSL tail runs while payment is parked"
    );
    assert_eq!(base.game.players[0].deck.len(), deck);
    let mut declined = DebugRunner::wrap(base.game.clone());
    act(&mut declined, PASS);
    assert_eq!(declined.memory(), 7);
    assert_dsl_success(&declined, result, hand, deck);
    act(&mut base, yes);
    assert_eq!(
        base.hand_size(0),
        hand,
        "tail also waits for suspend payment"
    );
    let mut suspend_clone = DebugRunner::wrap(base.game.clone());
    for branch in [&mut base, &mut suspend_clone] {
        suspend(branch, 3);
        assert_eq!(branch.memory(), 10);
        assert_dsl_success(branch, result, hand, deck);
    }
}

#[test]
fn dsl_route_choice_transfers_result_binding_and_outer_tails_to_reducer_chain() {
    let (mut base, host, result) = fixture(&[11, 12]);
    let hand = base.hand_size(0);
    let deck = base.game.players[0].deck.len();
    start_dsl(&mut base, host);
    let v = base
        .pending_selection_view()
        .expect("two payable evolution routes");
    assert_eq!(v.kind, SelectionKind::EffectChoice);
    assert!(!v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 2);
    for (i, action) in v.valid_action_ids.clone().into_iter().enumerate() {
        let mut r = DebugRunner::wrap(base.game.clone());
        act(&mut r, action);
        let yes = reducer_accept(&r);
        assert_eq!(
            r.hand_size(0),
            hand,
            "route selection cannot release the DSL tail before reducer payment"
        );
        assert_eq!(r.game.players[0].deck.len(), deck);
        let mut choice_clone = DebugRunner::wrap(r.game.clone());
        for branch in [&mut r, &mut choice_clone] {
            act(branch, yes);
            suspend(branch, 3);
            assert_eq!(branch.memory(), 7 - i as i16);
            assert_dsl_success(branch, result, hand, deck);
        }
    }
}

#[test]
fn failed_dsl_evolution_resumes_unconditional_tail_without_success_binding() {
    let (mut r, host, result) = fixture(&[6]);
    let hand = r.hand_size(0);
    let deck = r.game.players[0].deck.len();
    start_dsl(&mut r, host);
    let yes = reducer_accept(&r);
    let i = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.handle() == result)
        .unwrap();
    let removed = r.game.players[0].hand.remove(i);
    r.game.players[0].trash.push(removed);
    r.add_to_hand(0, "HPD-RESULT");
    act(&mut r, yes);
    if r.pending_selection().is_some() {
        suspend(&mut r, 3);
    }
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        "HPD-HOST"
    );
    assert_eq!(r.memory(), 10);
    assert_eq!(
        r.hand_size(0),
        hand + 3,
        "only the unconditional outer draw executes after abort"
    );
    assert_eq!(r.game.players[0].deck.len(), deck - 3);
    assert!(r.pending_selection().is_none());
}

#[test]
fn nested_field_payment_cannot_release_initiating_dsl_tail_before_player_reducer_commit() {
    let (mut r, host, result) = fixture(&[11]);
    r.place_on_field(0, "HPD-FIELD-REDUCER", Some(0));
    let hand = r.hand_size(0);
    let deck = r.game.players[0].deck.len();
    start_dsl(&mut r, host);
    let v = r
        .pending_selection_view()
        .expect("field and player reducers share legal order choice");
    assert_eq!(v.kind, SelectionKind::EffectChoice);
    assert!(!v.is_optional);
    let choices = v.effect_choices.as_ref().unwrap();
    assert_eq!(choices.len(), 2);
    let field = choices
        .iter()
        .find(|c| c.source_card == Some(r.game.players[0].battle_area[4].top_card().handle()))
        .expect("field reducer physical source choice")
        .action_id;
    act(&mut r, field);
    let payment = r
        .pending_selection_view()
        .expect("nested field payment parks its DSL select");
    assert_eq!(payment.kind, SelectionKind::OwnField);
    assert_eq!(
        r.hand_size(0),
        hand,
        "initiating evolution tail must wait for nested cost payment"
    );
    assert_eq!(r.game.players[0].deck.len(), deck);
    let action = payment.valid_action_ids[0];
    assert_eq!(payment.valid_action_ids.len(), 1);
    let mut clone = DebugRunner::wrap(r.game.clone());
    for branch in [&mut r, &mut clone] {
        act(branch, action);
        let yes = reducer_accept(branch);
        assert_eq!(
            branch.hand_size(0),
            hand,
            "field payment does not finish the evolution"
        );
        act(branch, yes);
        act(branch, encode_attack(0, 1));
        assert_eq!(
            branch.memory(),
            8,
            "printed11 minus effect3, field1 and player5"
        );
        assert_dsl_success(branch, result, hand, deck);
    }
}

#[test]
fn optional_hpd_keeps_effect_routes_affordable_only_after_its_payment() {
    let (mut base, host, result) = fixture(&[24, 25]);
    let hand = base.hand_size(0);
    let deck = base.game.players[0].deck.len();
    start(&mut base, host, result);
    let routes = base.pending_selection_view().expect(
        "both routes are payable: printed 24/25 minus effect 3 and optional HPD 5 costs 16/17",
    );
    assert_eq!(routes.kind, SelectionKind::EffectChoice);
    assert!(!routes.is_optional);
    assert_eq!(routes.valid_action_ids.len(), 2);
    assert_eq!(build_action_mask(&base.game, 0)[PASS as usize], 0.0);
    assert_eq!(base.memory(), 10);
    assert_eq!(base.hand_size(0), hand);
    for (index, route) in routes.valid_action_ids.clone().into_iter().enumerate() {
        let mut chosen = DebugRunner::wrap(base.game.clone());
        act(&mut chosen, route);
        let accept = reducer_accept(&chosen);
        let mut declined = DebugRunner::wrap(chosen.game.clone());
        // Optional payment remains optional even when declining makes the
        // already selected route impossible to finish. No card or memory moves.
        assert_eq!(build_action_mask(&declined.game, 0)[PASS as usize], 1.0);
        declined.game.resolve_selection(0, PASS).unwrap();
        assert_eq!(declined.memory(), 10);
        assert_eq!(declined.hand_size(0), hand);
        assert_eq!(declined.game.players[0].deck.len(), deck);
        assert_ne!(
            declined.game.players[0].battle_area[1].top_card().handle(),
            result
        );
        assert!(declined.game.players[0]
            .hand
            .iter()
            .any(|c| c.handle() == result));
        assert!(declined.pending_selection().is_none());

        act(&mut chosen, accept);
        let mut payment_clone = DebugRunner::wrap(chosen.game.clone());
        for branch in [&mut chosen, &mut payment_clone] {
            let pay = encode_attack(0, 3);
            assert_eq!(build_action_mask(&branch.game, 0)[pay as usize], 1.0);
            branch.game.resolve_selection(0, pay).unwrap();
            // Final payment crosses zero and normal resolution advances the
            // turn. Memory is stored relative to the new turn player.
            assert_eq!(branch.game.turn_player(), 1);
            let evolving_player_memory = -branch.memory();
            assert_eq!(evolving_player_memory, -6 - index as i16);
            assert_eq!(
                branch.game.players[0].battle_area[1].top_card().handle(),
                result
            );
            assert!(branch.game.players[0].battle_area[3].is_suspended);
            assert_eq!(
                branch.hand_size(0),
                hand + 1,
                "rule draw plus effect provenance witness, minus evolved card"
            );
            assert_eq!(branch.game.players[0].deck.len(), deck - 2);
            assert!(branch.game.player_digivolve_cost_reducers.is_empty());
            assert!(branch.pending_selection().is_none());
        }
    }
}

#[test]
fn nested_payment_evolution_owns_its_result_binding_and_tail_separately() {
    let (mut r, host, outer_result) = fixture_with_nested_payment(&[11], true);
    r.place_on_field(0, "HPD-FIELD-REDUCER", Some(0));
    let inner_index = r.add_to_hand(0, "HPD-INNER");
    let inner_result = r.game.players[0].hand[inner_index].handle();
    let hand = r.hand_size(0);
    let deck = r.game.players[0].deck.len();
    start_dsl(&mut r, host);
    let order = r
        .pending_selection_view()
        .expect("joint field/player order");
    let field = order
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|entry| {
            entry.source_card == Some(r.game.players[0].battle_area[4].top_card().handle())
        })
        .unwrap()
        .action_id;
    act(&mut r, field);
    let payment_host = r.pending_selection_view().unwrap();
    assert_eq!(payment_host.kind, SelectionKind::OwnField);
    assert_eq!(payment_host.valid_action_ids, vec![encode_attack(0, 3)]);
    act(&mut r, payment_host.valid_action_ids[0]);
    let payment_card = r.pending_selection_view().unwrap();
    assert_eq!(payment_card.kind, SelectionKind::Hand);
    assert_eq!(payment_card.valid_action_ids.len(), 1);
    act(&mut r, payment_card.valid_action_ids[0]);
    let inner_routes = r
        .pending_selection_view()
        .expect("nested evolution route choice");
    assert_eq!(inner_routes.kind, SelectionKind::EffectChoice);
    assert!(!inner_routes.is_optional);
    assert_eq!(inner_routes.valid_action_ids.len(), 2);
    assert_eq!(
        r.hand_size(0),
        hand,
        "neither evolution tail runs before nested route choice"
    );
    for (index, route) in inner_routes
        .valid_action_ids
        .clone()
        .into_iter()
        .enumerate()
    {
        let mut branch = DebugRunner::wrap(r.game.clone());
        act(&mut branch, route);
        assert_eq!(
            branch.game.players[0].battle_area[3].top_card().handle(),
            inner_result
        );
        assert_ne!(
            branch.game.players[0].battle_area[1].top_card().handle(),
            outer_result
        );
        assert_eq!(
            branch.hand_size(0),
            hand + 4,
            "inner result leaves hand, rule draw1, inner-only marker4; outer tail still parked"
        );
        assert_eq!(branch.game.players[0].deck.len(), deck - 5);
        let hpd = branch
            .pending_selection_view()
            .expect("outer HPD resumes after inner payment evolution");
        assert!(hpd.is_optional);
        let accept = *hpd.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        act(&mut branch, accept);
        act(&mut branch, encode_attack(0, 1));
        assert_eq!(
            branch.memory(),
            7 - index as i16,
            "inner route1/2 plus outer11-effect3-field1-HPD5"
        );
        assert_eq!(
            branch.game.players[0].battle_area[1].top_card().handle(),
            outer_result
        );
        assert_eq!(
            branch.hand_size(0),
            hand + 10,
            "two cards leave, two rule draws, witness1, inner marker4, outer success2 and tail3"
        );
        assert_eq!(branch.game.players[0].deck.len(), deck - 12);
        assert!(branch.pending_selection().is_none());
    }
}
