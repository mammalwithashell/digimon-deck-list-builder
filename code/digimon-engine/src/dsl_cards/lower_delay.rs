//! Lower `CompiledDeclarativeClause::Delay` into an engine `Effect` with
//! `timing == DelayEffect`.
//!
//! Trigger mapping:
//! - `CompiledTiming::Delayed` → `DelayTrigger::MainPhaseActivated`. This is
//!   the standard printed `<Delay>` (RULES_CONTEXT 16-16) — a player-visible
//!   `[Main]`-phase activation action. The body never auto-fires; the
//!   controller trashes the Option to activate it on a later main phase.
//! - `CompiledTiming::EndOfYourTurn` → `DelayTrigger::EndOfThisTurn`.
//! - `CompiledTiming::StartOfYourTurn` → `DelayTrigger::StartOfYourNextTurn`.
//! - `CompiledTiming::EndOfYourNextTurn` → `DelayTrigger::EndOfYourNextTurn`
//!   (engine-scheduled auto-fire; retained for cards not yet migrated to the
//!   `delayed` Main-phase trigger).
//! - event timings (`on_suspend` / `on_unsuspend` / `on_ally_played`) →
//!   `DelayTrigger::OnEvent`. `on_ally_played` event-gated Delay Options
//!   (P-229) park indefinitely and fire when a matching card is played after
//!   the placing turn (PUPPETS-G004).
//!
//! Body `active_when` predicates are evaluated when the delayed effect fires.
//! Body steps run through `run_step` (Phase 2a dispatcher).

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::dsl_cards::timing_map::compiled_timing_to_engine;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::{DelayTrigger, EffectTiming};

/// Lower a `Delay` declarative clause.
///
/// Returns an `Effect` with `timing == DelayEffect` and a `delay_trigger`
/// mapped from `compiled_trigger`. The `process` closure iterates the
/// `process_steps` via `run_step`.
///
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: CompiledTiming,
    process_steps: &[CompiledStep],
) -> Effect {
    lower_with_raw(
        card,
        scope,
        active_when,
        trigger,
        process_steps,
        Arc::new(EngineRawRustRegistry::new()),
    )
}

pub fn lower_with_raw(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: CompiledTiming,
    process_steps: &[CompiledStep],
    raw: Arc<EngineRawRustRegistry>,
) -> Effect {
    let delay_trigger = match trigger {
        // Standard printed `<Delay>` — player-visible `[Main]`-phase
        // activation action (PUPPETS-G009, RULES_CONTEXT 16-16).
        CompiledTiming::Delayed => DelayTrigger::MainPhaseActivated,
        CompiledTiming::EndOfYourTurn => DelayTrigger::EndOfThisTurn,
        CompiledTiming::StartOfYourTurn => DelayTrigger::StartOfYourNextTurn,
        CompiledTiming::EndOfYourNextTurn => DelayTrigger::EndOfYourNextTurn,
        // Event-gated `<Delay>` Options: park indefinitely and fire when the
        // gating event is observed after the placing turn. `on_ally_played`
        // closes the engine half of PUPPETS-G004 (see `effect_queue.rs`
        // `enqueue_triggered` for the `EnteredField` dispatch fan-out).
        CompiledTiming::OnSuspend
        | CompiledTiming::OnUnsuspend
        | CompiledTiming::OnAllyPlayed
        | CompiledTiming::OnAttack
        | CompiledTiming::OnAllyAttack
        | CompiledTiming::OnOpponentAttack => compiled_timing_to_engine(trigger)
            .map(DelayTrigger::OnEvent)
            .unwrap_or(DelayTrigger::EndOfYourNextTurn),
        _ => DelayTrigger::EndOfYourNextTurn,
    };
    let process_arc: Arc<[CompiledStep]> = Arc::from(process_steps);
    let runtime = StepRuntime::new(raw);
    let active_when = active_when.cloned();
    let mut builder = EffectBuilder::new(card, EffectTiming::DelayEffect)
        .delay(delay_trigger)
        .process(move |ctx| {
            let mut bindings = Bindings::new();
            let _ = run_steps_with_runtime(&process_arc, ctx, &mut bindings, &runtime);
        });
    if let Some(predicate) = active_when {
        builder =
            builder.condition(move |ctx| eval_predicate(&predicate, ctx, PredicateSubject::None));
    }
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    builder.build()
}
