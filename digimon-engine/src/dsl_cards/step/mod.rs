//! Process-step lowering dispatch. Phase 2a: memory + draw/trash helpers.

pub mod draw;
pub mod memory;

use digimon_dsl::compiled::CompiledPlayerRef;

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
