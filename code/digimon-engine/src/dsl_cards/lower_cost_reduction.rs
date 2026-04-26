//! Lower `CompiledDeclarativeClause::CostReduction`.
//!
//! Phase 1c supports only the `when_playing_this: true` + literal `amount`
//! shape. Emission caveats:
//!
//! * The engine's `scan_before_pay_cost_reduction` only inspects battle_area
//!   cards, so `when_playing_this: true` reductions DO NOT fire from hand
//!   today. The Effect is still emitted with correct shape; it will activate
//!   once the engine-side scan extends to the acting player's hand (tracked
//!   for Phase 2).
//! * `amount_fn`, `pay_cost`, `when_any_ally_played`, and non-`before_pay_cost`
//!   timings are skipped — caller should not emit in those cases.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope};

use crate::card_source::CardHandle;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect::Effect;

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    condition: Option<CompiledPredicate>,
    once_per_turn: bool,
    amount: i32,
) -> Effect {
    let active_when = active_when.map(Arc::new);
    let condition = condition.map(Arc::new);

    let mut builder = Effect::before_pay_cost(card).name("Cost reduction");
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    if once_per_turn {
        builder = builder.once_per_turn();
    }
    builder = builder.cost_reduction_fn(move |rctx| {
        if let Some(aw) = &active_when {
            if !eval_predicate(aw, rctx, PredicateSubject::None) {
                return 0;
            }
        }
        if let Some(c) = &condition {
            if !eval_predicate(c, rctx, PredicateSubject::None) {
                return 0;
            }
        }
        amount
    });
    builder.build()
}
