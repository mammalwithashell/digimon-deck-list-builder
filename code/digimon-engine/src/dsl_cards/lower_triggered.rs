//! Lower `CompiledClause::Triggered` — emits one engine `Effect` per
//! entry in `clause.when` that maps to a real `EffectTiming`.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledActivationCostKind, CompiledBindingRef, CompiledCardKind, CompiledPlayerRef,
    CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming, CompiledTriggeredClause,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::dsl_cards::timing_map::compiled_timing_to_engine;
use crate::effect::{ActivationCostKind, Effect, EffectBuilder};
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
    let has_opt_cap = clause.once_per_turn || matches!(clause.max_per_turn, Some(n) if n > 0);
    // Q1818 / CR 4.3 §15-14-1-4: a "you may" [X Per Turn] clause whose "may"
    // is folded into its first pick refunds the use when that pick is passed
    // (the effect was never activated). See `folded_may_refund_process`.
    // G-DSL-FOLDED-MAY-OPT-REFUND-AUDIT.
    let process: Vec<CompiledStep> = folded_may_refund_process(clause)
        .or_else(|| then_may_refund_process(clause))
        .unwrap_or_else(|| clause.process.clone());
    // A clause containing a `refund_opt` step needs its OPT key to be
    // STATICALLY known (the step un-records the activation at runtime via the
    // StepRuntime — G-OPT-REFUND-ON-DECLINE). Forcing such a clause into its
    // own shared-opt group makes the key `0x80 | clause_index` instead of the
    // effect-slot index, which the lowering cannot know here.
    let has_refund_opt = steps_contain_refund_opt(&process);
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
        let is_when_linked = matches!(*t, digimon_dsl::compiled::CompiledTiming::WhenLinked);
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
        ) = match process.split_first() {
            Some((CompiledStep::ActivationCost { kind }, rest)) => (Some(*kind), rest.to_vec()),
            _ => (None, process.clone()),
        };
        // G-OUTER-OPTIONAL-NOT-INSTALLED: decide whether a "you may" clause
        // needs an explicit outer accept/decline prompt — computed before
        // `body_steps` is moved into the `Arc` below.
        // `outer_prompt: true` FORCES the confirm even when the first body
        // step is declinable — DCGO's always-shown initial Yes/No
        // (G-OPT-REFUND-ON-DECLINE: declining the confirm drops the queued
        // effect before its OPT is recorded, i.e. DCGO `RemoveUse`).
        let needs_outer_optional =
            clause.optional && (clause.outer_prompt || !body_first_step_is_declinable(&body_steps));
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
        // Captured before `body_steps` is moved: the payability of the clause's
        // leading optional processing condition (§15-7-4, §15-8-4-4-1). Used
        // for optional clauses and for [Main] field abilities even when the
        // clause isn't "optional" (activating IS the player's opt-in).
        let cost_guard = leading_cost_guard(&body_steps);
        let process_steps = Arc::new(body_steps);
        let active_when = clause.active_when.clone().map(Arc::new);
        let condition = clause.condition.clone().map(Arc::new);
        let trigger_if = clause.trigger_if.clone().map(Arc::new);
        // G-SHARED-OPT-HETEROGENEOUS-TIMING: per-timing condition override for
        // THIS timing, if the clause declares one. AND-ed onto the clause-level
        // gates inside the condition closure so each timing of a shared-OPT
        // multi-timing clause can carry its own gate while sharing the counter.
        let timing_condition = clause
            .timing_conditions
            .iter()
            .find(|tc| tc.when == *t)
            .map(|tc| Arc::new(tc.condition.clone()));
        let scope = clause.scope;
        let opt_key_inherited =
            matches!(scope, CompiledScope::Inherited | CompiledScope::Linked) || is_when_linked;
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
        if matches!(scope, CompiledScope::Trash) {
            builder = builder.trash_zone();
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
        // One printed clause spanning several timings is ONE effect for
        // "activate 1 ... effect" enumerations
        // (G-ENGINE-REFIRE-SPLITS-COMBINED-TIMING).
        if real_timing_count > 1 {
            builder = builder.clause_group(clause_index as u32);
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
        // §15-7-4 / §15-8-4-4-1 (G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED):
        // the payability of the clause's optional processing condition — its
        // leading cost-shaped steps. An optional clause whose condition can't
        // be executed is neither offered nor run (the queue's prompt gates and
        // `run_queued_effect_inner`), and a `[Main]` field ability whose
        // condition can't be executed is not declarable (`field_main_match`;
        // activating the `[Main]` IS the opt-in, so this applies whether or not
        // the clause is marked `optional`). This also carries the former
        // `[Main]` <Digi-Burst N> gate (`CostGuard::OwnSources`).
        if optional || matches!(engine_timing, EffectTiming::MainOnField) {
            if let Some(guard) = cost_guard {
                builder = builder.optional_condition_guard(guard);
            }
        }
        if steps_provide_resource_flow(&process) {
            builder = builder.resource_flow();
        }
        // G-ENGINE-DELAY-OPTION-CONDITIONAL-PLACEMENT: an Option [Main] body
        // that authors its own `place_self_as_delay_option` owns the
        // placement — `dispose_option` trashes the card if the body ended
        // without placing it (e.g. a declined "By trashing …" cost).
        if matches!(
            engine_timing,
            EffectTiming::MainFromHand | EffectTiming::OptionMain
        ) && steps_contain_place_self_as_delay_option(&process)
        {
            builder = builder.explicit_self_placement();
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

        // §13-1-8-2 / Q6813: a security check's observers are captured at
        // the reveal, before the [Security] effect can change the state. Their
        // turn gates and "when THIS Digimon checks" self-gate are trigger
        // qualifiers, decided at that capture together with any authored
        // `trigger_if:` (`Game::discard_entries_not_triggered`); every other
        // clause condition stays a processing-time check (§15-8-3-9).
        // The gate terms are COPIED, not moved: they stay in `condition` too,
        // so the activation-time check reads them twice (the qualifier is
        // ANDed into `condition` by `EffectBuilder::build`). Deliberate and
        // harmless -- the copy exists only for the capture-time decision.
        let security_check_gate = if matches!(
            engine_timing,
            EffectTiming::OnSecurityCheck | EffectTiming::OnCheckFaceUpSecurity
        ) {
            let mut gate = digimon_dsl::compiled::CompiledPredicate::default();
            if let Some(aw) = &active_when {
                gate.your_turn = aw.your_turn;
                gate.opponents_turn = aw.opponents_turn;
            }
            if let Some(cc) = &condition {
                gate.event_target_is_source = cc.event_target_is_source;
            }
            Some(Arc::new(gate))
        } else {
            None
        };

        if active_when.is_some()
            || condition.is_some()
            || timing_condition.is_some()
            || is_when_linked
            || is_host_linked
            || is_would_link_to_this
        {
            let aw = active_when.clone();
            let cc = condition.clone();
            let tc = timing_condition.clone();
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
                // G-SHARED-OPT-HETEROGENEOUS-TIMING: this timing's own gate.
                if let Some(p) = &tc {
                    if !eval_predicate(p, rctx, subject) {
                        return false;
                    }
                }
                true
            });
        }
        // The trigger qualifier: decided when the event triggers the effect
        // (`Game::discard_entries_not_triggered`, §15-8-3-1 / §15-8-3-4);
        // `EffectBuilder::build` folds it into the activation-time
        // `condition` above (§15-4-4-5).
        if trigger_if.is_some() || security_check_gate.is_some() {
            builder = builder.trigger_qualifier(move |rctx| {
                let subject = predicate_subject_for_source(rctx);
                trigger_if
                    .as_ref()
                    .is_none_or(|p| eval_predicate(p, rctx, subject))
                    && security_check_gate
                        .as_ref()
                        .is_none_or(|p| eval_predicate(p, rctx, subject))
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
                    .with_dna_origin(rctx.effect.game.current_dna_origin)
                    .with_source_kind(rctx.effect.source_kind);
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
                    .with_opt_key(shared_opt_group)
                    .with_opt_key_inherited(opt_key_inherited)
                    .with_source_kind(ctx.source_kind);
                run_steps_with_runtime(process_steps.as_slice(), ctx, &mut bindings, &runtime);
            });
        }

        // Phase 2 Track B — wire the lifted activation-cost kind onto the
        // builder so `effect_queue::run_queued_effect_inner` consults the
        // closure between the condition gate and the body process.
        if let Some(kind) = activation_cost_kind {
            // `activation_cost_of_kind` sets the mutating closure AND the
            // read-only `ActivationCostKind` probe together, so the action
            // mask (`main_effect_select::field_main_match`) can suppress a
            // `[Main]` whose cost is unpayable while the resolution path
            // (`Game::activate_field_main`) actually pays it.
            // `G-ENGINE-MAIN-ON-FIELD-ACTIVATION-COST-UNPAID`.
            builder = builder.activation_cost_of_kind(match kind {
                CompiledActivationCostKind::SuspendSelf => ActivationCostKind::SuspendSelf,
                CompiledActivationCostKind::ReturnSelfToDeckBottom => {
                    ActivationCostKind::ReturnSelfToDeckBottom
                }
                CompiledActivationCostKind::TrashSelf => ActivationCostKind::TrashSelf,
                CompiledActivationCostKind::PlaceSelfAtSecurityBottom => {
                    ActivationCostKind::PlaceSelfAtSecurityBottom
                }
            });
        }

        out.push(builder.build());
    }
    out
}

pub(crate) fn predicate_subject_for_source(
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

/// Recursive scan for a `place_self_as_delay_option` step anywhere in a
/// clause body, at any nesting depth. Walks the serde form of the compiled
/// steps (the unit variant serializes as the bare string
/// `"PlaceSelfAsDelayOption"`) so every nesting construct (`if`, `optional`,
/// `for_each`, `per_selected`, selection `then:` tails, …) is covered without
/// a hand-maintained variant list. Runs once per card build (effects cached).
fn steps_contain_place_self_as_delay_option(steps: &[CompiledStep]) -> bool {
    fn walk(v: &serde_json::Value) -> bool {
        match v {
            serde_json::Value::String(s) => s == "PlaceSelfAsDelayOption",
            serde_json::Value::Array(items) => items.iter().any(walk),
            serde_json::Value::Object(map) => {
                map.contains_key("PlaceSelfAsDelayOption") || map.values().any(walk)
            }
            _ => false,
        }
    }
    serde_json::to_value(steps)
        .map(|v| walk(&v))
        .unwrap_or(false)
}

/// The binding of a `[X Per Turn]` clause's folded "you may" pick, when
/// passing that pick means the effect was never activated (official Q&A
/// Q1818, CR 4.3 §15-14-1-4), so the use must not be spent.
///
/// The shape (all must hold):
/// - it is a trigger-type effect (not [Main] / [Counter], whose activation is
///   the player's declaration);
/// - the clause is `optional: true` (its whole effect is a "you may") with no
///   `outer_prompt` (otherwise the outer confirm is the decision, and its
///   decline already drops the effect before the use is recorded);
/// - it has a per-turn cap and authors no `refund_opt` of its own;
/// - its first step is a declinable selection (`optional: true`) that binds
///   its pick (`bind_as`) and is NOT a cost (`cost: true`): declining an
///   optional COST of an activated effect still spends the use (BT24-026 /
///   P-250, owner ruling #20);
/// - every later step depends on that binding, so a pass leaves nothing else
///   of the effect to process: it names the binding, or it is an `if` on
///   `binding_present` of it whose `else` branch depends on it too. A tail
///   that runs without the pick ("You may X. Then, Y": Q1643) is a different
///   ruling -- the effect was activated -- and is left as authored.
pub fn folded_may_refund_binding(clause: &CompiledTriggeredClause) -> Option<String> {
    let capped = clause.once_per_turn || matches!(clause.max_per_turn, Some(n) if n > 0);
    if !capped || !clause.optional || clause.outer_prompt {
        return None;
    }
    // An activation-type effect ([Main], [Counter]) is activated by the
    // player's own declaration (§15-14-1-4 "when a player chooses to activate
    // ... even if the effect's processing can't be executed"): a pick passed
    // inside it is not a decision not to activate.
    if clause.when.iter().any(|t| {
        matches!(
            t,
            CompiledTiming::Main
                | CompiledTiming::MainFromHand
                | CompiledTiming::MainOnField
                | CompiledTiming::MainFromTrash
                | CompiledTiming::Counter
        )
    }) {
        return None;
    }
    if steps_contain_refund_opt(&clause.process) {
        return None;
    }
    let (first, tail) = clause.process.split_first()?;
    let value = serde_json::to_value(first).ok()?;
    let (_, fields) = value.as_object()?.iter().next()?;
    let fields = fields.as_object()?;
    if fields.get("optional").and_then(|v| v.as_bool()) != Some(true) {
        return None;
    }
    if fields.get("cost").and_then(|v| v.as_bool()) == Some(true) {
        return None;
    }
    let binding = fields.get("bind_as")?.as_str()?.to_string();
    tail.iter()
        .all(|step| step_depends_on_binding(step, &binding))
        .then_some(binding)
}

/// The clause body rewritten for the Q1818 refund, or `None` when the clause
/// does not have that shape (`folded_may_refund_binding`):
///
/// ```text
/// <first pick, continuing on PASS>
/// if binding_absent: <pick> then [refund_opt] else [<rest of the body>]
/// ```
///
/// A permanent pick's PASS drops the rest of the clause by default, so the
/// rewritten pick continues on decline (`continue_on_decline`) to reach the
/// refund; the rest of the body runs only when the pick was made, exactly as
/// before (it depends on the pick, so a pass never ran anything of it).
pub fn folded_may_refund_process(clause: &CompiledTriggeredClause) -> Option<Vec<CompiledStep>> {
    let binding = folded_may_refund_binding(clause)?;
    let (first, rest) = clause.process.split_first()?;
    let mut first = first.clone();
    match &mut first {
        CompiledStep::SelectOwnPermanent {
            continue_on_decline,
            ..
        }
        | CompiledStep::SelectOpponentPermanent {
            continue_on_decline,
            ..
        }
        | CompiledStep::SelectAnyPermanent {
            continue_on_decline,
            ..
        } => *continue_on_decline = true,
        _ => {}
    }
    Some(vec![
        first,
        CompiledStep::If {
            condition: CompiledPredicate {
                binding_absent: Some(binding),
                ..CompiledPredicate::default()
            },
            then: vec![CompiledStep::RefundOpt],
            else_branch: rest.to_vec(),
        },
    ])
}

/// Shared preconditions of the folded-"may" refunds: a trigger-type,
/// `optional: true` clause with a per-turn cap, no explicit `outer_prompt`
/// and no authored `refund_opt`.
fn folded_may_preconditions(clause: &CompiledTriggeredClause) -> bool {
    let capped = clause.once_per_turn || matches!(clause.max_per_turn, Some(n) if n > 0);
    capped
        && clause.optional
        && !clause.outer_prompt
        && !steps_contain_refund_opt(&clause.process)
        && !clause.when.iter().any(|t| {
            matches!(
                t,
                CompiledTiming::Main
                    | CompiledTiming::MainFromHand
                    | CompiledTiming::MainOnField
                    | CompiledTiming::MainFromTrash
                    | CompiledTiming::Counter
            )
        })
}

/// The binding of a declinable, non-cost selection that may pick nothing:
/// an `optional: true` pick, or a multi-pick whose minimum is zero.
fn declinable_pick_binding(step: &CompiledStep) -> Option<String> {
    let value = serde_json::to_value(step).ok()?;
    let (variant, fields) = value.as_object()?.iter().next()?;
    if !variant.starts_with("Select") {
        return None;
    }
    let fields = fields.as_object()?;
    if fields.get("cost").and_then(|v| v.as_bool()) == Some(true) {
        return None;
    }
    let is_true = |k: &str| fields.get(k).and_then(|v| v.as_bool()) == Some(true);
    // `min` is a plain count or a `CompiledCountBound` (`{"Literal": 0}`).
    let is_zero = |k: &str| {
        fields.get(k).is_some_and(|v| {
            v.as_u64() == Some(0) || v.get("Literal").and_then(|l| l.as_u64()) == Some(0)
        })
    };
    if !(is_true("optional") || is_true("optional_zero") || is_zero("min") || is_zero("min_picks")) {
        return None;
    }
    fields.get("bind_as")?.as_str().map(str::to_string)
}

/// The bindings of a "[X Per Turn] you may A. Then, you may B." clause: two or
/// more optional halves, each led by a declinable pick ([`declinable_pick_binding`])
/// that names none of the earlier halves' picks, with every other step
/// depending on its own half's pick ([`step_depends_on_binding`]). A half with
/// mandatory content that runs without its pick (Q1643, BT7-079) breaks the
/// shape, so its clause keeps spending the use.
pub fn then_may_refund_bindings(clause: &CompiledTriggeredClause) -> Option<Vec<String>> {
    if !folded_may_preconditions(clause) {
        return None;
    }
    let mut halves: Vec<String> = Vec::new();
    for step in &clause.process {
        let starts_half = declinable_pick_binding(step).filter(|_| {
            !halves.iter().any(|b| step_depends_on_binding(step, b))
        });
        if let Some(binding) = starts_half {
            halves.push(binding);
            continue;
        }
        let current = halves.last()?;
        if !step_depends_on_binding(step, current) {
            return None;
        }
    }
    (halves.len() >= 2).then_some(halves)
}

/// Owner-decided 2026-10-21 (mechanics-gaps W13: "DCGO has the
/// Dianamon/GraceNovamon effect correct"; the PDF is silent; DCGO
/// `BT25_028.cs` `if (!executed) activateClass.RemoveUse()`): a "you may A. Then, you may
/// B." [X Per Turn] effect whose halves were BOTH declined executed nothing,
/// so its use is refunded; it is spent as soon as either half executes. The
/// body gets `if <every half's pick is empty> then refund_opt` appended, and
/// each half's permanent pick continues on PASS so the refund is reached.
pub fn then_may_refund_process(clause: &CompiledTriggeredClause) -> Option<Vec<CompiledStep>> {
    let halves = then_may_refund_bindings(clause)?;
    let mut steps: Vec<CompiledStep> = clause
        .process
        .iter()
        .cloned()
        .map(|mut step| {
            if declinable_pick_binding(&step).is_some_and(|b| halves.contains(&b)) {
                match &mut step {
                    CompiledStep::SelectOwnPermanent {
                        continue_on_decline,
                        ..
                    }
                    | CompiledStep::SelectOpponentPermanent {
                        continue_on_decline,
                        ..
                    }
                    | CompiledStep::SelectAnyPermanent {
                        continue_on_decline,
                        ..
                    } => *continue_on_decline = true,
                    _ => {}
                }
            }
            step
        })
        .collect();
    steps.push(CompiledStep::If {
        condition: CompiledPredicate {
            all_of: halves
                .into_iter()
                .map(|b| CompiledPredicate {
                    binding_count_eq: Some((b, 0)),
                    ..CompiledPredicate::default()
                })
                .collect(),
            ..CompiledPredicate::default()
        },
        then: vec![CompiledStep::RefundOpt],
        else_branch: Vec::new(),
    });
    Some(steps)
}

/// Whether `step` does nothing when `binding` is unbound: an `if` on
/// `binding_present: <binding>` whose `else` branch only holds such steps, or
/// any other step that names the binding (a step acting on an unbound
/// binding is a no-op).
fn step_depends_on_binding(step: &CompiledStep, binding: &str) -> bool {
    fn names(v: &serde_json::Value, binding: &str) -> bool {
        match v {
            serde_json::Value::String(s) => s == binding,
            serde_json::Value::Array(items) => items.iter().any(|i| names(i, binding)),
            serde_json::Value::Object(map) => map.values().any(|i| names(i, binding)),
            _ => false,
        }
    }
    match step {
        CompiledStep::If {
            condition,
            else_branch,
            ..
        } => {
            condition.binding_present.as_deref() == Some(binding)
                && else_branch
                    .iter()
                    .all(|s| step_depends_on_binding(s, binding))
        }
        _ => serde_json::to_value(step)
            .map(|v| names(&v, binding))
            .unwrap_or(false),
    }
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
        CompiledStep::PaySourcePrefix { optional, .. }
        | CompiledStep::SelectHand { optional, .. }
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
        | CompiledStep::MayDnaDigivolveNow { optional, .. }
        | CompiledStep::RedirectAttackTarget { optional, .. }
        | CompiledStep::RefireEffect { optional, .. }
        // `use_option_from_hand` / `use_option_from_trash` with `optional: true`
        // install a PASS-able hand/trash pick (DCGO SelectHandEffect
        // `canNoSelect: true`) and install NOTHING when no Option qualifies, so
        // the inner PASS is the decline path. Without this arm an
        // `optional: true` clause led by one (EX7-073 Clause 1) got an
        // unguarded outer accept/decline that prompted even with zero
        // candidates. `G-OUTER-OPTIONAL-NOT-INSTALLED`
        | CompiledStep::UseOptionFromHand { optional, .. }
        | CompiledStep::UseOptionFromTrash { optional, .. }
        // `app_fuse` with `optional: true` installs PASS-able permanent/card
        // selections itself (effect-initiated App Fuse), so the inner PASS is
        // the decline path — no outer accept/decline prompt is needed.
        // `G-OUTER-OPTIONAL-NOT-INSTALLED`
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

/// True when a cost-reduction `pay_cost` body's FIRST step may install a
/// selection (`pending_selection`) rather than always completing synchronously. Such
/// a reducer cannot be resolved by the synchronous digivolve / Option-use cost
/// scan; the engine routes it through a dedicated interactive prompt instead
/// (`Effect::pay_cost_interactive`). `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
///
/// Conservative: lists the selection-installing steps actually used as cost
/// payments today. The self-suspend / synchronous-trash idioms are NOT here, so
/// they keep their synchronous scan handling.
pub(crate) fn body_first_step_installs_selection(body: &[CompiledStep]) -> bool {
    let Some(first) = body.first() else {
        return false;
    };
    matches!(
        first,
        // Effect evolution can park at a route, reduction payment, or would
        // window even when this clause has no explicit leading selector.
        CompiledStep::EffectInitiatedDigivolve { .. }
            | CompiledStep::PaySourcePrefix { .. }
            | CompiledStep::TrashBottomFaceDownSourceUnderTamer { .. }
            | CompiledStep::TrashBottomFaceDownSourcesUnderTamers { .. }
            | CompiledStep::TrashLinkCardOfOwnDigimon { .. }
            | CompiledStep::SelectHand { .. }
            | CompiledStep::SelectTrash { .. }
            | CompiledStep::SelectReveal { .. }
            | CompiledStep::SelectSecurity { .. }
            | CompiledStep::SelectOwnPermanent { .. }
            | CompiledStep::SelectOpponentPermanent { .. }
            | CompiledStep::SelectAnyPermanent { .. }
            | CompiledStep::SelectMaterial { .. }
            | CompiledStep::SelectOwnSources { .. }
            | CompiledStep::SelectCountCappedMulti { .. }
    )
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
pub(crate) fn first_step_candidate_guard(
    step: &CompiledStep,
) -> Option<Box<dyn Fn(&crate::effect_context::EffectReadContext<'_>) -> bool + Send + Sync>> {
    match step {
        CompiledStep::MayDnaDigivolveNow {
            anchor, partner_filter, target_filter, ignore_requirements, ..
        } => {
            // A leading step has no earlier selection bindings. Do not guess
            // an unresolved named anchor; source/carrier are readable here.
            if !matches!(anchor, CompiledBindingRef::Source | CompiledBindingRef::Carrier) {
                return None;
            }
            let partner_filter = partner_filter.clone();
            let target_filter = target_filter.clone();
            let ignore_requirements = *ignore_requirements;
            Some(Box::new(move |rctx| {
                let Some(anchor) = rctx.source_permanent else { return false; };
                (0..rctx.game.player(rctx.player).battle_area.len()).any(|i| {
                    let partner = crate::permanent::PermanentHandle {
                        player: rctx.player, index: i as u8,
                    };
                    crate::effect_context::dna_partner_has_target(
                        rctx.game, rctx.player, anchor, partner, ignore_requirements,
                        &|_, h| eval_predicate(&partner_filter, rctx, PredicateSubject::Permanent(h)),
                        &|game, i| eval_predicate(
                            &target_filter, rctx,
                            PredicateSubject::Card(game.player(rctx.player).hand[i].handle()),
                        ),
                    )
                })
            }))
        }
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
        // NOTE: cost-shaped first steps (`SelectOwnSources` "by trashing N
        // sources", `trash_top_security`, `suspend`, memory, hand / trash
        // pick runs, face-down cards under Tamers, …) are not handled here:
        // their payability is the optional processing condition's, gated by
        // `leading_cost_guard` (§15-7-4), which also suppresses the run.
        //
        // Other first-step kinds (select_effect_choice, budget selects, etc.)
        // are not pre-evaluated — the outer prompt installs unconditionally
        // for them.
        _ => None,
    }
}

// ─── Optional processing condition payability (§15-7-4, §15-8-4-4-1) ──────

/// The payability of a clause's optional processing condition — the "by X"
/// of "by X, Y" (Comprehensive Rules Ver. 4.3 §15-7-1) — derived from the
/// clause's LEADING cost-shaped steps (`leading_cost_guard`).
///
/// §15-7-4 (Ver. 4.3; it said the opposite in 3.6): "If executing an
/// optional processing condition's contents isn't possible, then the player
/// can't choose to execute them." §15-8-4-4-1: "If an activation-type effect
/// has optional processing conditions, a player can only declare activation
/// while the optional processing conditions can be performed." §15-7-3: a
/// condition can't be performed only in part, so every guard takes a COUNT
/// ("by trashing 2 cards in your hand" needs 2), never a boolean. Owner
/// ruling 2026-10-10 #6: never offer a trigger whose condition can't be paid.
/// DCGO gates the same thing per card in `CanActivateCondition`
/// (`BT22_041.cs`: `card.Owner.SecurityCards.Any()`; `BT25_084.cs`:
/// `card.Owner.HandCards.Count() >= 2`).
///
/// Plain data (clone-safe, `Game: Clone`), evaluated read-only by
/// [`crate::game::Game::cost_guard_holds_at`]. The engine-side interim for the
/// DSL `by:` block (`G-DSL-OPTIONAL-PROCESSING-CONDITION-BLOCK`), which will
/// mark the condition explicitly instead of recognising its shape.
/// `G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED`.
#[derive(Debug, Clone, PartialEq)]
pub enum CostGuard {
    /// "By trashing your top N security cards": N cards in your security.
    Security { n: usize },
    /// "By paying / losing N memory": the memory can move N toward the
    /// opponent's side within the gauge (`Game::can_pay_memory_for`).
    Memory { n: u16 },
    /// "By suspending this Digimon / Tamer": the source permanent can be
    /// suspended (`ActivationCostKind::SuspendSelf`'s probe).
    SuspendSource,
    /// "By suspending 1 of your Digimon": one of your permanents matching
    /// `filter` can be suspended.
    SuspendChosen { filter: CompiledPredicate },
    /// "By trashing N cards in your hand" / "by returning 1 [A] and 1 [B]
    /// from your trash": one DISTINCT card in `zone` per pick filter.
    ZonePicks {
        zone: CostPickZone,
        filters: Vec<CompiledPredicate>,
    },
    /// "By trashing N of this Digimon's digivolution cards" (incl.
    /// <Digi-Burst N>): `min` matching cards under the source's top card
    /// (that can be trashed, when the pick pays a trash cost).
    OwnSources {
        min: usize,
        filter: CompiledPredicate,
        trash_cost: bool,
    },
    /// "By trashing the bottom face-down card / N bottom face-down cards from
    /// under your Tamers": N face-down cards under your Tamers. "The bottom
    /// face-down card" is a Tamer's LOWEST face-down card, whether or not
    /// face-up cards sit below it (card text BT26-005 / BT25-035; DCGO
    /// `BT26_005.cs` `LastOrDefault(IsFlipped)`; the rules manual is silent),
    /// so every face-down card counts. Read through
    /// `step::selections::face_down_sources_under_tamers`, the same count the
    /// steps' Tamer picks and the trash use.
    FaceDownUnderTamers { n: usize },
    /// "By trashing 1 [X] from your hand / your Digimon's digivolution cards"
    /// (BT25-083 "1 Option card from any of your Digimon's digivolution
    /// cards"): the `select_union_zone` pick that `trash_union_bound` pays
    /// offers a card whose trash isn't blocked -- asked through the pick's own
    /// candidate builder (`union_zone_candidate_actions`), so the guard reads a
    /// card exactly as the pick does (a DUAL digivolution card is an Option
    /// card there, §4-6-6-2, official Q&A Q6395).
    UnionZoneTrash {
        zones: crate::selection::UnionZoneSet,
        /// The material carrier is the effect's own permanent
        /// (`material_of: source`); `false` scans every own Digimon.
        material_of_source: bool,
        filter: CompiledPredicate,
        zone_filters: digimon_dsl::compiled::CompiledUnionZoneFilters,
        material_carrier_filter: Option<CompiledPredicate>,
    },
}

/// The zone a `CostGuard::ZonePicks` condition draws its cards from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostPickZone {
    Hand,
    Trash,
}

/// Derive the payability guard of the optional processing condition at the
/// head of `steps` (a clause body with any `activation_cost` already lifted
/// off), or `None` when the body does not open with a recognised cost shape.
///
/// Recognised leading conditions:
/// - `trash_top_security { of: you }` (literal count) ⇒ that many security.
/// - `lose_memory: N` ⇒ the memory can be paid.
/// - `suspend { target: source }` ⇒ the source can be suspended;
///   `select_own_permanent` (mandatory) then `suspend` of that pick ⇒ a
///   matching permanent can be suspended.
/// - A run of own-hand picks each consumed by `trash_from_hand_by_index` or
///   `place_selected_card_under_tamer` (the run's first pick `cost: true` or
///   mandatory) ⇒ one distinct hand card per pick; the same for own-trash
///   picks consumed by `return_trash_list_to_deck_bottom`.
/// - `select_own_sources { target: source }` ⇒ `min` matching sources.
/// - `trash_bottom_face_down_source(s)_under_tamer(s)` ⇒ that many face-down
///   cards under your Tamers (any face-down card; the trash takes the lowest).
/// - `select_union_zone { of: you }` (`cost: true` or mandatory; material
///   carrier unpinned or `source`) whose pick `trash_union_bound` trashes next
///   ⇒ the pick has a trashable card.
///
/// One guard per clause: a compound condition is gated as a whole only when
/// it is one pick run (§15-7-3, "1 [A] and 1 [B]" from the same zone, matched
/// to distinct cards). Compounds across DIFFERENT step kinds ("by suspending
/// this Tamer and trashing 1 Option card", BT25-092) are gated on their FIRST
/// recognised part only — composing them is the DSL `by:` block's job
/// (`G-DSL-OPTIONAL-PROCESSING-CONDITION-BLOCK`).
///
/// A declinable pick WITHOUT `cost: true` is not a condition (declining it
/// continues the clause), so it yields no guard. Self-targeted costs
/// ("return / delete this Digimon") are payable whenever the source is live,
/// which the queue's source-liveness gate already checks.
pub(crate) fn leading_cost_guard(steps: &[CompiledStep]) -> Option<CostGuard> {
    let first = steps.first()?;
    match first {
        CompiledStep::TrashTopSecurityAndCancelReplacement {
            of: CompiledPlayerRef::You,
            ..
        } => Some(CostGuard::Security { n: 1 }),
        CompiledStep::TrashTopSecurity {
            of: CompiledPlayerRef::You,
            count,
            leave: None,
            ..
        } => {
            let n = match count {
                None => 1,
                Some(digimon_dsl::compiled::CompiledFormula::Literal(n)) => (*n).max(0) as usize,
                Some(_) => return None,
            };
            (n > 0).then_some(CostGuard::Security { n })
        }
        CompiledStep::LoseMemory(n) if *n > 0 => Some(CostGuard::Memory {
            n: u16::try_from(*n).ok()?,
        }),
        CompiledStep::Suspend {
            target: CompiledBindingRef::Source,
        }
        | CompiledStep::SuspendAndCancelAllMatchingLeaves {
            target: CompiledBindingRef::Source, ..
        } => Some(CostGuard::SuspendSource),
        CompiledStep::SelectOwnPermanent {
            filter,
            bind_as: Some(bound),
            optional: false,
            ..
        } => matches!(
            steps.get(1),
            Some(CompiledStep::Suspend { target }) if binding_ref_names(target, bound)
        )
        .then(|| CostGuard::SuspendChosen {
            filter: filter.clone(),
        }),
        CompiledStep::SelectHand {
            of: CompiledPlayerRef::You,
            optional,
            cost,
            ..
        }
        | CompiledStep::SelectTrash {
            of: CompiledPlayerRef::You,
            optional,
            cost,
            ..
        } if *cost || !*optional => {
            let zone = if matches!(first, CompiledStep::SelectHand { .. }) {
                CostPickZone::Hand
            } else {
                CostPickZone::Trash
            };
            let filters = leading_pick_run(steps, zone);
            (!filters.is_empty()).then_some(CostGuard::ZonePicks { zone, filters })
        }
        CompiledStep::SelectOwnSources {
            target: Some(CompiledBindingRef::Source),
            min,
            filter,
            bind_as,
            then,
            ..
        } => Some(CostGuard::OwnSources {
            min: *min as usize,
            filter: filter.clone(),
            trash_cost: crate::dsl_cards::step::selections::source_selection_pays_trash_cost(
                then,
                bind_as.as_deref(),
            ),
        }),
        CompiledStep::TrashBottomFaceDownSourceUnderTamer {
            of: CompiledPlayerRef::You,
            ..
        } => Some(CostGuard::FaceDownUnderTamers { n: 1 }),
        CompiledStep::TrashBottomFaceDownSourcesUnderTamers {
            of: CompiledPlayerRef::You,
            count,
        } if *count > 0 => Some(CostGuard::FaceDownUnderTamers {
            n: *count as usize,
        }),
        CompiledStep::SelectUnionZone {
            of: CompiledPlayerRef::You,
            zones,
            material_of,
            filter,
            zone_filters,
            material_carrier_filter,
            bind_as: Some(bound),
            optional,
            cost,
            then,
            ..
        } if *cost || !*optional => {
            // The pick's success tail is `then` followed by the clause's
            // remaining steps; its first step must trash the picked card.
            let consumer = if then.is_empty() {
                steps.get(1)
            } else {
                then.first()
            };
            let pays_trash = matches!(
                consumer,
                Some(CompiledStep::TrashUnionBound { binding }) if binding == bound
            );
            let material_of_source = match material_of {
                None => false,
                Some(CompiledBindingRef::Source) => true,
                // A carrier bound earlier in the clause is not known before
                // the clause activates.
                Some(_) => return None,
            };
            let set = crate::dsl_cards::step::selections::union_zone_set(zones);
            (pays_trash && set.0 != 0).then(|| CostGuard::UnionZoneTrash {
                zones: set,
                material_of_source,
                filter: filter.clone(),
                zone_filters: zone_filters.clone(),
                material_carrier_filter: material_carrier_filter.clone(),
            })
        }
        _ => None,
    }
}

/// The filters of the run of own-`zone` picks at the head of `steps`, each
/// pick immediately consumed by the zone's cost step (hand: trash it or place
/// it under a Tamer; trash: return it to the deck). Stops at the first step
/// that breaks the pattern; empty when the first pick is not consumed.
fn leading_pick_run(steps: &[CompiledStep], zone: CostPickZone) -> Vec<CompiledPredicate> {
    let mut filters = Vec::new();
    let mut rest = steps;
    loop {
        let (pick, consumer) = match rest {
            [pick, consumer, ..] => (pick, consumer),
            _ => break,
        };
        let (filter, bound) = match (zone, pick) {
            (
                CostPickZone::Hand,
                CompiledStep::SelectHand {
                    of: CompiledPlayerRef::You,
                    filter,
                    bind_as: Some(bound),
                    ..
                },
            )
            | (
                CostPickZone::Trash,
                CompiledStep::SelectTrash {
                    of: CompiledPlayerRef::You,
                    filter,
                    bind_as: Some(bound),
                    ..
                },
            ) => (filter, bound),
            _ => break,
        };
        let consumed = match (zone, consumer) {
            (
                CostPickZone::Hand,
                CompiledStep::TrashFromHandByIndex {
                    of: CompiledPlayerRef::You,
                    hand_index,
                },
            ) => binding_ref_names(hand_index, bound),
            (CostPickZone::Hand, CompiledStep::PlaceSelectedCardUnderTamer { card, .. }) => {
                binding_ref_names(card, bound)
            }
            (
                CostPickZone::Trash,
                CompiledStep::ReturnTrashListToDeckBottom {
                    of: CompiledPlayerRef::You,
                    cards,
                    ..
                },
            ) => binding_ref_names(cards, bound),
            _ => false,
        };
        if !consumed {
            break;
        }
        filters.push(filter.clone());
        rest = &rest[2..];
    }
    filters
}

/// True when `r` refers to the binding named `name`.
fn binding_ref_names(r: &CompiledBindingRef, name: &str) -> bool {
    matches!(
        r,
        CompiledBindingRef::Named(n) | CompiledBindingRef::Binding(n) | CompiledBindingRef::Permanent(n)
            if n == name
    )
}

impl CostGuard {
    /// Can the condition be executed right now, for the effect `rctx`
    /// describes (its source card / permanent and controller)?
    pub(crate) fn holds(&self, rctx: &crate::effect_context::EffectReadContext<'_>) -> bool {
        let game = rctx.game;
        match self {
            CostGuard::Security { n } => game.player(rctx.player).security.len() >= *n,
            CostGuard::Memory { n } => game.can_pay_memory_for(rctx.player, *n),
            CostGuard::SuspendSource => {
                ActivationCostKind::SuspendSelf.is_payable(game, rctx.source_permanent)
            }
            CostGuard::SuspendChosen { filter } => {
                let player = rctx.player;
                (0..game.player(player).battle_area.len()).any(|i| {
                    let handle = crate::permanent::PermanentHandle {
                        player,
                        index: i as u8,
                    };
                    ActivationCostKind::SuspendSelf.is_payable(game, Some(handle))
                        && eval_predicate(filter, rctx, PredicateSubject::Permanent(handle))
                })
            }
            CostGuard::ZonePicks { zone, filters } => {
                let player = game.player(rctx.player);
                let cards: Vec<CardHandle> = match zone {
                    CostPickZone::Hand => player.hand.iter().map(|c| c.handle()).collect(),
                    CostPickZone::Trash => player.trash.iter().map(|c| c.handle()).collect(),
                };
                let candidates: Vec<Vec<usize>> = filters
                    .iter()
                    .map(|filter| {
                        cards
                            .iter()
                            .enumerate()
                            .filter(|(_, card)| {
                                eval_predicate(filter, rctx, PredicateSubject::Card(**card))
                            })
                            .map(|(i, _)| i)
                            .collect()
                    })
                    .collect();
                picks_have_distinct_cards(&candidates, cards.len())
            }
            CostGuard::OwnSources {
                min,
                filter,
                trash_cost,
            } => rctx.source_permanent.is_some_and(|host| {
                game.player(host.player)
                    .battle_area
                    .get(host.index as usize)
                    .is_some_and(|perm| {
                        perm.card_sources
                            .iter()
                            .take(perm.card_sources.len().saturating_sub(1))
                            .enumerate()
                            .filter(|(index, card)| {
                                let source = crate::selection::SourceSelectionRef {
                                    permanent: host,
                                    field_index: host.index,
                                    source_index: *index as u8,
                                    card: card.handle(),
                                };
                                (!trash_cost
                                    || !game.source_card_trash_blocked(
                                        host,
                                        card.handle(),
                                        rctx.player,
                                    ))
                                    && eval_predicate(
                                        filter,
                                        rctx,
                                        PredicateSubject::Source(source),
                                    )
                            })
                            .count()
                            >= *min
                    })
            }),
            CostGuard::FaceDownUnderTamers { n } => {
                crate::dsl_cards::step::selections::face_down_sources_under_tamers(
                    rctx,
                    rctx.player,
                ) >= *n
            }
            CostGuard::UnionZoneTrash {
                zones,
                material_of_source,
                filter,
                zone_filters,
                material_carrier_filter,
            } => {
                let material_of = if *material_of_source {
                    match rctx.source_permanent {
                        Some(host) => Some(host),
                        None => return false,
                    }
                } else {
                    None
                };
                // The candidate builder hands back `game` (== `rctx.game`); the
                // predicates read through the effect's own context.
                let carrier_filter =
                    |_: &crate::game::Game, handle: crate::permanent::PermanentHandle| {
                        material_carrier_filter.as_ref().is_none_or(|pred| {
                            eval_predicate(pred, rctx, PredicateSubject::Permanent(handle))
                        })
                    };
                // The pick's own card matcher (no clause bindings exist before
                // the clause activates; the guard reads a leading step).
                let card_filter = |_: &crate::game::Game,
                                   card: &crate::card_source::CardSource,
                                   zone: crate::selection::UnionZoneSet| {
                    crate::dsl_cards::step::selections::union_zone_card_matches(
                        rctx,
                        filter,
                        zone_filters,
                        card.handle(),
                        zone,
                        None,
                    )
                };
                let offered = crate::effect_context::selections::union_zone_candidate_actions(
                    game,
                    rctx.player,
                    *zones,
                    material_of,
                    Some(&carrier_filter),
                    &card_filter,
                );
                // A digivolution card whose trash is prohibited can't pay
                // "by trashing" (§15-7-4); a hand card can always be trashed.
                crate::dsl_cards::step::selections::union_zone_candidates(
                    game,
                    rctx.player,
                    &offered,
                )
                .into_iter()
                .any(|(_, card, origin)| match origin {
                    crate::selection::UnionZoneOrigin::Material { carrier, .. } => {
                        !game.source_card_trash_blocked(carrier, card, rctx.player)
                    }
                    _ => true,
                })
            }
        }
    }
}

/// True when every pick can take a DISTINCT card: `candidates[pick]` lists
/// the indices (of `card_count` zone cards) that pick's filter accepts.
/// Bipartite matching by augmenting paths (Kuhn); the runs are tiny.
fn picks_have_distinct_cards(candidates: &[Vec<usize>], card_count: usize) -> bool {
    if candidates.len() > card_count {
        return false;
    }
    fn augment(
        pick: usize,
        candidates: &[Vec<usize>],
        owner: &mut [Option<usize>],
        seen: &mut [bool],
    ) -> bool {
        for &card in &candidates[pick] {
            if seen[card] {
                continue;
            }
            seen[card] = true;
            if owner[card].is_none_or(|other| augment(other, candidates, owner, seen)) {
                owner[card] = Some(pick);
                return true;
            }
        }
        false
    }
    let mut owner = vec![None; card_count];
    (0..candidates.len()).all(|pick| {
        let mut seen = vec![false; card_count];
        augment(pick, candidates, &mut owner, &mut seen)
    })
}

impl crate::game::Game {
    /// Whether the optional processing condition `guard` can be executed
    /// now (§15-7-4, §15-8-4-4-1), for a caller that already knows the
    /// effect's source permanent (a queued effect, a `[Main]` mask scan).
    pub(crate) fn cost_guard_holds_at(
        &self,
        guard: &CostGuard,
        source: CardHandle,
        source_permanent: Option<crate::permanent::PermanentHandle>,
        controller: crate::enums::PlayerId,
    ) -> bool {
        let rctx = crate::effect_context::EffectReadContext::new(
            self,
            source,
            source_permanent,
            controller,
        );
        guard.holds(&rctx)
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
        | CompiledStep::DrawFn { .. }
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
        EffectTiming::StartOfOpponentsMainPhase => Effect::start_of_opponents_main_phase(card),
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
        EffectTiming::OnTopCardTrashed => Effect::on_top_card_trashed(card),
        EffectTiming::OnAddDigivolutionCards => Effect::on_add_digivolution_cards(card),
        EffectTiming::OnDigivolutionCardReturnedToDeckBottom => {
            Effect::on_digivolution_card_returned_to_deck_bottom(card)
        }
        EffectTiming::OnSecurityCheck => Effect::on_security_check(card),
        EffectTiming::OnCheckFaceUpSecurity => {
            EffectBuilder::new(card, EffectTiming::OnCheckFaceUpSecurity)
        }
        EffectTiming::OnLoseSecurity => Effect::on_lose_security(card),
        EffectTiming::OnDiscardSecurity => Effect::on_discard_security(card),
        EffectTiming::OnPlaceSecurity => Effect::on_place_security(card),
        // A DSL `[Counter]` clause is a Counter-window ability: the Counter
        // window (`try_enter_counter`), the hand-Option scan and the mask all
        // require `Effect.counter` alongside the `CounterEffect` timing.
        // Without the flag no DSL field [Counter] ability was ever offered.
        // G-ENGINE-DSL-FIELD-COUNTER-WINDOW (= G-DSL-COUNTER-TIMING-NO-COUNTER-FLAG).
        EffectTiming::CounterEffect => {
            EffectBuilder::new(card, EffectTiming::CounterEffect).counter()
        }
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

    /// Both face-down-under-Tamer trash costs park a Tamer pick, so a
    /// cost-reduction `pay_cost` built on either is interactive
    /// (`G-COST-REDUCTION-INTERACTIVE-PAY-COST`).
    #[test]
    fn face_down_under_tamer_trash_costs_install_a_selection() {
        assert!(body_first_step_installs_selection(&[
            CompiledStep::TrashBottomFaceDownSourceUnderTamer {
                of: CompiledPlayerRef::You,
                optional: false,
            }
        ]));
        assert!(body_first_step_installs_selection(&[
            CompiledStep::TrashBottomFaceDownSourcesUnderTamers {
                of: CompiledPlayerRef::You,
                count: 2,
            }
        ]));
    }

    #[test]
    fn resource_flow_catalog_ignores_non_hand_resource_steps() {
        let steps = vec![CompiledStep::GainMemory(1)];

        assert!(!steps_provide_resource_flow(&steps));
    }
}
