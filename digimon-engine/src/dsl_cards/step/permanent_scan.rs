//! Enumerate battle-area permanents matching a `CompiledPredicate`.
//! Used by `ForEach` and `AddModifier { target: Filter(...) }`.
//! `PerSelected` does NOT go through this helper — it iterates a
//! pre-bound `PermanentList` / `CardList` directly (the binding was
//! produced by an earlier `select_count_capped_multi`).
//!
//! Iteration order: P0's battle_area in ascending index, then P1's. Stable
//! and turn-independent (callers that need turn-relative order should
//! re-sort).

use digimon_dsl::compiled::CompiledPredicate;

use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

pub fn scan(ctx: &EffectContext<'_>, pred: &CompiledPredicate) -> Vec<PermanentHandle> {
    let player_count = ctx.game.players.len();
    let mut out = Vec::new();
    for player_idx in 0..player_count {
        let battle_len = ctx.game.players[player_idx].battle_area.len();
        for i in 0..battle_len {
            let h = PermanentHandle {
                player: player_idx as u8,
                index: i as u8,
            };
            let rctx = ctx.as_read();
            if eval_predicate(pred, &rctx, PredicateSubject::Permanent(h)) {
                out.push(h);
            }
        }
    }
    out
}
