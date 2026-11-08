use super::*;
use crate::debug_runner::{make_test_card, DebugRunner};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn group(bits: u8) -> TriggerContext {
    let mut root = TriggerContext::default();
    root.security_check_face_generation = Some(90);
    root.projected_reference_reads_guarded = bits & 1 != 0;
    for index in 0..2 {
        let mut member = TriggerContext::default();
        member.security_check_face_generation = Some(10 + index);
        member.projected_reference_reads_guarded = bits & (2 << index) != 0;
        if index == 1 {
            let mut nested = TriggerContext::default();
            nested.security_check_face_generation = Some(30);
            nested.projected_reference_reads_guarded = bits & 8 != 0;
            member.simultaneous_play_members.push(nested);
        }
        root.simultaneous_play_members.push(member);
    }
    root
}

fn fixture(player: u8, context: TriggerContext) -> (DebugRunner, QueuedEffect) {
    let mut r = DebugRunner::builder().add_card(make_test_card("SG", "Carrier")).memory(2).start();
    let host = r.place_on_field(player, "SG", Some(0));
    let source = r.game.players[player as usize].battle_area[host.index as usize].top_card().handle();
    let qe = QueuedEffect {
        source_card: source, source_permanent: Some(host), source_kind: EffectSourceKind::Digimon,
        source_identity: None, // Provenance has not been captured in this synthetic queue entry.
        attribution_source_card: None, attribution_source_kind: None, bypass_once_per_turn: false,
        controller: player, timing: EffectTiming::OnPlay, trigger_context: Some(context),
        effect_slot: 0, is_optional: false, is_turn_player: player == 0, card_id: "SG".into(),
        allow_below_top_liveness: false, dna_origin_context: None, granted_effect_id: Some(922),
        keyword_effect: None, trigger_batch: 0, pending_processing: None,
    };
    (r, qe)
}

#[test]
fn simultaneous_qualifier_cannot_short_circuit_away_parent_or_sibling_obligation() {
    for player in [0, 1] { for bits in 0..16 { for accepted in [false, true] {
        let (mut r, qe) = fixture(player, group(bits));
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        r.game.granted_effect_bodies.qualifiers.insert(922, Arc::new(move |ctx| {
            let n = count.fetch_add(1, Ordering::SeqCst);
            assert_eq!(ctx.game.current_trigger_context.as_ref().unwrap().security_check_face_generation, Some(10 + n as u64));
            accepted
        }));
        let mut outer = TriggerContext::default();
        outer.security_check_face_generation = Some(77);
        r.game.current_trigger_context = Some(outer);
        let result = catch_unwind(AssertUnwindSafe(||
            r.game.queued_entry_gate_passes(&qe, QueuedGate::TriggerQualifier)));
        if bits != 0 {
            assert!(result.is_err(), "group bits {bits} must reject before any qualifier");
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        } else {
            assert_eq!(result.unwrap(), accepted);
            assert_eq!(calls.load(Ordering::SeqCst), if accepted { 1 } else { 2 });
        }
        assert_eq!(r.game.current_trigger_context.as_ref().unwrap().security_check_face_generation, Some(77));
    } } }
}

#[test]
fn simultaneous_matching_members_retain_union_without_changing_subjects_or_input() {
    for player in [0, 1] { for bits in 0..16 {
        let (mut r, qe) = fixture(player, group(bits));
        let before = format!("{:?}", qe.trigger_context);
        let members = r.game.matching_simultaneous_play_contexts(&qe);
        assert_eq!(members.len(), 2);
        for (index, member) in members.iter().enumerate() {
            assert_eq!(member.security_check_face_generation, Some(10 + index as u64));
            assert_eq!(member.projected_reference_reads_guarded, bits != 0, "bits {bits}, member {index}");
            assert_eq!(member.simultaneous_play_members.len(), index);
        }
        assert_eq!(format!("{:?}", qe.trigger_context), before, "never mutate original queue entry");
        // Condition/outer-option consumers retain their ordinary no-callback results.
        assert!(r.game.queued_entry_gate_passes(&qe, QueuedGate::Condition));
        assert!(!r.game.queued_effect_wants_outer_optional_prompt(&qe));
    } }
}

#[test]
fn simultaneous_body_rejects_obligation_even_after_single_member_selection() {
    for player in [0, 1] { for bits in 0..16 { for members in [1, 2] {
        let mut input = group(bits);
        if members == 1 {
            // Retain a nested group in the sole member so all four obligation
            // positions remain represented when the outer group becomes one.
            let children = std::mem::take(&mut input.simultaneous_play_members);
            let mut only = TriggerContext::default();
            only.security_check_face_generation = Some(10);
            only.simultaneous_play_members = children;
            input.simultaneous_play_members.push(only);
        }
        let (mut r, qe) = fixture(player, input);
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        r.game.granted_effect_bodies.bodies.insert(922, Arc::new(move |ctx| {
            count.fetch_add(1, Ordering::SeqCst);
            assert_eq!(ctx.game.current_trigger_context.as_ref().unwrap().security_check_face_generation, Some(10));
            ctx.game.memory = 9;
        }));
        let result = catch_unwind(AssertUnwindSafe(|| r.game.run_queued_effect(qe)));
        if bits != 0 {
            assert!(result.is_err(), "group bits {bits} with {members} members must reject");
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(r.game.memory, 2);
            assert!(r.game.granted_effect_bodies.bodies.contains_key(&922));
        } else {
            result.unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert_eq!(r.game.memory, 9);
            assert!(!r.game.granted_effect_bodies.bodies.contains_key(&922));
        }
    } } }
}
