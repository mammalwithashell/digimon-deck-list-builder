//! Affordability must stop at the evolving effect's payment boundary.
//! Waiting sibling triggers may resolve after it, never fund its route search.
use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CostDelta, EffectTiming, GamePhase, PlaySource};
use digimon_engine::selection::TriggerSource;

#[test]
fn queued_sibling_memory_gain_cannot_fund_effect_evolution_payment_probe() {
    let mut host = make_test_card("BOUNDARY-HOST", "Boundary host");
    host.level = Some(4);
    host.colors = vec![CardColor::Green];
    let mut result = make_test_card("BOUNDARY-RESULT", "Boundary result");
    result.level = Some(5);
    result.colors = vec![CardColor::Red];
    result.evo_costs = [17, 18].into_iter().map(|memory_cost| EvoCost {
        card_color: 3, level: 4, memory_cost,
    }).collect();
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-103").unwrap()
        .from_dsl_yaml(r#"
card: BOUNDARY-WAITING
name: Waiting sibling memory gain
kind: tamer
color: [white]
cost: 1
effects:
  - when: on_play
    process:
      - gain_memory: 5
"#).unwrap()
        .add_card(host)
        .add_card(result)
        .add_card(make_test_card("BOUNDARY-FILLER", "Boundary filler"))
        .hand(0, &["BT3-103", "BOUNDARY-RESULT"])
        .deck(0, &["BOUNDARY-FILLER"; 20])
        .deck(1, &["BOUNDARY-FILLER"; 20])
        .memory(10)
        .start();
    r.game.current_phase = GamePhase::Main;
    let host = r.place_on_field(0, "BOUNDARY-HOST", Some(0));
    let waiting = r.place_on_field(0, "BOUNDARY-WAITING", Some(0));
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
    r.game.set_memory(0);
    // Stage the exact mid-resolution boundary: another triggered effect is
    // waiting while this effect attempts evolution. This is a unit fixture,
    // not a claim of legal-opening or DCGO oracle coverage.
    r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(waiting));
    let queued = r.game.effect_queue.len();
    assert!(queued > 0, "fixture must contain a real waiting DSL effect");
    let hand = r.game.players[0].hand.iter().position(|card|
        card.card_id(&r.game.card_data) == "BOUNDARY-RESULT").unwrap();
    let physical = r.game.players[0].hand[hand].handle();
    let evolved = r.game.effect_initiated_digivolve(
        0, hand, host, CostDelta::Reduce(0), false, PlaySource::ByEffect,
    );
    assert!(!evolved);
    assert!(r.pending_selection().is_none(),
        "17/18 minus optional HPD5 costs 12/13: neither fits the memory range at zero; a waiting sibling's +5 must not manufacture a route prompt");
    assert_eq!(r.memory(), 0);
    assert_eq!(r.game.effect_queue.len(), queued);
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
    assert!(!r.game.players[0].battle_area[host.index as usize].is_suspended);
    assert!(r.game.players[0].hand.iter().any(|card| card.handle() == physical));

    // Positive control: once that separate effect actually resolves, those
    // same routes become payable and the real player still chooses the cost.
    r.game.drain_effect_queue();
    assert_eq!(r.memory(), 5);
    let hand = r.game.players[0].hand.iter().position(|card| card.handle() == physical).unwrap();
    r.game.effect_initiated_digivolve(
        0, hand, host, CostDelta::Reduce(0), false, PlaySource::ByEffect,
    );
    let routes = r.pending_selection_view().expect("funded routes are offered after the sibling truly resolves");
    assert!(!routes.is_optional);
    assert_eq!(routes.valid_action_ids.len(), 2);
    assert_eq!(r.memory(), 5, "probing alone must not pay or suspend anything");
}

#[test]
fn synchronous_suspend_self_payment_cannot_drain_waiting_sibling_in_probe() {
    let mut host = make_test_card("SYNC-BOUNDARY-HOST", "Boundary host");
    host.level = Some(4);
    host.colors = vec![CardColor::Green];
    let mut result = make_test_card("SYNC-BOUNDARY-RESULT", "Boundary result");
    result.level = Some(5);
    result.colors = vec![CardColor::Red];
    result.evo_costs = [17, 18].into_iter().map(|memory_cost| EvoCost {
        card_color: 3, level: 4, memory_cost,
    }).collect();
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(r#"
card: SYNC-BOUNDARY-REDUCER
name: Synchronous suspend-self reducer
kind: tamer
color: [white]
cost: 1
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    when_any_ally_digivolves_into: { name_is: Boundary result }
    amount: 5
    pay_cost:
      - suspend: { target: source }
"#).unwrap()
        .from_dsl_yaml(r#"
card: SYNC-BOUNDARY-WAITING
name: Waiting sibling memory gain
kind: tamer
color: [white]
cost: 1
effects:
  - when: on_play
    process:
      - gain_memory: 5
"#).unwrap()
        .add_card(host)
        .add_card(result)
        .add_card(make_test_card("SYNC-BOUNDARY-FILLER", "Boundary filler"))
        .hand(0, &["SYNC-BOUNDARY-RESULT"])
        .deck(0, &["SYNC-BOUNDARY-FILLER"; 20])
        .deck(1, &["SYNC-BOUNDARY-FILLER"; 20])
        .memory(10)
        .start();
    r.game.current_phase = GamePhase::Main;
    let host = r.place_on_field(0, "SYNC-BOUNDARY-HOST", Some(0));
    let waiting = r.place_on_field(0, "SYNC-BOUNDARY-WAITING", Some(0));
    let reducer = r.place_on_field(0, "SYNC-BOUNDARY-REDUCER", Some(0));
    assert!(r.pending_selection().is_none());
    assert!(r.game.player_digivolve_cost_reducers.is_empty());
    r.game.set_memory(0);
    // Stage the exact mid-resolution boundary: another triggered effect is
    // waiting while this effect attempts evolution. This is a unit fixture,
    // not a claim of legal-opening or DCGO oracle coverage.
    r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(waiting));
    let queued = r.game.effect_queue.len();
    assert!(queued > 0, "fixture must contain a real waiting DSL effect");
    let hand = r.game.players[0].hand.iter().position(|card|
        card.card_id(&r.game.card_data) == "SYNC-BOUNDARY-RESULT").unwrap();
    let physical = r.game.players[0].hand[hand].handle();
    let evolved = r.game.effect_initiated_digivolve(
        0, hand, host, CostDelta::Reduce(0), false, PlaySource::ByEffect,
    );
    assert!(!evolved);
    assert!(r.pending_selection().is_none(),
        "17/18 minus mandatory synchronous suspend-self5 costs 12/13: neither fits the memory range at zero; a waiting sibling's +5 must not manufacture a route prompt");
    assert_eq!(r.memory(), 0);
    assert_eq!(r.game.effect_queue.len(), queued);
    assert!(r.game.player_digivolve_cost_reducers.is_empty());
    assert!(!r.game.players[0].battle_area[reducer.index as usize].is_suspended,
        "synchronous payment execution in the probe must not mutate the real reducer");
    assert!(!r.game.players[0].battle_area[host.index as usize].is_suspended);
    assert!(r.game.players[0].hand.iter().any(|card| card.handle() == physical));

    // Positive control: once that separate effect actually resolves, those
    // same routes become payable and the real player still chooses the cost.
    r.game.drain_effect_queue();
    assert_eq!(r.memory(), 5);
    let hand = r.game.players[0].hand.iter().position(|card| card.handle() == physical).unwrap();
    r.game.effect_initiated_digivolve(
        0, hand, host, CostDelta::Reduce(0), false, PlaySource::ByEffect,
    );
    let routes = r.pending_selection_view().expect("funded routes are offered after the sibling truly resolves");
    assert!(!routes.is_optional);
    assert_eq!(routes.valid_action_ids.len(), 2);
    assert_eq!(r.memory(), 5, "probing alone must not pay or suspend anything");
    assert!(!r.game.players[0].battle_area[reducer.index as usize].is_suspended);
}
