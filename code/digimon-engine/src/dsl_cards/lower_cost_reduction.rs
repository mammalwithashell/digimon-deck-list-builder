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
        None,
    )
}

#[allow(clippy::too_many_arguments)]
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
    when_any_ally_digivolves_into: Option<CompiledPredicate>,
) -> Effect {
    let active_when = active_when.map(Arc::new);
    let condition = condition.map(Arc::new);
    let when_any_ally_played = when_any_ally_played.map(Arc::new);
    let when_any_ally_digivolves_into = when_any_ally_digivolves_into.map(Arc::new);
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
    let condition_when_digivolve = when_any_ally_digivolves_into.clone();
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
        // G-COST-REDUCTION-DIGIVOLVE-INTO: fire only for a DIGIVOLVE cost
        // whose target (the card being digivolved into) matches.
        if let Some(wdi) = &condition_when_digivolve {
            if !rctx.cost_is_digivolve {
                return false;
            }
            let Some(target) = rctx.cost_target_card else {
                return false;
            };
            if !eval_predicate(wdi, rctx, PredicateSubject::Card(target)) {
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
        if let Some(wdi) = &when_any_ally_digivolves_into {
            if !rctx.cost_is_digivolve {
                return 0;
            }
            let Some(target) = rctx.cost_target_card else {
                return 0;
            };
            if !eval_predicate(wdi, rctx, PredicateSubject::Card(target)) {
                return 0;
            }
        }
        amount_fn
            .as_ref()
            .map(|f| evaluate_amount(f, rctx, amount_runtime.raw()))
            .unwrap_or(0)
    });
    if !pay_cost.is_empty() {
        // When the `pay_cost` begins with a declinable (PASS-able) selection
        // — e.g. BT12-112's optional "place 1 [Shoutmon]" — running it
        // surfaces the player's own opt-in/opt-out, so the cost-reduction
        // dispatch can auto-apply it (the inner optional select IS the
        // acceptance prompt) instead of wrapping it in the redundant
        // "Use X to reduce play cost?" confirmation gate. Mandatory-cost
        // reducers (the self-suspend idiom below, "trash 2 cards", ...) leave
        // this `false` and keep their gate. Reuses the same first-step probe
        // as the triggered-effect outer-optional lowering for consistency.
        builder = builder.pay_cost_self_gated(
            crate::dsl_cards::lower_triggered::body_first_step_is_declinable(pay_cost.as_ref()),
        );
        // An INTERACTIVE pay_cost (its first step installs a selection — e.g.
        // `trash_bottom_face_down_source_under_tamer`) parks. The synchronous
        // digivolve / Option-use cost scan cannot host a park, so the engine
        // routes such a reducer through a dedicated interactive prompt.
        // `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
        builder = builder.pay_cost_interactive(
            crate::dsl_cards::lower_triggered::body_first_step_installs_selection(
                pay_cost.as_ref(),
            ),
        );
        // Special-case the "by suspending this Tamer" idiom: a `pay_cost` of
        // a single self-targeted `suspend` must FAIL when the source is
        // already suspended (the cost is unpayable → reduction does not
        // apply). The generic `Suspend` step always reports success, so it
        // cannot express the gate; route through `suspend_self_as_cost`,
        // which returns `false` for an already-suspended source.
        // `G-COST-REDUCTION-DIGIVOLVE-INTO` (BT5-092).
        if pay_cost_is_self_suspend(&pay_cost) {
            builder = builder.pay_cost_fn(move |ctx| ctx.suspend_self_as_cost());
        } else {
            builder = builder.pay_cost_fn(move |ctx| {
                ctx.cost_unpayable = false;
                let mut bindings = crate::dsl_cards::bindings::Bindings::new();
                let outcome =
                    run_steps_with_runtime(pay_cost.as_ref(), ctx, &mut bindings, &runtime);
                // A PARKED pay_cost (an interactive step like
                // `trash_bottom_face_down_source_under_tamer`'s Tamer pick) will
                // be paid when its selection resolves; it returns `false` here
                // and `apply_cost_reduction_candidate` credits the deferred
                // amount behind the park (play-from-hand path only). A
                // SYNCHRONOUS outcome means the cost completed — UNLESS a step
                // signalled it was unpayable (`cost_unpayable`), in which case
                // nothing was paid and the reduction must not be credited.
                // `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
                matches!(outcome, RunOutcome::Synchronous) && !ctx.cost_unpayable
            });
        }
    }
    builder.build()
}

/// True when `pay_cost` is exactly a single self-targeted `suspend` step —
/// the "by suspending this Tamer" cost idiom.
fn pay_cost_is_self_suspend(pay_cost: &[CompiledStep]) -> bool {
    use digimon_dsl::compiled::CompiledBindingRef;
    matches!(
        pay_cost,
        [CompiledStep::Suspend { target }]
            if matches!(target, CompiledBindingRef::Source | CompiledBindingRef::SelfRef)
    )
}
