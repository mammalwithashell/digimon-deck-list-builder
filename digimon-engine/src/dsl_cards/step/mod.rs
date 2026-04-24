//! Process-step lowering dispatch. Phase 2a: memory + draw/trash helpers.
//! Phase 2b: continuation-passing dispatcher + selection handlers + zone-moves.
//! Phase 2c: permanent mutations + modifier steps (AddDpModifier, AddModifier, GrantKeyword)
//!           + control-flow steps (Optional).

pub mod control_flow;
pub mod draw;
pub mod memory;
pub mod modifiers;
pub mod permanent_mutations;
pub mod selections;
pub mod zone_moves;

use digimon_dsl::compiled::CompiledPlayerRef;
use digimon_dsl::compiled::CompiledStackPosition;
use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
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

/// Drive the full step slice to completion. When a selection step is
/// encountered, `selections::try_install` captures the tail as a
/// heap-allocated callback and returns early; the rest of the slice
/// will execute once the player resolves the selection.
pub fn run_steps(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) {
    let mut i = 0;
    while i < steps.len() {
        let step = &steps[i];

        // Control flow: drive the body via a recursive run_steps call, then
        // advance the outer index. A selection step inside the body parks
        // its own inner tail (the remainder of that body) as its callback.
        // Steps AFTER the control-flow step in this outer slice run
        // immediately on return — they are NOT captured by any inner park.
        // Phase 2c card tests never exercise [CtrlFlow(has-select), More]
        // patterns, so the current semantics are safe. Phase 2d must extend
        // run_steps to propagate inner-park state upward before adding
        // richer opt-out / delayed flows that need sequencing after the
        // inner callback resolves.
        if control_flow::try_run(step, ctx, bindings) {
            i += 1;
            continue;
        }

        // Selection steps install the remainder as their callback and return.
        if selections::try_install(step, &steps[i + 1..], ctx, bindings.clone()) {
            return;
        }

        // Synchronous families — execute and advance.
        run_step(step, ctx, bindings);
        i += 1;
    }
}

/// Dispatch a compiled step to its family-specific handler. Unhandled
/// steps are silently skipped in Phase 2a; Phase 2b/c/d add more families.
pub fn run_step(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) {
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
    if modifiers::try_run(step, ctx, bindings) {
        return;
    }
    // Phase 2d+: other families.
}
