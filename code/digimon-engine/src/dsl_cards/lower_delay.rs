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
    // Rules manual 16-16-2: "The processing from <Delay> is optional." An
    // event-gated `<Delay>` (P-228 / P-229 / BT22-098 / BT23-096 / BT24-089 /
    // EX10-069) reaches its timing when the gating event is observed
    // — at that point the CONTROLLER chooses whether to activate (trash the
    // Option + run the body) or decline (the Option stays parked and can fire
    // on a later matching event). DCGO registers these triggers with
    // `SetUpActivateClass(..., isOptional: true, ...)` and shows a bool
    // prompt. Mark the effect optional and force the outer accept/decline
    // prompt so the choice surfaces through `pending_selection` (the
    // no-approximations policy forbids the previous mandatory auto-fire).
    // The prompt is unconditional (no first-step candidate guard): DCGO
    // prompts whenever the event condition matches, and accepting trashes
    // the Option even when the body then finds no legal target — declining
    // the trash is itself the meaningful choice.
    //
    // `MainPhaseActivated` needs no flag (activation IS an explicit player
    // `[Main]` action — flagging it would double-prompt), and the turn-
    // scheduled triggers fire through `resolve_delayed_options_matching`
    // (game_phases.rs), which bypasses the queue's optional machinery.
    if matches!(delay_trigger, DelayTrigger::OnEvent(_)) {
        builder = builder.optional().needs_outer_optional_prompt();
    }
    if let Some(predicate) = active_when {
        builder =
            builder.condition(move |ctx| eval_predicate(&predicate, ctx, PredicateSubject::None));
    }
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    builder.build()
}
