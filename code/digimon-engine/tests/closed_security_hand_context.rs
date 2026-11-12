//! Synthetic mechanism tests only: no checkpoint, scored request, or recorded
//! action prefix is loaded. Unentitled references reject actual reads; opaque
//! callbacks reject before their bodies. Ordinary controls retain gameplay checks.
#[path = "determinization/support.rs"]
#[allow(dead_code)]
mod support;

use digimon_dsl::compiled::{CompiledFormula, CompiledStep};
use digimon_engine::{
    action::space::{ATTACK_START, HAND_EFFECT_START, PASS},
    card_source::CardHandle,
    determinize::{beliefs::*, closed::materialize_closed},
    dsl_cards::{bindings::Bindings, step::StepRuntime},
    enums::{EffectSourceKind, GamePhase, Zone},
    permanent::Permanent,
    resume::{
        OuterContinuation, ResumeDecline, ResumeFrame, ResumeProvenance, ResumeSelectKind,
        ResumeStack,
    },
    selection::{PendingSelection, SelectionKind},
    trigger_context::{EventSubject, MovedCardSet, TriggerContext},
    Game,
};
use std::{collections::BTreeSet, sync::{Arc, atomic::{AtomicUsize, Ordering}}, panic::{catch_unwind, AssertUnwindSafe}};

fn prior() -> WeightedDeckPrior {
    let cards = support::owned(support::D1);
    WeightedDeckPrior::new(
        PriorSnapshot::new(
            "standard",
            "synthetic-security-hand-context",
            cards.iter().cloned().collect(),
            vec![CandidateDeck { cards, weight: 1.0 }],
            0.1,
            128,
        )
        .unwrap(),
    )
    .unwrap()
}

fn fixture(viewer: u8, dependent: bool) -> (Game, CardHandle) {
    let mut g = support::new_game_with(730031, support::D1, support::D1);
    g.enable_belief_knowledge().unwrap();
    g.start_game();
    g.mulligan_pending.clear();
    let owner = 1 - viewer;
    let event = g.players[owner as usize].security.last().unwrap().handle();
    assert!(g.add_to_hand_from_security(owner, event));
    let source_card = g.players[viewer as usize].hand[0].handle();
    g.turn_order = vec![viewer, owner];
    g.turn_player_idx = 0;
    g.memory = 0;
    g.current_phase = GamePhase::EffectChoice;
    g.pending_selection = Some(PendingSelection {
        card_choice: None,
        kind: SelectionKind::EffectChoice,
        selecting_player: viewer,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![HAND_EFFECT_START],
        is_optional: false,
        prompt: "synthetic security-to-hand continuation".into(),
        effect_choices: None,
        source_card,
        source_permanent: None,
        source_kind: EffectSourceKind::Digimon,
        callback: Box::new(|_, _| panic!("typed resume required")),
        on_decline: None,
        zone_owner: None,
    });
    let mut context = TriggerContext::default();
    context.event_card = (Some(event)).into();
    g.pending_selection_resume = Some(ResumeStack {
        frames: vec![ResumeFrame::RunTail {
            prov: ResumeProvenance {
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                controller: viewer,
                override_pin: None,
            },
            select_kind: ResumeSelectKind::EffectChoice { post: None },
            bind_as: None,
            inner_tail: Arc::new(vec![if dependent {
                CompiledStep::GainMemoryFn {
                    formula: CompiledFormula::EventTargetLevel,
                }
            } else {
                CompiledStep::GainMemory(1)
            }]),
            outer_conts: vec![],
            bindings: Bindings::new(),
            runtime: StepRuntime::default(),
            trigger_context: Some(context),
            decline: ResumeDecline::None,
        }],
    });
    (g, event)
}

fn anonymous_hand_token(memory: &KnowledgeLedger, event: CardHandle) -> bool {
    let json = serde_json::to_value(memory).unwrap();
    assert!(
        !json["cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["instance"] == event.0),
        "concealed movement must not disclose the actual card identity"
    );
    json["concealed_positions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["instance"] == event.0 && c["zone"] == "hand")
}

#[test]
fn tracked_security_to_hand_context_uses_sampled_identity_on_both_seats() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (g, event) = fixture(viewer, true);
            let memory = g.belief_knowledge(viewer).unwrap().clone();
            assert!(anonymous_hand_token(&memory, event));
            let mut identities = BTreeSet::new();
            for seed in 0..16 {
                let mut world = materialize_closed(&g, viewer, &memory, &prior(), seed).unwrap();
                let card = world.card_data_for_handle(event).unwrap();
                let expected = card.level.unwrap_or(0) as i16;
                identities.insert(
                    world.players[(1 - viewer) as usize]
                        .hand
                        .iter()
                        .find(|c| c.handle() == event)
                        .unwrap()
                        .data_index,
                );
                world.decode_action(HAND_EFFECT_START, viewer);
                assert_eq!(world.memory, expected);
            }
            assert!(
                identities.len() > 1,
                "a trackable token is not a known identity"
            );
            assert_eq!(g.belief_knowledge(viewer).unwrap(), &memory);
            assert_eq!(g.memory, 0, "hypotheses must not mutate the real root");
        }
    });
}

#[test]
fn concealed_true_identity_does_not_change_materialized_context_or_outcome() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (g, event) = fixture(viewer, true);
            let mut twin = g.clone();
            let p = &mut twin.players[(1 - viewer) as usize];
            let index = p.hand.iter().position(|c| c.handle() == event).unwrap();
            let other = p
                .deck
                .iter()
                .position(|c| c.data_index != p.hand[index].data_index)
                .unwrap();
            std::mem::swap(&mut p.hand[index].data_index, &mut p.deck[other].data_index);
            let memory = g.belief_knowledge(viewer).unwrap();
            assert_eq!(memory, twin.belief_knowledge(viewer).unwrap());
            for seed in 0..8 {
                let mut a = materialize_closed(&g, viewer, memory, &prior(), seed).unwrap();
                let mut b = materialize_closed(&twin, viewer, memory, &prior(), seed).unwrap();
                assert_eq!(support::full_zone_dump(&a), support::full_zone_dump(&b));
                a.decode_action(HAND_EFFECT_START, viewer);
                b.decode_action(HAND_EFFECT_START, viewer);
                assert_eq!(a.memory, b.memory);
            }
        }
    });
}

fn assert_untracked_context_rejected(dependent: bool) {
    for viewer in 0..2 {
        let (g, event) = fixture(viewer, dependent);
        let original = g.belief_knowledge(viewer).unwrap().clone();
        let mut memory = original.clone();
        // Public conservative invalidation; not a staged real departure.
        memory.forget_zone(1 - viewer, KnowledgeZone::Hand);
        assert!(!anonymous_hand_token(&memory, event));
        for seed in 0..4 {
            let mut world = materialize_closed(&g, viewer, &memory, &prior(), seed).unwrap();
            assert_read_rejected(|| {
                if let ResumeFrame::RunTail { trigger_context, .. } = frame(&mut world) {
                    trigger_context.as_ref().unwrap().event_card.read();
                } else { panic!("unexpected fixture frame"); }
            });
            assert_legal_resume_rejected(&mut world, viewer, HAND_EFFECT_START,
                "closed-list unavailable event-card reference read");
            assert_eq!(world.memory, 0);
        }
        assert_eq!(g.belief_knowledge(viewer).unwrap(), &original);
        assert_eq!(g.memory, 0);
    }
}

#[test]
fn invalidated_identity_dependent_tail_remains_strictly_rejected() {
    support::with_big_stack(|| assert_untracked_context_rejected(true));
}

#[test]
fn invalidated_literal_tail_is_projected_only_in_hypothesis() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (g, _) = fixture(viewer, false);
            let original = g.belief_knowledge(viewer).unwrap().clone();
            let mut memory = original.clone();
            memory.forget_zone(1 - viewer, KnowledgeZone::Hand);
            let mut w = materialize_closed(&g, viewer, &memory, &prior(), 81).unwrap();
            w.resolve_selection(viewer, HAND_EFFECT_START).unwrap();
            assert_eq!(w.memory, 1);
            assert_eq!(g.belief_knowledge(viewer).unwrap(), &original);
            if let ResumeFrame::RunTail {
                trigger_context, ..
            } = &g.pending_selection_resume.as_ref().unwrap().frames[0]
            {
                assert!(trigger_context.as_ref().unwrap().event_card.read().is_some());
            }
        }
    });
}

fn security_context(event: CardHandle, field: usize) -> TriggerContext {
    let mut c = TriggerContext::default();
    if field == 0 || field == 3 {
        c.event_card = (Some(event)).into();
    }
    if field == 1 || field == 3 {
        c.subject = (Some(EventSubject::Card {
            card: event,
            zone: Zone::Security,
        })).into();
    }
    if field == 2 || field == 3 {
        c.moved_card_sets.push(MovedCardSet {
            cards: vec![event],
            from: Some(Zone::Security),
            to: Some(Zone::Hand),
        });
    }
    c
}

fn frame(g: &mut Game) -> &mut ResumeFrame {
    &mut g.pending_selection_resume.as_mut().unwrap().frames[0]
}

fn field_fixture(viewer: u8) -> (Game, CardHandle) {
    let (mut g, event) = fixture(viewer, false);
    let owner = viewer as usize;
    let index = g.players[owner]
        .hand
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, c)| g.card_data[c.data_index].level.is_some())
        .unwrap()
        .0;
    let card = g.players[owner].hand.remove(index);
    g.players[owner].battle_area.push(Permanent::new(card, 0));
    let p = g.pending_selection.as_mut().unwrap();
    p.kind = SelectionKind::OwnField;
    p.valid_action_ids = vec![ATTACK_START];
    p.is_optional = true;
    if let ResumeFrame::RunTail {
        select_kind,
        trigger_context,
        decline,
        ..
    } = frame(&mut g)
    {
        *select_kind = ResumeSelectKind::FieldPermanent {
            of_player: viewer,
            post: None,
        };
        *trigger_context = Some(security_context(event, 3));
        *decline = ResumeDecline::RunTail {
            tail: Arc::new(vec![CompiledStep::GainMemory(3)]),
            aborts_clause: false,
        };
    }
    (g, event)
}

fn add_outer(g: &mut Game, viewer: u8, context: Option<TriggerContext>, tail: Vec<CompiledStep>) {
    let source_card = g.pending_selection.as_ref().unwrap().source_card;
    if let ResumeFrame::RunTail { outer_conts, .. } = frame(g) {
        outer_conts.push(OuterContinuation {
            tail: Arc::new(tail),
            bindings: Bindings::new(),
            runtime: StepRuntime::default(),
            trigger_context: context,
            source_card,
            source_permanent: None,
            player: viewer,
        });
    }
}

fn assert_panics_with<T>(f: impl FnOnce() -> T, expected: &str) {
    let error = match catch_unwind(AssertUnwindSafe(f)) {
        Err(error) => error,
        Ok(_) => panic!("expected checked rejection: {expected}"),
    };
    let message = error.downcast_ref::<String>().map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied()).unwrap_or("");
    assert!(message.contains(expected), "unexpected panic: {message}");
}

fn assert_read_rejected<T>(f: impl FnOnce() -> T) {
    assert_panics_with(f, "closed-list unavailable event-card reference read");
}

fn assert_legal_resume_rejected(world: &mut Game, viewer: u8, action: u16, expected: &str) {
    assert!(digimon_engine::build_action_mask(world, viewer)[action as usize] > 0.5);
    assert_panics_with(|| world.resolve_selection(viewer, action).unwrap(), expected);
}

#[test]
fn security_origin_subject_and_moved_sets_each_require_entitlement() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            for field in 0..3 {
                let (mut g, event) = field_fixture(viewer);
                if let ResumeFrame::RunTail {
                    trigger_context,
                    inner_tail,
                    ..
                } = frame(&mut g)
                {
                    *trigger_context = Some(security_context(event, field));
                    *inner_tail = Arc::new(vec![CompiledStep::GainMemoryFn {
                        formula: CompiledFormula::EventTargetLevel,
                    }]);
                }
                let mut memory = g.belief_knowledge(viewer).unwrap().clone();
                let mut w = materialize_closed(&g, viewer, &memory, &prior(), 81).unwrap();
                let expected = if field == 0 { w.card_data_for_handle(event).unwrap().level.unwrap_or(0) as i16 } else { 0 };
                w.resolve_selection(viewer, ATTACK_START).unwrap();
                assert_eq!(w.memory, expected);
                memory.forget_zone(1 - viewer, KnowledgeZone::Hand);
                let mut checked = materialize_closed(&g, viewer, &memory, &prior(), 81).unwrap();
                if let ResumeFrame::RunTail { trigger_context, .. } = frame(&mut checked) {
                    let c = trigger_context.as_ref().unwrap();
                    match field {
                        0 => assert_read_rejected(|| c.event_card.read()),
                        1 => assert_read_rejected(|| c.subject.read()),
                        _ => assert_read_rejected(|| c.moved_card_sets.as_slice()),
                    }
                    assert!(!format!("{c:?}").contains(&format!("CardHandle({})", event.0)));
                }
                if field == 0 {
                    assert_legal_resume_rejected(&mut checked, viewer, ATTACK_START,
                        "closed-list unavailable event-card reference read");
                } else {
                    checked.resolve_selection(viewer, ATTACK_START).unwrap();
                    assert_eq!(checked.memory, 0, "this formula does not read subject or movement sets");
                }
                assert_eq!(g.memory, 0);
            }
        }
    });
}

#[test]
fn field_success_and_decline_preserve_two_outer_contexts() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (mut g, event) = field_fixture(viewer);
            add_outer(
                &mut g,
                viewer,
                Some(security_context(event, 3)),
                vec![CompiledStep::GainMemory(2)],
            );
            add_outer(
                &mut g,
                viewer,
                Some(security_context(event, 3)),
                vec![CompiledStep::GainMemory(4)],
            );
            for (action, expected) in [(ATTACK_START, 7), (PASS, 9)] {
                let mut w = materialize_closed(
                    &g,
                    viewer,
                    g.belief_knowledge(viewer).unwrap(),
                    &prior(),
                    81,
                )
                .unwrap();
                w.resolve_selection(viewer, action).unwrap();
                assert_eq!(w.memory, expected);
                assert!(w.pending_selection.is_none());
            }
            assert_eq!(g.memory, 0);
        }
    });
}

#[test]
fn literal_inner_tail_cannot_hide_identity_dependent_decline() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (mut g, event) = field_fixture(viewer);
            if let ResumeFrame::RunTail { decline, .. } = frame(&mut g) {
                *decline = ResumeDecline::RunTail {
                    tail: Arc::new(vec![CompiledStep::GainMemoryFn {
                        formula: CompiledFormula::EventTargetLevel,
                    }]),
                    aborts_clause: false,
                };
            }
            for seed in 0..8 {
                let mut w = materialize_closed(
                    &g,
                    viewer,
                    g.belief_knowledge(viewer).unwrap(),
                    &prior(),
                    seed,
                )
                .unwrap();
                let level = w.card_data_for_handle(event).unwrap().level.unwrap_or(0) as i16;
                w.resolve_selection(viewer, PASS).unwrap();
                assert_eq!(w.memory, level);
            }
            let mut memory = g.belief_knowledge(viewer).unwrap().clone();
            memory.forget_zone(1 - viewer, KnowledgeZone::Hand);
            let mut checked = materialize_closed(&g, viewer, &memory, &prior(), 81).unwrap();
            assert_legal_resume_rejected(&mut checked, viewer, PASS,
                "closed-list unavailable event-card reference read");
            assert_eq!(checked.memory, 0);
            assert_eq!(g.memory, 0);
        }
    });
}

#[test]
fn literal_inner_tail_cannot_hide_identity_dependent_outer_tail() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (mut g, event) = field_fixture(viewer);
            if let ResumeFrame::RunTail {
                trigger_context, ..
            } = frame(&mut g)
            {
                *trigger_context = None;
            }
            add_outer(
                &mut g,
                viewer,
                Some(security_context(event, 3)),
                vec![CompiledStep::GainMemoryFn {
                    formula: CompiledFormula::EventTargetLevel,
                }],
            );
            for seed in 0..8 {
                let mut w = materialize_closed(
                    &g,
                    viewer,
                    g.belief_knowledge(viewer).unwrap(),
                    &prior(),
                    seed,
                )
                .unwrap();
                let level = w.card_data_for_handle(event).unwrap().level.unwrap_or(0) as i16;
                w.resolve_selection(viewer, ATTACK_START).unwrap();
                assert_eq!(w.memory, 1 + level);
            }
            let mut memory = g.belief_knowledge(viewer).unwrap().clone();
            memory.forget_zone(1 - viewer, KnowledgeZone::Hand);
            let mut checked = materialize_closed(&g, viewer, &memory, &prior(), 81).unwrap();
            assert_legal_resume_rejected(&mut checked, viewer, ATTACK_START,
                "closed-list unavailable event-card reference read");
            assert_eq!(checked.memory, 1, "inner body completed before the outer read; no rollback claim");
            assert_eq!(g.memory, 0);
        }
    });
}

#[test]
fn literal_only_synthetic_projection_is_equivalent_for_success_and_decline() {
    // A differential proof for this exact synthetic literal-only body, not an
    // admission rule. The real audit remains strict and no production context
    // is changed. In particular, this says nothing about arbitrary post-actions.
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (mut g, event) = field_fixture(viewer);
            add_outer(
                &mut g,
                viewer,
                Some(security_context(event, 3)),
                vec![CompiledStep::GainMemory(2)],
            );
            let original = g.belief_knowledge(viewer).unwrap().clone();
            let mut forgotten = original.clone();
            forgotten.forget_zone(1 - viewer, KnowledgeZone::Hand);
            let mut projected = g.clone();
            if let ResumeFrame::RunTail {
                trigger_context,
                outer_conts,
                ..
            } = frame(&mut projected)
            {
                *trigger_context = None;
                for c in outer_conts {
                    c.trigger_context = None;
                }
            }
            for seed in 0..8 {
                for action in [ATTACK_START, PASS] {
                    let mut a = materialize_closed(&g, viewer, &original, &prior(), seed).unwrap();
                    let mut b = materialize_closed(&g, viewer, &forgotten, &prior(), seed).unwrap();
                    let mut oracle =
                        materialize_closed(&projected, viewer, &forgotten, &prior(), seed).unwrap();
                    a.resolve_selection(viewer, action).unwrap();
                    b.resolve_selection(viewer, action).unwrap();
                    oracle.resolve_selection(viewer, action).unwrap();
                    assert_eq!(b.memory, oracle.memory);
                    assert_eq!(a.memory, b.memory);
                    assert_eq!(a.pending_selection.is_none(), b.pending_selection.is_none());
                    assert_eq!(
                        a.players[viewer as usize].battle_area.len(),
                        b.players[viewer as usize].battle_area.len()
                    );
                }
            }
            assert_eq!(g.belief_knowledge(viewer).unwrap(), &original);
        }
    });
}

#[test]
fn checked_references_preserve_metadata_and_available_handles() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (mut g, event) = field_fixture(viewer);
            let source = g.pending_selection.as_ref().unwrap().source_card;
            if let ResumeFrame::RunTail {
                trigger_context, ..
            } = frame(&mut g)
            {
                let c = trigger_context.as_mut().unwrap();
                c.target_card = (Some(source)).into();
                c.affected_player = Some(1 - viewer);
                c.source_player = Some(viewer);
                c.cause = Some(digimon_engine::trigger_context::EventCause::SecurityRemoval);
                c.effect_initiated = true;
            }
            let mut memory = g.belief_knowledge(viewer).unwrap().clone();
            memory.forget_zone(1 - viewer, KnowledgeZone::Hand);
            let mut w = materialize_closed(&g, viewer, &memory, &prior(), 81).unwrap();
            if let ResumeFrame::RunTail {
                trigger_context, ..
            } = frame(&mut w)
            {
                let c = trigger_context.as_ref().unwrap();
                assert_read_rejected(|| c.event_card.read());
                assert_read_rejected(|| c.subject.read());
                assert_read_rejected(|| c.moved_card_sets.as_slice());
                assert!(!format!("{c:?}").contains(&format!("CardHandle({})", event.0)));
                // Movement metadata is retained internally, tested at its private
                // typed boundary. Public access to the unavailable container rejects.
                assert_eq!(c.target_card.read(), Some(source));
                assert_eq!(c.affected_player, Some(1 - viewer));
                assert_eq!(c.source_player, Some(viewer));
                assert_eq!(
                    c.cause,
                    Some(digimon_engine::trigger_context::EventCause::SecurityRemoval)
                );
                assert!(c.effect_initiated);
            }
            if let ResumeFrame::RunTail {
                trigger_context, ..
            } = frame(&mut g)
            {
                assert_eq!(trigger_context.as_ref().unwrap().event_card.read(), Some(event));
            }
        }
    });
}

#[test]
fn native_post_actions_keep_control_flow_and_registered_raw_tails_reject_before_body() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            for variant in 0..5 {
                let (mut g, event) = field_fixture(viewer);
                let calls = Arc::new(AtomicUsize::new(0));
                let c = calls.clone();
                let mut registry = digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry::new();
                registry.register_step("synthetic_opaque_not_invoked", move |_, _| { c.fetch_add(1, Ordering::SeqCst); });
                let runtime = StepRuntime::new(Arc::new(registry));
                let raw = CompiledStep::RawRust {
                    fn_name: "synthetic_opaque_not_invoked".into(), consumes: vec![], binds: vec![],
                };
                if let ResumeFrame::RunTail { select_kind, inner_tail, decline, runtime: r, .. } = frame(&mut g) {
                    *r = runtime.clone();
                    match variant {
                        0 => *select_kind = ResumeSelectKind::FieldPermanent { of_player: viewer,
                            post: Some(digimon_engine::resume::FieldPermanentPostAction::TrashBottomFaceDownSource) },
                        1 => *select_kind = ResumeSelectKind::EffectChoice {
                            post: Some(digimon_engine::resume::EffectChoicePostAction::RunTailBranch {
                                branches: vec![Arc::new(vec![CompiledStep::GainMemory(1)])] }) },
                        2 => *inner_tail = Arc::new(vec![raw.clone()]),
                        3 => *decline = ResumeDecline::RunTail { tail: Arc::new(vec![raw.clone()]), aborts_clause: false },
                        _ => {},
                    }
                }
                // The old variant 1 combined an EffectChoice frame with a field
                // action. Use its real choice ID to execute the branch legally.
                if variant == 1 {
                    let p = g.pending_selection.as_mut().unwrap();
                    p.kind = SelectionKind::EffectChoice;
                    p.valid_action_ids = vec![HAND_EFFECT_START];
                }
                // Variant 0's post action trashes the picked permanent's bottom
                // face-down card. Its installer (install_trash_bottom_face_down_
                // source_under_tamer) only offers permanents with a payable
                // face-down card (face_down_cards_under > 0), and the post arm
                // debug_asserts that invariant (1b1871be2). Give the picked
                // permanent a real face-down card so the state is one the
                // engine can reach; the cost is then paid and the cost-gated
                // tail runs.
                if variant == 0 {
                    let owner = viewer as usize;
                    let mut under = g.players[owner].hand.pop().unwrap();
                    under.face_down = true;
                    g.players[owner].battle_area[0].card_sources.insert(0, under);
                }
                if variant == 4 {
                    add_outer(&mut g, viewer, Some(security_context(event, 3)), vec![raw]);
                    if let ResumeFrame::RunTail { outer_conts, .. } = frame(&mut g) { outer_conts[0].runtime = runtime; }
                }
                let action = if variant == 1 { HAND_EFFECT_START } else if variant == 3 { PASS } else { ATTACK_START };
                let original = g.belief_knowledge(viewer).unwrap().clone();
                let mut ordinary = materialize_closed(&g, viewer, &original, &prior(), 81).unwrap();
                assert!(digimon_engine::build_action_mask(&ordinary, viewer)[action as usize] > 0.5);
                // Expected zones after resolution: unchanged, except that
                // variant 0's paid cost moves the face-down card to its trash.
                let after_zones = |w: &Game| {
                    let mut zones = support::full_zone_dump(w);
                    if variant == 0 {
                        let card = &w.players[viewer as usize].battle_area[0].card_sources[0];
                        assert!(card.face_down);
                        zones[viewer as usize * 5 + 4].push(card.card_id(&w.card_data).to_owned());
                    }
                    zones
                };
                let ordinary_zones = after_zones(&ordinary);
                ordinary.resolve_selection(viewer, action).unwrap();
                assert_eq!(calls.load(Ordering::SeqCst), usize::from(variant >= 2));
                calls.store(0, Ordering::SeqCst);
                let mut memory = original.clone(); memory.forget_zone(1 - viewer, KnowledgeZone::Hand);
                let mut checked = materialize_closed(&g, viewer, &memory, &prior(), 81).unwrap();
                let checked_zones = after_zones(&checked);
                if variant >= 2 {
                    assert_legal_resume_rejected(&mut checked, viewer, action,
                        "closed-list projected context reached unproven callback: raw_dsl_step");
                    assert_eq!(calls.load(Ordering::SeqCst), 0);
                } else {
                    checked.resolve_selection(viewer, action).unwrap();
                    assert_eq!(checked.memory, ordinary.memory);
                    assert_eq!(checked.pending_selection.is_none(), ordinary.pending_selection.is_none());
                    assert_eq!(support::full_zone_dump(&ordinary), ordinary_zones);
                    assert_eq!(support::full_zone_dump(&checked), checked_zones);
                    // Variant 0: cost paid, so the tail's GainMemory(1) runs.
                    assert_eq!(checked.memory, 1);
                }
                assert_eq!(g.memory, 0);
                assert_eq!(g.belief_knowledge(viewer).unwrap(), &original);
            }
        }
    });
}

#[test]
fn aborting_decline_retains_its_control_flow_after_projection() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (mut g, event) = field_fixture(viewer);
            if let ResumeFrame::RunTail { decline, .. } = frame(&mut g) {
                *decline = ResumeDecline::RunTail {
                    tail: Arc::new(vec![CompiledStep::GainMemory(3)]),
                    aborts_clause: true,
                };
            }
            add_outer(
                &mut g,
                viewer,
                Some(security_context(event, 3)),
                vec![CompiledStep::GainMemory(4)],
            );
            let original = g.belief_knowledge(viewer).unwrap().clone();
            let mut forgotten = original.clone();
            forgotten.forget_zone(1 - viewer, KnowledgeZone::Hand);
            let mut a = materialize_closed(&g, viewer, &original, &prior(), 81).unwrap();
            let mut b = materialize_closed(&g, viewer, &forgotten, &prior(), 81).unwrap();
            a.resolve_selection(viewer, PASS).unwrap();
            b.resolve_selection(viewer, PASS).unwrap();
            assert_eq!(a.memory, b.memory);
            assert_eq!(b.memory, 0);
            assert_eq!(a.pending_selection.is_none(), b.pending_selection.is_none());
        }
    });
}

#[test]
fn global_deferred_tail_cannot_borrow_a_literal_frame_certificate() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (mut g, _) = field_fixture(viewer);
            g.dsl_outer_tail = Some((
                vec![CompiledStep::GainMemoryFn {
                    formula: CompiledFormula::EventTargetLevel,
                }],
                Bindings::new(),
                StepRuntime::default(),
            ));
            let mut memory = g.belief_knowledge(viewer).unwrap().clone();
            memory.forget_zone(1 - viewer, KnowledgeZone::Hand);
            let mut checked = materialize_closed(&g, viewer, &memory, &prior(), 81).unwrap();
            assert_legal_resume_rejected(&mut checked, viewer, ATTACK_START,
                "closed-list unavailable event-card reference read");
            assert_eq!(checked.memory, 1, "inner body ran before deferred read");
            assert_eq!(g.memory, 0);
        }
    });
}

#[test]
fn opaque_until_condition_callback_cannot_borrow_a_literal_frame_certificate() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (mut g, _) = field_fixture(viewer);
            let calls = Arc::new(AtomicUsize::new(0)); let c = calls.clone();
            g.modifiers.add_player_modifier(
                viewer,
                digimon_engine::modifiers::PlayerModifierEntry::simple(
                    digimon_engine::enums::ModifierType::CannotAttack,
                    0,
                    digimon_engine::enums::Expiry::UntilCondition,
                    None,
                    viewer,
                )
                .with_until_condition(Arc::new(move |g, _| {
                    c.fetch_add(1, Ordering::SeqCst);
                    g.current_trigger_context
                        .as_ref()
                        .is_some_and(|c| c.event_card.read().is_some())
                })),
            );
            let mut memory = g.belief_knowledge(viewer).unwrap().clone();
            memory.forget_zone(1 - viewer, KnowledgeZone::Hand);
            let mut checked = materialize_closed(&g, viewer, &memory, &prior(), 81).unwrap();
            assert_legal_resume_rejected(&mut checked, viewer, ATTACK_START,
                "closed-list projected context reached unproven callback:");
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(g.memory, 0);
        }
    });
}

#[test]
fn literal_dp_tail_projects_only_unused_context_and_preserves_selection() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledModifierValue};
    support::with_big_stack(|| {
        for viewer in 0..2 {
            for value in [-15000, 3000] {
                let (mut g, event) = field_fixture(viewer);
                if let ResumeFrame::RunTail { bind_as, inner_tail, .. } = frame(&mut g) {
                    *bind_as = Some("dp_target".into());
                    *inner_tail = Arc::new(vec![CompiledStep::AddDpModifier {
                        target: CompiledBindingRef::Named("dp_target".into()),
                        value: CompiledModifierValue::Literal(value),
                        expiry: "end_of_opponents_turn".into(),
                    }]);
                }
                let original = g.belief_knowledge(viewer).unwrap().clone();
                let mut forgotten = original.clone();
                forgotten.forget_zone(1 - viewer, KnowledgeZone::Hand);
                for action in [ATTACK_START, PASS] {
                    let mut a = materialize_closed(&g, viewer, &original, &prior(), 91).unwrap();
                    let mut b = materialize_closed(&g, viewer, &forgotten, &prior(), 91).unwrap();
                    assert_eq!(digimon_engine::build_action_mask(&a, viewer),
                               digimon_engine::build_action_mask(&b, viewer));
                    a.resolve_selection(viewer, action).unwrap();
                    b.resolve_selection(viewer, action).unwrap();
                    assert_eq!(a.memory, b.memory);
                    let target = digimon_engine::permanent::PermanentHandle { player: viewer, index: 0 };
                    let dp = |world: &Game| world.modifiers.get(target,
                        digimon_engine::enums::ModifierType::ChangeDp)
                        .iter().map(|entry| entry.value).sum::<i32>();
                    assert_eq!(dp(&a), dp(&b));
                    if action == ATTACK_START && value > 0 {
                        assert_eq!(dp(&b), value, "the selected public target receives the modifier");
                    }
                    if action == ATTACK_START && value < 0 {
                        assert!(b.players[viewer as usize].battle_area.is_empty(),
                                "lethal reduction still executes the state-based deletion");
                    }
                    assert_eq!(a.players[viewer as usize].battle_area.len(),
                               b.players[viewer as usize].battle_area.len());
                    assert_eq!(a.pending_selection.is_some(), b.pending_selection.is_some());
                    assert_eq!(serde_json::to_value(a.drain_events()).unwrap(),
                               serde_json::to_value(b.drain_events()).unwrap());
                }
                assert_eq!(g.belief_knowledge(viewer).unwrap(), &original);
                if let ResumeFrame::RunTail { trigger_context, .. } = frame(&mut g) {
                    assert_eq!(trigger_context.as_ref().unwrap().event_card.read(), Some(event));
                }
            }
        }
    });
}

#[test]
fn dp_tail_cannot_project_event_target_or_formula_or_opaque_reader() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledModifierValue};
    support::with_big_stack(|| {
        for variant in 0..3 {
            let (mut g, _) = field_fixture(0);
            if let ResumeFrame::RunTail { bind_as, inner_tail, .. } = frame(&mut g) {
                *bind_as = Some("dp_target".into());
                *inner_tail = Arc::new(vec![CompiledStep::AddDpModifier {
                    target: if variant == 0 { CompiledBindingRef::EventCard }
                            else { CompiledBindingRef::Named("dp_target".into()) },
                    value: if variant == 1 { CompiledModifierValue::Formula(CompiledFormula::EventTargetLevel) }
                           else { CompiledModifierValue::Literal(-15000) },
                    expiry: "end_of_opponents_turn".into(),
                }]);
            }
            if variant == 2 {
                g.dsl_outer_tail = Some((vec![CompiledStep::GainMemoryFn {
                    formula: CompiledFormula::EventTargetLevel,
                }], Bindings::new(), StepRuntime::default()));
            }
            let mut memory = g.belief_knowledge(0).unwrap().clone();
            memory.forget_zone(1, KnowledgeZone::Hand);
            let mut checked = materialize_closed(&g, 0, &memory, &prior(), 81).unwrap();
            assert_legal_resume_rejected(&mut checked, 0, ATTACK_START,
                "closed-list unavailable event-card reference read");
            assert_eq!(g.memory, 0);
        }
    });
}
