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

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

pub fn scan(
    ctx: &EffectContext<'_>,
    pred: &CompiledPredicate,
    bindings: Option<&Bindings>,
) -> Vec<PermanentHandle> {
    let player_count = ctx.game.players.len();
    let mut out = Vec::new();
    for player_idx in 0..player_count {
        let battle_len = ctx.game.players[player_idx].battle_area.len();
        // PermanentHandle.{player,index} are u8. Engine invariant: the
        // battle_area is hard-capped at MAX_FIELD_SLOTS (14, see
        // `action/space.rs`), and the player count is similarly small (2 for
        // standard 1v1 — Rules::player_count is u8 too). The casts below
        // cannot truncate under any in-game configuration; the assertions
        // catch a future expansion of either limit before silent corruption.
        debug_assert!(player_idx <= u8::MAX as usize);
        debug_assert!(battle_len <= u8::MAX as usize);
        for i in 0..battle_len {
            let h = PermanentHandle {
                player: player_idx as u8,
                index: i as u8,
            };
            let rctx = ctx.as_read();
            if eval_predicate_with_bindings(pred, &rctx, PredicateSubject::Permanent(h), bindings) {
                out.push(h);
            }
        }
    }
    out
}
