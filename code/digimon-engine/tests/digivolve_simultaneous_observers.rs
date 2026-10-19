//! One evolution procedure must expose its native and global observer triggers together.
//! Official rules 15-4-3-2, 15-4-3-4/5 and 15-4-5-2; actual oracle Hina/
//! Groundramon live line offers MultipleSkills [BT20-042, EX3-065].
//! Real Hina and Groundramon use unchanged production DSL; only the explicitly
//! named reaction fixture below supplies deterministic derived source removal.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CostDelta, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const HINA: &str = "EX3-065";
const GROUND: &str = "BT20-042";

fn setup(target_count: usize, reaction: bool) -> (DebugRunner, PermanentHandle, usize) {
    let mut builder = DebugRunner::builder()
        .dsl_card(HINA)
        .unwrap()
        .dsl_card(GROUND)
        .unwrap()
        .dsl_card("EX3-018")
        .unwrap()
        .dsl_card("ST1-03")
        .unwrap()
        .deck(0, &["ST1-03"; 20])
        .deck(1, &["ST1-03"; 20])
        .memory(10);
    if reaction {
        builder = builder
            .from_dsl_yaml(
                r#"
card: EVO-REACTION
name: Evolution reaction fixture
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_suspend
    condition: { event_permanent_is_source: true }
    process:
      - select_opponent_permanent:
          filter: { name_is: Groundramon }
          bind_as: victim
          prompt: "Delete the evolved host"
      - delete_permanent: { target: victim }
"#,
            )
            .unwrap();
    }
    let mut r = builder.start();
    r.skip_mulligan();
    r.place_on_field(0, HINA, Some(0));
    let host = r.place_on_field(0, "EX3-018", Some(0));
    for index in 0..target_count {
        r.place_on_field(
            1,
            if reaction && index == 0 {
                "EVO-REACTION"
            } else {
                "ST1-03"
            },
            Some(0),
        );
    }
    let hand = r.add_to_hand(0, GROUND);
    (r, host, hand)
}

fn act(r: &mut DebugRunner, action: u16) {
    let player = r
        .pending_selection_view()
        .expect("a player-visible choice")
        .selecting_player;
    assert_eq!(build_action_mask(&r.game, player)[action as usize], 1.0);
    r.execute_action(player, action).unwrap();
}

fn native_evolve(r: &mut DebugRunner, host: PermanentHandle, hand: usize) {
    let action = encode_digivolve(hand as u16, host.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
}

fn order_action(r: &DebugRunner, card: &str) -> u16 {
    let view = r
        .pending_selection_view()
        .expect("same evolution must expose trigger order");
    assert_eq!(
        view.kind,
        SelectionKind::TriggerOrder,
        "native WhenDigivolving must not start before the global Hina observer is queued"
    );
    assert_eq!(view.selecting_player, 0);
    assert!(
        !view.is_optional,
        "mandatory Groundramon trigger cannot be skipped wholesale"
    );
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    let choices = view
        .effect_choices
        .as_ref()
        .expect("explicit trigger provenance");
    assert_eq!(choices.len(), 2);
    for entry in choices {
        assert!(entry.source_card.is_some());
        assert_eq!(build_action_mask(&r.game, 0)[entry.action_id as usize], 1.0);
    }
    assert!(choices.iter().any(|e| e.label.starts_with(HINA)));
    assert!(choices.iter().any(|e| e.label.starts_with(GROUND)));
    choices
        .iter()
        .find(|e| e.label.starts_with(card))
        .unwrap()
        .action_id
}

fn accept_hina_if_prompted(r: &mut DebugRunner) {
    let view = r
        .pending_selection_view()
        .expect("Hina activation or refired target choice");
    if view.is_optional {
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
        let action = *view.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        act(r, action);
    }
}

fn resolve_ground_targets(r: &mut DebugRunner, target_ordinal: usize, expected_candidates: usize) {
    for _ in 0..2 {
        let view = r
            .pending_selection_view()
            .expect("independent suspend and lock choices");
        assert_eq!(view.selecting_player, 0);
        assert_eq!(view.valid_action_ids.len(), expected_candidates);
        assert!(!view.is_optional);
        act(r, view.valid_action_ids[target_ordinal]);
    }
}

fn exercise_both_orders(target_count: usize, effect_initiated: bool) {
    let (mut native_first, host, hand) = setup(target_count, false);
    let hand_before = native_first.hand_size(0);
    if effect_initiated {
        assert!(native_first.game.effect_initiated_digivolve(
            0,
            hand,
            host,
            CostDelta::Free,
            false,
            PlaySource::ByEffect
        ));
    } else {
        native_evolve(&mut native_first, host, hand);
    }
    assert_eq!(
        native_first.hand_size(0),
        hand_before,
        "evolution card leaves and procedure draws once before triggers"
    );
    let native_action = order_action(&native_first, GROUND);
    let hina_action = order_action(&native_first, HINA);
    let mut hina_first = DebugRunner::wrap(native_first.game.clone());
    assert_eq!(order_action(&hina_first, GROUND), native_action);
    assert_eq!(order_action(&hina_first, HINA), hina_action);

    act(&mut native_first, native_action);
    assert!(!native_first.game.players[0].battle_area[0].is_suspended);
    resolve_ground_targets(&mut native_first, 0, target_count);
    accept_hina_if_prompted(&mut native_first);
    assert!(native_first.game.players[0].battle_area[0].is_suspended);
    resolve_ground_targets(&mut native_first, target_count - 1, target_count);

    act(&mut hina_first, hina_action);
    accept_hina_if_prompted(&mut hina_first);
    assert!(
        hina_first.game.players[0].battle_area[0].is_suspended,
        "Hina-first ordering pays its cost before native Groundramon starts"
    );
    resolve_ground_targets(&mut hina_first, target_count - 1, target_count);
    resolve_ground_targets(&mut hina_first, 0, target_count);
    for r in [&native_first, &hina_first] {
        assert!(r.pending_selection().is_none());
        assert!(r.game.players[1].battle_area.iter().all(|p| p.is_suspended));
        assert_eq!(r.memory(), if effect_initiated { 10 } else { 7 });
        assert_eq!(r.hand_size(0), hand_before);
    }
}

#[test]
fn native_evolution_offers_both_orders_and_clone_with_one_opponent_target() {
    exercise_both_orders(1, false);
}

#[test]
fn native_evolution_offers_both_orders_and_clone_with_two_independent_targets() {
    exercise_both_orders(2, false);
}

#[test]
fn effect_evolution_offers_the_same_order_choice_without_losing_origin() {
    exercise_both_orders(2, true);
}

#[test]
fn derived_host_deletion_leaves_hina_optional_but_cannot_refire_departed_host() {
    let (mut r, host, hand) = setup(1, true);
    native_evolve(&mut r, host, hand);
    let ground = order_action(&r, GROUND);
    act(&mut r, ground);
    resolve_ground_targets(&mut r, 0, 1);
    let reaction = r
        .pending_selection_view()
        .expect("derived opponent reaction precedes pending Hina");
    assert_eq!(reaction.selecting_player, 1);
    assert!(reaction.prompt.contains("Delete the evolved host"));
    assert_eq!(reaction.valid_action_ids.len(), 1);
    act(&mut r, reaction.valid_action_ids[0]);
    // EX3-018 supplies inherited Evade. This branch intentionally permits
    // departure; auto_resolve would legally accept Evade and keep the host.
    let evade = r.pending_selection_view().expect("inherited Evade choice");
    assert_eq!(evade.selecting_player, 0);
    assert!(evade.is_optional);
    assert!(evade.prompt.to_lowercase().contains("evade"));
    act(&mut r, PASS);
    // A triggered Hina remains pending: its suspension cost is payable even
    // after the event host leaves. DCGO EX3_065 CanActivateCondition checks
    // Hina and its cost; the later refire requires an extant host.
    let hina = r
        .pending_selection_view()
        .expect("pending Hina remains optional");
    assert_eq!(hina.selecting_player, 0);
    assert!(hina.is_optional);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    let mut declined = r.game.clone();
    declined.decode_action(PASS, 0);
    assert!(declined.pending_selection.is_none());
    assert!(!declined.players[0].battle_area[0].is_suspended);
    let accept = *hina.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    act(&mut r, accept);
    assert_eq!(r.battle_area_size(0), 1);
    assert!(
        r.game.players[0].battle_area[0].is_suspended,
        "accepted Hina pays its legal cost but cannot refire the departed host"
    );
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == GROUND));
    assert!(r.pending_selection().is_none());
}

// Minimal DSL fixtures isolate three event families from one completed DNA
// procedure; point values make duplicate/lost activation observable.
fn batching_fixture(dna: bool, self_delete: bool) -> DebugRunner {
    let paths = if dna {
        "  - kind: dna_digivolve\n    materials:\n      - { name_is: Batch Left }\n      - { name_is: Batch Right }\n    cost: 0\n    stacks_unsuspended: true\n"
    } else {
        "  - kind: app_fusion\n    materials:\n      - filter: { name_in: [Batch Left, Batch Right] }\n    cost: 0\n"
    };
    let result = format!(
        "card: EVO-BATCH-RESULT\nname: Batch Result\nkind: digimon\nlevel: 5\ncolor: [red, blue]\ncost: 7\ndp: 9000\ntraits: [Appmon]\nalt_paths:\n{paths}effects:\n  - when: when_digivolving\n    condition: {{ dna_origin: {dna} }}\n    process:\n      - gain_memory: 1\n{}{}",
        if self_delete { "      - delete_permanent: { target: source }\n" } else { "" },
        if dna { "  - when: on_dna_digivolve\n    condition: { dna_origin: true }\n    process:\n      - gain_memory: 2\n" } else { "" }
    );
    let observer = format!(
        "card: EVO-BATCH-OBSERVER\nname: Batch Observer\nkind: tamer\ncolor: [white]\ncost: 1\neffects:\n  - when: on_digivolve\n    condition:\n      all_of:\n        - event_card_name_is: Batch Result\n        - event_is_effect_initiated: true\n        - dna_origin: {dna}\n    process:\n      - gain_memory: 4\n"
    );
    let mut left = digimon_engine::debug_runner::make_test_card("EVO-BATCH-LEFT", "Batch Left");
    left.level = Some(4);
    let mut right = digimon_engine::debug_runner::make_test_card("EVO-BATCH-RIGHT", "Batch Right");
    right.level = Some(4);
    let mut outer = digimon_engine::debug_runner::make_test_card("EVO-OUTER-DNA", "Outer DNA");
    outer.level = Some(5);
    let requirement = digimon_engine::card_data::DnaRequirement {
        level: 4,
        card_colors: vec![],
        name_contains: String::new(),
        text_contains: String::new(),
    };
    outer.dna_costs = vec![digimon_engine::card_data::DnaCost {
        memory_cost: 0,
        requirement1: requirement.clone(),
        requirement2: requirement,
    }];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&result)
        .unwrap()
        .from_dsl_yaml(&observer)
        .unwrap()
        .add_card(left)
        .add_card(right)
        .add_card(outer)
        .deck(0, &["EVO-BATCH-RIGHT"; 20])
        .deck(1, &["EVO-BATCH-LEFT"; 20])
        .memory(0)
        .start();
    r.skip_mulligan();
    r.place_on_field(0, "EVO-BATCH-OBSERVER", Some(0));
    r
}

fn push_fixture_trash(r: &mut DebugRunner, id: &str) -> digimon_engine::card_source::CardHandle {
    let hand = r.add_to_hand(0, id);
    let card = r.game.players[0].hand.remove(hand);
    let handle = card.handle();
    r.game.players[0].trash.push(card);
    handle
}

fn start_dna_variant(
    variant: u8,
    self_delete: bool,
) -> (DebugRunner, digimon_engine::card_source::CardHandle) {
    use digimon_engine::effect_context::EffectContext;
    let mut r = batching_fixture(true, self_delete);
    let left = r.place_on_field(0, "EVO-BATCH-LEFT", Some(0));
    let result_index = r.add_to_hand(0, "EVO-BATCH-RESULT");
    let result = r.game.players[0].hand[result_index].handle();
    let observer = r.top_card(PermanentHandle {
        player: 0,
        index: 0,
    });
    let merged = match variant {
        0 => {
            let right = r.place_on_field(0, "EVO-BATCH-RIGHT", Some(0));
            let mut ctx = EffectContext::new(
                &mut r.game,
                observer,
                Some(PermanentHandle {
                    player: 0,
                    index: 0,
                }),
                0,
            );
            ctx.effect_initiated_dna_digivolve(left, right, result, 0, false)
        }
        1 => {
            let right_index = r.add_to_hand(0, "EVO-BATCH-RIGHT");
            let right = r.game.players[0].hand[right_index].handle();
            let mut ctx = EffectContext::new(
                &mut r.game,
                observer,
                Some(PermanentHandle {
                    player: 0,
                    index: 0,
                }),
                0,
            );
            ctx.effect_initiated_dna_digivolve_with_hand_partner(left, right, result, 0, false)
        }
        2 => {
            let right = push_fixture_trash(&mut r, "EVO-BATCH-RIGHT");
            let mut ctx = EffectContext::new(
                &mut r.game,
                observer,
                Some(PermanentHandle {
                    player: 0,
                    index: 0,
                }),
                0,
            );
            ctx.effect_initiated_dna_digivolve_trash_partner(left, right, result, 0, false)
        }
        _ => unreachable!(),
    };
    assert!(merged.is_some(), "legal fixture recipe, variant {variant}");
    (r, result)
}

fn fixture_order(r: &DebugRunner, timings: &[digimon_engine::enums::EffectTiming]) {
    let view = r
        .pending_selection_view()
        .expect("one joint evolution trigger bundle");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert_eq!(view.selecting_player, 0);
    assert!(!view.is_optional);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    let choices = view.effect_choices.as_ref().unwrap();
    assert_eq!(choices.len(), timings.len());
    for timing in timings {
        let entry = choices
            .iter()
            .find(|c| c.timing == Some(*timing))
            .expect("each timing has its own legal order choice");
        let expected_source = if *timing == digimon_engine::enums::EffectTiming::OnDigivolve {
            "EVO-BATCH-OBSERVER"
        } else {
            "EVO-BATCH-RESULT"
        };
        assert!(entry.label.starts_with(expected_source));
        assert!(entry.source_card.is_some());
        assert_eq!(build_action_mask(&r.game, 0)[entry.action_id as usize], 1.0);
    }
}

fn pick_fixture_timing(r: &mut DebugRunner, timing: digimon_engine::enums::EffectTiming) {
    let view = r.pending_selection_view().unwrap();
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    let action = view
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|c| c.timing == Some(timing))
        .expect("requested pending timing")
        .action_id;
    act(r, action);
}

#[test]
fn all_three_dna_paths_expose_all_six_orders_with_clone_and_provenance() {
    use digimon_engine::enums::EffectTiming::{
        OnDigivolve as O, OnDnaDigivolve as D, WhenDigivolving as W,
    };
    for variant in 0..3 {
        let (base, result) = start_dna_variant(variant, false);
        fixture_order(&base, &[W, D, O]);
        assert_eq!(
            base.memory(),
            0,
            "nothing resolves before choosing an order"
        );
        for order in [
            [W, D, O],
            [W, O, D],
            [D, W, O],
            [D, O, W],
            [O, W, D],
            [O, D, W],
        ] {
            let mut branch = DebugRunner::wrap(base.game.clone());
            pick_fixture_timing(&mut branch, order[0]);
            fixture_order(&branch, &order[1..]);
            pick_fixture_timing(&mut branch, order[1]);
            branch.auto_resolve().unwrap();
            assert_eq!(
                branch.memory(),
                7,
                "all three triggers resolve exactly once with correct DNA/effect origin"
            );
            assert!(branch.pending_selection().is_none());
            assert_eq!(
                branch.game.players[0].battle_area[1].top_card().handle(),
                result
            );
            assert_eq!(branch.game.players[0].battle_area[1].card_sources.len(), 3);
        }
    }
}

#[test]
fn all_three_dna_paths_capture_global_event_card_before_native_self_deletion() {
    use digimon_engine::enums::EffectTiming::{
        OnDigivolve as O, OnDnaDigivolve as D, WhenDigivolving as W,
    };
    for variant in 0..3 {
        let (mut r, result) = start_dna_variant(variant, true);
        fixture_order(&r, &[W, D, O]);
        let mut cloned = DebugRunner::wrap(r.game.clone());
        for branch in [&mut r, &mut cloned] {
            pick_fixture_timing(branch, W);
            branch.auto_resolve().unwrap();
            assert_eq!(branch.memory(), 5,
                "native +1 and immutable event-card observer +4 survive; removed host's DNA +2 cannot activate");
            assert!(branch.game.players[0]
                .trash
                .iter()
                .any(|c| c.handle() == result));
            assert_eq!(branch.battle_area_size(0), 1);
            assert!(branch.pending_selection().is_none());
        }
    }
}

#[test]
fn app_fuse_hand_and_trash_offer_native_observer_orders_and_preserve_link_sources() {
    use digimon_engine::enums::EffectTiming::{OnDigivolve as O, WhenDigivolving as W};
    use digimon_engine::game_actions::AppFuseSourceZone;
    for from_trash in [false, true] {
        let mut r = batching_fixture(false, false);
        let host = r.place_on_field(0, "EVO-BATCH-LEFT", Some(0));
        r.push_linked_owned(host, "EVO-BATCH-RIGHT", 0);
        let result = if from_trash {
            push_fixture_trash(&mut r, "EVO-BATCH-RESULT")
        } else {
            let i = r.add_to_hand(0, "EVO-BATCH-RESULT");
            r.game.players[0].hand[i].handle()
        };
        assert!(r.game.commit_effect_app_fuse(
            0,
            host,
            result,
            if from_trash {
                AppFuseSourceZone::Trash
            } else {
                AppFuseSourceZone::Hand
            }
        ));
        fixture_order(&r, &[W, O]);
        assert_eq!(r.memory(), 0);
        for first in [W, O] {
            let mut branch = DebugRunner::wrap(r.game.clone());
            pick_fixture_timing(&mut branch, first);
            branch.auto_resolve().unwrap();
            assert_eq!(
                branch.memory(),
                5,
                "native and effect-origin observer each resolve once"
            );
            let perm = &branch.game.players[0].battle_area[host.index as usize];
            assert_eq!(perm.top_card().handle(), result);
            assert_eq!(perm.card_sources.len(), 3);
            assert!(perm.linked_cards.is_empty());
            assert!(!branch.game.players[0]
                .trash
                .iter()
                .any(|c| c.handle() == result));
            assert!(branch.pending_selection().is_none());
        }
    }
}

fn nested_non_dna_fixture(
    optional_native: bool,
) -> (
    DebugRunner,
    digimon_engine::card_source::CardHandle,
    PermanentHandle,
) {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::{CardEffect, Effect};
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::game_actions::AppFuseSourceZone;
    struct NestedAppFuse {
        host: PermanentHandle,
        result: CardHandle,
    }
    impl CardEffect for NestedAppFuse {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let host = self.host;
            let result = self.result;
            vec![Effect::when_digivolving(card)
                .name("DNA effect performs non-DNA evolution")
                .process(move |ctx| {
                    assert!(ctx.as_read().dna_origin(), "outer DNA origin is installed");
                    assert!(ctx.game.commit_effect_app_fuse(
                        0,
                        host,
                        result,
                        AppFuseSourceZone::Hand
                    ));
                    assert!(
                        ctx.as_read().dna_origin(),
                        "nested procedure restores its caller's DNA origin"
                    );
                })
                .build()]
        }
    }
    let mut r = batching_fixture(false, false);
    if optional_native {
        // Isolate the one optional native trigger so it parks directly inside
        // the outer DNA callback, while its caller's ambient origin is true.
        r.game.players[0].battle_area.clear();
        struct OptionalNative;
        impl CardEffect for OptionalNative {
            fn effects(&self, card: CardHandle) -> Vec<Effect> {
                vec![Effect::when_digivolving(card)
                    .name("optional non-DNA nested trigger")
                    .optional()
                    .needs_outer_optional_prompt()
                    .process(|ctx| {
                        assert!(
                            !ctx.as_read().dna_origin(),
                            "parked non-DNA trigger must preserve explicit false"
                        );
                        ctx.gain_memory(1);
                    })
                    .build()]
            }
        }
        r.register_effect("EVO-BATCH-RESULT", std::sync::Arc::new(OptionalNative));
    }
    let host = r.place_on_field(0, "EVO-BATCH-LEFT", Some(0));
    r.push_linked_owned(host, "EVO-BATCH-RIGHT", 0);
    let result_i = r.add_to_hand(0, "EVO-BATCH-RESULT");
    let result = r.game.players[0].hand[result_i].handle();
    let left = r.place_on_field(0, "EVO-BATCH-LEFT", Some(0));
    let right = r.place_on_field(0, "EVO-BATCH-RIGHT", Some(0));
    let outer_i = r.add_to_hand(0, "EVO-OUTER-DNA");
    let outer = r.game.players[0].hand[outer_i].handle();
    r.register_effect(
        "EVO-OUTER-DNA",
        std::sync::Arc::new(NestedAppFuse { host, result }),
    );
    let mut ctx = EffectContext::new(&mut r.game, outer, None, 0);
    assert!(ctx
        .effect_initiated_dna_digivolve(left, right, outer, 0, false)
        .is_some());
    (r, result, host)
}

#[test]
fn non_dna_nested_inside_dna_has_false_origin_and_restores_outer_true_origin() {
    use digimon_engine::enums::EffectTiming::{OnDigivolve as O, WhenDigivolving as W};
    let (r, result, host) = nested_non_dna_fixture(false);
    fixture_order(&r, &[W, O]);
    assert_eq!(
        r.memory(),
        0,
        "nested native and observer are offered before either resolves"
    );
    for first in [W, O] {
        let mut branch = DebugRunner::wrap(r.game.clone());
        pick_fixture_timing(&mut branch, first);
        branch.auto_resolve().unwrap();
        assert_eq!(
            branch.memory(),
            5,
            "native false-origin +1 and global false-origin +4"
        );
        assert_eq!(
            branch.game.players[0].battle_area[host.index as usize]
                .top_card()
                .handle(),
            result
        );
    }
}

#[test]
fn optional_non_dna_nested_in_dna_keeps_false_origin_after_parking_and_clone() {
    let (r, result, host) = nested_non_dna_fixture(true);
    let view = r
        .pending_selection_view()
        .expect("optional native trigger is parked");
    assert!(view.is_optional);
    assert_eq!(view.selecting_player, 0);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    assert_eq!(r.memory(), 0);
    let accept = *view.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    for _ in 0..2 {
        let mut branch = DebugRunner::wrap(r.game.clone());
        act(&mut branch, accept);
        assert_eq!(branch.memory(), 1);
        assert!(branch.pending_selection().is_none());
        assert_eq!(
            branch.game.players[0].battle_area[host.index as usize]
                .top_card()
                .handle(),
            result
        );
    }
    let mut declined = DebugRunner::wrap(r.game.clone());
    act(&mut declined, PASS);
    assert_eq!(declined.memory(), 0);
    assert!(declined.pending_selection().is_none());
}
