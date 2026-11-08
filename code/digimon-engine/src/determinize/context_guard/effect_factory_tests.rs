//! Production registry/factory boundaries; no test manually installs guards.
use super::*;
use crate::{card_source::CardHandle, debug_runner::{DebugRunner, make_test_card},
    effect::Effect, effect_context::{EffectContext, EffectReadContext}, game::Game,
    enums::Keyword, permanent::PermanentHandle, selection::SourceSelectionRef,
    replacement::{ReplacementCause, ReplacementContext, ReplacementOutcome, ReplacementSubject},
    trigger_context::TriggerContext};
use digimon_dsl::compiled::CompiledPredicate;
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::atomic::{AtomicUsize, Ordering}};

struct Probe { calls: Arc<AtomicUsize>, slots: Arc<Vec<CompiledPredicate>> }
impl CardEffect for Probe {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let mut effect = Effect::on_play(card).name("guard factory probe").build();
        macro_rules! one { ($field:ident, $container:ident, $result:expr) => {{
            let calls = self.calls.clone();
            effect.$field = Some($container::new(move |_| { calls.fetch_add(1, Ordering::SeqCst); $result }));
        }}; }
        macro_rules! two { ($field:ident, $result:expr) => {{
            let calls = self.calls.clone();
            effect.$field = Some(Box::new(move |_, _| { calls.fetch_add(1, Ordering::SeqCst); $result }));
        }}; }
        one!(condition, Arc, true);
        one!(trigger_qualifier, Arc, true);
        one!(outer_optional_guard, Arc, true);
        one!(option_color_requirement_bypass, Arc, true);
        one!(cost_reduction_fn, Box, 7);
        one!(pay_cost_fn, Box, true);
        one!(activation_cost_fn, Box, true);
        one!(use_cost_increase_fn, Box, 11);
        one!(replacement_process, Box, ());
        two!(replacement_condition, true);
        two!(dp_modifier_fn, Some(2000));
        two!(security_attack_fn, Some(2));
        two!(link_filter, true);
        two!(succession_filter, true);
        two!(overclock_cost_filter, true);
        let calls = self.calls.clone();
        effect.process = Some(Box::new(move |ctx| {
            assert_eq!(ctx.source_card, card);
            calls.fetch_add(1, Ordering::SeqCst);
            ctx.game.memory += 2;
        }));
        vec![effect]
    }
    fn partition_slots(&self, inherited: bool) -> Option<Arc<Vec<CompiledPredicate>>> {
        inherited.then(|| self.slots.clone())
    }
}

fn mark(game: &mut Game, mode: u8) {
    let mut context = TriggerContext::default();
    context.projected_reference_reads_guarded = mode != 0;
    if mode == 2 {
        let mut parent = TriggerContext::default();
        parent.simultaneous_play_members.push(context);
        context = parent;
    }
    game.current_trigger_context = Some(context);
}

fn verify_all(effect: &Effect, game: &mut Game, card: CardHandle, seat: u8, guarded: bool, calls: &AtomicUsize) {
    let before = calls.load(Ordering::SeqCst);
    let memory = game.memory;
    let permanent = PermanentHandle { player: seat, index: 0 };
    let source = SourceSelectionRef { permanent, field_index: 0, source_index: 0, card };
    macro_rules! verify { ($call:expr, $expected:expr) => {{
        let result = catch_unwind(AssertUnwindSafe(|| $call));
        if guarded {
            let panic = result.expect_err("marked callback must reject");
            let message = panic.downcast_ref::<String>().map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied()).unwrap_or("");
            assert!(message.contains("closed-list projected context reached unproven callback"), "{message}");
        } else { assert_eq!(result.unwrap(), $expected); }
    }}; }
    let read = EffectReadContext::new(game, card, None, seat);
    verify!(effect.condition.as_ref().unwrap()(&read), true);
    verify!(effect.trigger_qualifier.as_ref().unwrap()(&read), true);
    verify!(effect.outer_optional_guard.as_ref().unwrap()(&read), true);
    verify!(effect.option_color_requirement_bypass.as_ref().unwrap()(&read), true);
    verify!(effect.cost_reduction_fn.as_ref().unwrap()(&read), 7);
    verify!(effect.use_cost_increase_fn.as_ref().unwrap()(&read), 11);
    verify!(effect.replacement_condition.as_ref().unwrap()(&read, &ReplacementSubject::Player(seat)), true);
    verify!(effect.dp_modifier_fn.as_ref().unwrap()(&read, permanent), Some(2000));
    verify!(effect.security_attack_fn.as_ref().unwrap()(&read, permanent), Some(2));
    verify!(effect.link_filter.as_ref().unwrap()(&read, permanent), true);
    verify!(effect.succession_filter.as_ref().unwrap()(&read, source), true);
    verify!(effect.overclock_cost_filter.as_ref().unwrap()(&read, permanent), true);
    let mut context = EffectContext::new(game, card, None, seat);
    verify!(effect.process.as_ref().unwrap()(&mut context), ());
    verify!(effect.pay_cost_fn.as_ref().unwrap()(&mut context), true);
    verify!(effect.activation_cost_fn.as_ref().unwrap()(&mut context), true);
    let mut replacement = ReplacementContext { effect: &mut context, cause: ReplacementCause::OwnEffect,
        subject: ReplacementSubject::Player(seat), original_destination: None,
        outcome: ReplacementOutcome::None, cancel_if_deleted: None };
    verify!(effect.replacement_process.as_ref().unwrap()(&mut replacement), ());
    assert_eq!(calls.load(Ordering::SeqCst) - before, if guarded { 0 } else { 16 });
    assert_eq!(game.memory, memory + if guarded { 0 } else { 2 });
}

#[test]
fn registry_clones_prefetched_factories_and_late_registration_guard_all_fields() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = CardEffectRegistry::new();
    registry.insert("probe", Arc::new(Probe { calls: calls.clone(), slots: Arc::new(vec![]) }));
    let prefetched = registry.get("probe").unwrap();
    let cloned = registry.clone();
    assert!(Arc::ptr_eq(&prefetched, &cloned.get("probe").unwrap()));
    let mut game = DebugRunner::builder().memory(5).start().game;
    for seat in 0..2 { for mode in 0..3 {
        mark(&mut game, mode);
        registry.insert("late", Arc::new(Probe { calls: calls.clone(), slots: Arc::new(vec![]) }));
        for factory in [prefetched.clone(), cloned.get("probe").unwrap(), registry.get("late").unwrap()] {
            let effects = factory.effects(CardHandle(71));
            assert_eq!(effects.len(), 1);
            verify_all(&effects[0], &mut game, CardHandle(71), seat, mode != 0, &calls);
        }
    }}
    assert!(registry.get("missing").is_none());
}

#[test]
fn cached_uncached_cloned_and_owned_effects_preserve_invocation_guards() {
    for known in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut game = DebugRunner::builder().add_card(make_test_card("GUARD-KNOWN", "Guard known"))
            .hand(0, &["GUARD-KNOWN"]).memory(5).start().game;
        let card = game.players[0].hand[0].handle();
        let id = if known { "GUARD-KNOWN" } else { "GUARD-UNKNOWN" };
        game.effect_registry.insert(id, Arc::new(Probe { calls: calls.clone(), slots: Arc::new(vec![]) }));
        let prefetched = game.effects_for_card(id, card).unwrap();
        let registry_only = game.registry_effects(id, card).unwrap();
        let mut cloned = game.clone();
        let cached = cloned.effects_for_card(id, card).unwrap();
        let cached_registry = cloned.registry_effects(id, card).unwrap();
        assert_eq!(Arc::ptr_eq(&prefetched, &cached), known);
        assert_eq!(Arc::ptr_eq(&registry_only, &cached_registry), known);
        for seat in 0..2 { for mode in 0..3 {
            mark(&mut cloned, mode);
            for effects in [&prefetched, &registry_only, &cached, &cached_registry] {
                verify_all(&effects[0], &mut cloned, card, seat, mode != 0, &calls);
            }
            let mut owned = cloned.effects_for_card_owned(id, card).unwrap();
            verify_all(&owned[0], &mut cloned, card, seat, mode != 0, &calls);
            // Real cost consumers move the stored Box out of an owned Effect.
            let payment = owned[0].pay_cost_fn.take().unwrap();
            let before = calls.load(Ordering::SeqCst);
            let result = catch_unwind(AssertUnwindSafe(|| payment(&mut EffectContext::new(&mut cloned, card, None, seat))));
            assert_eq!(result.is_err(), mode != 0);
            assert_eq!(calls.load(Ordering::SeqCst) - before, usize::from(mode == 0));
        }}
    }
}

#[test]
fn registry_preserves_partition_metadata_and_shared_identity() {
    let slots = Arc::new(vec![]);
    let mut registry = CardEffectRegistry::new();
    registry.insert("probe", Arc::new(Probe { calls: Arc::new(AtomicUsize::new(0)), slots: slots.clone() }));
    let factory = registry.clone().get("probe").unwrap();
    assert!(factory.partition_slots(false).is_none());
    assert!(Arc::ptr_eq(&slots, &factory.partition_slots(true).unwrap()));
    assert_eq!(registry.registered_card_ids(), vec!["probe"]);
    assert_eq!(registry.len(), 1);
}

#[test]
fn direct_keyword_factory_guards_callbacks_and_retains_metadata() {
    let mut game = DebugRunner::builder().memory(5).start().game;
    // Barrier has read and replacement-body callbacks; no carrier is an ordinary no-op.
    let effects = keyword_effects::keyword_to_auto_effect(Keyword::Barrier, CardHandle(71));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].keyword_source, Some(Keyword::Barrier));
    let condition = effects[0].condition.as_ref().unwrap().clone();
    for seat in 0..2 { for mode in 0..3 {
        mark(&mut game, mode);
        let read = EffectReadContext::new(&game, CardHandle(71), None, seat);
        let result = catch_unwind(AssertUnwindSafe(|| condition(&read)));
        assert_eq!(result.is_err(), mode != 0);
        if mode == 0 { assert!(!result.unwrap()); }
        let mut context = EffectContext::new(&mut game, CardHandle(71), None, seat);
        let mut replacement = ReplacementContext { effect: &mut context, cause: ReplacementCause::Battle,
            subject: ReplacementSubject::Player(seat), original_destination: None,
            outcome: ReplacementOutcome::None, cancel_if_deleted: None };
        let result = catch_unwind(AssertUnwindSafe(|| effects[0].replacement_process.as_ref().unwrap()(&mut replacement)));
        assert_eq!(result.is_err(), mode != 0);
        assert_eq!(game.memory, 5);
    }}
    let marker = keyword_effects::keyword_to_auto_effect(Keyword::BlastDigivolve, CardHandle(71));
    assert_eq!(marker[0].keyword_source, Some(Keyword::BlastDigivolve));
    assert!(marker[0].process.is_none());
}

#[test]
fn compiled_dsl_registry_process_checks_before_literal_mutation() {
    let yaml = "card: GUARD-DSL\nname: Guard DSL\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 1000\neffects:\n  - when: on_play\n    process:\n      - gain_memory: 2\n";
    let mut game = DebugRunner::builder().from_dsl_yaml(yaml).unwrap().hand(0, &["GUARD-DSL"]).memory(5).start().game;
    let card = game.players[0].hand[0].handle();
    let factory = game.effect_registry.get("GUARD-DSL").unwrap();
    let effects = factory.effects(card);
    for seat in 0..2 { for mode in 0..3 {
        game.memory = 5;
        mark(&mut game, mode);
        let result = catch_unwind(AssertUnwindSafe(|| effects[0].process.as_ref().unwrap()(
            &mut EffectContext::new(&mut game, card, None, seat))));
        assert_eq!(result.is_err(), mode != 0);
        assert_eq!(game.memory, if mode != 0 { 5 } else if seat == 0 { 7 } else { 3 });
    }}
}
