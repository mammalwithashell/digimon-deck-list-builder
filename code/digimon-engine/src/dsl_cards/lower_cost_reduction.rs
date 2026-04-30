//! Lower `CompiledDeclarativeClause::CostReduction`.
//!
//! Phase 3 supports literal and formula-backed amounts plus synchronous
//! `pay_cost` bodies. The engine's `scan_before_pay_cost_reduction` still
//! scans battle-area cards, so reducers authored for hand-only activation
//! rely on the engine-side scan semantics.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledFormula, CompiledPredicate, CompiledScope, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::formula_eval;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, RunOutcome, StepRuntime};
use crate::effect::Effect;
use crate::effect_context::EffectReadContext;
use crate::permanent::PermanentHandle;

fn evaluate_amount(
    formula: &CompiledFormula,
    rctx: &EffectReadContext<'_>,
    raw: &EngineRawRustRegistry,
) -> i32 {
    // Use the source permanent as the formula target when available.  When the
    // effect fires during `before_pay_cost` for a card still in hand,
    // `source_permanent` is `None`.  In that case we supply a sentinel handle
    // (`player=controller, index=255`).  Formulas that do not dereference the
    // target (e.g. `CardCountInZoneScoped`) evaluate correctly; formulas that
    // DO dereference it (e.g. `StackSize`, `MaterialCount`) call
    // `battle_area.get(255)` which returns `None` and safely short-circuit to 0.
    let target = rctx.source_permanent.unwrap_or(PermanentHandle {
        player: rctx.player,
        index: 255,
    });
    formula_eval::evaluate_read_with_raw(formula, rctx, target, raw)
}

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    condition: Option<CompiledPredicate>,
    once_per_turn: bool,
    amount: i32,
) -> Effect {
    lower_with_formula(
        card,
        scope,
        active_when,
        condition,
        once_per_turn,
        Some(CompiledFormula::Literal(amount)),
        vec![],
        Arc::new(EngineRawRustRegistry::new()),
        false,
        false,
        None,
    )
}

pub fn lower_with_formula(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    condition: Option<CompiledPredicate>,
    once_per_turn: bool,
    amount_fn: Option<CompiledFormula>,
    pay_cost: Vec<CompiledStep>,
    raw: Arc<EngineRawRustRegistry>,
    optional: bool,
    when_playing_this: bool,
    when_any_ally_played: Option<CompiledPredicate>,
) -> Effect {
    let active_when = active_when.map(Arc::new);
    let condition = condition.map(Arc::new);
    let when_any_ally_played = when_any_ally_played.map(Arc::new);
    let amount_fn = amount_fn.map(Arc::new);
    let pay_cost: Arc<[CompiledStep]> = Arc::from(pay_cost);
    let runtime = StepRuntime::new(raw);
    let amount_runtime = runtime.clone();

    let mut builder = Effect::before_pay_cost(card).name("Cost reduction");
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    if once_per_turn {
        builder = builder.once_per_turn();
    }
    if optional {
        builder = builder.optional();
    }
    if when_playing_this {
        builder = builder.when_playing_this();
    }
    let condition_active_when = active_when.clone();
    let condition_condition = condition.clone();
    let condition_when_any = when_any_ally_played.clone();
    builder = builder.condition(move |rctx| {
        if let Some(aw) = &condition_active_when {
            let subject = rctx
                .source_permanent
                .map(PredicateSubject::Permanent)
                .unwrap_or(PredicateSubject::None);
            if !eval_predicate(aw, rctx, subject) {
                return false;
            }
        }
        if let Some(c) = &condition_condition {
            if !eval_predicate(c, rctx, PredicateSubject::None) {
                return false;
            }
        }
        if let Some(wap) = &condition_when_any {
            let Some(target) = rctx.cost_target_card else {
                return false;
            };
            if !eval_predicate(wap, rctx, PredicateSubject::Card(target)) {
                return false;
            }
        }
        true
    });
    builder = builder.cost_reduction_fn(move |rctx| {
        if let Some(aw) = &active_when {
            let subject = rctx
                .source_permanent
                .map(PredicateSubject::Permanent)
                .unwrap_or(PredicateSubject::None);
            if !eval_predicate(aw, rctx, subject) {
                return 0;
            }
        }
        if let Some(c) = &condition {
            if !eval_predicate(c, rctx, PredicateSubject::None) {
                return 0;
            }
        }
        if let Some(wap) = &when_any_ally_played {
            let Some(target) = rctx.cost_target_card else {
                return 0;
            };
            if !eval_predicate(wap, rctx, PredicateSubject::Card(target)) {
                return 0;
            }
        }
        amount_fn
            .as_ref()
            .map(|f| evaluate_amount(f, rctx, amount_runtime.raw()))
            .unwrap_or(0)
    });
    if !pay_cost.is_empty() {
        builder = builder.pay_cost_fn(move |ctx| {
            let mut bindings = crate::dsl_cards::bindings::Bindings::new();
            matches!(
                run_steps_with_runtime(pay_cost.as_ref(), ctx, &mut bindings, &runtime),
                RunOutcome::Synchronous
            )
        });
    }
    builder.build()
}
