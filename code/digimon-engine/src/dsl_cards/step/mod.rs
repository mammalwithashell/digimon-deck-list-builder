//! Process-step lowering dispatch. Phase 2a: memory + draw/trash helpers.
//! Phase 2b: continuation-passing dispatcher + selection handlers + zone-moves.
//! Phase 2c: permanent mutations + modifier steps (AddDpModifier, AddModifier, GrantKeyword)
//!           + control-flow steps (Optional, If).

pub mod as_selecting_player;
pub mod control_flow;
pub mod draw;
pub mod iteration;
pub mod memory;
pub mod modifiers;
pub mod permanent_mutations;
pub mod permanent_scan;
pub mod play_digivolve;
pub mod replacement_outcomes;
pub mod schedule_delayed;
pub mod selections;
pub mod zone_moves;

use digimon_dsl::compiled::CompiledPlayerRef;
use digimon_dsl::compiled::CompiledStackPosition;
use digimon_dsl::compiled::CompiledStep;
use std::sync::Arc;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::effect_context::EffectContext;
use crate::enums::PlayerId;
use crate::enums::StackPosition;

/// Map a `CompiledStackPosition` to the engine's `StackPosition`.
/// Shared by `zone_moves` and `permanent_mutations` — lives here to avoid
/// duplicate private copies in each sub-module.
pub(super) fn map_stack_position(p: CompiledStackPosition) -> StackPosition {
    match p {
        CompiledStackPosition::Top => StackPosition::Top,
        CompiledStackPosition::Bottom => StackPosition::Bottom,
        CompiledStackPosition::Random => StackPosition::Random,
    }
}

/// Resolve a `CompiledPlayerRef` to the concrete `PlayerId`. `Any` resolves
/// to `ctx.player` — callers that want to fan out to every player should
/// enumerate `ctx.game.players.len()` directly.
pub fn resolve_player(ctx: &EffectContext<'_>, r: CompiledPlayerRef) -> PlayerId {
    match r {
        CompiledPlayerRef::You => ctx.player,
        CompiledPlayerRef::Opponent => ctx.opponent_id(),
        CompiledPlayerRef::Active => ctx.game.turn_player(),
        CompiledPlayerRef::Any => ctx.player,
    }
}

/// Whether a step ran synchronously to completion or installed a parked
/// selection. Phase 2d Task 7 propagates this outward across nested
/// `run_steps` re-entries so a parked selection inside an `If` /
/// `ForEach` body suspends the outer slice instead of letting subsequent
/// outer steps race ahead of the resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Synchronous,
    Parked,
}

#[derive(Clone, Debug)]
pub struct StepRuntime {
    raw: Arc<EngineRawRustRegistry>,
}

impl Default for StepRuntime {
    fn default() -> Self {
        Self::new(Arc::new(EngineRawRustRegistry::new()))
    }
}

impl StepRuntime {
    pub fn new(raw: Arc<EngineRawRustRegistry>) -> Self {
        Self { raw }
    }

    pub fn raw(&self) -> &EngineRawRustRegistry {
        &self.raw
    }
}

/// Park the outer-slice tail (steps that follow the just-parked control-
/// flow / iteration step) onto `Game::dsl_outer_tail` so the eventual
/// selection-callback can resume it via `drain_dsl_outer_tail`.
///
/// Empty tails are dropped — there's nothing to resume. The
/// `debug_assert!` enforces the single-outstanding invariant documented
/// on the slot.
fn park_outer_tail(
    ctx: &mut EffectContext<'_>,
    bindings: &Bindings,
    steps: &[CompiledStep],
    i: usize,
    runtime: &StepRuntime,
) {
    let outer_tail = steps[i + 1..].to_vec();
    if outer_tail.is_empty() {
        return;
    }
    debug_assert!(
        ctx.game.dsl_outer_tail.is_none(),
        "dsl_outer_tail overwrite: an earlier outer continuation \
         was never drained — likely a nested-park bug",
    );
    ctx.game
        .dsl_outer_tail
        .replace((outer_tail, bindings.clone(), runtime.clone()));
}

/// Drain `Game::dsl_outer_tail` if a parked outer continuation is
/// pending. Selection-install callbacks call this after their own inner
/// tail completes so the steps that followed an outer control-flow /
/// iteration step (parked by `park_outer_tail`) actually run.
///
/// Public to the selections sub-module so new selection kinds added in
/// future phases (`SelectReveal`, `SelectMaterial`, …) pick this up by
/// calling one helper instead of duplicating the take + run_steps
/// boilerplate per callback.
///
/// Phase 2f3: `cb_ctx` arrives with `override_selecting_player` carrying
/// the override that was active at the parking select's install time.
/// Outer-tail steps live OUTSIDE the `AsSelectingPlayer` body whose select
/// parked, so they must not inherit the override. Clear it here before
/// running the outer tail; the inner body's chained selects already saw
/// the override via the parked-callback's reconstructed ctx (Task 1).
pub(crate) fn drain_dsl_outer_tail(cb_ctx: &mut EffectContext<'_>) {
    if let Some((outer_tail, mut outer_b, runtime)) = cb_ctx.game.dsl_outer_tail.take() {
        cb_ctx.set_override_selecting_player(None);
        run_steps_with_runtime(&outer_tail, cb_ctx, &mut outer_b, &runtime);
    }
}

/// Drive the full step slice to completion. When a selection step is
/// encountered, `selections::try_install` captures the tail as a
/// heap-allocated callback and returns `Parked`; the rest of the slice
/// will execute once the player resolves the selection.
///
/// Phase 2d: the dispatcher fans out to control-flow / iteration handlers
/// that may themselves park. A `Parked` from any of them propagates up.
/// Task 7 wires outer-tail capture so steps after a parked control-flow
/// step are deferred until the inner callback resolves.
pub fn run_steps(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> RunOutcome {
    run_steps_with_runtime(steps, ctx, bindings, &StepRuntime::default())
}

pub fn run_steps_with_runtime(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> RunOutcome {
    let mut i = 0;
    while i < steps.len() {
        let step = &steps[i];

        if let Some(outcome) = control_flow::try_run(step, ctx, bindings, runtime) {
            if matches!(outcome, RunOutcome::Parked) {
                park_outer_tail(ctx, bindings, steps, i, runtime);
                return RunOutcome::Parked;
            }
            i += 1;
            continue;
        }

        if let Some(outcome) = as_selecting_player::try_run(step, ctx, bindings, runtime) {
            if matches!(outcome, RunOutcome::Parked) {
                park_outer_tail(ctx, bindings, steps, i, runtime);
                return RunOutcome::Parked;
            }
            i += 1;
            continue;
        }

        if let Some(outcome) = iteration::try_run(step, ctx, bindings, runtime) {
            if matches!(outcome, RunOutcome::Parked) {
                park_outer_tail(ctx, bindings, steps, i, runtime);
                return RunOutcome::Parked;
            }
            i += 1;
            continue;
        }

        // Selection steps install the remainder as their callback and return.
        if selections::try_install(step, &steps[i + 1..], ctx, bindings.clone(), runtime) {
            return RunOutcome::Parked;
        }

        // Synchronous families — execute and advance.
        run_step_with_runtime(step, ctx, bindings, runtime);
        i += 1;
    }
    RunOutcome::Synchronous
}

/// Dispatch a compiled step to its family-specific handler. Unhandled
/// steps are silently skipped in Phase 2a; Phase 2b/c/d add more families.
pub fn run_step(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) {
    run_step_with_runtime(step, ctx, bindings, &StepRuntime::default());
}

pub fn run_step_with_runtime(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) {
    if let CompiledStep::RawRust { fn_name, .. } = step {
        if let Some(f) = runtime.raw().step_fn(fn_name) {
            f(ctx, bindings);
        }
        return;
    }
    if replacement_outcomes::try_run(step, ctx, bindings) {
        return;
    }
    if memory::try_run(step, ctx) {
        return;
    }
    if draw::try_run(step, ctx) {
        return;
    }
    if zone_moves::try_run(step, ctx, bindings) {
        return;
    }
    if permanent_mutations::try_run(step, ctx, bindings) {
        return;
    }
    if modifiers::try_run(step, ctx, bindings, runtime) {
        return;
    }
    if play_digivolve::try_run(step, ctx, bindings) {
        return;
    }
    if schedule_delayed::try_run(step, ctx, bindings, runtime) {
        return;
    }
    // Phase 2d+: other families.
}
