//! Effect cost routes must survive nested DSL tails, cloning and index shifts.
use digimon_engine::action::{
    build_action_mask,
    space::{HAND_EFFECT_START, PASS},
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardColor, CardSourceRef, CostDelta, Expiry, GamePhase, ModifierType, PlaySource,
};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const HOST: &str = "TEST-EFFECT-HOST";
const YAML: &str = r#"
card: TEST-EFFECT-HOST
name: Route Host
kind: digimon
level: 4
color: [blue]
cost: 5
dp: 5000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: evo
          optional: true
          filter: { name_is: Route Result }
          prompt: Choose the result
      - if:
          condition: { binding_present: evo }
          then:
            - effect_initiated_digivolve:
                target: source
                from_hand: evo
                cost: { reduce: 2 }
            - if:
                condition: { effect_digivolved_any_digimon: true }
                then:
                  - gain_memory: 3
      - draw: { of: you, count: 2 }
"#;

fn fixture() -> (DebugRunner, PermanentHandle) {
    let mut result = make_test_card("RESULT", "Route Result");
    result.level = Some(5);
    result.colors = vec![CardColor::Blue];
    result.evo_costs = vec![3, 4]
        .into_iter()
        .map(|memory_cost| EvoCost {
            card_color: 1,
            level: 4,
            memory_cost,
        })
        .collect();
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .add_card(result)
        .add_card(make_test_card("FILL", "Fill"))
        .hand(0, &["FILL", "RESULT"])
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(6)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let host = r.place_on_field(0, HOST, None);
    (r, host)
}

fn park(r: &mut DebugRunner, host: PermanentHandle) {
    r.fire_play_event_triggers(0, host.index as usize, true, false);
    let pick = r.pending_selection_view().unwrap().valid_action_ids[0];
    r.execute_action(0, pick).unwrap();
    let view = r
        .pending_selection_view()
        .expect("must choose between costs 1 and 2 before evolving");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    assert!(!view.is_optional);
    assert_eq!(
        view.valid_action_ids,
        vec![HAND_EFFECT_START, HAND_EFFECT_START + 1]
    );
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    assert_eq!(r.memory(), 6);
    assert_eq!(r.game.players[0].hand.len(), 2);
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        HOST
    );
}

#[test]
fn effect_cost_choice_cloned_branches_preserve_reduction_and_nested_result_tail() {
    let (mut r, host) = fixture();
    park(&mut r, host);
    let mut clone = r.game.clone();
    r.execute_action(0, HAND_EFFECT_START).unwrap();
    clone.decode_action(HAND_EFFECT_START + 1, 0);
    for (game, paid) in [(&r.game, 1), (&clone, 2)] {
        assert_eq!(
            game.memory,
            6 - paid + 3,
            "successful-result tail must run exactly once"
        );
        assert_eq!(
            game.players[0].hand.len(),
            4,
            "one procedure draw and two outer-tail draws"
        );
        assert_eq!(
            game.players[0].battle_area[0]
                .top_card()
                .card_id(&game.card_data),
            "RESULT"
        );
        assert!(game.pending_selection.is_none());
    }
}

#[test]
fn effect_cost_choice_follows_indices_but_not_result_zone_changes() {
    let (mut r, host) = fixture();
    park(&mut r, host);
    r.game.players[0].hand.remove(0);
    let spare = r.place_on_field(0, "FILL", None);
    r.game.players[0]
        .battle_area
        .swap(host.index as usize, spare.index as usize);
    r.execute_action(0, HAND_EFFECT_START + 1).unwrap();
    assert_eq!(r.memory(), 7);
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        "RESULT"
    );

    let (mut r, host) = fixture();
    park(&mut r, host);
    let selected = r.game.players[0].hand.remove(1);
    r.game.players[0].trash.push(selected);
    r.add_to_hand(0, "RESULT");
    r.execute_action(0, HAND_EFFECT_START).unwrap();
    assert_eq!(
        r.memory(),
        6,
        "failure must not pay or run the success-only tail"
    );
    assert_eq!(
        r.game.players[0].hand.len(),
        4,
        "unconditional outer tail still runs once"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        HOST
    );
}

#[test]
fn effect_cost_choice_rejects_replaced_host() {
    let (mut r, host) = fixture();
    park(&mut r, host);
    r.game.players[0].battle_area.remove(0);
    r.place_on_field(0, HOST, None);
    r.execute_action(0, HAND_EFFECT_START).unwrap();
    assert_eq!(r.memory(), 6);
    assert_eq!(r.game.players[0].hand.len(), 4);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        HOST
    );
}

#[test]
fn effect_cost_fixed_free_and_duplicate_payable_costs_do_not_prompt() {
    for (cost, paid) in [
        (CostDelta::Free, 0),
        (CostDelta::Fixed(2), 2),
        (CostDelta::Reduce(9), 0),
    ] {
        let (mut r, host) = fixture();
        assert!(r
            .game
            .effect_initiated_digivolve(0, 1, host, cost, false, PlaySource::ByEffect));
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 6 - paid);
    }
}

#[test]
fn effect_cost_choice_rechecks_reduction_and_effect_prohibitions() {
    for prohibition in [
        ModifierType::CannotReduceCost,
        ModifierType::CannotReduceDigivolveCost,
        ModifierType::OpponentCannotReduceDigivolveCost,
        ModifierType::CannotDigivolveDigimonByEffect,
    ] {
        let (mut r, host) = fixture();
        park(&mut r, host);
        let owner = if prohibition == ModifierType::OpponentCannotReduceDigivolveCost {
            1
        } else {
            0
        };
        r.game.modifiers.add_player_modifier(
            owner,
            PlayerModifierEntry::simple(prohibition, 0, Expiry::Permanent, None, owner),
        );
        r.execute_action(0, HAND_EFFECT_START + 1).unwrap();
        assert_eq!(
            r.memory(),
            if prohibition == ModifierType::CannotDigivolveDigimonByEffect {
                6
            } else {
                5
            }
        );
    }
}

#[test]
fn effect_cost_choice_retains_color_waiver() {
    let (mut r, host) = fixture();
    let host_data = r.game.players[0].battle_area[0].top_card().data_index;
    r.game.card_data[host_data].colors = vec![CardColor::Red];
    assert!(!r.game.effect_initiated_digivolve(
        0,
        1,
        host,
        CostDelta::Reduce(2),
        false,
        PlaySource::ByEffect
    ));
    assert!(r.pending_selection().is_none());
    r.game
        .effect_initiated_digivolve(0, 1, host, CostDelta::Reduce(2), true, PlaySource::ByEffect);
    assert!(
        r.pending_selection().is_some(),
        "waived color still retains both printed costs"
    );
    r.execute_action(0, HAND_EFFECT_START + 1).unwrap();
    assert_eq!(r.memory(), 4);
}

#[test]
fn effect_cost_choice_trash_source_clone_keeps_selected_instance_and_zone() {
    let (mut r, host) = fixture();
    let result = r.game.players[0].hand.remove(1);
    r.game.players[0].trash.push(result);
    r.game.effect_initiated_digivolve_from_source(
        0,
        CardSourceRef::Trash(0, 0),
        host,
        CostDelta::Reduce(2),
        false,
        PlaySource::ByEffect,
    );
    assert!(r.pending_selection().is_some());
    let mut clone = r.game.clone();
    r.execute_action(0, HAND_EFFECT_START).unwrap();
    let moved = clone.players[0].trash.remove(0);
    clone.players[1].trash.push(moved);
    clone.decode_action(HAND_EFFECT_START + 1, 0);
    assert_eq!(r.memory(), 5);
    assert_eq!(
        clone.memory, 6,
        "same physical card in another owner's trash is not a valid source"
    );
}

#[test]
fn effect_cost_choice_does_not_fall_back_when_chosen_route_disappears() {
    let (mut r, host) = fixture();
    park(&mut r, host);
    let result_data = r.game.players[0].hand[1].data_index;
    r.game.card_data[result_data]
        .evo_costs
        .retain(|route| route.memory_cost == 3);
    r.execute_action(0, HAND_EFFECT_START + 1).unwrap();
    assert_eq!(r.memory(), 6);
    assert_eq!(r.game.players[0].hand.len(), 4);
}

#[test]
fn effect_cost_choice_excludes_unaffordable_routes_at_memory_floor() {
    for (memory, expected_success) in [(-9, true), (-10, false)] {
        let (mut r, host) = fixture();
        r.game.memory = memory;
        let succeeded = r.game.effect_initiated_digivolve(
            0,
            1,
            host,
            CostDelta::Reduce(2),
            false,
            PlaySource::ByEffect,
        );
        assert!(
            r.pending_selection().is_none(),
            "at most the cost-1 route is payable"
        );
        assert_eq!(succeeded, expected_success);
        assert_eq!(r.memory(), -10);
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            if expected_success { "RESULT" } else { HOST }
        );
    }
}

struct PaidDiscountAndMemoryObserver;
impl digimon_engine::effect::CardEffect for PaidDiscountAndMemoryObserver {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        use digimon_engine::effect::Effect;
        vec![
            Effect::before_pay_cost(card)
                .once_per_turn()
                .pay_cost_fn(|ctx| {
                    ctx.trash_from_top(0, 1);
                    true
                })
                .cost_reduction(1)
                .build(),
            Effect::before_pay_cost_observe(card)
                .once_per_turn()
                .process(|ctx| ctx.gain_memory(1))
                .build(),
        ]
    }
}

#[test]
fn effect_cost_choice_affordability_includes_hooks_without_paying_them_early() {
    let (mut r, host) = fixture();
    r.register_effect("FILL", std::sync::Arc::new(PaidDiscountAndMemoryObserver));
    r.place_on_field(0, "FILL", None);
    r.game.memory = -10;
    assert!(!r.game.effect_initiated_digivolve(
        0,
        1,
        host,
        CostDelta::Reduce(2),
        false,
        PlaySource::ByEffect,
    ));
    assert_eq!(
        r.pending_selection().unwrap().valid_action_ids.len(),
        2,
        "discount and memory observer make both routes payable"
    );
    assert_eq!(
        r.memory(),
        -10,
        "observer is not applied until a route is chosen"
    );
    assert!(
        r.game.players[0].trash.is_empty(),
        "paid reduction is not applied early"
    );
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        game.resolve_selection(0, HAND_EFFECT_START + 1).unwrap();
        assert!(game.pending_selection.is_none());
        let controller_memory = if game.turn_player() == 0 {
            game.memory
        } else {
            -game.memory
        };
        assert_eq!(
            controller_memory, -10,
            "gain 1, then pay reduced cost 1 exactly once"
        );
        assert_eq!(game.players[0].trash.len(), 1, "reducer cost paid once");
        assert_eq!(
            game.players[0].battle_area[0]
                .top_card()
                .card_id(&game.card_data),
            "RESULT"
        );
    }
}
