//! Lower `CompiledClause::Triggered` — emits one engine `Effect` per
//! entry in `clause.when` that maps to a real `EffectTiming`.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledActivationCostKind, CompiledCardKind, CompiledPlayerRef, CompiledPredicate,
    CompiledBindingRef, CompiledScope, CompiledStep, CompiledTriggeredClause,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::dsl_cards::timing_map::compiled_timing_to_engine;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::EffectTiming;
use crate::resource_flow::identifier_indicates_resource_flow;

pub fn lower(card: CardHandle, clause: &CompiledTriggeredClause) -> Vec<Effect> {
    lower_with_raw(card, clause, Arc::new(EngineRawRustRegistry::new()))
}

pub fn lower_with_raw(
    card: CardHandle,
    clause: &CompiledTriggeredClause,
    raw: Arc<EngineRawRustRegistry>,
) -> Vec<Effect> {
    lower_with_raw_and_option_use_requirement(card, clause, raw, None)
}

pub fn lower_with_raw_and_option_use_requirement(
    card: CardHandle,
    clause: &CompiledTriggeredClause,
    raw: Arc<EngineRawRustRegistry>,
    option_use_requirement: Option<Arc<CompiledPredicate>>,
) -> Vec<Effect> {
    lower_with_raw_and_option_use_requirement_for_kind(
        card,
        clause,
        raw,
        option_use_requirement,
        None,
    )
}

pub fn lower_with_raw_and_option_use_requirement_for_kind(
    card: CardHandle,
    clause: &CompiledTriggeredClause,
    raw: Arc<EngineRawRustRegistry>,
    option_use_requirement: Option<Arc<CompiledPredicate>>,
    card_kind: Option<CompiledCardKind>,
) -> Vec<Effect> {
    lower_for_kind_with_clause_index(card, clause, raw, option_use_requirement, card_kind, 0)
}

/// Lower a triggered clause, supplying its index among the card's clauses.
/// `clause_index` seeds the `shared_opt_group` for a once-per-turn clause
/// that lowers to MULTIPLE effects (one per `when:` timing) so all of those
/// effects share a single OPT lockout per turn — DCGO's `SharedHashString`.
/// See `G-OPT-MULTI-TIMING-SHARED-LOCKOUT`.
pub fn lower_for_kind_with_clause_index(
    card: CardHandle,
    clause: &CompiledTriggeredClause,
    raw: Arc<EngineRawRustRegistry>,
    option_use_requirement: Option<Arc<CompiledPredicate>>,
    card_kind: Option<CompiledCardKind>,
    clause_index: usize,
) -> Vec<Effect> {
    let mut out = Vec::new();
    // Count the timings that map to a real engine timing. When a clause is
    // once-per-turn AND spans more than one real timing, every lowered effect
    // must share a single OPT counter rather than each timing keeping its own.
    let real_timing_count = clause
        .when
        .iter()
        .filter(|t| compiled_timing_to_engine(**t).is_some())
        .count();
    let has_opt_cap =
        clause.once_per_turn || matches!(clause.max_per_turn, Some(n) if n > 0);
    // A clause containing a `refund_opt` step needs its OPT key to be
    // STATICALLY known (the step un-records the activation at runtime via the
    // StepRuntime — G-OPT-REFUND-ON-DECLINE). Forcing such a clause into its
    // own shared-opt group makes the key `0x80 | clause_index` instead of the
    // effect-slot index, which the lowering cannot know here.
    let has_refund_opt = steps_contain_refund_opt(&clause.process);
    let needs_shared_opt = has_opt_cap && (real_timing_count > 1 || has_refund_opt);
    // Disjoint keyspace: real effect slots are vec indices (0..N). A shared
    // group key sets the high bit so it can never collide with a slot index.
    let shared_opt_group: Option<u8> = if needs_shared_opt {
        Some(0x80u8 | (clause_index as u8 & 0x7f))
    } else {
        None
    };
    for t in &clause.when {
        let Some(mut engine_timing) = compiled_timing_to_engine(*t) else {
            continue;
        };
        // DigiLink Shape-B: `when: when_linked` lowers to `OnLink` + `.linked()`
        // + a self-filter (`event_card == source_card`) so a linked card's
        // "when this Digimon gets linked" effect fires once for itself and not
        // when a sibling links to the same host (design D6).
        let is_when_linked =
            matches!(*t, digimon_dsl::compiled::CompiledTiming::WhenLinked);
        // DigiLink host-side: `when: when_card_linked_to_this` lowers to
        // `OnLink` + a HOST self-filter (`event_permanent == source_permanent`)
        // so the host's "[When Linked]" fires once for the host the card
        // actually attached to and not for a sibling host (facet #6/#11).
        let is_host_linked = matches!(
            *t,
            digimon_dsl::compiled::CompiledTiming::WhenCardLinkedToThis
        );
        // DigiLink host-side pre-link REPLACEMENT: `when: when_would_link_to_this`
        // lowers to a `WhenWouldLink` replacement effect + a HOST self-filter
        // (`pending_link_host() == source_permanent`). Its body runs through
        // `.replacement_process(...)` (not `.process(...)`), and the optional
        // accept/decline is the replacement framework's own prompt — so it must
        // NOT also install the outer-optional triggered prompt (Gap 5).
        let is_would_link_to_this = matches!(
            *t,
            digimon_dsl::compiled::CompiledTiming::WhenWouldLinkToThis
        );
        if matches!(
            card_kind,
            Some(CompiledCardKind::Option | CompiledCardKind::Dual)
        ) && engine_timing == EffectTiming::MainFromHand
        {
            engine_timing = EffectTiming::OptionMain;
        }

        // Phase 2 Track B — lift a leading `CompiledStep::ActivationCost`
        // out of the body and bind it onto `EffectBuilder::activation_cost(...)`.
        // The compile-side validator guarantees the step only appears as
        // the first body step of a triggered clause; mid-body uses are
        // rejected. Remaining steps form the process body.
        let (activation_cost_kind, body_steps): (
            Option<CompiledActivationCostKind>,
            Vec<CompiledStep>,
        ) = match clause.process.split_first() {
            Some((CompiledStep::ActivationCost { kind }, rest)) => (Some(*kind), rest.to_vec()),
            _ => (None, clause.process.clone()),
        };
        // G-OUTER-OPTIONAL-NOT-INSTALLED: decide whether a "you may" clause
        // needs an explicit outer accept/decline prompt — computed before
        // `body_steps` is moved into the `Arc` below.
        // `outer_prompt: true` FORCES the confirm even when the first body
        // step is declinable — DCGO's always-shown initial Yes/No
        // (G-OPT-REFUND-ON-DECLINE: declining the confirm drops the queued
        // effect before its OPT is recorded, i.e. DCGO `RemoveUse`).
        let needs_outer_optional = clause.optional
            && (clause.outer_prompt || !body_first_step_is_declinable(&body_steps));
        // Guard predicate: when the body's first step is a selection, the
        // outer prompt only installs if it has >=1 candidate. `None` ⇒ no
        // guard (always prompt). A FORCED outer prompt (`outer_prompt: true`)
        // takes no guard — DCGO shows the bool prompt regardless of targets
        // (the body may still have target-free legs, e.g. a self-unsuspend).
        let outer_optional_first_step = if needs_outer_optional && !clause.outer_prompt {
            body_steps.first().cloned()
        } else {
            None
        };
        // Captured before `body_steps` is moved — used to gate [Main]/activated
        // abilities on first-step actionability even when the clause isn't
        // "optional" (activating IS the player's opt-in).
        let activation_first_step = body_steps.first().cloned();
        let process_steps = Arc::new(body_steps);
        let active_when = clause.active_when.clone().map(Arc::new);
        let condition = clause.condition.clone().map(Arc::new);
        let scope = clause.scope;
        let optional = clause.optional;
        let once_per_turn = clause.once_per_turn;
        let max_per_turn = clause.max_per_turn;
        let summary = clause.summary.clone();
        let raw_for_process = raw.clone();

        let mut builder = new_builder(card, engine_timing);
        if matches!(scope, CompiledScope::Inherited) {
            builder = builder.inherited();
        }
        if matches!(scope, CompiledScope::Linked) || is_when_linked {
            builder = builder.linked();
        }
        if matches!(scope, CompiledScope::Security) {
            builder = builder.security_zone();
        }
        if let Some(s) = summary {
            builder = builder.name(&s);
        }
        if once_per_turn {
            builder = builder.once_per_turn();
        } else if let Some(n) = max_per_turn {
            // EffectBuilder only exposes once_per_turn today; n > 1 can't be
            // expressed. Mark once_per_turn for n == 1 so the common case
            // works; higher caps are a no-op in Phase 2a.
            if n == 1 {
                builder = builder.once_per_turn();
            }
        }
        // Multi-timing OPT cluster: all timings lowered from this clause
        // share one once-per-turn counter (G-OPT-MULTI-TIMING-SHARED-LOCKOUT).
        if let Some(group) = shared_opt_group {
            builder = builder.shared_opt_group(group);
        }
        if optional {
            builder = builder.optional();
            // G-OUTER-OPTIONAL-NOT-INSTALLED: a "you may" clause whose body's
            // first step is NOT itself a declinable (PASS-able) selection
            // needs an explicit outer accept/decline prompt — otherwise the
            // body would force a mandatory action the printed "you may"
            // forbids. When the first step IS an optional selection, the
            // inner PASS already provides the decline path (no double prompt).
            //
            // A `when_would_link_to_this` replacement is EXEMPT: the replacement
            // framework installs its own accept/decline prompt for an optional
            // `WhenWouldLink` effect, so an outer-optional prompt would
            // double-prompt and never reach the replacement dispatch (Gap 5).
            if needs_outer_optional && !is_would_link_to_this {
                builder = builder.needs_outer_optional_prompt();
                // Suppress the prompt when the body's first selection step
                // has zero candidates — DCGO does not prompt for an optional
                // ability with no legal target.
                if let Some(guard) = outer_optional_first_step
                    .as_ref()
                    .and_then(first_step_candidate_guard)
                {
                    builder = builder.outer_optional_guard(guard);
                }
            }
        }
        // [Main] activated field abilities: the action mask (`field_main_match`)
        // and the effect-queue both consult `outer_optional_guard` to suppress
        // an activation whose leading selection/cost has no candidate — e.g.
        // <Digi-Burst N> with too few sources to trash. Install it from the
        // first body step for `MainOnField` even when the clause isn't
        // "optional" (activating the [Main] IS the opt-in). `!optional` avoids
        // double-installing over the optional path above.
        if !optional && matches!(engine_timing, EffectTiming::MainOnField) {
            if let Some(guard) = digi_burst_main_activation_guard(activation_first_step.as_ref()) {
                builder = builder.outer_optional_guard(guard);
            }
        }
        if steps_provide_resource_flow(&clause.process) {
            builder = builder.resource_flow();
        }
        if matches!(
            engine_timing,
            EffectTiming::MainFromHand | EffectTiming::OptionMain
        ) {
            if let Some(use_req) = option_use_requirement.clone() {
                builder = builder.option_color_requirement_bypass_condition(move |rctx| {
                    eval_predicate(use_req.as_ref(), rctx, PredicateSubject::None)
                });
            }
        }

        if active_when.is_some()
            || condition.is_some()
            || is_when_linked
            || is_host_linked
            || is_would_link_to_this
        {
            let aw = active_when.clone();
            let cc = condition.clone();
            builder = builder.condition(move |rctx| {
                // DigiLink Shape-B self-filter: a `when_linked` effect fires
                // only when THIS card is the just-linked card.
                if is_when_linked && rctx.event_card() != Some(rctx.source_card) {
                    return false;
                }
                // DigiLink host-side self-filter: a `when_card_linked_to_this`
                // effect fires only when the receiving host is THIS permanent.
                if is_host_linked && rctx.event_permanent() != rctx.source_permanent {
                    return false;
                }
                // DigiLink host-side pre-link self-filter: a
                // `when_would_link_to_this` replacement fires only while the
                // linking card is attaching to THIS permanent (Gap 5).
                if is_would_link_to_this && rctx.pending_link_host() != rctx.source_permanent {
                    return false;
                }
                let subject = predicate_subject_for_source(rctx);
                if let Some(p) = &aw {
                    if !eval_predicate(p, rctx, subject) {
                        return false;
                    }
                }
                if let Some(p) = &cc {
                    if !eval_predicate(p, rctx, subject) {
                        return false;
                    }
                }
                true
            });
        }

        if is_would_link_to_this {
            // Gap 5 — the host-side pre-link reducer body runs inside the
            // `WhenWouldLink` REPLACEMENT process, not the normal triggered
            // process. The replacement framework installs the optional
            // accept/decline prompt; on accept it invokes this closure with a
            // `ReplacementContext` whose `effect` is the mutable
            // `EffectContext`. The body (a `reduce_link_cost` step) mutates the
            // in-flight `pending_digimon_link.cost` and leaves the outcome
            // `None` so the link still resolves at the reduced cost.
            builder = builder.replacement_process(move |rctx| {
                let mut bindings = Bindings::new();
                let runtime = StepRuntime::new(raw_for_process.clone())
                    .with_dna_origin(rctx.effect.game.current_dna_origin);
                run_steps_with_runtime(
                    process_steps.as_slice(),
                    rctx.effect,
                    &mut bindings,
                    &runtime,
                );
            });
        } else {
            builder = builder.process(move |ctx| {
                let mut bindings = Bindings::new();
                // Phase 2b: `run_steps` drives the slice and yields control to
                // a selection step if one is encountered — installing the tail
                // as the selection's resolve callback so it continues once the
                // player picks a target.
                let runtime = StepRuntime::new(raw_for_process.clone())
                    .with_dna_origin(ctx.game.current_dna_origin)
                    .with_opt_key(shared_opt_group);
                run_steps_with_runtime(process_steps.as_slice(), ctx, &mut bindings, &runtime);
            });
        }

        // Phase 2 Track B — wire the lifted activation-cost kind onto the
        // builder so `effect_queue::run_queued_effect_inner` consults the
        // closure between the condition gate and the body process.
        if let Some(kind) = activation_cost_kind {
            builder = builder.activation_cost(move |ctx| match kind {
                CompiledActivationCostKind::SuspendSelf => ctx.suspend_self_as_cost(),
                CompiledActivationCostKind::ReturnSelfToDeckBottom => {
                    ctx.return_self_to_deck_bottom_as_cost()
                }
                CompiledActivationCostKind::TrashSelf => ctx.trash_self_as_cost(),
            });
        }

        out.push(builder.build());
    }
    out
}

fn predicate_subject_for_source(
    rctx: &crate::effect_context::EffectReadContext<'_>,
) -> PredicateSubject {
    let Some(handle) = rctx.source_permanent else {
        return PredicateSubject::None;
    };
    // **Post-batched-deletion (2026-05-23).** Under the DCGO-modeled
    // batched deletion flow, OnDeletion handlers fire AFTER the carrier
    // has been trashed. `rctx.source_permanent` still points at the
    // pre-trash handle, but `battle_area[handle.index]` is now empty.
    // Permanent-subject predicates would hit `eval_permanent_fields`'s
    // "slot empty → return false" early-exit and the condition would
    // spuriously fail.
    //
    // When the trigger context carries a `deleted_object` snapshot
    // (indicating a post-trash OnDeletion fire) AND the source slot is
    // empty, fall back to `None` so subject-agnostic checks (like
    // `count_gte` on hand) still evaluate correctly. Subject-dependent
    // checks (level_eq, dp_lte, etc.) silently no-match under `None`,
    // matching DCGO behavior for "self is no longer the live card."
    let source_gone = handle.index as usize
        >= rctx.game.players[handle.player as usize].battle_area.len()
        || rctx.game.players[handle.player as usize]
            .battle_area
            .get(handle.index as usize)
            .is_none();
    let post_deletion = rctx
        .game
        .current_trigger_context
        .as_ref()
        .and_then(|t| t.deleted_object.as_ref())
        .is_some();
    if source_gone && post_deletion {
        return PredicateSubject::None;
    }
    PredicateSubject::Permanent(handle)
}

fn steps_provide_resource_flow(steps: &[CompiledStep]) -> bool {
    steps.iter().any(step_provides_resource_flow)
}

/// Recursive scan for a `refund_opt` step anywhere in a clause body
/// (including `if:` / `optional:` / `for_each:` nests). A refund-bearing
/// once-per-turn clause is forced into its own shared-opt group so the
/// OPT key is statically known to the StepRuntime. G-OPT-REFUND-ON-DECLINE.
fn steps_contain_refund_opt(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::RefundOpt => true,
        CompiledStep::Optional(body) => steps_contain_refund_opt(body),
        CompiledStep::If {
            then, else_branch, ..
        } => steps_contain_refund_opt(then) || steps_contain_refund_opt(else_branch),
        CompiledStep::ForEach { body, .. } => steps_contain_refund_opt(body),
        _ => false,
    })
}

/// True when the body's first step already exposes a declinable (PASS-able)
/// choice to the player, so a `optional: true` clause does NOT also need an
/// outer accept/decline prompt. A step-level `optional:` selection, an
/// allow-zero multi-select, an explicit `optional`/`min_picks: 0` budget /
/// attack step, or a `CompiledStep::Optional(...)` wrapper is itself the
/// inner decline path. Only a clause whose first step is unconditionally
/// mandatory needs the outer prompt. `G-OUTER-OPTIONAL-NOT-INSTALLED`.
pub(crate) fn body_first_step_is_declinable(body: &[CompiledStep]) -> bool {
    let Some(first) = body.first() else {
        // Empty body — nothing to run; an outer prompt would be pointless.
        return true;
    };
    match first {
        // Explicit step-level optional wrapper — the player may skip it.
        CompiledStep::Optional(_) => true,
        // Single-pick selection steps carry their own `optional` flag; when
        // set, the player can PASS the prompt — itself the decline path.
        CompiledStep::SelectHand { optional, .. }
        | CompiledStep::SelectTrash { optional, .. }
        | CompiledStep::SelectReveal { optional, .. }
        | CompiledStep::SelectSecurity { optional, .. }
        | CompiledStep::SelectOwnPermanent { optional, .. }
        | CompiledStep::SelectOpponentPermanent { optional, .. }
        | CompiledStep::SelectAnyPermanent { optional, .. }
        | CompiledStep::SelectMaterial { optional, .. }
        | CompiledStep::SelectUnionZone { optional, .. }
        | CompiledStep::SelectDnaPair { optional, .. }
        | CompiledStep::LinkToOwnDigimon { optional, .. }
        | CompiledStep::MayAttackNow { optional, .. }
        | CompiledStep::RedirectAttackTarget { optional, .. }
        | CompiledStep::RefireEffect { optional, .. }
        // `link_card_to_self` with `optional: true` installs a PASS-able
        // card-pick selection on its own — the inner PASS is the decline
        // path, so no outer accept/decline prompt is needed.
        // `G-OUTER-OPTIONAL-NOT-INSTALLED`
        | CompiledStep::LinkCardToSelf { optional, .. }
        // `app_fuse` with `optional: true` likewise installs PASS-able
        // permanent/card selections itself (effect-initiated App Fuse).
        | CompiledStep::AppFuse { optional, .. } => *optional,
        // Multi-pick selections are declinable when their minimum is zero
        // (the player may pick nothing — PASS at `picked >= min`).
        CompiledStep::SelectCountCappedMulti {
            min, optional_zero, ..
        } => *min == 0 || *optional_zero,
        CompiledStep::SelectOwnSources { min, .. } => *min == 0,
        // `link_cards` with `up_to` exposes PASS on its first card pick, so the
        // clause's first decision point is already declinable; `exactly` is a
        // mandatory link, so it needs the outer accept/decline prompt.
        CompiledStep::LinkCards { count, .. } => {
            matches!(count, digimon_dsl::compiled::CompiledLinkCount::UpTo(_))
        }
        CompiledStep::SelectOpponentDpBudget { min_picks, .. }
        | CompiledStep::SelectOpponentPlayCostBudget { min_picks, .. } => *min_picks == 0,
        // Any other leading step is mandatory work — the printed "you may"
        // can only be honored via an outer accept/decline prompt.
        _ => false,
    }
}

/// Resolve a `CompiledPlayerRef` against a read context to concrete player
/// ids whose zone the predicate scans.
fn players_for_compiled_ref(
    of: CompiledPlayerRef,
    rctx: &crate::effect_context::EffectReadContext<'_>,
) -> Vec<crate::enums::PlayerId> {
    match of {
        CompiledPlayerRef::You => vec![rctx.player],
        CompiledPlayerRef::Opponent => vec![rctx.opponent_id()],
        CompiledPlayerRef::Active => vec![rctx.game.turn_player()],
        CompiledPlayerRef::Any => (0..rctx.game.players.len() as crate::enums::PlayerId).collect(),
    }
}

/// Build a guard closure for the outer optional prompt: it returns `true`
/// iff the body's first selection step would have at least one candidate.
/// Returns `None` for step kinds whose actionability cannot be cheaply
/// pre-evaluated (the outer prompt then always installs). Only the common
/// single-pick selection steps are covered — they are the ones that drive
/// the `G-OUTER-OPTIONAL-NOT-INSTALLED` failure mode (a useless prompt for
/// a body that would silently no-op).
/// For a [Main]-activated field ability whose leading step is a ＜Digi-Burst N＞
/// self-cost (`SelectOwnSources` over THIS Digimon's own sources), return a
/// guard requiring N digivolution cards under the top to trash
/// (`card_sources.len() > N`). Used ONLY in the `MainOnField` branch to keep the
/// action mask from offering a [Main] activation that can't pay its cost —
/// execution would refuse it, a no-op the deterministic policy loops on to the
/// step limit. Returns None for any other first step (no [Main]-side gate). This
/// is deliberately separate from `first_step_candidate_guard` (which serves the
/// optional-TRIGGERED path, where the same step instead silently skips).
fn digi_burst_main_activation_guard(
    step: Option<&CompiledStep>,
) -> Option<Box<dyn Fn(&crate::effect_context::EffectReadContext<'_>) -> bool + Send + Sync>> {
    if let Some(CompiledStep::SelectOwnSources {
        target: Some(CompiledBindingRef::Source),
        min,
        ..
    }) = step
    {
        let need = *min as usize;
        Some(Box::new(move |rctx| {
            rctx.source_permanent
                .and_then(|h| rctx.game.player(h.player).battle_area.get(h.index as usize))
                .is_some_and(|perm| perm.card_sources.len() > need)
        }))
    } else {
        None
    }
}

fn first_step_candidate_guard(
    step: &CompiledStep,
) -> Option<Box<dyn Fn(&crate::effect_context::EffectReadContext<'_>) -> bool + Send + Sync>> {
    match step {
        CompiledStep::SelectHand { of, filter, .. } => {
            let of = *of;
            let filter = Arc::new(filter.clone());
            Some(Box::new(move |rctx| {
                players_for_compiled_ref(of, rctx).into_iter().any(|p| {
                    rctx.game.player(p).hand.iter().any(|card| {
                        eval_predicate(&filter, rctx, PredicateSubject::Card(card.handle()))
                    })
                })
            }))
        }
        CompiledStep::SelectTrash { of, filter, .. } => {
            let of = *of;
            let filter = Arc::new(filter.clone());
            Some(Box::new(move |rctx| {
                players_for_compiled_ref(of, rctx).into_iter().any(|p| {
                    rctx.game.player(p).trash.iter().any(|card| {
                        eval_predicate(&filter, rctx, PredicateSubject::Card(card.handle()))
                    })
                })
            }))
        }
        CompiledStep::SelectOwnPermanent { filter, .. } => {
            let filter = Arc::new(filter.clone());
            Some(Box::new(move |rctx| {
                permanent_zone_has_match(rctx, rctx.player, &filter)
            }))
        }
        CompiledStep::SelectOpponentPermanent { filter, .. } => {
            let filter = Arc::new(filter.clone());
            Some(Box::new(move |rctx| {
                let opp = rctx.opponent_id();
                permanent_zone_has_match(rctx, opp, &filter)
            }))
        }
        CompiledStep::SelectAnyPermanent { filter, .. } => {
            let filter = Arc::new(filter.clone());
            Some(Box::new(move |rctx| {
                (0..rctx.game.players.len() as crate::enums::PlayerId)
                    .any(|p| permanent_zone_has_match(rctx, p, &filter))
            }))
        }
        // NOTE: `SelectOwnSources` is intentionally NOT handled here. A
        // "by trashing N sources" cost lowers to it, but for an OPTIONAL
        // TRIGGERED ability (e.g. EX10-034's [All Turns] when-attacking) the
        // designed behavior is to fire and SILENTLY SKIP the trash when fewer
        // than N sources exist — it triggers on an event, so it can't loop.
        // The [Main]-activated digi_burst case (BlitzGreymon) IS gated, but
        // only in the `MainOnField` branch of `lower` (mask-offered, so it
        // would loop) — see `digi_burst_main_activation_guard`.
        //
        // Other first-step kinds (suspend, select_effect_choice, budget
        // selects, etc.) are not pre-evaluated — the outer prompt installs
        // unconditionally for them.
        _ => None,
    }
}

/// True when `player`'s battle area holds a permanent satisfying `filter`.
fn permanent_zone_has_match(
    rctx: &crate::effect_context::EffectReadContext<'_>,
    player: crate::enums::PlayerId,
    filter: &CompiledPredicate,
) -> bool {
    let count = rctx.game.player(player).battle_area.len();
    (0..count).any(|i| {
        let handle = crate::permanent::PermanentHandle {
            player,
            index: i as u8,
        };
        eval_predicate(filter, rctx, PredicateSubject::Permanent(handle))
    })
}

fn step_provides_resource_flow(step: &CompiledStep) -> bool {
    match step {
        CompiledStep::Draw { .. }
        | CompiledStep::AddToHandFromDeck { .. }
        | CompiledStep::AddToHandFromTrash { .. }
        | CompiledStep::AddToHandFromReveal { .. } => true,
        CompiledStep::RawRust { fn_name, .. } => identifier_indicates_resource_flow(fn_name),
        CompiledStep::AsSelectingPlayer { body, .. }
        | CompiledStep::ForEach { body, .. }
        | CompiledStep::PerSelected { body, .. }
        | CompiledStep::ScheduleDelayed { body, .. }
        | CompiledStep::Optional(body) => steps_provide_resource_flow(body),
        CompiledStep::If {
            then, else_branch, ..
        } => steps_provide_resource_flow(then) || steps_provide_resource_flow(else_branch),
        _ => false,
    }
}

fn new_builder(card: CardHandle, timing: EffectTiming) -> EffectBuilder {
    match timing {
        EffectTiming::OnPlay => Effect::on_play(card),
        EffectTiming::WhenDigivolving => Effect::when_digivolving(card),
        EffectTiming::OnAttack => Effect::on_attack(card),
        EffectTiming::OnDeletion => Effect::on_deletion(card),
        EffectTiming::SecuritySkill => Effect::security(card),
        EffectTiming::BeforePayCost => Effect::before_pay_cost(card),
        EffectTiming::BeforePayCostObserve => Effect::before_pay_cost_observe(card),
        EffectTiming::OptionMain => Effect::on_play(card).option_main(),
        EffectTiming::WhenAttacking => Effect::when_attacking(card),
        EffectTiming::EndOfAttack => Effect::end_of_attack(card),
        EffectTiming::EndOfBattle => Effect::end_of_battle(card),
        EffectTiming::StartOfYourTurn => Effect::start_of_your_turn(card),
        EffectTiming::StartOfOpponentsTurn => Effect::start_of_opponents_turn(card),
        EffectTiming::StartOfYourMainPhase => Effect::start_of_your_main_phase(card),
        EffectTiming::EndOfYourTurn => Effect::end_of_your_turn(card),
        EffectTiming::EndOfOpponentsTurn => Effect::end_of_opponents_turn(card),
        EffectTiming::OnEnterFieldAnyone => Effect::on_enter_field_anyone(card),
        EffectTiming::OnAllyPlayed => Effect::on_ally_played(card),
        EffectTiming::OnAnyDeletion => Effect::on_any_deletion(card),
        EffectTiming::OnDigivolve => Effect::on_digivolve(card),
        EffectTiming::OnSuspend => Effect::on_suspend(card),
        EffectTiming::OnUnsuspend => Effect::on_unsuspend(card),
        EffectTiming::OnAttackTargetChange => Effect::on_attack_target_change(card),
        EffectTiming::OnBlock => Effect::on_block(card),
        EffectTiming::OnAllyAttack => Effect::on_ally_attack(card),
        EffectTiming::OnOpponentAttack => Effect::on_opponent_attack(card),
        EffectTiming::OnHatch => Effect::on_hatch(card),
        EffectTiming::OnOpponentSecurityRemoved => Effect::on_opponent_security_removed(card),
        EffectTiming::OnOwnSecurityRemoved => Effect::on_own_security_removed(card),
        EffectTiming::OnDigivolutionCardTrashed => Effect::on_digivolution_card_trashed(card),
        EffectTiming::OnSecurityCheck => Effect::on_security_check(card),
        EffectTiming::OnCheckFaceUpSecurity => {
            EffectBuilder::new(card, EffectTiming::OnCheckFaceUpSecurity)
        }
        EffectTiming::OnLoseSecurity => Effect::on_lose_security(card),
        EffectTiming::OnDiscardSecurity => Effect::on_discard_security(card),
        EffectTiming::OnPlaceSecurity => Effect::on_place_security(card),
        other => EffectBuilder::new(card, other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use digimon_dsl::compiled::CompiledPlayerRef;

    #[test]
    fn resource_flow_catalog_detects_nested_draw_steps() {
        let steps = vec![CompiledStep::Optional(vec![CompiledStep::Draw {
            of: CompiledPlayerRef::You,
            count: 1,
        }])];

        assert!(steps_provide_resource_flow(&steps));
    }

    #[test]
    fn resource_flow_catalog_ignores_non_hand_resource_steps() {
        let steps = vec![CompiledStep::GainMemory(1)];

        assert!(!steps_provide_resource_flow(&steps));
    }
}
