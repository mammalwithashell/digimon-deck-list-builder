//! Lower `CompiledDeclarativeClause::CostReduction`.
//!
//! Phase 3 supports literal and formula-backed amounts plus synchronous
//! `pay_cost` bodies. The engine's `scan_before_pay_cost_reduction` still
//! scans battle-area cards, so reducers authored for hand-only activation
//! rely on the engine-side scan semantics.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledFormula, CompiledPlayerRef, CompiledPredicate, CompiledScope,
    CompiledStep, CompiledZone,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::formula_eval;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, RunOutcome, StepRuntime};
use crate::effect::{Effect, PayCostDemand, PayCostPreview, PayCostToken};
use crate::effect_context::EffectReadContext;
use crate::enums::PlayerId;
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
    // When set, this reducer shares its once-per-turn accounting slot with any
    // sibling reducer carrying the same group id. Used to give the two
    // `scope: both` copies (FaceUp + Inherited) ONE shared OPT lockout so a
    // card cannot reduce a cost once as an active top and again the same turn
    // as a digivolution source. G-ENGINE-SHARED-OPT-SCOPE-BOTH-REDUCER.
    shared_opt_group: Option<u8>,
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
    if let Some(group) = shared_opt_group {
        builder = builder.shared_opt_group(group);
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
    // Pay-cost actionability guard (`G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST`).
    // DCGO gates an INTERACTIVE cost reducer's offer on
    // `CanActivateCondition = HasMatchConditionPermanent(...)` — the reducer is
    // not offered at all when the "by <verb> 1 of your ... Digimon" cost has no
    // eligible target (e.g. BT18-073 / BT13-083 / BT13-103's "by deleting 1 of
    // your [Composite/Gizmon/level-3] Digimon"). Without this gate the reducer
    // would surface a spurious accept prompt and, worse, a MANDATORY
    // `select_own_permanent` pay_cost with no candidates would run its tail
    // (the delete) as a no-op and credit the reduction for a cost never paid.
    // We derive the guard from the pay_cost's FIRST step (the cost selection);
    // it fires only for the single-pick selection kinds `first_step_candidate_guard`
    // recognises (`select_own/opponent/any_permanent`, `select_hand`,
    // `select_trash`).
    //
    // ONLY applied when that first select is MANDATORY. A DECLINABLE first step
    // (`pay_cost_self_gated` — e.g. BT12-112's optional "place 1 [Shoutmon]") is
    // its own opt-in/opt-out surface (the inner PASS is the decline), so
    // candidate-gating the OFFER there would change the self-gated flow; leave it
    // alone. Purpose-built interactive steps (`trash_bottom_face_down_source_under_tamer`)
    // are not recognised by `first_step_candidate_guard` and keep their runtime
    // `cost_unpayable` abort — no behavior change for them either.
    let pay_cost_guard = pay_cost
        .first()
        .filter(|_| {
            !crate::dsl_cards::lower_triggered::body_first_step_is_declinable(pay_cost.as_ref())
        })
        .and_then(crate::dsl_cards::lower_triggered::first_step_candidate_guard);
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
        // Do not offer when the interactive cost has no eligible target.
        if let Some(guard) = &pay_cost_guard {
            if !guard(rctx) {
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
    let pay_cost_self_suspend = pay_cost_is_self_suspend(&pay_cost);
    if let Some(demands) = parse_pay_cost_probe(pay_cost.as_ref()) {
        builder = builder.pay_cost_probe(move |rctx| run_pay_cost_probe(&demands, rctx));
    }
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
        if pay_cost_self_suspend {
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
    let mut effect = builder.build();
    // Data twin of the suspend-self pay_cost: lets the digivolve-path optional
    // reducer prompt ask "is this cost payable?" WITHOUT paying it, so an
    // already-suspended (or CannotSuspend) Tamer is never offered (DCGO
    // `CanActivateSuspendCostEffect` gates CanActivate). Only the data field is
    // set — no `activation_cost_fn` — so no triggered-effect path runs it.
    // `G-COST-REDUCTION-OPTIONAL-SYNC-PAY-COST-DIGIVOLVE` (P-200).
    if pay_cost_self_suspend {
        effect.activation_cost_kind = Some(crate::effect::ActivationCostKind::SuspendSelf);
    }
    effect
}

// ─── Read-only pay_cost payability probe ─────────────────────────────────────
//
// `G-ENGINE-PLAY-MASK-IGNORES-WHEN-PLAYING-REDUCTION` follow-ups (1)/(2): the
// action mask and the Option mode check must count a PAID reducer's reduction
// exactly when the player could pay its cost right now. The probe is derived
// from the same compiled `pay_cost` steps the `pay_cost_fn` runs, and mirrors
// how that run credits the reduction:
//
// - an INTERACTIVE first step credits as soon as it parks
//   (`apply_cost_reduction_candidate(.., true)`), and it parks iff it has
//   candidates — so the demand is "≥ `required` candidates exist";
// - a SYNCHRONOUS step credits iff it completes without `cost_unpayable`.
//
// Shapes that are not recognised yield `None` (not provable → not counted).

/// Extra reduction one consumed token credits beyond the reducer's amount.
#[derive(Clone, Copy)]
enum ProbeBonus {
    None,
    /// `preattach_digixros_material { cost_delta }` — `-cost_delta` per card.
    Fixed(i32),
    /// `delete_for_cost_reduction` — the deleted permanent's printed play cost
    /// (DCGO credits nothing for a 0-cost deletion).
    DeletedPlayCost,
}

#[derive(Clone)]
enum ProbeDemand {
    /// "by suspending this Tamer" (synchronous): the source is unsuspended and
    /// may be suspended. Mirrors `suspend_self_as_cost`.
    SuspendSelf,
    /// "by returning this Tamer to the bottom of the deck" (synchronous):
    /// payable while the source is on the field.
    SourceOnField,
    /// "by trashing the bottom face-down card from under any of your Tamers".
    FaceDownUnderTamers { of: CompiledPlayerRef, count: usize },
    /// A battle-area pick (`select_own_permanent` / `select_any_permanent`)
    /// consumed by the next step.
    FieldPick {
        any_side: bool,
        filter: CompiledPredicate,
        required: usize,
        optional_max: usize,
        bonus: ProbeBonus,
    },
    /// `select_under_tamer_sources` (`SelectOwnSources`, no target) feeding a
    /// per-card pre-attach.
    OwnSources {
        filter: CompiledPredicate,
        min: usize,
        max: usize,
        bonus: ProbeBonus,
    },
    /// A hand / trash pick (`select_count_capped_multi`, `select_hand`,
    /// `select_trash`). The card being played is never a candidate: it is the
    /// object of the play, not a payment for it.
    ZonePick {
        of: CompiledPlayerRef,
        hand: bool,
        filter: CompiledPredicate,
        required: usize,
    },
}

fn binding_name(r: &CompiledBindingRef) -> Option<&str> {
    match r {
        CompiledBindingRef::Named(n)
        | CompiledBindingRef::Binding(n)
        | CompiledBindingRef::Permanent(n) => Some(n.as_str()),
        _ => None,
    }
}

fn is_self_ref(r: &CompiledBindingRef) -> bool {
    matches!(r, CompiledBindingRef::Source | CompiledBindingRef::SelfRef)
}

/// What the step right after a field pick does with the picked permanent.
fn field_pick_consumer(next: Option<&CompiledStep>, bound: &str) -> Option<ProbeBonus> {
    let refers = |r: &CompiledBindingRef| binding_name(r) == Some(bound);
    match next? {
        CompiledStep::DeletePermanent { target } | CompiledStep::Suspend { target }
            if refers(target) =>
        {
            Some(ProbeBonus::None)
        }
        CompiledStep::DeleteForCostReduction { target } if refers(target) => {
            Some(ProbeBonus::DeletedPlayCost)
        }
        CompiledStep::PreattachDigixrosMaterial { card, cost_delta } if refers(card) => {
            Some(ProbeBonus::Fixed(-i32::from(*cost_delta)))
        }
        _ => None,
    }
}

/// Parse a cost reducer's `pay_cost` into read-only demands, or `None` when
/// its shape is not provable.
fn parse_pay_cost_probe(steps: &[CompiledStep]) -> Option<Vec<ProbeDemand>> {
    let mut out = Vec::new();
    // Once an interactive step has parked, the reduction is already credited;
    // later steps cannot take it back, so an unrecognised TAIL is harmless.
    let mut parked = false;
    let mut i = 0;
    while i < steps.len() {
        let next = steps.get(i + 1);
        match &steps[i] {
            CompiledStep::Suspend { target } if is_self_ref(target) && !parked => {
                out.push(ProbeDemand::SuspendSelf);
                i += 1;
            }
            CompiledStep::ReturnToDeck { target, .. } if is_self_ref(target) && !parked => {
                out.push(ProbeDemand::SourceOnField);
                i += 1;
            }
            CompiledStep::AllowDigixrosMaterialZone { .. } => i += 1,
            CompiledStep::TrashBottomFaceDownSourceUnderTamer { of, .. } => {
                out.push(ProbeDemand::FaceDownUnderTamers { of: *of, count: 1 });
                parked = true;
                i += 1;
            }
            CompiledStep::TrashBottomFaceDownSourcesUnderTamers { of, count } => {
                out.push(ProbeDemand::FaceDownUnderTamers {
                    of: *of,
                    count: *count as usize,
                });
                parked = true;
                i += 1;
            }
            CompiledStep::SelectOwnPermanent {
                filter,
                bind_as: Some(bound),
                selector: None,
                optional,
                then,
                ..
            }
            | CompiledStep::SelectAnyPermanent {
                filter,
                bind_as: Some(bound),
                selector: None,
                optional,
                then,
                ..
            } if then.is_empty() => {
                let any_side = matches!(steps[i], CompiledStep::SelectAnyPermanent { .. });
                let Some(bonus) = field_pick_consumer(next, bound) else {
                    if parked {
                        break;
                    }
                    return None;
                };
                out.push(ProbeDemand::FieldPick {
                    any_side,
                    filter: filter.clone(),
                    required: usize::from(!*optional),
                    optional_max: usize::from(*optional),
                    bonus,
                });
                parked = true;
                i += 2;
            }
            CompiledStep::SelectOwnSources {
                target: None,
                filter,
                min,
                max,
                bind_as: Some(bound),
                then,
                ..
            } if then.is_empty() => {
                let bonus = match next {
                    Some(CompiledStep::PerSelected {
                        selection,
                        bind_as: each,
                        body,
                    }) if selection == bound => match body.as_slice() {
                        [CompiledStep::PreattachDigixrosMaterial { card, cost_delta }]
                            if binding_name(card) == Some(each.as_str()) =>
                        {
                            ProbeBonus::Fixed(-i32::from(*cost_delta))
                        }
                        _ => return None,
                    },
                    _ => return None,
                };
                out.push(ProbeDemand::OwnSources {
                    filter: filter.clone(),
                    min: *min as usize,
                    max: *max as usize,
                    bonus,
                });
                parked = true;
                i += 2;
            }
            CompiledStep::SelectCountCappedMulti {
                of,
                zone: zone @ (CompiledZone::Hand | CompiledZone::Trash),
                min,
                clamp_to_available: false,
                optional_zero: false,
                distinct_by: None,
                filter,
                ..
            } if *min > 0 => {
                out.push(ProbeDemand::ZonePick {
                    of: *of,
                    hand: matches!(zone, CompiledZone::Hand),
                    filter: filter.clone(),
                    required: *min as usize,
                });
                parked = true;
                i += 1;
            }
            CompiledStep::SelectHand {
                of,
                filter,
                optional: false,
                then,
                ..
            }
            | CompiledStep::SelectTrash {
                of,
                filter,
                optional: false,
                then,
                ..
            } if then.is_empty() => {
                out.push(ProbeDemand::ZonePick {
                    of: *of,
                    hand: matches!(steps[i], CompiledStep::SelectHand { .. }),
                    filter: filter.clone(),
                    required: 1,
                });
                parked = true;
                i += 1;
            }
            _ if parked => break,
            _ => return None,
        }
    }
    (!out.is_empty()).then_some(out)
}

fn probe_player(rctx: &EffectReadContext<'_>, of: CompiledPlayerRef) -> PlayerId {
    // Mirrors `dsl_cards::step::resolve_player` (`Any` resolves to the
    // controller there too).
    match of {
        CompiledPlayerRef::You | CompiledPlayerRef::Any => rctx.player,
        CompiledPlayerRef::Opponent => rctx.opponent_id(),
        CompiledPlayerRef::Active => rctx.game.turn_player(),
    }
}

fn run_pay_cost_probe(
    demands: &[ProbeDemand],
    rctx: &EffectReadContext<'_>,
) -> Option<PayCostPreview> {
    let game = rctx.game;
    let mut preview = PayCostPreview::default();
    for demand in demands {
        match demand {
            ProbeDemand::SuspendSelf => {
                if !crate::effect::ActivationCostKind::SuspendSelf
                    .is_payable(game, rctx.source_permanent)
                {
                    return None;
                }
            }
            ProbeDemand::SourceOnField => {
                let on_field = rctx.source_permanent.is_some_and(|h| {
                    game.player(h.player)
                        .battle_area
                        .get(h.index as usize)
                        .is_some()
                });
                if !on_field {
                    return None;
                }
            }
            ProbeDemand::FaceDownUnderTamers { of, count } => {
                // Same Tamer filter as the step
                // (`{ kind: tamer, has_face_down_source: true }`).
                let tamer_filter = CompiledPredicate {
                    kind: Some(digimon_dsl::compiled::CompiledCardKind::Tamer),
                    has_face_down_source: Some(true),
                    ..CompiledPredicate::default()
                };
                let player = probe_player(rctx, *of);
                let mut options = Vec::new();
                for (index, perm) in game.player(player).battle_area.iter().enumerate() {
                    let handle = PermanentHandle {
                        player,
                        index: index as u8,
                    };
                    if !eval_predicate(&tamer_filter, rctx, PredicateSubject::Permanent(handle)) {
                        continue;
                    }
                    options.extend(
                        perm.card_sources
                            .iter()
                            .filter(|c| c.face_down)
                            .map(|c| (PayCostToken::Card(c.handle()), 0)),
                    );
                }
                preview.demands.push(PayCostDemand {
                    required: *count,
                    optional_max: 0,
                    options,
                });
            }
            ProbeDemand::FieldPick {
                any_side,
                filter,
                required,
                optional_max,
                bonus,
            } => {
                let players: Vec<PlayerId> = if *any_side {
                    (0..game.players.len() as PlayerId).collect()
                } else {
                    vec![rctx.player]
                };
                let mut options = Vec::new();
                for player in players {
                    for (index, perm) in game.player(player).battle_area.iter().enumerate() {
                        let handle = PermanentHandle {
                            player,
                            index: index as u8,
                        };
                        if !eval_predicate(filter, rctx, PredicateSubject::Permanent(handle)) {
                            continue;
                        }
                        let extra = match bonus {
                            ProbeBonus::None => 0,
                            ProbeBonus::Fixed(n) => *n,
                            ProbeBonus::DeletedPlayCost => {
                                i32::from(perm.top_card().play_cost(&game.card_data))
                            }
                        };
                        options.push((PayCostToken::Permanent(handle), extra));
                    }
                }
                preview.demands.push(PayCostDemand {
                    required: *required,
                    optional_max: *optional_max,
                    options,
                });
            }
            ProbeDemand::OwnSources {
                filter,
                min,
                max,
                bonus,
            } => {
                // Mirrors `source_multi_candidates`: every below-top card of
                // the controller's battle-area permanents, filtered on
                // `PredicateSubject::Source`.
                let extra = match bonus {
                    ProbeBonus::Fixed(n) => *n,
                    _ => 0,
                };
                let player = rctx.player;
                let mut options = Vec::new();
                for (field_index, perm) in game.player(player).battle_area.iter().enumerate() {
                    if perm.card_sources.len() <= 1 {
                        continue;
                    }
                    for source_index in 0..(perm.card_sources.len() - 1) {
                        let card = perm.card_sources[source_index].handle();
                        let source = crate::selection::SourceSelectionRef {
                            permanent: PermanentHandle {
                                player,
                                index: field_index as u8,
                            },
                            field_index: field_index as u8,
                            source_index: source_index as u8,
                            card,
                        };
                        if crate::action::space::encode_source_select(
                            field_index as u16,
                            source_index as u16,
                        )
                        .is_none()
                        {
                            continue;
                        }
                        if eval_predicate(filter, rctx, PredicateSubject::Source(source)) {
                            options.push((PayCostToken::Card(card), extra));
                        }
                    }
                }
                preview.demands.push(PayCostDemand {
                    required: *min,
                    optional_max: max.saturating_sub(*min),
                    options,
                });
            }
            ProbeDemand::ZonePick {
                of,
                hand,
                filter,
                required,
            } => {
                let player = probe_player(rctx, *of);
                let zone = if *hand {
                    &game.player(player).hand
                } else {
                    &game.player(player).trash
                };
                let options = zone
                    .iter()
                    .map(|c| c.handle())
                    .filter(|h| Some(*h) != rctx.cost_target_card)
                    .filter(|h| eval_predicate(filter, rctx, PredicateSubject::Card(*h)))
                    .map(|h| (PayCostToken::Card(h), 0))
                    .collect();
                preview.demands.push(PayCostDemand {
                    required: *required,
                    optional_max: 0,
                    options,
                });
            }
        }
    }
    Some(preview)
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
