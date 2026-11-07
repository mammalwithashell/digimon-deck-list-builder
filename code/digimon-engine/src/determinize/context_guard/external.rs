//! Inactive prototype wrappers for callback families outside Effect.
//! Not installed by any production registry or selection builder.
use crate::{game::Game, trigger_context::TriggerContext};
use std::sync::Arc;

/// Only the obligation bit is retained, never a hidden card or captured payload.
/// Callers must account for every actual capture. This cannot introspect opaque
/// closures, nor can a snapshot made before later projection authorize them.
#[derive(Clone, Copy, Default)]
pub(crate) struct CapturedReadObligation(bool);

impl CapturedReadObligation {
    pub(crate) fn from_context(context: Option<&TriggerContext>) -> Self {
        fn marked(context: &TriggerContext) -> bool {
            context.projected_reference_reads_guarded ||
                context.simultaneous_play_members.iter().any(marked)
        }
        Self(context.is_some_and(marked))
    }

    fn check(self, game: &Game, callback: &'static str) {
        assert!(!self.0, "closed-list projected captured context reached unproven callback: {callback}");
        super::check(game, callback);
    }
}

pub(crate) type RuntimePredicate = Arc<dyn Fn(
    &crate::effect_context::EffectReadContext<'_>,
    crate::permanent::PermanentHandle,
) -> bool + Send + Sync>;

pub(crate) fn guard_until_condition(body: crate::modifiers::UntilConditionFn, captured: CapturedReadObligation) -> crate::modifiers::UntilConditionFn {
    Arc::new(move |game, subject| {
        captured.check(game, "until_condition");
        body(game, subject)
    })
}

pub(crate) fn guard_passive_replacement(body: crate::replacement::ReplacementConditionFn, captured: CapturedReadObligation) -> crate::replacement::ReplacementConditionFn {
    Arc::new(move |ctx, subject| {
        captured.check(ctx.game, "passive_replacement_condition");
        body(ctx, subject)
    })
}

pub(crate) fn guard_granted_body(body: crate::modifiers::GrantedEffectBody, captured: CapturedReadObligation) -> crate::modifiers::GrantedEffectBody {
    Arc::new(move |ctx| {
        captured.check(ctx.game, "granted_effect_body");
        body(ctx)
    })
}

pub(crate) fn guard_granted_qualifier(body: crate::modifiers::GrantedTriggerQualifier, captured: CapturedReadObligation) -> crate::modifiers::GrantedTriggerQualifier {
    Arc::new(move |ctx| {
        captured.check(ctx.game, "granted_trigger_qualifier");
        body(ctx)
    })
}

pub(crate) fn guard_runtime_predicate(body: RuntimePredicate, captured: CapturedReadObligation) -> RuntimePredicate {
    Arc::new(move |ctx, target| {
        captured.check(ctx.game, "runtime_permanent_predicate");
        body(ctx, target)
    })
}

pub(crate) fn guard_selection(body: crate::selection::SelectionCallback, captured: CapturedReadObligation) -> crate::selection::SelectionCallback {
    Box::new(move |game, action| {
        captured.check(game, "selection_callback");
        body(game, action)
    })
}

pub(crate) fn guard_decline(body: crate::selection::DeclineCallback, captured: CapturedReadObligation) -> crate::selection::DeclineCallback {
    Box::new(move |game| {
        captured.check(game, "decline_callback");
        body(game)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        card_source::CardHandle, debug_runner::DebugRunner,
        effect::{Effect}, effect_context::{EffectContext, EffectReadContext},
        modifiers::ModifierSubject, permanent::PermanentHandle,
        replacement::ReplacementSubject,
    };
    use std::{fmt::Debug, panic::{catch_unwind, AssertUnwindSafe},
        sync::atomic::{AtomicUsize, Ordering}};

    fn context(marked: bool) -> TriggerContext {
        let mut context = TriggerContext::default();
        context.projected_reference_reads_guarded = marked;
        context
    }
    fn verify<T: Debug + PartialEq>(must_reject: bool, f: impl FnOnce() -> T, expected: T) {
        let result = catch_unwind(AssertUnwindSafe(f));
        if must_reject {
            assert!(result.is_err(), "marked current or captured context must reject");
        } else {
            assert_eq!(result.unwrap(), expected);
        }
    }

    #[test]
    fn external_families_check_current_and_captured_context_before_body() {
        for current_marked in [false, true] {
            for captured_marked in [false, true] {
                let calls = Arc::new(AtomicUsize::new(0));
                // Nested groups must carry the same obligation.
                let mut captured = context(false);
                captured.simultaneous_play_members.push(context(captured_marked));
                let obligation = CapturedReadObligation::from_context(Some(&captured));
                let reject = current_marked || captured_marked;
                let mut game = DebugRunner::builder().memory(5).start().game;
                game.current_trigger_context = Some(context(current_marked));
                let target = PermanentHandle { player: 1, index: 2 };

                let seen = calls.clone();
                let until = guard_until_condition(Arc::new(move |_, subject| {
                    seen.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(subject, ModifierSubject::Player(1)); true
                }), obligation);
                verify(reject, || until(&game, ModifierSubject::Player(1)), true);

                let seen = calls.clone();
                let replacement = guard_passive_replacement(Arc::new(move |_, subject| {
                    seen.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(*subject, ReplacementSubject::Player(1)); true
                }), obligation);
                let read = EffectReadContext::new(&game, CardHandle(1), None, 0);
                verify(reject, || replacement(&read, &ReplacementSubject::Player(1)), true);

                let seen = calls.clone();
                let qualifier = guard_granted_qualifier(Arc::new(move |_| {
                    seen.fetch_add(1, Ordering::SeqCst); true
                }), obligation);
                verify(reject, || qualifier(&read), true);

                let seen = calls.clone();
                let predicate = guard_runtime_predicate(Arc::new(move |_, handle| {
                    seen.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(handle, target); true
                }), obligation);
                verify(reject, || predicate(&read, target), true);

                let seen = calls.clone();
                let granted = guard_granted_body(Arc::new(move |ctx| {
                    seen.fetch_add(1, Ordering::SeqCst); ctx.game.memory += 1;
                }), obligation);
                verify(reject, || granted(&mut EffectContext::new(&mut game, CardHandle(1), None, 0)), ());

                let seen = calls.clone();
                let selected = guard_selection(Box::new(move |game, action| {
                    seen.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(action, 17); game.memory += 1;
                }), obligation);
                verify(reject, || selected(&mut game, 17), ());

                let seen = calls.clone();
                let declined = guard_decline(Box::new(move |game| {
                    seen.fetch_add(1, Ordering::SeqCst); game.memory += 1;
                }), obligation);
                verify(reject, || declined(&mut game), ());
                assert_eq!(calls.load(Ordering::SeqCst), if reject { 0 } else { 7 });
                assert_eq!(game.memory, if reject { 5 } else { 8 });
            }
        }
    }

    #[test]
    fn effect_guard_rejects_marked_nested_context_member() {
        let mut game = DebugRunner::builder().start().game;
        let mut group = context(false);
        group.simultaneous_play_members.push(context(true));
        game.current_trigger_context = Some(group);
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = calls.clone();
        let mut effect = Effect::on_play(CardHandle(1)).condition(move |_| {
            seen.fetch_add(1, Ordering::SeqCst); true
        }).build();
        super::super::guard_effect(&mut effect);
        verify(true, || effect.condition.as_ref().unwrap()(
            &EffectReadContext::new(&game, CardHandle(1), None, 0)), true);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn selection_rejects_before_restoring_its_own_captured_context() {
        let mut game = DebugRunner::builder().memory(5).start().game;
        game.current_trigger_context = None;
        let captured = context(true);
        let obligation = CapturedReadObligation::from_context(Some(&captured));
        let callback = guard_selection(Box::new(move |game, _| {
            game.current_trigger_context = Some(captured);
            game.memory = 99;
        }), obligation);
        verify(true, || callback(&mut game, 17), ());
        assert!(game.current_trigger_context.is_none());
        assert_eq!(game.memory, 5);
    }
}
