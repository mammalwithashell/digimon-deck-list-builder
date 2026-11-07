use super::*;
use crate::debug_runner::{make_test_card, DebugRunner};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn context(marked: bool, nested: bool) -> TriggerContext {
    let mut result = TriggerContext::default();
    result.projected_reference_reads_guarded = marked;
    result.security_check_face_generation = Some(3);
    if nested {
        let mut outer = TriggerContext::default();
        outer.security_check_face_generation = Some(3);
        outer.simultaneous_play_members.push(result);
        result = outer;
    }
    result
}

fn queued(player: u8, source: CardHandle, host: PermanentHandle, context: TriggerContext) -> QueuedEffect {
    QueuedEffect {
        source_card: source, source_permanent: Some(host), source_kind: EffectSourceKind::Digimon,
        source_identity: None, // Provenance has not been captured in this synthetic queue entry.
        attribution_source_card: None, attribution_source_kind: None, bypass_once_per_turn: false,
        controller: player, timing: EffectTiming::OnPlay, trigger_context: Some(context),
        effect_slot: 0, is_optional: false, is_turn_player: player == 0, card_id: "GG".into(),
        allow_below_top_liveness: false, dna_origin_context: None, granted_effect_id: Some(811),
        keyword_effect: None, trigger_batch: 0, pending_processing: None,
    }
}

#[test]
fn granted_qualifier_checks_installed_context_and_restores_outer_context() {
    for player in [0, 1] { for marked in [false, true] { for nested in [false, true] {
        for accepted in [false, true] {
            let mut r = DebugRunner::builder().add_card(make_test_card("GG", "Carrier")).start();
            let host = r.place_on_field(player, "GG", Some(0));
            let source = r.game.players[player as usize].battle_area[host.index as usize].top_card().handle();
            let qe = queued(player, source, host, context(marked, nested));
            let calls = Arc::new(AtomicUsize::new(0));
            let count = calls.clone();
            // Insert after queue construction, directly through the public map.
            r.game.granted_effect_bodies.qualifiers.insert(811, Arc::new(move |ctx| {
                count.fetch_add(1, Ordering::SeqCst);
                assert_eq!(ctx.source_card, source);
                assert_eq!(ctx.source_permanent, Some(host));
                assert_eq!(ctx.player(), player);
                assert_eq!(ctx.game.current_trigger_context.as_ref().unwrap().security_check_face_generation, Some(3));
                accepted
            }));
            r.game.granted_effect_bodies = r.game.granted_effect_bodies.clone();
            let mut outer = TriggerContext::default();
            outer.security_check_face_generation = Some(6);
            r.game.current_trigger_context = Some(outer);
            let result = catch_unwind(AssertUnwindSafe(||
                r.game.queued_single_effect_gate_passes(&qe, QueuedGate::TriggerQualifier)));
            if marked {
                assert!(result.is_err(), "marked granted qualifier must reject");
                assert_eq!(calls.load(Ordering::SeqCst), 0);
            } else {
                assert_eq!(result.unwrap(), accepted);
                assert_eq!(calls.load(Ordering::SeqCst), 1);
            }
            assert_eq!(r.game.current_trigger_context.as_ref().unwrap().security_check_face_generation, Some(6));
            assert!(r.game.granted_effect_bodies.qualifiers.contains_key(&811));
        }
    } } }
}

#[test]
fn granted_body_checks_queued_context_before_body_or_registry_cleanup() {
    for player in [0, 1] { for marked in [false, true] { for nested in [false, true] {
        let mut r = DebugRunner::builder().add_card(make_test_card("GG", "Carrier")).memory(2).start();
        let host = r.place_on_field(player, "GG", Some(0));
        let source = r.game.players[player as usize].battle_area[host.index as usize].top_card().handle();
        let qe = queued(player, source, host, context(marked, nested));
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        // No live modifier entry retains this body: successful execution must prune it.
        r.game.granted_effect_bodies.bodies.insert(811, Arc::new(move |ctx| {
            count.fetch_add(1, Ordering::SeqCst);
            assert_eq!(ctx.source_card, source);
            assert_eq!(ctx.source_permanent, Some(host));
            assert_eq!(ctx.player, player);
            assert_eq!(ctx.game.current_trigger_context.as_ref().unwrap().security_check_face_generation, Some(3));
            ctx.game.memory = 5;
        }));
        r.game.granted_effect_bodies = r.game.granted_effect_bodies.clone();
        let mut outer = TriggerContext::default();
        outer.security_check_face_generation = Some(6);
        r.game.current_trigger_context = Some(outer);
        let result = catch_unwind(AssertUnwindSafe(|| r.game.run_queued_effect(qe)));
        if marked {
            assert!(result.is_err(), "marked granted body must reject");
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(r.game.memory, 2);
            assert!(r.game.granted_effect_bodies.bodies.contains_key(&811));
            // The failed hypothetical state is discarded; this test does not
            // promise rollback of run_queued_effect's enclosing attribution scope.
        } else {
            result.unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert_eq!(r.game.memory, 5);
            assert!(!r.game.granted_effect_bodies.bodies.contains_key(&811));
            assert_eq!(r.game.current_trigger_context.as_ref().unwrap().security_check_face_generation, Some(6));
        }
    } } }
}

#[test]
fn absent_granted_callbacks_keep_existing_noop_and_true_gate_results() {
    for player in [0, 1] {
        let mut r = DebugRunner::builder().add_card(make_test_card("GG", "Carrier")).memory(2).start();
        let host = r.place_on_field(player, "GG", Some(0));
        let source = r.game.players[player as usize].battle_area[host.index as usize].top_card().handle();
        let qe = queued(player, source, host, context(true, true));
        assert!(r.game.queued_single_effect_gate_passes(&qe, QueuedGate::TriggerQualifier));
        assert!(r.game.queued_single_effect_gate_passes(&qe, QueuedGate::Condition));
        r.game.run_queued_effect(qe);
        assert_eq!(r.game.memory, 2);
        assert!(r.game.granted_effect_bodies.bodies.is_empty());
    }
}

#[test]
fn remaining_granted_inline_body_checks_current_context() {
    for player in [0, 1] { for marked in [false, true] { for nested in [false, true] {
        let mut r = DebugRunner::builder().add_card(make_test_card("GG", "Carrier")).memory(2).start();
        let host = r.place_on_field(player, "GG", Some(0));
        let source = r.game.players[player as usize].battle_area[host.index as usize].top_card().handle();
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        EffectContext::new(&mut r.game, source, Some(host), player).grant_triggered_effect(
            host, EffectTiming::OnPlay, crate::enums::Expiry::Permanent, move |ctx| {
                count.fetch_add(1, Ordering::SeqCst);
                assert_eq!(ctx.source_card, source);
                assert_eq!(ctx.source_permanent, Some(host));
                assert_eq!(ctx.player, player);
                ctx.game.memory = 5;
            });
        r.game.current_trigger_context = Some(context(marked, nested));
        r.game.fire_granted_triggered_effects(host, EffectTiming::OnDeletion);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let result = catch_unwind(AssertUnwindSafe(||
            r.game.fire_granted_triggered_effects(host, EffectTiming::OnPlay)));
        if marked {
            assert!(result.is_err(), "marked inline granted body must reject");
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(r.game.memory, 2);
        } else {
            result.unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert_eq!(r.game.memory, 5);
        }
        assert_eq!(r.game.granted_effect_bodies.bodies.len(), 1);
    } } }
}

#[test]
fn remaining_granted_dynamic_predicate_checks_current_context() {
    for player in [0, 1] { for marked in [false, true] { for nested in [false, true] {
        for accepted in [false, true] {
            let mut r = DebugRunner::builder().add_card(make_test_card("GG", "Carrier")).start();
            let host = r.place_on_field(player, "GG", Some(0));
            let source = r.game.players[player as usize].battle_area[host.index as usize].top_card().handle();
            let calls = Arc::new(AtomicUsize::new(0));
            let count = calls.clone();
            EffectContext::new(&mut r.game, source, Some(host), player).grant_dynamic_triggered_effect(
                EffectTiming::OnPlay, crate::enums::Expiry::EndOfTurn,
                move |ctx, candidate| {
                    count.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(ctx.source_card, source);
                    assert_eq!(ctx.source_permanent, Some(host));
                    assert_eq!(ctx.player(), player);
                    assert_eq!(candidate, host);
                    accepted
                },
                |_| panic!("collecting dynamic grants must not run the body"));
            r.game.current_trigger_context = Some(context(marked, nested));
            assert!(r.game.granted_entries_for_carrier(host, EffectTiming::OnDeletion).is_empty());
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            let result = catch_unwind(AssertUnwindSafe(||
                r.game.granted_entries_for_carrier(host, EffectTiming::OnPlay)));
            if marked {
                assert!(result.is_err(), "marked dynamic grant predicate must reject");
                assert_eq!(calls.load(Ordering::SeqCst), 0);
            } else {
                assert_eq!(result.unwrap().len(), usize::from(accepted));
                assert_eq!(calls.load(Ordering::SeqCst), 1);
            }
            assert_eq!(r.game.modifiers.dynamic_triggered.len(), 1);
        }
    } } }
}
