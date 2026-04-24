//! Process-step lowering dispatch. Phase 2a: memory + draw/trash helpers.

pub mod draw;
pub mod memory;

use digimon_dsl::compiled::CompiledPlayerRef;
use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;
use crate::enums::PlayerId;

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

/// Dispatch a compiled step to its family-specific handler. Unhandled
/// steps are silently skipped in Phase 2a; Phase 2b/c/d add more families.
pub fn run_step(step: &CompiledStep, ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    if memory::try_run(step, ctx) {
        return;
    }
    if draw::try_run(step, ctx) {
        return;
    }
    // Phase 2b+: other families.
}

/// Like [`run_step`] but also handles `CompiledStep::RawRust` by looking up
/// the fn name in `raw` and invoking it with `(&mut EffectContext, &mut
/// Bindings)` — so raw_rust fns can read/write the same binding environment
/// DSL steps use. Unregistered names or a missing registry are silent no-ops
/// (safe for RL: the agent sees no action to take but the game continues
/// without crashing).
pub fn run_step_with_raw(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    raw: Option<&crate::dsl_cards::raw_rust::EngineRawRustRegistry>,
) {
    if let CompiledStep::RawRust { fn_name, .. } = step {
        if let Some(r) = raw {
            if let Some(f) = r.step_fn(fn_name) {
                f(ctx, bindings);
            }
        }
        return;
    }
    run_step(step, ctx, bindings);
}
