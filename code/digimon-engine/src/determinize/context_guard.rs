//! Invocation guards for projected context obligations.
//! Checked event-reference worlds permit fixed compiler/keyword construction;
//! unknown factories and opaque callbacks still fail before execution. The
//! older whole-context/world prototype remains strict and separate. Construction
//! origin is not an unconditional liveness certificate or a model qualification.
#![allow(dead_code)]
use crate::{effect::Effect, game::Game};
pub(crate) mod external;
#[cfg(test)]
mod raw_dispatch_tests;
#[cfg(test)]
mod world_capture_tests;
#[cfg(test)]
mod factory_origin_tests;
#[cfg(test)]
mod partition_factory_tests;

fn marked(context: &crate::trigger_context::TriggerContext) -> bool {
    context.projected_reference_reads_guarded
        || context.simultaneous_play_members.iter().any(marked)
}

/// Selecting a simultaneous subject must not discard the group's obligation.
/// Conservatively retain the union, including siblings and nested members,
/// until a separate proof can establish that any part is no longer relevant.
/// Only a boolean is transferred; the selected subject's payload is unchanged.
/// This is propagation of an existing obligation, never projection admission.
pub(crate) fn inherited_member_context(
    group: &crate::trigger_context::TriggerContext,
    member: &crate::trigger_context::TriggerContext,
) -> crate::trigger_context::TriggerContext {
    let mut selected = member.clone();
    selected.projected_reference_reads_guarded |= marked(group);
    selected
}

pub(crate) fn check(game: &Game, callback: &'static str) {
    check_compiled(game, callback);
    assert!(!game.projected_event_card_read_fence(),
        "closed-list projected context reached unproven callback: {callback}");
}

/// Fixed constructors capture structured data and scalar source handles.
/// Their event-reference reads use EventCardReference. Raw callbacks invoked
/// by compiled code still enter the stricter check above.
pub(crate) fn check_compiled(game: &Game, callback: &'static str) {
    if game.projected_world_reads_guarded()
        || game.current_trigger_context.as_ref().is_some_and(marked) {
        panic!("closed-list projected context reached unproven callback: {callback}");
    }
}

/// Reject unknown factory work even on cache hits: cached metadata may have
/// been computed from opaque captures. Bound registry entries use the separate
/// construction check below. Both the prototype world obligation and the
/// production checked-reference fence keep this unknown-factory route strict.
pub(crate) fn check_world_factory(game: &Game, factory: &'static str) {
    assert!(
        !game.projected_world_reads_guarded() && !game.projected_event_card_read_fence(),
        "closed-list projected world reached unproven factory: {factory}"
    );
}

pub(crate) fn check_registry_factory(game: &Game, card_id: &str, factory: &'static str) {
    assert!(!game.projected_world_reads_guarded(),
        "closed-list projected world reached unproven factory: {factory}");
    assert!(!game.projected_event_card_read_fence()
        || game.effect_registry.read_fence_constructible(card_id),
        "closed-list event-reference fence reached opaque factory: {factory} ({card_id})");
}

/// Structured continuations may invoke native work without an Effect callback.
/// Until their transitive reads are certified, reject an existing world
/// obligation at entry. This does not originate a marker or authorize projection.
pub(crate) fn check_world_continuation(game: &Game, continuation: &'static str) {
    assert!(
        !game.projected_world_reads_guarded(),
        "closed-list projected world reached unproven continuation: {continuation}"
    );
}

/// Check both sides before replacing the active context with a saved one.
/// Neither an unmarked caller nor an unmarked saved context can discharge an
/// existing obligation. This checks context markers only; callers retain their
/// separate WORLD entry check. No projection or execution permission is created.
pub(crate) fn check_continuation_context(
    game: &Game,
    saved: Option<&crate::trigger_context::TriggerContext>,
    continuation: &'static str,
) {
    assert!(
        !game.current_trigger_context.as_ref().is_some_and(marked)
            && !saved.is_some_and(marked),
        "closed-list projected context reached unproven continuation: {continuation}"
    );
}

pub(crate) fn guard_effect(effect: &mut Effect) {
    guard_effect_with(effect, check);
}

pub(crate) fn guard_compiled_effect(effect: &mut Effect) {
    guard_effect_with(effect, check_compiled);
}

fn guard_effect_with(effect: &mut Effect, invocation_check: fn(&Game, &'static str)) {
    // Wrap the callback itself: evaluating a one-time world snapshot cannot
    // authorize a callback added later or a captured context restored later.
    macro_rules! arc_read {
        ($($field:ident),+ $(,)?) => { $(
            if let Some(body) = effect.$field.take() {
                effect.$field = Some(std::sync::Arc::new(move |ctx| {
                    invocation_check(ctx.game, stringify!($field));
                    body(ctx)
                }));
            }
        )+ };
    }
    macro_rules! boxed_one {
        ($($field:ident),+ $(,)?) => { $(
            if let Some(body) = effect.$field.take() {
                effect.$field = Some(Box::new(move |ctx| {
                    invocation_check(ctx.game, stringify!($field));
                    body(ctx)
                }));
            }
        )+ };
    }
    macro_rules! boxed_two {
        ($($field:ident),+ $(,)?) => { $(
            if let Some(body) = effect.$field.take() {
                effect.$field = Some(Box::new(move |ctx, argument| {
                    invocation_check(ctx.game, stringify!($field));
                    body(ctx, argument)
                }));
            }
        )+ };
    }
    arc_read!(condition, trigger_qualifier, option_color_requirement_bypass, outer_optional_guard);
    boxed_one!(process, cost_reduction_fn, pay_cost_fn, activation_cost_fn, use_cost_increase_fn);
    boxed_two!(replacement_condition, dp_modifier_fn, security_attack_fn,
        link_filter, succession_filter, overclock_cost_filter);
    if let Some(body) = effect.replacement_process.take() {
        effect.replacement_process = Some(Box::new(move |ctx| {
            invocation_check(ctx.effect.game, "replacement_process");
            body(ctx)
        }));
    }
}

#[cfg(test)]
mod event_read_fence_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{card_source::CardHandle, debug_runner::DebugRunner,
        effect_context::{EffectContext, EffectReadContext}, trigger_context::TriggerContext};
    use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicBool, Ordering}}};

    fn game(marked: bool) -> Game {
        let mut game = DebugRunner::builder().memory(5).start().game;
        let mut context = TriggerContext::default();
        context.projected_reference_reads_guarded = marked;
        game.current_trigger_context = Some(context);
        game
    }

    #[test]
    fn unproven_cost_reader_is_rejected_before_callback_body() {
        let called = Arc::new(AtomicBool::new(false));
        let seen = called.clone();
        let mut effect = Effect::before_pay_cost(CardHandle(1))
            .cost_reduction_fn(move |_| { seen.store(true, Ordering::SeqCst); 7 }).build();
        guard_effect(&mut effect);
        let game = game(true);
        let ctx = EffectReadContext::new(&game, CardHandle(1), None, 0);
        assert!(catch_unwind(AssertUnwindSafe(|| effect.cost_reduction_fn.as_ref().unwrap()(&ctx))).is_err());
        assert!(!called.load(Ordering::SeqCst), "reject before the callback can read or mutate anything");
    }

    #[test]
    fn late_reader_and_cloned_context_do_not_inherit_an_earlier_admission() {
        let initial = game(false);
        let mut effect = Effect::before_pay_cost(CardHandle(1)).condition(|_| true).build();
        guard_effect(&mut effect);
        assert!(effect.condition.as_ref().unwrap()(&EffectReadContext::new(&initial, CardHandle(1), None, 0)));
        let mut later = initial.clone();
        let mut captured = TriggerContext::default();
        captured.projected_reference_reads_guarded = true;
        // Equivalent to restoring a captured context after a parked choice.
        later.current_trigger_context = Some(captured.clone());
        let mut new_effect = Effect::before_pay_cost(CardHandle(2)).condition(|_| true).build();
        guard_effect(&mut new_effect);
        assert!(catch_unwind(AssertUnwindSafe(|| new_effect.condition.as_ref().unwrap()(
            &EffectReadContext::new(&later, CardHandle(2), None, 0)))).is_err());
    }

    #[test]
    fn ordinary_unprojected_callbacks_retain_their_results() {
        let game = game(false);
        let mut effect = Effect::before_pay_cost(CardHandle(1)).cost_reduction_fn(|_| 7).build();
        guard_effect(&mut effect);
        assert_eq!(effect.cost_reduction_fn.as_ref().unwrap()(
            &EffectReadContext::new(&game, CardHandle(1), None, 0)), 7);
    }

    #[test]
    fn unproven_process_is_rejected_before_state_mutation() {
        let mut game = game(true);
        let mut effect = Effect::on_play(CardHandle(1)).process(|ctx| ctx.game.memory = 99).build();
        guard_effect(&mut effect);
        assert!(catch_unwind(AssertUnwindSafe(|| effect.process.as_ref().unwrap()(
            &mut EffectContext::new(&mut game, CardHandle(1), None, 0)))).is_err());
        assert_eq!(game.memory, 5);
    }

    #[test]
    fn every_effect_callback_rejects_before_body_and_preserves_unmarked_calls() {
        use crate::{
            permanent::PermanentHandle,
            replacement::{ReplacementCause, ReplacementContext, ReplacementOutcome, ReplacementSubject},
            selection::SourceSelectionRef,
        };
        use std::sync::atomic::AtomicUsize;
        for marked in [false, true] {
            let calls = Arc::new(AtomicUsize::new(0));
            let mut effect = Effect::on_play(CardHandle(1)).build();
            macro_rules! one {
                ($field:ident, $container:ident, $result:expr) => {{
                    let calls = calls.clone();
                    effect.$field = Some($container::new(move |_| {
                        calls.fetch_add(1, Ordering::SeqCst);
                        $result
                    }));
                }};
            }
            macro_rules! two {
                ($field:ident, $result:expr) => {{
                    let calls = calls.clone();
                    effect.$field = Some(Box::new(move |_, _| {
                        calls.fetch_add(1, Ordering::SeqCst);
                        $result
                    }));
                }};
            }
            one!(condition, Arc, true);
            one!(trigger_qualifier, Arc, true);
            one!(outer_optional_guard, Arc, true);
            one!(option_color_requirement_bypass, Arc, true);
            one!(process, Box, ());
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
            guard_effect(&mut effect);
            let mut state = game(marked);
            let permanent = PermanentHandle { player: 0, index: 0 };
            let source = SourceSelectionRef {
                permanent, field_index: 0, source_index: 0, card: CardHandle(2),
            };
            let read = EffectReadContext::new(&state, CardHandle(1), None, 0);
            macro_rules! verify {
                ($call:expr, $expected:expr) => {{
                    let outcome = catch_unwind(AssertUnwindSafe(|| $call));
                    if marked {
                        assert!(outcome.is_err(), "unknown marked callback must reject");
                    } else {
                        assert_eq!(outcome.unwrap(), $expected);
                    }
                }};
            }
            verify!(effect.condition.as_ref().unwrap()(&read), true);
            verify!(effect.trigger_qualifier.as_ref().unwrap()(&read), true);
            verify!(effect.outer_optional_guard.as_ref().unwrap()(&read), true);
            verify!(effect.option_color_requirement_bypass.as_ref().unwrap()(&read), true);
            verify!(effect.cost_reduction_fn.as_ref().unwrap()(&read), 7);
            verify!(effect.use_cost_increase_fn.as_ref().unwrap()(&read), 11);
            verify!(effect.replacement_condition.as_ref().unwrap()(&read, &ReplacementSubject::Player(0)), true);
            verify!(effect.dp_modifier_fn.as_ref().unwrap()(&read, permanent), Some(2000));
            verify!(effect.security_attack_fn.as_ref().unwrap()(&read, permanent), Some(2));
            verify!(effect.link_filter.as_ref().unwrap()(&read, permanent), true);
            verify!(effect.succession_filter.as_ref().unwrap()(&read, source), true);
            verify!(effect.overclock_cost_filter.as_ref().unwrap()(&read, permanent), true);
            let mut context = EffectContext::new(&mut state, CardHandle(1), None, 0);
            verify!(effect.process.as_ref().unwrap()(&mut context), ());
            verify!(effect.pay_cost_fn.as_ref().unwrap()(&mut context), true);
            verify!(effect.activation_cost_fn.as_ref().unwrap()(&mut context), true);
            let mut replacement = ReplacementContext {
                effect: &mut context, cause: ReplacementCause::OwnEffect,
                subject: ReplacementSubject::Player(0), original_destination: None,
                outcome: ReplacementOutcome::None, cancel_if_deleted: None,
            };
            verify!(effect.replacement_process.as_ref().unwrap()(&mut replacement), ());
            assert_eq!(calls.load(Ordering::SeqCst), if marked { 0 } else { 16 });
        }
    }

    #[test]
    fn guard_failure_survives_existing_panic_restoration_scopes() {
        let mut state = game(true);
        let mut effect = Effect::on_play(CardHandle(1))
            .process(|ctx| ctx.game.memory = 99).build();
        guard_effect(&mut effect);
        let result = catch_unwind(AssertUnwindSafe(|| {
            crate::replacement::with_effect_body(&mut state, |state| {
                effect.process.as_ref().unwrap()(
                    &mut EffectContext::new(state, CardHandle(1), None, 0));
            });
        }));
        assert!(result.is_err(), "restoration scope must rethrow a guard rejection");
        assert_eq!(state.memory, 5);
    }

    #[test]
    fn default_debug_is_unchanged_and_guarded_clone_keeps_obligation() {
        let mut context = TriggerContext::default();
        let ordinary = format!("{context:?}");
        assert!(!ordinary.contains("projected_reference_reads_guarded"));
        context.projected_reference_reads_guarded = true;
        let captured = context.clone();
        assert!(captured.projected_reference_reads_guarded);
        assert!(format!("{captured:?}").contains("projected_reference_reads_guarded: true"));
        context.projected_reference_reads_guarded = false;
        assert_eq!(format!("{context:?}"), ordinary);
    }
}

#[cfg(test)]
mod resume_world_tests;

#[cfg(test)]
mod owned_recovery_world_tests;

#[cfg(test)]
mod cleanup_world_tests;

#[cfg(test)]
mod source_restoration_world_tests;

#[cfg(test)]
mod registry_generation_tests;

#[cfg(test)]
mod step_runner_world_tests;

#[cfg(test)]
mod outer_tail_world_tests;

#[cfg(test)]
mod outer_continuations_world_tests;

#[cfg(test)]
mod saved_tail_context_world_tests;

#[cfg(test)]
mod saved_tail_obligation_tests;

#[cfg(test)]
mod outer_tail_context_tests;

#[cfg(test)]
mod outer_continuations_context_tests;

#[cfg(test)]
mod step_runner_context_tests;
