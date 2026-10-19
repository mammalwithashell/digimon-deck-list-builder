//! Immediate-type effects remain available until their cause resolves (15-8-5-4).
//! Native instrumentation only; every decision is mask-checked.
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::CardEffect;
use digimon_engine::effect::{Effect, EffectBuilder};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::SelectionKind;
use std::sync::Arc;
struct Replacement {
    cancel: bool,
    draw: u8,
}
impl CardEffect for Replacement {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let cancel = self.cancel;
        let draw = self.draw;
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldBeDeleted)
            .name(if cancel {
                "Prevent deletion"
            } else {
                "Draw without replacing"
            })
            .optional()
            .replacement_process(move |ctx| {
                if draw > 0 {
                    ctx.effect.draw(0, draw);
                }
                if cancel {
                    ctx.cancel();
                }
            })
            .build()]
    }
}
fn runner(first_cancel: bool) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("VICTIM", "Victim"))
        .add_card(make_test_card("FIRST", "First"))
        .add_card(make_test_card("SECOND", "Second"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    r.register_effect(
        "FIRST",
        Arc::new(Replacement {
            cancel: first_cancel,
            draw: if first_cancel { 0 } else { 1 },
        }),
    );
    r.register_effect(
        "SECOND",
        Arc::new(Replacement {
            cancel: true,
            draw: 0,
        }),
    );
    r.skip_mulligan();
    let h = r.place_on_field(0, "VICTIM", None);
    r.place_on_field(0, "FIRST", None);
    r.place_on_field(0, "SECOND", None);
    r.game.delete_permanent_with_effects(h);
    r
}
fn choose_if_order(r: &mut DebugRunner, index: usize) {
    let v = r
        .pending_selection_view()
        .expect("replacement window must offer a decision");
    if matches!(
        v.kind,
        SelectionKind::EffectChoice | SelectionKind::TriggerOrder
    ) {
        assert_eq!(v.valid_action_ids.len(), 2);
        let a = v.valid_action_ids[index];
        assert_eq!(
            build_action_mask(&r.game, v.selecting_player)[a as usize],
            1.0
        );
        r.game.resolve_selection(v.selecting_player, a).unwrap();
    }
}
fn decide(r: &mut DebugRunner, a: u16) {
    let v = r
        .pending_selection_view()
        .expect("remaining replacement must still be selectable");
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[a as usize],
        1.0
    );
    r.game.resolve_selection(v.selecting_player, a).unwrap();
}
fn victim_alive(r: &DebugRunner) -> bool {
    r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "VICTIM")
}
#[test]
fn decline_first_then_accept_second_preserves_subject() {
    let mut r = runner(true);
    choose_if_order(&mut r, 0);
    decide(&mut r, PASS);
    assert!(
        victim_alive(&r),
        "original cause must remain pending while another candidate is available"
    );
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert!(victim_alive(&r));
    assert!(r.pending_selection().is_none());
}
#[test]
fn decline_all_commits_original_cause_once() {
    let mut r = runner(true);
    choose_if_order(&mut r, 0);
    decide(&mut r, PASS);
    decide(&mut r, PASS);
    assert!(!victim_alive(&r));
    assert_eq!(
        r.game.players[0]
            .trash
            .iter()
            .filter(|c| c.card_id(&r.game.card_data) == "VICTIM")
            .count(),
        1
    );
    assert!(r.pending_selection().is_none());
}
#[test]
fn accepted_non_replacing_effect_continues_without_reoffering_it() {
    let mut r = runner(false);
    let before = r.hand_size(0);
    choose_if_order(&mut r, 0);
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(r.hand_size(0), before + 1);
    assert!(victim_alive(&r));
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(r.hand_size(0), before + 1);
    assert!(victim_alive(&r));
    assert!(r.pending_selection().is_none());
}
#[test]
fn cloning_after_first_decline_keeps_remaining_candidate_and_original_event() {
    let mut r = runner(true);
    choose_if_order(&mut r, 0);
    decide(&mut r, PASS);
    let saved = r.game.clone();
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert!(victim_alive(&r));
    r.game = saved;
    decide(&mut r, PASS);
    assert!(!victim_alive(&r));
    assert!(r.pending_selection().is_none());
}
#[test]
fn controller_can_choose_second_candidate_before_first() {
    let mut r = runner(false);
    let before = r.hand_size(0);
    let v = r.pending_selection_view().unwrap();
    assert!(
        matches!(
            v.kind,
            SelectionKind::EffectChoice | SelectionKind::TriggerOrder
        ),
        "two immediate candidates require controller-chosen order"
    );
    assert_eq!(v.selecting_player, 0);
    assert_eq!(v.valid_action_ids.len(), 2);
    choose_if_order(&mut r, 1);
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert!(victim_alive(&r));
    assert_eq!(
        r.hand_size(0),
        before,
        "the draw has not activated yet; cancellation does not auto-run it"
    );
    assert!(
        r.pending_selection().is_some(),
        "the remaining immediate effect must be offered"
    );
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(r.hand_size(0), before + 1);
    assert!(victim_alive(&r));
    assert!(r.pending_selection().is_none());
}

struct LaterPayable {
    minimum_hand: usize,
}
impl CardEffect for LaterPayable {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let minimum = self.minimum_hand;
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldBeDeleted)
            .name("Available after rider draws")
            .optional()
            .replacement_condition(move |ctx, _| ctx.game.player(0).hand.len() >= minimum)
            .replacement_process(|ctx| ctx.cancel())
            .build()]
    }
}
#[test]
fn pending_immediate_effect_becomes_payable_after_non_replacing_rider() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("VICTIM", "Victim"))
        .add_card(make_test_card("FIRST", "First"))
        .add_card(make_test_card("SECOND", "Second"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    r.skip_mulligan();
    let initial = r.hand_size(0);
    r.register_effect(
        "FIRST",
        Arc::new(Replacement {
            cancel: false,
            draw: 1,
        }),
    );
    r.register_effect(
        "SECOND",
        Arc::new(LaterPayable {
            minimum_hand: initial + 1,
        }),
    );
    let h = r.place_on_field(0, "VICTIM", None);
    r.place_on_field(0, "FIRST", None);
    r.place_on_field(0, "SECOND", None);
    r.game.delete_permanent_with_effects(h);
    // Only the draw rider is currently payable; the other effect is nevertheless pending.
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(r.hand_size(0), initial + 1);
    assert!(victim_alive(&r));
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert!(victim_alive(&r));
    assert!(r.pending_selection().is_none());
}
#[test]
fn turn_player_replacement_precedes_affected_subject_controller() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("VICTIM", "Victim"))
        .add_card(make_test_card("FIRST", "First"))
        .add_card(make_test_card("SECOND", "Second"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    r.register_effect(
        "FIRST",
        Arc::new(Replacement {
            cancel: false,
            draw: 1,
        }),
    );
    r.register_effect(
        "SECOND",
        Arc::new(Replacement {
            cancel: true,
            draw: 0,
        }),
    );
    r.skip_mulligan();
    let h = r.place_on_field(0, "VICTIM", None);
    r.place_on_field(0, "FIRST", None);
    r.place_on_field(1, "SECOND", None);
    r.game.turn_player_idx = r.game.turn_order.iter().position(|&p| p == 1).unwrap();
    r.game.memory_pair = (1, 0);
    let initial = r.hand_size(0);
    r.game.delete_permanent_with_effects(h);
    assert_eq!(
        r.pending_selection_view().unwrap().selecting_player,
        1,
        "turn-player controller chooses their own immediate effect before non-turn player's effect"
    );
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert!(victim_alive(&r));
    assert_eq!(r.hand_size(0), initial);
    assert_eq!(
        r.pending_selection_view().unwrap().selecting_player,
        0,
        "after turn-player prevention the non-turn player's immediate effect remains available"
    );
    let saved = r.game.clone();
    let digest = saved.verification_digest();
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(r.hand_size(0), initial + 1);
    assert!(victim_alive(&r));
    assert!(r.pending_selection().is_none());
    r.game = saved;
    assert_eq!(r.game.verification_digest(), digest);
    decide(&mut r, PASS);
    assert_eq!(r.hand_size(0), initial);
    assert!(victim_alive(&r));
    assert!(r.pending_selection().is_none());
}

struct NestedRider;
impl CardEffect for NestedRider {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldBeDeleted)
            .name("Choose a body target, then draw without replacing")
            .optional()
            .replacement_process(|ctx| {
                ctx.effect.select_own_permanent(
                    "Choose any own Digimon",
                    false,
                    |_, _| true,
                    |ctx, _| {
                        ctx.draw(0, 1);
                    },
                );
            })
            .build()]
    }
}
#[test]
fn nested_replacement_body_finishes_before_remaining_window_and_enclosing_dsl_tail() {
    const DRIVER: &str = r#"
card: DRIVER
name: Driver
kind: digimon
color: [red]
level: 3
cost: 3
dp: 2000
effects:
 - when: on_play
   process:
    - delete_permanent: { target: source }
    - gain_memory: 5
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(DRIVER)
        .unwrap()
        .add_card(make_test_card("FIRST", "First"))
        .add_card(make_test_card("SECOND", "Second"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    r.register_effect("FIRST", Arc::new(NestedRider));
    r.register_effect(
        "SECOND",
        Arc::new(Replacement {
            cancel: true,
            draw: 0,
        }),
    );
    r.skip_mulligan();
    let h = r.place_on_field(0, "DRIVER", None);
    r.place_on_field(0, "FIRST", None);
    r.place_on_field(0, "SECOND", None);
    r.game.enqueue_triggered(
        EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(h),
    );
    r.game.drain_effect_queue();
    let initial = r.hand_size(0);
    let memory = r.memory();
    choose_if_order(&mut r, 0);
    decide(&mut r, REPLACEMENT_ACCEPT);
    let v = r
        .pending_selection_view()
        .expect("replacement body target selection");
    assert_eq!(v.kind, SelectionKind::OwnField);
    let action = v.valid_action_ids[0];
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[action as usize],
        1.0
    );
    r.game
        .resolve_selection(v.selecting_player, action)
        .unwrap();
    assert_eq!(
        r.hand_size(0),
        initial + 1,
        "replacement body resolves before remaining replacement"
    );
    assert_eq!(
        r.memory(),
        memory,
        "enclosing OnPlay tail must wait for complete immediate window"
    );
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "DRIVER"));
    assert_eq!(
        r.memory(),
        memory + 5,
        "enclosing tail runs once after deletion is cancelled"
    );
    assert!(r.pending_selection().is_none());
}

struct ManyReplacements;
impl CardEffect for ManyReplacements {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        (0..35u8)
            .map(|slot| {
                EffectBuilder::new(c, EffectTiming::WhenWouldBeDeleted)
                    .name(&format!("Cancel number {}", slot + 1))
                    .optional()
                    .replacement_process(move |ctx| {
                        ctx.effect.draw(0, slot + 1);
                        ctx.cancel();
                    })
                    .build()
            })
            .collect()
    }
}
#[test]
fn replacement_order_pages_are_state_neutral_cloneable_and_do_not_truncate() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("VICTIM", "Victim"))
        .add_card(make_test_card("MANY", "Many"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 80])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    r.register_effect("MANY", Arc::new(ManyReplacements));
    r.skip_mulligan();
    let h = r.place_on_field(0, "VICTIM", None);
    r.place_on_field(0, "MANY", None);
    r.game.delete_permanent_with_effects(h);
    let before = r.hand_size(0);
    let memory = r.memory();
    let digest = r.game.verification_digest();
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.kind, SelectionKind::EffectChoice);
    assert_eq!(v.valid_action_ids.len(), 29);
    assert_eq!(build_action_mask(&r.game, 0)[59], 1.0);
    r.game.resolve_selection(0, 59).unwrap();
    assert_eq!(r.hand_size(0), before);
    assert_eq!(r.memory(), memory);
    assert!(victim_alive(&r));
    let saved = r.game.clone();
    let page_digest = r.game.verification_digest();
    assert_ne!(page_digest, digest);
    assert_eq!(build_action_mask(&r.game, 0)[58], 1.0);
    r.game.resolve_selection(0, 58).unwrap();
    assert_eq!(r.game.verification_digest(), digest);
    r.game = saved;
    assert_eq!(r.game.verification_digest(), page_digest);
    // Second page has seven actual effects, actions30..36, and Previous58.
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        8
    );
    assert_eq!(build_action_mask(&r.game, 0)[36], 1.0);
    r.game.resolve_selection(0, 36).unwrap();
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(r.hand_size(0), before + 35);
    assert!(victim_alive(&r));
    for _ in 0..34 {
        let v = r
            .pending_selection_view()
            .expect("one of the remaining 34 effects");
        if v.kind == SelectionKind::EffectChoice {
            decide(&mut r, v.valid_action_ids[0]);
        }
        decide(&mut r, PASS);
    }
    assert_eq!(r.hand_size(0), before + 35);
    assert!(victim_alive(&r));
    assert!(r.pending_selection().is_none());
}
#[test]
fn pending_candidate_cannot_rebind_to_new_same_id_physical_card() {
    let mut r = runner(true);
    choose_if_order(&mut r, 0);
    decide(&mut r, PASS);
    let index = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "SECOND")
        .unwrap();
    r.game.players[0].battle_area.remove(index);
    r.place_on_field(0, "SECOND", None);
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert!(
        !victim_alive(&r),
        "fresh same-ID replacement source cannot inherit original event's pending activation"
    );
    assert!(r.pending_selection().is_none());
}
#[test]
fn removed_subject_does_not_commit_old_event_against_fresh_same_id_permanent() {
    let mut r = runner(true);
    choose_if_order(&mut r, 0);
    r.game.players[0].battle_area.remove(0);
    r.place_on_field(0, "VICTIM", None);
    decide(&mut r, PASS);
    assert!(victim_alive(&r));
    assert!(r.pending_selection().is_none());
}
#[test]
fn remaining_candidate_tracks_same_physical_host_after_compaction() {
    let mut r = runner(true);
    choose_if_order(&mut r, 0);
    decide(&mut r, PASS);
    let index = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "FIRST")
        .unwrap();
    r.game.players[0].battle_area.remove(index);
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert!(victim_alive(&r));
    assert!(r.pending_selection().is_none());
}

struct DeleteOwnNestedCost;
impl CardEffect for DeleteOwnNestedCost {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldBeDeleted)
            .name("Opponent replacement deletes its own cost")
            .optional()
            .replacement_condition(|ctx, subject| {
                subject.permanent().is_some_and(|h| {
                    ctx.game.player(h.player).battle_area[h.index as usize]
                        .top_card()
                        .card_id(&ctx.game.card_data)
                        == "VICTIM"
                })
            })
            .replacement_process(|ctx| {
                let index = ctx
                    .effect
                    .game
                    .player(1)
                    .battle_area
                    .iter()
                    .position(|p| p.top_card().card_id(&ctx.effect.game.card_data) == "COST")
                    .unwrap();
                ctx.effect
                    .delete_permanent(digimon_engine::permanent::PermanentHandle {
                        player: 1,
                        index: index as u8,
                    });
                ctx.cancel();
            })
            .build()]
    }
}
struct OwnEffectCostProtection;
impl CardEffect for OwnEffectCostProtection {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldBeDeleted)
            .name("Cost protected from its own controller's effect")
            .replacement_condition(|ctx, subject| {
                ctx.replacement_source_controller() == Some(1)
                    && subject.permanent().is_some_and(|h| {
                        ctx.game.player(h.player).battle_area[h.index as usize]
                            .top_card()
                            .card_id(&ctx.game.card_data)
                            == "COST"
                    })
            })
            .replacement_process(|ctx| ctx.cancel())
            .build()]
    }
}
#[test]
fn replacement_subeffect_uses_its_controller_not_original_event_controller_including_clone() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("VICTIM", "Victim"))
        .add_card(make_test_card("FIRST", "Replacement"))
        .add_card(make_test_card("COST", "Cost"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    r.register_effect("FIRST", Arc::new(DeleteOwnNestedCost));
    r.register_effect("COST", Arc::new(OwnEffectCostProtection));
    r.skip_mulligan();
    let victim = r.place_on_field(0, "VICTIM", None);
    r.place_on_field(1, "FIRST", None);
    r.place_on_field(1, "COST", None);
    r.game.set_effect_source_player_for_test(Some(0));
    r.game.delete_permanent_with_effects(victim);
    let saved = r.game.clone();
    for game in [saved.clone(), saved] {
        r.game = game;
        decide(&mut r, REPLACEMENT_ACCEPT);
        assert!(victim_alive(&r));
        assert!(r.game.player(1).battle_area.iter().any(|p|p.top_card().card_id(&r.game.card_data)=="COST"),"P1 replacement's nested deletion is an own effect for P1; original P0 cause must not leak into it");
        assert!(r.pending_selection().is_none());
    }
}

fn is_named_subject(
    ctx: &digimon_engine::effect_context::EffectReadContext<'_>,
    subject: &digimon_engine::replacement::ReplacementSubject,
    id: &str,
) -> bool {
    subject.permanent().is_some_and(|h| {
        ctx.game.player(h.player).battle_area[h.index as usize]
            .top_card()
            .card_id(&ctx.game.card_data)
            == id
    })
}
struct OuterNestedDeletion;
impl CardEffect for OuterNestedDeletion {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldBeDeleted)
            .replacement_condition(|ctx, s| is_named_subject(ctx, s, "DRIVER-NEST"))
            .replacement_process(|ctx| {
                let index = ctx
                    .effect
                    .game
                    .player(0)
                    .battle_area
                    .iter()
                    .position(|p| p.top_card().card_id(&ctx.effect.game.card_data) == "COST-NEST")
                    .unwrap();
                ctx.effect
                    .delete_permanent(digimon_engine::permanent::PermanentHandle {
                        player: 0,
                        index: index as u8,
                    });
            })
            .build()]
    }
}
struct InnerNativePick;
impl CardEffect for InnerNativePick {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldBeDeleted)
            .replacement_condition(|ctx, s| is_named_subject(ctx, s, "COST-NEST"))
            .replacement_process(|ctx| {
                ctx.effect.select_own_permanent(
                    "Nested body choice",
                    false,
                    |_, _| true,
                    |ctx, _| {
                        ctx.draw(0, 1);
                    },
                )
            })
            .build()]
    }
}
struct AfterNestedCost;
impl CardEffect for AfterNestedCost {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldBeDeleted)
            .optional()
            .replacement_condition(|ctx, s| {
                is_named_subject(ctx, s, "DRIVER-NEST")
                    && !ctx
                        .game
                        .player(0)
                        .battle_area
                        .iter()
                        .any(|p| p.top_card().card_id(&ctx.game.card_data) == "COST-NEST")
            })
            .replacement_process(|ctx| ctx.cancel())
            .build()]
    }
}
#[test]
fn enclosing_dsl_tail_waits_for_outer_window_after_nested_native_replacement_body() {
    const DRIVER: &str = r#"
card: DRIVER-NEST
name: Driver nested
kind: digimon
color: [red]
level: 3
cost: 3
dp: 2000
effects:
 - when: on_play
   process:
    - delete_permanent: { target: source }
    - gain_memory: 5
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(DRIVER)
        .unwrap()
        .add_card(make_test_card("OUTER-NEST", "Outer"))
        .add_card(make_test_card("INNER-NEST", "Inner"))
        .add_card(make_test_card("AFTER-NEST", "After"))
        .add_card(make_test_card("COST-NEST", "Cost"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    r.register_effect("OUTER-NEST", Arc::new(OuterNestedDeletion));
    r.register_effect("INNER-NEST", Arc::new(InnerNativePick));
    r.register_effect("AFTER-NEST", Arc::new(AfterNestedCost));
    r.skip_mulligan();
    let h = r.place_on_field(0, "DRIVER-NEST", None);
    for id in ["OUTER-NEST", "INNER-NEST", "AFTER-NEST", "COST-NEST"] {
        r.place_on_field(0, id, None);
    }
    r.game.enqueue_triggered(
        EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(h),
    );
    r.game.drain_effect_queue();
    let view = r
        .pending_selection_view()
        .expect("nested native body choice");
    assert_eq!(view.kind, SelectionKind::OwnField);
    let a = view.valid_action_ids[0];
    assert_eq!(
        build_action_mask(&r.game, view.selecting_player)[a as usize],
        1.0
    );
    r.game.resolve_selection(view.selecting_player, a).unwrap();
    assert!(!r
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "COST-NEST"));
    assert_eq!(
        r.memory(),
        3,
        "initiating DSL tail belongs to outer event, not the innermost parked replacement"
    );
    decide(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(r.memory(), 8);
    assert!(r.pending_selection().is_none());
}

#[test]
fn one_printed_partition_is_one_optional_effect_not_two_ordered_candidates() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-037")
        .unwrap()
        .dsl_card("BT20-036")
        .unwrap()
        .dsl_card("EX8-074")
        .unwrap()
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    r.skip_mulligan();
    let h = r.place_stack(0, &["BT20-036", "EX8-074", "BT20-037"]);
    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanent_with_effects(h);
    let v = r
        .pending_selection_view()
        .expect("Partition must be offered");
    assert_eq!(
        v.kind,
        SelectionKind::Replacement,
        "printed/self-declarative Partition is one optional effect, not two synthetic copies"
    );
    assert!(v.is_optional);
    let saved = r.game.clone();
    for game in [saved.clone(), saved] {
        r.game = game;
        decide(&mut r, PASS);
        assert!(
            r.pending_selection().is_none(),
            "declining the sole Partition must not offer a duplicate"
        );
        assert_eq!(
            r.game
                .player(0)
                .trash
                .iter()
                .filter(|c| c.card_id(&r.game.card_data) == "BT20-037")
                .count(),
            1
        );
    }
}
