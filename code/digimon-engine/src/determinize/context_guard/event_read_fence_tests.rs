use crate::{
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{bindings::Bindings, raw_rust::EngineRawRustRegistry, step::run_steps},
    effect::{CardEffect, Effect},
    effect_context::{EffectContext, EffectReadContext},
    enums::{Expiry, GamePhase, ModifierType},
    modifiers::ModifierEntry,
    permanent::PermanentHandle,
    trigger_context::TriggerContext,
    Game,
};
use digimon_dsl::compiled::CompiledStep;
use std::{collections::HashSet, panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn fixture(seat: u8) -> (Game, CardHandle) {
    let mut r = DebugRunner::builder().add_card(make_test_card("FENCE", "Fence"))
        .deck(0, &["FENCE"; 20]).deck(1, &["FENCE"; 20])
        .hand(seat, &["FENCE"]).memory(3).start();
    r.skip_mulligan(); r.set_first_player(seat); r.set_phase(GamePhase::Main);
    let handle = r.game.players[seat as usize].hand[0].handle();
    (r.game, handle)
}

fn compiled(effects: &str) -> Arc<digimon_dsl::compiled::CompiledCard> {
    let yaml = format!("card: FENCE\nname: Fence\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 1000\neffects:\n{effects}");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).unwrap();
    Arc::new(digimon_dsl::compile::compile(&spec).unwrap())
}

fn unavailable(game: &mut Game) {
    let mut context = TriggerContext::default();
    context.event_card = Some(CardHandle(65000)).into();
    context.subject = Some(crate::trigger_context::EventSubject::Card {
        card: CardHandle(65000), zone: crate::enums::Zone::Security,
    }).into();
    context.moved_card_sets.push(crate::trigger_context::MovedCardSet {
        cards: vec![CardHandle(65000)], from: Some(crate::enums::Zone::Security), to: None,
    });
    assert!(context.event_card.project_if_unentitled(&HashSet::new()));
    assert!(context.subject.project_if_unentitled(&HashSet::new()));
    assert!(context.moved_card_sets.project_if_unentitled(&HashSet::new()));
    game.current_trigger_context = Some(context);
    game.require_event_card_read_fence_for_test();
}

#[test]
fn compiled_hand_predicates_park_and_resume_but_opaque_hand_filters_do_not_execute() {
    for seat in [0, 1] { for cloned in [false, true] { for decline in [false, true] {
        let (mut game, card) = fixture(seat);
        game.effect_registry.insert_compiled("FENCE", compiled(
            "  - when: on_play\n    process:\n      - select_hand:\n          of: you\n          optional: true\n          filter: {}\n          bind_as: picked\n          prompt: Pick a card\n      - gain_memory: 2\n"), Arc::new(EngineRawRustRegistry::new()));
        let effects = game.effects_for_card("FENCE", card).unwrap();
        unavailable(&mut game);
        let calls = Arc::new(AtomicUsize::new(0)); let c = calls.clone();
        assert!(catch_unwind(AssertUnwindSafe(|| EffectContext::new(&mut game, card, None, seat)
            .select_hand(seat, "Opaque filter", false,
                move |_, _| { c.fetch_add(1, Ordering::SeqCst); true }, |_, _| {}))).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(game.pending_selection.is_none());
        effects[0].process.as_ref().unwrap()(&mut EffectContext::new(&mut game, card, None, seat));
        assert!(game.pending_selection_resume.is_some()); assert_eq!(game.memory, 3);
        let mut game = if cloned { game.clone() } else { game };
        let action = if decline { crate::action::PASS } else { game.pending_selection.as_ref().unwrap().valid_action_ids[0] };
        assert!(crate::build_action_mask(&game, seat)[action as usize] > 0.5);
        game.resolve_selection(seat, action).unwrap();
        assert_eq!(game.memory, 5); assert!(game.pending_selection.is_none());
        let context = game.current_trigger_context.as_ref().unwrap();
        assert!(context.event_card.is_unavailable()); assert!(context.subject.is_unavailable());
        assert!(context.moved_card_sets.is_unavailable());
    } } }
}

#[test]
fn fixed_compiler_outputs_execute_independent_bodies_through_all_game_lookup_routes() {
    for seat in [0, 1] { for cloned in [false, true] { for warm in [false, true] {
        let (mut game, card) = fixture(seat);
        game.effect_registry.insert_compiled("FENCE",
            compiled("  - when: on_play\n    process:\n      - gain_memory: 2\n"), Arc::new(EngineRawRustRegistry::new()));
        let retained = game.effect_registry.get("FENCE").unwrap().effects(card);
        if warm { game.effects_for_card("FENCE", card).unwrap(); game.registry_effects("FENCE", card).unwrap(); }
        unavailable(&mut game);
        let mut game = if cloned { game.clone() } else { game };
        assert!(!game.projected_world_reads_guarded());
        let full = game.effects_for_card("FENCE", card).unwrap();
        let registry = game.registry_effects("FENCE", card).unwrap();
        let owned = game.effects_for_card_owned("FENCE", card).unwrap();
        for effects in [&retained[..], &full[..], &registry[..], &owned[..]] {
            game.memory = 3;
            let before = game.memory;
            effects[0].process.as_ref().unwrap()(&mut EffectContext::new(&mut game, card, None, seat));
            assert_eq!(game.memory, before + 2);
            assert!(game.current_trigger_context.as_ref().unwrap().event_card.is_unavailable());
        }
    } } }
}

#[test]
fn compiled_event_reader_rejects_the_actual_read_before_body_mutation() {
    for seat in [0, 1] { for cloned in [false, true] {
        let (mut game, card) = fixture(seat);
        game.effect_registry.insert_compiled("FENCE", compiled(
            "  - when: on_play\n    condition:\n      event_card_name_contains: Fence\n    process:\n      - gain_memory: 2\n"), Arc::new(EngineRawRustRegistry::new()));
        let retained = game.effects_for_card("FENCE", card).unwrap();
        let mut context = TriggerContext::default(); context.event_card = Some(card).into();
        game.current_trigger_context = Some(context);
        assert!(retained[0].condition.as_ref().unwrap()(&EffectReadContext::new(&game, card, None, seat)));
        unavailable(&mut game);
        let game = if cloned { game.clone() } else { game };
        let error = catch_unwind(AssertUnwindSafe(|| retained[0].condition.as_ref().unwrap()(
            &EffectReadContext::new(&game, card, None, seat)))).unwrap_err();
        let text = error.downcast_ref::<String>().map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied()).unwrap();
        assert_eq!(text, "closed-list unavailable event-card reference read");
        assert_eq!(game.memory, 3);
    } }
}

struct Opaque { factory_calls: Arc<AtomicUsize>, body_calls: Arc<AtomicUsize>, captured: TriggerContext }
impl CardEffect for Opaque {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        self.factory_calls.fetch_add(1, Ordering::SeqCst);
        let captured = self.captured.clone(); let calls = self.body_calls.clone();
        vec![Effect::on_play(card).process(move |ctx| {
            calls.fetch_add(1, Ordering::SeqCst);
            ctx.game.memory += i16::from(captured.event_card.read().is_some());
        }).build()]
    }
}

#[test]
fn opaque_factory_cache_replacement_and_retained_captures_remain_closed() {
    for seat in [0, 1] { for cloned in [false, true] {
        let (mut game, card) = fixture(seat);
        let factory_calls = Arc::new(AtomicUsize::new(0)); let body_calls = Arc::new(AtomicUsize::new(0));
        let mut captured = TriggerContext::default(); captured.event_card = Some(CardHandle(65000)).into();
        game.effect_registry.insert("FENCE", Arc::new(Opaque { factory_calls: factory_calls.clone(), body_calls: body_calls.clone(), captured }));
        let retained = game.effects_for_card("FENCE", card).unwrap();
        let retained_registry = game.registry_effects("FENCE", card).unwrap();
        let before = factory_calls.load(Ordering::SeqCst);
        game.require_event_card_read_fence_for_test(); // no marked current context
        let mut game = if cloned { game.clone() } else { game };
        assert!(catch_unwind(AssertUnwindSafe(|| game.effects_for_card("FENCE", card))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| game.registry_effects("FENCE", card))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| game.effects_for_card_owned("FENCE", card))).is_err());
        assert_eq!(factory_calls.load(Ordering::SeqCst), before);
        // A current compiled entry cannot relabel previously returned callbacks.
        game.effect_registry.insert_compiled("FENCE", compiled("  - when: on_play\n    process:\n      - gain_memory: 2\n"), Arc::new(EngineRawRustRegistry::new()));
        for effects in [&retained, &retained_registry] {
            assert!(catch_unwind(AssertUnwindSafe(|| effects[0].process.as_ref().unwrap()(
                &mut EffectContext::new(&mut game, card, None, seat)))).is_err());
        }
        assert_eq!(body_calls.load(Ordering::SeqCst), 0); assert_eq!(game.memory, 3);
    } }
}

#[test]
fn raw_declarative_construction_and_retained_raw_steps_remain_closed() {
    for seat in [0, 1] {
        let (mut game, card) = fixture(seat); let calls = Arc::new(AtomicUsize::new(0));
        let mut raw = EngineRawRustRegistry::new();
        let c = calls.clone(); raw.register_declarative("opaque", move |card| { c.fetch_add(1, Ordering::SeqCst); vec![Effect::on_play(card).build()] });
        let c = calls.clone(); raw.register_step("opaque", move |ctx, _| { c.fetch_add(1, Ordering::SeqCst); ctx.game.memory += 1; });
        let step = raw.step_fn("opaque").unwrap();
        game.effect_registry.insert_compiled("FENCE", compiled("  - kind: raw_rust\n    fn: opaque\n"), Arc::new(raw));
        unavailable(&mut game);
        assert!(catch_unwind(AssertUnwindSafe(|| game.effects_for_card("FENCE", card))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| step(&mut EffectContext::new(&mut game, card, None, seat), &mut Bindings::new()))).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0); assert_eq!(game.memory, 3);
    }
}

#[test]
fn opaque_until_condition_aborts_transitive_native_work() {
    for seat in [0, 1] {
        let (mut game, card) = fixture(seat); let calls = Arc::new(AtomicUsize::new(0)); let c = calls.clone();
        game.modifiers.add(PermanentHandle { player: seat, index: 0 },
            ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::UntilCondition, seat)
                .with_until_condition(Arc::new(move |_, _| { c.fetch_add(1, Ordering::SeqCst); true })));
        unavailable(&mut game);
        assert!(catch_unwind(AssertUnwindSafe(|| run_steps(&[CompiledStep::GainMemory(1)],
            &mut EffectContext::new(&mut game, card, None, seat), &mut Bindings::new()))).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        // GainMemory precedes expiry reevaluation: rejection is not rollback.
        assert_eq!(game.memory, 4);
    }
}

#[test]
fn structured_choice_preserves_unavailable_reference_through_park_clone_and_legal_resume() {
    for seat in [0, 1] { for cloned in [false, true] {
        let (mut game, card) = fixture(seat); unavailable(&mut game);
        let steps = vec![CompiledStep::SelectEffectChoice { labels: vec!["Continue".into(), "Also continue".into()], legal_when: None,
            bind_as: None, prompt: "Continue".into(), prompt_key: None }, CompiledStep::GainMemory(2)];
        run_steps(&steps, &mut EffectContext::new(&mut game, card, None, seat), &mut Bindings::new());
        assert!(game.pending_selection_resume.is_some()); assert_eq!(game.memory, 3);
        let mut game = if cloned { game.clone() } else { game };
        let action = game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        game.resolve_selection(seat, action).unwrap();
        assert!(game.pending_selection.is_none()); assert!(game.pending_selection_resume.is_none());
        assert_eq!(game.memory, 5);
        assert!(game.current_trigger_context.as_ref().unwrap().event_card.is_unavailable());
        assert!(game.projected_event_card_read_fence());
    } }
}

#[test]
fn compiled_field_predicates_preserve_legal_choices_and_opaque_filters_stay_closed() {
    for seat in [0, 1] { for own in [false, true] { for cloned in [false, true] { for decline in [false, true] {
        let (mut game, card) = fixture(seat);
        for pid in [0usize, 1] { for _ in 0..2 {
            let source = game.players[pid].deck.pop().unwrap();
            game.players[pid].battle_area.push(crate::permanent::Permanent::new(source, 0));
        } }
        unavailable(&mut game);
        let calls = Arc::new(AtomicUsize::new(0)); let c = calls.clone();
        assert!(catch_unwind(AssertUnwindSafe(|| {
            let mut ctx = EffectContext::new(&mut game, card, None, seat);
            if own { ctx.select_own_permanent("Opaque", true,
                move |_, _| { c.fetch_add(1, Ordering::SeqCst); true }, |_, _| {}); }
            else { ctx.select_opponent_permanent("Opaque", true,
                move |_, _| { c.fetch_add(1, Ordering::SeqCst); true }, |_, _| {}); }
        })).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0); assert!(game.pending_selection.is_none());
        let selection = if own { CompiledStep::SelectOwnPermanent {
            filter: Default::default(), bind_as: Some("picked".into()), selector: None,
            prompt: "Pick".into(), prompt_key: None, optional: true, continue_on_decline: true, then: vec![],
        } } else { CompiledStep::SelectOpponentPermanent {
            filter: Default::default(), bind_as: Some("picked".into()), selector: None,
            prompt: "Pick".into(), prompt_key: None, optional: true, continue_on_decline: true, then: vec![],
        } };
        run_steps(&[selection, CompiledStep::GainMemory(2)],
            &mut EffectContext::new(&mut game, card, None, seat), &mut Bindings::new());
        let pending = game.pending_selection.as_ref().unwrap();
        assert_eq!(pending.zone_owner, Some(if own {seat} else {1-seat}));
        assert_eq!(pending.valid_action_ids.len(), 2); assert!(game.pending_selection_resume.is_some());
        let action = if decline {crate::action::PASS} else {pending.valid_action_ids[1]};
        let mut game = if cloned {game.clone()} else {game};
        assert!(crate::build_action_mask(&game, seat)[action as usize] > 0.5);
        game.resolve_selection(seat, action).unwrap();
        assert_eq!(game.memory, 5); assert!(game.pending_selection.is_none());
        assert!(game.current_trigger_context.as_ref().unwrap().event_card.is_unavailable());
    } } } }
}

#[test]
fn compiled_trash_and_material_predicates_preserve_owned_choices_and_strict_raw_filters() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPlayerRef};
    for seat in [0, 1] { for zone in [0, 1, 2] { for cloned in [false, true] {
        for decline in [false, true] { for fenced in [false, true] {
        let (mut game, card) = fixture(seat);
        let pid = seat as usize;
        let host = PermanentHandle { player: seat,
            index: if zone == 2 { crate::action::space::BREEDING_TARGET as u8 } else { 0 } };
        let mut bindings = Bindings::new();
        if zone == 0 {
            for _ in 0..2 { let c = game.players[pid].deck.pop().unwrap(); game.players[pid].put_in_trash(c); }
        } else {
            let top = game.players[pid].deck.pop().unwrap();
            let mut permanent = crate::permanent::Permanent::new(top, 0);
            for _ in 0..2 { permanent.card_sources.insert(0, game.players[pid].deck.pop().unwrap()); }
            if zone == 2 { game.players[pid].breeding_area = Some(permanent); }
            else { game.players[pid].battle_area.push(permanent); }
            bindings.insert_permanent("host", host);
        }
        if fenced {
            unavailable(&mut game);
            let calls = Arc::new(AtomicUsize::new(0)); let c = calls.clone();
            assert!(catch_unwind(AssertUnwindSafe(|| {
                let mut ctx = EffectContext::new(&mut game, card, None, seat);
                if zone == 0 { ctx.select_trash(seat, "Opaque", true,
                    move |_, _| { c.fetch_add(1, Ordering::SeqCst); true }, |_, _| {}); }
                else { ctx.select_material(host, "Opaque", true,
                    move |_, _| { c.fetch_add(1, Ordering::SeqCst); true }, |_, _| {}); }
            })).is_err());
            assert_eq!(calls.load(Ordering::SeqCst), 0); assert!(game.pending_selection.is_none());
        }
        let selection = if zone == 0 { CompiledStep::SelectTrash {
            of: CompiledPlayerRef::You, filter: Default::default(), bind_as: Some("picked".into()),
            prompt: "Pick".into(), prompt_key: None, optional: true, cost: false, then: vec![],
        } } else { CompiledStep::SelectMaterial {
            of_permanent: CompiledBindingRef::Named("host".into()), filter: Default::default(),
            bind_as: Some("picked".into()), prompt: "Pick".into(), prompt_key: None, optional: true,
        } };
        run_steps(&[selection, CompiledStep::GainMemory(2)],
            &mut EffectContext::new(&mut game, card, None, seat), &mut bindings);
        let pending = game.pending_selection.as_ref().unwrap();
        assert_eq!(pending.valid_action_ids.iter().filter(|&&a| a != crate::action::PASS).count(), 2);
        assert_eq!(pending.card_choice.is_some(), zone != 0);
        assert!(game.pending_selection_resume.is_some()); assert_eq!(game.memory, 3);
        let action = if decline { crate::action::PASS } else { *pending.valid_action_ids.iter().filter(|&&a| a != crate::action::PASS).nth(1).unwrap() };
        let mut game = if cloned { game.clone() } else { game };
        assert!(crate::build_action_mask(&game, seat)[action as usize] > 0.5);
        game.resolve_selection(seat, action).unwrap();
        assert_eq!(game.memory, if decline && zone != 0 { 3 } else { 5 });
        assert!(game.pending_selection.is_none());
        assert_eq!(game.projected_event_card_read_fence(), fenced);
        if fenced { assert!(game.current_trigger_context.as_ref().unwrap().event_card.is_unavailable()); }
        } }
    } } }
}


fn revealed_fence_fixture(seat: u8) -> (Game, CardHandle) {
    let (mut game, source) = fixture(seat);
    let own = game.players[seat as usize].deck.pop().unwrap();
    let opponent = game.players[(1 - seat) as usize].deck.pop().unwrap();
    let mut overlaid = game.players[seat as usize].deck.pop().unwrap();
    overlaid.reveal_overlay = Some(crate::card_source::RevealOverlay {
        name: Some("Other revealed identity".into()), kind: None,
    });
    game.revealed_cards = vec![own, opponent, overlaid];
    (game, source)
}

#[test]
fn compiled_reveal_owner_and_overlay_filters_preserve_cloned_accept_and_decline() {
    use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate};
    use crate::action::space::SEL_REVEAL_START;
    for seat in [0, 1] { for fenced in [false, true] { for cloned in [false, true] {
        for decline in [false, true] { for owner in 0..4 {
        let (mut game, source) = revealed_fence_fixture(seat);
        if fenced { unavailable(&mut game); }
        let of = match owner { 0 => CompiledPlayerRef::You, 1 => CompiledPlayerRef::Opponent,
            2 => CompiledPlayerRef::Active, _ => CompiledPlayerRef::Any };
        let expected = if owner == 1 { vec![SEL_REVEAL_START + 1] }
            else if owner == 3 { vec![SEL_REVEAL_START, SEL_REVEAL_START + 1, SEL_REVEAL_START + 2] }
            else { vec![SEL_REVEAL_START, SEL_REVEAL_START + 2] };
        let selection = CompiledStep::SelectReveal {
            of, filter: CompiledPredicate { name_contains: Some("Fence".into()), ..Default::default() },
            bind_as: Some("picked".into()), prompt: "Compiled reveal".into(), prompt_key: None,
            optional: true, then: vec![CompiledStep::GainMemory(1)],
        };
        let before: Vec<_> = game.revealed_cards.iter().map(|c| (c.handle(), c.owner)).collect();
        run_steps(&[selection, CompiledStep::GainMemory(2)],
            &mut EffectContext::new(&mut game, source, None, seat), &mut Bindings::new());
        let pending = game.pending_selection.as_ref().unwrap();
        assert_eq!(pending.valid_action_ids, expected, "seat={seat} owner={owner} fenced={fenced}");
        assert_eq!(game.memory, 3); assert!(game.pending_selection_resume.is_some());
        let action = if decline { crate::action::PASS } else { *expected.last().unwrap() };
        let mut game = if cloned { game.clone() } else { game };
        assert!(crate::build_action_mask(&game, seat)[action as usize] > 0.5);
        game.resolve_selection(seat, action).unwrap();
        assert_eq!(game.memory, if decline { 5 } else { 6 });
        assert!(game.pending_selection.is_none());
        assert_eq!(game.revealed_cards.iter().map(|c| (c.handle(), c.owner)).collect::<Vec<_>>(), before);
        assert_eq!(game.projected_event_card_read_fence(), fenced);
        if fenced { assert!(game.current_trigger_context.as_ref().unwrap().event_card.is_unavailable()); }
        } }
    } } }
}

#[test]
fn compiled_reveal_event_reads_reject_at_the_checked_reference_before_prompt_installation() {
    use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate};
    for seat in [0, 1] { for cloned in [false, true] {
        let (mut game, source) = revealed_fence_fixture(seat); unavailable(&mut game);
        let mut game = if cloned { game.clone() } else { game };
        let before_phase = game.current_phase;
        let before: Vec<_> = game.revealed_cards.iter().map(|c| c.handle()).collect();
        let selection = CompiledStep::SelectReveal {
            of: CompiledPlayerRef::You,
            filter: CompiledPredicate { event_card_name_contains: Some("Fence".into()), ..Default::default() },
            bind_as: Some("picked".into()), prompt: "Read unavailable event".into(), prompt_key: None,
            optional: true, then: vec![CompiledStep::GainMemory(1)],
        };
        let result = catch_unwind(AssertUnwindSafe(|| run_steps(&[selection],
            &mut EffectContext::new(&mut game, source, None, seat), &mut Bindings::new())));
        let error = result.expect_err("the actual event-reference read must reject");
        let message = error.downcast_ref::<String>().map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied()).unwrap_or("");
        assert!(message.contains("closed-list unavailable event-card reference read"), "{message}");
        assert_eq!(game.memory, 3); assert_eq!(game.current_phase, before_phase);
        assert!(game.pending_selection.is_none()); assert!(game.pending_selection_resume.is_none());
        assert_eq!(game.revealed_cards.iter().map(|c| c.handle()).collect::<Vec<_>>(), before);
    } }
}

#[test]
fn compiled_reveal_opaque_api_keeps_prebody_rejection_and_ordinary_filter_behavior() {
    for seat in [0, 1] { for cloned in [false, true] { for fenced in [false, true] {
        for allowed in [false, true] {
        let (mut game, source) = revealed_fence_fixture(seat);
        if fenced { unavailable(&mut game); }
        let mut game = if cloned { game.clone() } else { game };
        let phase = game.current_phase; let calls = Arc::new(AtomicUsize::new(0)); let counter = calls.clone();
        let result = catch_unwind(AssertUnwindSafe(|| EffectContext::new(&mut game, source, None, seat)
            .select_reveal("Opaque reveal", false,
                move |_, _| { counter.fetch_add(1, Ordering::SeqCst); allowed }, |_, _| {})));
        if fenced {
            assert!(result.is_err()); assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(game.pending_selection.is_none()); assert_eq!(game.current_phase, phase);
        } else {
            assert_eq!(result.unwrap(), allowed); assert_eq!(calls.load(Ordering::SeqCst), 3);
            assert_eq!(game.pending_selection.is_some(), allowed);
        }
        assert_eq!(game.memory, 3);
        }
    } } }
}


#[test]
fn compiled_reveal_overlay_names_are_visible_only_to_revealed_subjects() {
    use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate};
    use crate::action::space::SEL_REVEAL_START;
    use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
    for seat in [0, 1] { for fenced in [false, true] { for cloned in [false, true] {
        let (mut game, source) = revealed_fence_fixture(seat);
        if fenced { unavailable(&mut game); }
        let predicate = CompiledPredicate { name_is: Some("Other revealed identity".into()), ..Default::default() };
        let overlaid = game.revealed_cards[2].handle();
        let read = EffectReadContext::new(&game, source, None, seat);
        assert!(eval_predicate(&predicate, &read, PredicateSubject::RevealedCard(overlaid)));
        assert!(!eval_predicate(&predicate, &read, PredicateSubject::Card(overlaid)));
        let selection = CompiledStep::SelectReveal {
            of: CompiledPlayerRef::You, filter: predicate, bind_as: Some("picked".into()),
            prompt: "Overlay identity".into(), prompt_key: None, optional: false,
            then: vec![CompiledStep::GainMemory(1)],
        };
        run_steps(&[selection], &mut EffectContext::new(&mut game, source, None, seat), &mut Bindings::new());
        assert_eq!(game.pending_selection.as_ref().unwrap().valid_action_ids, vec![SEL_REVEAL_START + 2]);
        let mut game = if cloned { game.clone() } else { game };
        assert!(crate::build_action_mask(&game, seat)[(SEL_REVEAL_START + 2) as usize] > 0.5);
        game.resolve_selection(seat, SEL_REVEAL_START + 2).unwrap();
        assert_eq!(game.memory, 4); assert!(game.pending_selection.is_none());
        assert_eq!(game.projected_event_card_read_fence(), fenced);
    } } }
}

fn source_fence_fixture(seat: u8) -> (Game, CardHandle) {
    let (mut game, source) = fixture(seat);
    for owner in [0usize, 1] { for _ in 0..2 {
        let top = game.players[owner].deck.pop().unwrap();
        let mut host = crate::permanent::Permanent::new(top, 0);
        for _ in 0..2 { host.card_sources.insert(0, game.players[owner].deck.pop().unwrap()); }
        host.card_sources[0].face_down = true;
        game.players[owner].battle_area.push(host);
    } }
    (game, source)
}

fn compiled_source_step(own: bool, filter: digimon_dsl::compiled::CompiledPredicate,
    target: Option<digimon_dsl::compiled::CompiledBindingRef>, min: u8,
    then: Vec<CompiledStep>) -> CompiledStep {
    if own { CompiledStep::SelectOwnSources { target, filter, min, max: 2,
        bind_as: Some("picked_sources".into()), prompt: "Pick sources".into(), then } }
    else { CompiledStep::SelectOpponentSources { target, filter,
        min: digimon_dsl::compiled::CompiledCountBound::Literal(min),
        max: digimon_dsl::compiled::CompiledCountBound::Literal(2), clamp_to_available: false,
        bind_as: Some("picked_sources".into()), prompt: "Pick sources".into(), then } }
}

#[test]
fn compiled_source_filters_preserve_physical_picks_optional_pass_and_cloned_resume() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate};
    for seat in [0, 1] { for own in [false, true] { for fenced in [false, true] {
      for cloned in [false, true] { for targeted in [false, true] { for finish in 0..3 {
        let (mut game, source) = source_fence_fixture(seat);
        let owner = if own { seat } else { 1-seat };
        let before_hand = game.players[owner as usize].hand.len();
        let before_stacks: Vec<_> = game.players[owner as usize].battle_area.iter()
            .map(|p| p.card_sources.iter().map(|c| c.handle()).collect::<Vec<_>>()).collect();
        let mut bindings = Bindings::new();
        bindings.insert_permanent("host", PermanentHandle { player: owner, index: 1 });
        if fenced { unavailable(&mut game); }
        let filter = CompiledPredicate { is_bottom_source: Some(true), is_face_down: Some(true),
            ..Default::default() };
        let selection = compiled_source_step(own, filter,
            targeted.then(|| CompiledBindingRef::Named("host".into())), if finish == 0 {0} else {1},
            vec![CompiledStep::ReturnSelectedSourcesToHand { source_refs: "picked_sources".into() },
                CompiledStep::GainMemory(1)]);
        run_steps(&[selection, CompiledStep::GainMemory(2)],
            &mut EffectContext::new(&mut game, source, None, seat), &mut bindings);
        assert_eq!(game.memory, 3);
        let mut picked = Vec::new();
        loop {
            let state = match &game.pending_selection_resume.as_ref().unwrap().frames[0] {
                crate::resume::ResumeFrame::SourceMultiStep(s) => s,
                other => panic!("wrong structured frame: {other:?}"),
            };
            assert!(state.candidates.iter().all(|s| s.permanent.player == owner && s.source_index == 0));
            assert_eq!(state.candidates.len(), (if targeted {1} else {2}) - picked.len());
            if targeted { assert!(state.candidates.iter().all(|s| s.permanent.index == 1)); }
            let passing = finish == 0 || (finish == 1 && !picked.is_empty());
            let action = if passing { crate::action::PASS } else {
                let next = state.candidates[0];
                let pending = game.pending_selection.as_ref().unwrap();
                let position = next.source_index.to_string();
                let row = pending.card_choice.as_ref().unwrap().view(&game, seat).rows.into_iter()
                    .find(|r| r.owner == Some(owner) && r.carrier_slot == Some(next.permanent.index)
                        && r.position.as_deref() == Some(position.as_str())
                        && matches!(r.operation, crate::owned_card_choice::CardChoiceOperation::CommitCard)).unwrap();
                picked.push(next.card); row.action_id
            };
            if cloned { game = game.clone(); }
            assert!(crate::build_action_mask(&game, seat)[action as usize] > 0.5);
            game.resolve_selection(seat, action).unwrap();
            if game.pending_selection.is_none() { break; }
        }
        assert!(game.pending_selection_resume.is_none()); assert_eq!(game.memory, 6);
        assert_eq!(picked.len(), if finish == 0 {0} else if finish == 1 || targeted {1} else {2});
        assert_eq!(game.players[owner as usize].hand.len(), before_hand + picked.len());
        for (i, before) in before_stacks.into_iter().enumerate() {
            let expected: Vec<_> = before.into_iter().filter(|c| !picked.contains(c)).collect();
            assert_eq!(game.players[owner as usize].battle_area[i].card_sources.iter()
                .map(|c| c.handle()).collect::<Vec<_>>(), expected);
        }
        assert_eq!(game.projected_event_card_read_fence(), fenced);
        if fenced { assert!(game.current_trigger_context.as_ref().unwrap().event_card.is_unavailable()); }
      } } }
    } } }
}

#[test]
fn compiled_source_missing_target_and_filtered_empty_preserve_tail_contract() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate};
    for seat in [0, 1] { for own in [false, true] { for fenced in [false, true] {
      for missing in [false, true] { for min in [0, 1] {
        let (mut game, source) = source_fence_fixture(seat);
        if fenced { unavailable(&mut game); }
        let phase = game.current_phase;
        let filter = CompiledPredicate { name_contains: (!missing).then(|| "No such source".into()),
            ..Default::default() };
        let selection = compiled_source_step(own, filter,
            missing.then(|| CompiledBindingRef::Named("absent".into())), min,
            vec![CompiledStep::GainMemory(1)]);
        run_steps(&[selection, CompiledStep::GainMemory(2)],
            &mut EffectContext::new(&mut game, source, None, seat), &mut Bindings::new());
        assert!(game.pending_selection.is_none()); assert!(game.pending_selection_resume.is_none());
        assert_eq!(game.current_phase, phase); assert_eq!(game.memory, if min == 0 {6} else {3});
        assert_eq!(game.projected_event_card_read_fence(), fenced);
      } }
    } } }
}

#[test]
fn compiled_source_event_reads_fail_at_checked_reference_before_installation() {
    use digimon_dsl::compiled::CompiledPredicate;
    for seat in [0, 1] { for own in [false, true] { for cloned in [false, true] {
        let (mut game, source) = source_fence_fixture(seat); unavailable(&mut game);
        let mut game = if cloned { game.clone() } else { game };
        let phase = game.current_phase;
        let selection = compiled_source_step(own, CompiledPredicate {
            event_card_name_contains: Some("Fence".into()), ..Default::default()
        }, None, 1, vec![CompiledStep::GainMemory(1)]);
        let error = catch_unwind(AssertUnwindSafe(|| run_steps(&[selection],
            &mut EffectContext::new(&mut game, source, None, seat), &mut Bindings::new()))).unwrap_err();
        let message = error.downcast_ref::<String>().map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied()).unwrap_or("");
        assert_eq!(message, "closed-list unavailable event-card reference read");
        assert_eq!(game.memory, 3); assert_eq!(game.current_phase, phase);
        assert!(game.pending_selection.is_none()); assert!(game.pending_selection_resume.is_none());
    } } }
}

#[test]
fn compiled_source_opaque_apis_keep_prebody_rejection_and_ordinary_candidates() {
    for seat in [0, 1] { for own in [false, true] { for fenced in [false, true] {
      for allowed in [false, true] {
        let (mut game, source) = source_fence_fixture(seat);
        if fenced { unavailable(&mut game); }
        let calls = Arc::new(AtomicUsize::new(0)); let count = calls.clone();
        let phase = game.current_phase;
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut ctx = EffectContext::new(&mut game, source, None, seat);
            let filter = move |_: &Game, _: crate::selection::SourceSelectionRef| {
                count.fetch_add(1, Ordering::SeqCst); allowed
            };
            if own { ctx.select_own_sources("Opaque", 1, 2, filter, |_, _| {}); }
            else { ctx.select_opponent_sources("Opaque", 1, 2, filter, |_, _| {}); }
        }));
        if fenced {
            assert!(result.is_err()); assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(game.pending_selection.is_none()); assert_eq!(game.current_phase, phase);
        } else {
            result.unwrap(); assert_eq!(calls.load(Ordering::SeqCst), 4);
            assert_eq!(game.pending_selection.is_some(), allowed);
        }
        assert_eq!(game.memory, 3);
      }
    } } }
}


fn assembly_fence_fixture(seat: u8) -> (Game, CardHandle, Vec<CardHandle>) {
    let yaml = "card: ASM-FENCE\nname: Assembly Fence\nkind: digimon\nlevel: 7\ncolor: [red]\ncost: 8\ndp: 15000\nalt_paths:\n  - kind: assembly\n    materials:\n      - { name_contains: Material, zones: [trash], stack_under: true }\n      - { name_contains: Material, zones: [trash], stack_under: true }\n    cost: 3\n";
    let mut r = DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
        .add_card(make_test_card("MAT-FENCE", "Material"))
        .add_card(make_test_card("OTHER-FENCE", "Other"))
        .hand(seat, &["ASM-FENCE"]).memory(10).start();
    r.skip_mulligan(); r.set_first_player(seat); r.set_phase(GamePhase::Main);
    r.game.memory = 10;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).unwrap();
    r.game.effect_registry.insert_compiled("ASM-FENCE",
        Arc::new(digimon_dsl::compile::compile(&spec).unwrap()),
        Arc::new(EngineRawRustRegistry::new()));
    let source = r.game.players[seat as usize].hand[0].handle();
    let mut materials = Vec::new();
    for id in ["MAT-FENCE", "MAT-FENCE", "MAT-FENCE", "OTHER-FENCE"] {
        let data_index = r.game.card_data.iter().position(|d| d.card_id == id).unwrap();
        let index = r.game.next_card_index();
        let card = crate::card_source::CardSource::new(data_index, seat, index);
        materials.push(card.handle());
        r.game.players[seat as usize].put_in_trash(card);
    }
    (r.game, source, materials)
}

#[test]
fn assembly_fence_legal_play_keeps_material_choices_costs_and_cloned_continuations() {
    use crate::action::{build_action_mask, PLAY_HAND_START, TRASH_EFFECT_START, PASS};
    // Ordinary, fenced with absent caller, fenced with unavailable event data.
    for seat in [0, 1] { for fence in 0..3 { for cloned in [false, true] {
      for decline in [false, true] {
        let (mut game, source, materials) = assembly_fence_fixture(seat);
        if fence == 1 { game.require_event_card_read_fence_for_test(); }
        if fence == 2 { unavailable(&mut game); }
        assert_eq!(build_action_mask(&game, seat)[PLAY_HAND_START as usize], 1.0);
        game.decode_action(PLAY_HAND_START, seat);
        assert_eq!(game.memory, 10);
        assert!(game.pending_selection_resume.is_some());
        let mask = build_action_mask(&game, seat);
        for i in 0..3 { assert_eq!(mask[(TRASH_EFFECT_START + i) as usize], 1.0); }
        assert_eq!(mask[(TRASH_EFFECT_START + 3) as usize], 0.0);
        assert_eq!(mask[PASS as usize], 1.0);
        let mut game = if cloned { game.clone() } else { game };
        if decline {
            game.resolve_selection(seat, PASS).unwrap();
        } else {
            game.resolve_selection(seat, TRASH_EFFECT_START).unwrap();
            assert_eq!(game.memory, 10);
            let mask = build_action_mask(&game, seat);
            assert_eq!(mask[TRASH_EFFECT_START as usize], 0.0, "exclude earlier element's card");
            assert_eq!(mask[(TRASH_EFFECT_START + 1) as usize], 1.0);
            assert_eq!(mask[(TRASH_EFFECT_START + 2) as usize], 1.0);
            assert_eq!(mask[(TRASH_EFFECT_START + 3) as usize], 0.0);
            assert_eq!(mask[PASS as usize], 0.0, "later element is required");
            if cloned { game = game.clone(); }
            game.resolve_selection(seat, TRASH_EFFECT_START + 1).unwrap();
        }
        assert!(game.pending_selection.is_none());
        assert!(game.pending_selection_resume.is_none());
        let player = &game.players[seat as usize];
        assert!(player.hand.is_empty());
        let stack: Vec<_> = player.battle_area[0].card_sources.iter().map(|c| c.handle()).collect();
        let trash: Vec<_> = player.trash.iter().map(|c| c.handle()).collect();
        if decline {
            assert_eq!(game.memory, 2); assert_eq!(stack, vec![source]); assert_eq!(trash, materials);
        } else {
            assert_eq!(game.memory, 5); assert_eq!(stack, vec![materials[1], materials[0], source]);
            assert_eq!(trash, materials[2..]);
        }
        assert_eq!(game.projected_event_card_read_fence(), fence != 0);
      }
    } } }
}

#[test]
fn assembly_fence_opaque_count_capped_filters_remain_strict() {
    use crate::effect_context::CountCappedZone;
    for seat in [0, 1] { for cloned in [false, true] { for fenced in [false, true] {
      for allowed in [false, true] {
        let (mut game, source, _) = assembly_fence_fixture(seat);
        if fenced { unavailable(&mut game); }
        let mut game = if cloned { game.clone() } else { game };
        let calls = Arc::new(AtomicUsize::new(0)); let c = calls.clone();
        let completed = Arc::new(AtomicUsize::new(0)); let done = completed.clone();
        let phase = game.current_phase;
        let outcome = catch_unwind(AssertUnwindSafe(|| EffectContext::new(&mut game, source, None, seat)
            .select_count_capped_multi_min(seat, CountCappedZone::Trash, 0, 1, "Opaque", true, None,
                move |_, _| { c.fetch_add(1, Ordering::SeqCst); allowed },
                move |_, _| { done.fetch_add(1, Ordering::SeqCst); })));
        if fenced {
            assert!(outcome.is_err()); assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(completed.load(Ordering::SeqCst), 0);
            assert!(game.pending_selection.is_none()); assert_eq!(game.current_phase, phase);
        } else {
            outcome.unwrap(); assert_eq!(calls.load(Ordering::SeqCst), 4);
            assert_eq!(game.pending_selection.is_some(), allowed);
            assert_eq!(completed.load(Ordering::SeqCst), usize::from(!allowed));
        }
        assert_eq!(game.memory, 10);
      }
    } } }
}


#[test]
fn assembly_fence_compiled_predicates_check_actual_event_reads_and_keep_legacy_guards() {
    use crate::effect_context::CountCappedZone;
    use digimon_dsl::compiled::CompiledPredicate;
    use crate::action::{build_action_mask, TRASH_EFFECT_START};
    for seat in [0, 1] { for cloned in [false, true] { for fenced in [false, true] {
      for event_reader in [false, true] {
        let (mut game, source, materials) = assembly_fence_fixture(seat);
        let mut context = TriggerContext::default(); context.event_card = Some(materials[0]).into();
        game.current_trigger_context = Some(context);
        if fenced { unavailable(&mut game); }
        let mut game = if cloned { game.clone() } else { game };
        let phase = game.current_phase;
        let predicate = if event_reader {
            CompiledPredicate { event_card_name_contains: Some("Material".into()), ..Default::default() }
        } else {
            CompiledPredicate { name_contains: Some("Material".into()), ..Default::default() }
        };
        let outcome = catch_unwind(AssertUnwindSafe(|| EffectContext::new(&mut game, source, None, seat)
            .select_count_capped_multi_min_compiled(seat, CountCappedZone::Trash, 1, 1,
                "Compiled", false, None, predicate, vec![materials[0]],
                |_, _| panic!("installing a nonempty picker must not execute its callback"))));
        if fenced && event_reader {
            let error = outcome.unwrap_err();
            let message = error.downcast_ref::<String>().map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied()).unwrap_or("");
            assert_eq!(message, "closed-list unavailable event-card reference read");
            assert!(game.pending_selection.is_none()); assert_eq!(game.current_phase, phase);
        } else {
            outcome.unwrap();
            let mask = build_action_mask(&game, seat);
            assert_eq!(mask[TRASH_EFFECT_START as usize], 0.0);
            assert_eq!(mask[(TRASH_EFFECT_START + 1) as usize], 1.0);
            assert_eq!(mask[(TRASH_EFFECT_START + 2) as usize], 1.0);
            assert_eq!(mask[(TRASH_EFFECT_START + 3) as usize], if event_reader { 1.0 } else { 0.0 });
        }
        assert_eq!(game.memory, 10); assert_eq!(game.players[seat as usize].trash.len(), 4);
      }
    } } }
    for seat in [0, 1] { for world in [false, true] {
        let (mut game, source, _) = assembly_fence_fixture(seat);
        if world { game.require_projected_world_read_guard_for_test(); }
        else {
            let mut context = TriggerContext::default(); context.projected_reference_reads_guarded = true;
            game.current_trigger_context = Some(context);
        }
        let phase = game.current_phase;
        let result = catch_unwind(AssertUnwindSafe(|| EffectContext::new(&mut game, source, None, seat)
            .select_count_capped_multi_min_compiled(seat, CountCappedZone::Trash, 1, 1,
                "Legacy guard", false, None, CompiledPredicate::default(), Vec::new(), |_, _| {})));
        assert!(result.is_err()); assert!(game.pending_selection.is_none());
        assert_eq!(game.current_phase, phase); assert_eq!(game.memory, 10);
    } }
}
