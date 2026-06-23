//! Process-step lowering dispatch. Phase 2a: memory + draw/trash helpers.
//! Phase 2b: continuation-passing dispatcher + selection handlers + zone-moves.
//! Phase 2c: permanent mutations + modifier steps (AddDpModifier, AddModifier, GrantKeyword)
//!           + control-flow steps (Optional, If).

pub mod as_selecting_player;
pub mod combat;
pub mod control_flow;
pub mod digixros_transaction;
pub mod dna_digivolve;
pub mod draw;
pub mod effects;
pub mod grant_triggered;
pub mod iteration;
pub mod link_cards;
pub mod memory;
pub mod modifiers;
pub mod permanent_mutations;
pub mod permanent_scan;
pub mod play_digivolve;
pub mod replacement_outcome;
pub mod schedule_delayed;
pub mod selections;
pub mod zone_moves;

use digimon_dsl::compiled::{
    CompiledPlayerRef, CompiledStackPosition, CompiledStep, CompiledStepDiscriminant,
};
use std::sync::Arc;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::effect_context::EffectContext;
use crate::effect_context::EffectReadContext;
use crate::enums::PlayerId;
use crate::enums::StackPosition;
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::selection::{OptionResolutionPhase, PendingOption};

/// Map a `CompiledStackPosition` to the engine's `StackPosition`.
/// Shared by `zone_moves` and `permanent_mutations` — lives here to avoid
/// duplicate private copies in each sub-module.
pub(super) fn map_stack_position(p: CompiledStackPosition) -> StackPosition {
    match p {
        CompiledStackPosition::Top => StackPosition::Top,
        CompiledStackPosition::Bottom => StackPosition::Bottom,
        CompiledStackPosition::Random => StackPosition::Random,
        // collapse §3.1 — `Choice` is a player-elected position that the
        // supporting step's executor must resolve to a concrete Top/Bottom via
        // a binary pick BEFORE any placement. Reaching here means a step that
        // doesn't support `position: choice` was given it — a card-authoring
        // error, surfaced loudly rather than silently approximated.
        CompiledStackPosition::Choice => {
            panic!("position: choice reached map_stack_position — only place_remainder_on_deck and the place-on-security verb support it; the step must intercept Choice before placing")
        }
    }
}

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

/// Whether a step ran synchronously to completion or installed a parked
/// selection. Phase 2d Task 7 propagates this outward across nested
/// `run_steps` re-entries so a parked selection inside an `If` /
/// `ForEach` body suspends the outer slice instead of letting subsequent
/// outer steps race ahead of the resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Synchronous,
    Parked,
}

#[derive(Clone, Debug)]
pub struct StepRuntime {
    raw: Arc<EngineRawRustRegistry>,
    dna_origin: Option<bool>,
    /// The clause's once-per-turn counter key (`Effect::shared_opt_group`),
    /// captured statically by the lowering so the `refund_opt` step can
    /// un-record the activation — DCGO `RemoveUse` (G-OPT-REFUND-ON-DECLINE).
    /// Cloned into parked tails, so it survives mid-process selections.
    opt_key: Option<u8>,
}

impl Default for StepRuntime {
    fn default() -> Self {
        Self::new(Arc::new(EngineRawRustRegistry::new()))
    }
}

impl StepRuntime {
    pub fn new(raw: Arc<EngineRawRustRegistry>) -> Self {
        Self {
            raw,
            dna_origin: None,
            opt_key: None,
        }
    }

    pub fn raw(&self) -> &EngineRawRustRegistry {
        &self.raw
    }

    pub fn with_dna_origin(mut self, dna_origin: Option<bool>) -> Self {
        self.dna_origin = dna_origin;
        self
    }

    pub fn with_opt_key(mut self, opt_key: Option<u8>) -> Self {
        self.opt_key = opt_key;
        self
    }

    pub fn opt_key(&self) -> Option<u8> {
        self.opt_key
    }
}

/// Park the outer-slice tail (steps that follow the just-parked control-
/// flow / iteration step) onto `Game::dsl_outer_tail` so the eventual
/// selection-callback can resume it via `drain_dsl_outer_tail`.
///
/// Empty tails are dropped — there's nothing to resume. The
/// `debug_assert!` enforces the single-outstanding invariant documented
/// on the slot.
fn park_outer_tail(
    ctx: &mut EffectContext<'_>,
    bindings: &Bindings,
    steps: &[CompiledStep],
    i: usize,
    runtime: &StepRuntime,
) {
    let outer_tail = steps[i + 1..].to_vec();
    if outer_tail.is_empty() {
        return;
    }
    if ctx.game.dsl_outer_tail.is_some() {
        // Always-fire (not gated on debug_assertions): silently
        // overwriting a parked outer continuation corrupts game state
        // by dropping the original tail. Better to fail loudly so the
        // crash recorder captures the offending game; the wrapper
        // converts the panic to a terminal step so training continues.
        // Best-effort identity for the card whose effect is currently
        // resolving. Falls back to the raw handle if the lookup misses
        // (token rebases, mid-deletion handles, etc.).
        let source_card_id = ctx
            .game
            .card_data_for_handle(ctx.source_card)
            .map(|cd| cd.card_id.clone())
            .unwrap_or_else(|| format!("handle:{}", ctx.source_card.0));
        let parking_step = CompiledStepDiscriminant::from(&steps[i]);
        let (parked_first, parked_len) = ctx
            .game
            .dsl_outer_tail
            .as_ref()
            .map(|(t, _, _)| (t.first().map(CompiledStepDiscriminant::from), t.len()))
            .unwrap_or((None, 0));
        panic!(
            "dsl_outer_tail overwrite: card={} player={:?} parking_step={:?} \
             outer_tail_len={} previously_parked_first_step={:?} \
             previously_parked_len={} — likely a nested-park bug",
            source_card_id,
            ctx.player,
            parking_step,
            outer_tail.len(),
            parked_first,
            parked_len,
        );
    }
    ctx.game.dsl_outer_tail = Some((outer_tail, bindings.clone(), runtime.clone()));
}

pub(crate) fn drain_or_rewrap_pending_tail(
    game: &mut Game,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<PermanentHandle>,
    player: PlayerId,
    outer_tail: Vec<CompiledStep>,
    mut bindings: Bindings,
    runtime: StepRuntime,
    tail_trigger_context: Option<crate::trigger_context::TriggerContext>,
) {
    if game.pending_selection.is_some() {
        wrap_pending_selection_with_tail(
            game,
            source_card,
            source_permanent,
            player,
            outer_tail,
            bindings,
            runtime,
            tail_trigger_context,
        );
        return;
    }

    if outer_tail.is_empty() {
        return;
    }

    // Re-assert the trigger context the tail was AUTHORED under (captured at
    // wrap/park time). A synchronous mid-tail step can fire an interrupt that
    // replaces `current_trigger_context` before this deferred tail finally
    // runs — e.g. an [On Deletion] tail's free play raising a BeforePayCost
    // cost-reduction dialog (Yuu Amano BT10-093 over Damemon EX10-044's
    // "Then, <Save>") — and event bindings (`event_card`, the deleted-object
    // snapshot) would silently fail to resolve.
    // G-TRIGGER-CONTEXT-CLOBBERED-BY-COST-REDUCTION-INTERRUPT.
    let previous = game.current_trigger_context.clone();
    game.current_trigger_context = tail_trigger_context;
    let mut ctx = EffectContext::new(game, source_card, source_permanent, player);
    run_steps_with_runtime(&outer_tail, &mut ctx, &mut bindings, &runtime);
    game.current_trigger_context = previous;
}

#[allow(clippy::too_many_arguments)]
fn wrap_pending_selection_with_tail(
    game: &mut Game,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<PermanentHandle>,
    player: PlayerId,
    outer_tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
    tail_trigger_context: Option<crate::trigger_context::TriggerContext>,
) {
    let Some(mut pending) = game.pending_selection.take() else {
        return;
    };

    // make-engine-cloneable coexistence (Batch 1). This helper composes an outer
    // tail onto the selection's CLOSURE callback/on_decline. A selection ported to
    // the resumable VM is driven by `run_resume` (the closure is bypassed), so
    // wrapping the closure would silently DROP the outer tail. Instead, compose
    // the tail as DATA onto the resume frame: `run_resume` runs it after the
    // inner/decline tail via the same `drain_or_rewrap_pending_tail` path, so a
    // further nested select re-composes here recursively (deep chains thread).
    if game.pending_selection_resume.is_some() {
        use crate::resume::ResumeFrame;
        let cont = crate::resume::OuterContinuation {
            tail: std::sync::Arc::new(outer_tail),
            bindings,
            runtime,
            trigger_context: tail_trigger_context,
            source_card,
            source_permanent,
            player,
        };
        // Compose onto whatever frame is active. RunTail runs its conts after the
        // inner/decline tail; multi-pick frames run theirs at the TERMINAL (after
        // the final pick), so a wrap mid-accumulation still fires exactly once
        // when the whole multi-pick completes.
        match game
            .pending_selection_resume
            .as_mut()
            .and_then(|s| s.frames.last_mut())
        {
            Some(ResumeFrame::RunTail { outer_conts, .. }) => outer_conts.push(cont),
            Some(ResumeFrame::MultiPickStep(s)) => s.outer_conts.push(cont),
            Some(ResumeFrame::PermutationStep(s)) => s.outer_conts.push(cont),
            Some(ResumeFrame::BudgetStep(s)) => s.outer_conts.push(cont),
            Some(ResumeFrame::SourceMultiStep(s)) => s.outer_conts.push(cont),
            None => unreachable!("pending_selection_resume set but frame stack empty"),
        }
        game.pending_selection = Some(pending);
        return;
    }

    let original_callback = pending.callback;
    let accept_tail = outer_tail.clone();
    let accept_bindings = bindings.clone();
    let accept_runtime = runtime.clone();
    let accept_trigger_context = tail_trigger_context.clone();
    pending.callback = Box::new(move |game: &mut Game, action_id: u16| {
        // Binding-freshness channel: clear before, take after — whatever the
        // inner resolution published reflects picks made AFTER our wrap-time
        // snapshot, and must be visible to binding-gated outer-tail steps
        // (G-OPT-REFUND-ON-DECLINE).
        game.dsl_resolved_tail_bindings = None;
        original_callback(game, action_id);
        let mut merged = accept_bindings;
        if let Some(fresh) = game.dsl_resolved_tail_bindings.take() {
            merged.merge_slots_from(&fresh);
        }
        drain_or_rewrap_pending_tail(
            game,
            source_card,
            source_permanent,
            player,
            accept_tail,
            merged,
            accept_runtime,
            accept_trigger_context,
        );
    });

    if let Some(original_decline) = pending.on_decline.take() {
        let decline_tail = outer_tail;
        let decline_bindings = bindings;
        let decline_runtime = runtime;
        let decline_trigger_context = tail_trigger_context;
        pending.on_decline = Some(Box::new(move |game: &mut Game| {
            game.dsl_resolved_tail_bindings = None;
            original_decline(game);
            let mut merged = decline_bindings;
            if let Some(fresh) = game.dsl_resolved_tail_bindings.take() {
                merged.merge_slots_from(&fresh);
            }
            drain_or_rewrap_pending_tail(
                game,
                source_card,
                source_permanent,
                player,
                decline_tail,
                merged,
                decline_runtime,
                decline_trigger_context,
            );
        }));
    }

    game.pending_selection = Some(pending);
}

fn park_pending_selection_tail(
    ctx: &mut EffectContext<'_>,
    bindings: &Bindings,
    steps: &[CompiledStep],
    i: usize,
    runtime: &StepRuntime,
) {
    let outer_tail = steps[i + 1..].to_vec();
    // Capture the LIVE trigger context: the wrapped tail must run under the
    // context its clause was authored against, even if the interloping
    // selection's resolution replaces it (G-TRIGGER-CONTEXT-CLOBBERED-BY-
    // COST-REDUCTION-INTERRUPT).
    let tail_trigger_context = ctx.game.current_trigger_context.clone();
    wrap_pending_selection_with_tail(
        ctx.game,
        ctx.source_card,
        ctx.source_permanent,
        ctx.player,
        outer_tail,
        bindings.clone(),
        runtime.clone(),
        tail_trigger_context,
    );
}

/// Drain `Game::dsl_outer_tail` if a parked outer continuation is
/// pending. Selection-install callbacks call this after their own inner
/// tail completes so the steps that followed an outer control-flow /
/// iteration step (parked by `park_outer_tail`) actually run.
///
/// Public to the selections sub-module so new selection kinds added in
/// future phases (`SelectReveal`, `SelectMaterial`, …) pick this up by
/// calling one helper instead of duplicating the take + run_steps
/// boilerplate per callback.
///
/// Phase 2f3: `cb_ctx` arrives with `override_selecting_player` carrying
/// the override that was active at the parking select's install time.
/// Outer-tail steps live OUTSIDE the `AsSelectingPlayer` body whose select
/// parked, so they must not inherit the override. Clear it here before
/// running the outer tail; the inner body's chained selects already saw
/// the override via the parked-callback's reconstructed ctx (Task 1).
pub(crate) fn drain_dsl_outer_tail(cb_ctx: &mut EffectContext<'_>) {
    drain_dsl_outer_tail_with_bindings(cb_ctx, None)
}

/// Variant of [`drain_dsl_outer_tail`] that overlays the inner resolution's
/// bindings onto the park-time snapshot before the outer tail runs, so
/// binding-gated sibling steps (`binding_exists` / `binding_absent`) see the
/// picks a nested selection made after the park. G-OPT-REFUND-ON-DECLINE.
pub(crate) fn drain_dsl_outer_tail_with_bindings(
    cb_ctx: &mut EffectContext<'_>,
    inner_bindings: Option<&Bindings>,
) {
    if let Some((outer_tail, mut outer_b, runtime)) = cb_ctx.game.dsl_outer_tail.take() {
        if let Some(inner) = inner_bindings {
            outer_b.merge_slots_from(inner);
        }
        cb_ctx.set_override_selecting_player(None);
        if cb_ctx.game.pending_selection.is_some() {
            let tail_trigger_context = cb_ctx.game.current_trigger_context.clone();
            wrap_pending_selection_with_tail(
                cb_ctx.game,
                cb_ctx.source_card,
                cb_ctx.source_permanent,
                cb_ctx.player,
                outer_tail,
                outer_b,
                runtime,
                tail_trigger_context,
            );
        } else {
            // Wrap the outer-tail run in a deferred-drain scope so any
            // `fire_on_*` observer triggered by an outer-tail step routes
            // through `maybe_drain_effect_queue` and only enqueues — the
            // exit below flushes them at a safe checkpoint with
            // `dsl_outer_tail = None`. Without this, a step like
            // `trash_top_security` could fire `OnLoseSecurity` observers
            // whose inline drain would process a second triggered effect
            // and collide on the next park. Post-2026-05-23 G-DSL-OUTER-
            // TAIL-NESTED-PARK fix.
            cb_ctx.game.enter_deferred_drain();
            run_steps_with_runtime(&outer_tail, cb_ctx, &mut outer_b, &runtime);
            cb_ctx.game.exit_deferred_drain_and_flush();
        }
    }
}

/// Drive the full step slice to completion. When a selection step is
/// encountered, `selections::try_install` captures the tail as a
/// heap-allocated callback and returns `Parked`; the rest of the slice
/// will execute once the player resolves the selection.
///
/// Phase 2d: the dispatcher fans out to control-flow / iteration handlers
/// that may themselves park. A `Parked` from any of them propagates up.
/// Task 7 wires outer-tail capture so steps after a parked control-flow
/// step are deferred until the inner callback resolves.
pub fn run_steps(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> RunOutcome {
    run_steps_with_runtime(steps, ctx, bindings, &StepRuntime::default())
}

pub fn run_steps_with_runtime(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> RunOutcome {
    if let Some(dna_origin) = runtime.dna_origin {
        let prev = ctx.game.current_dna_origin;
        ctx.game.current_dna_origin = Some(dna_origin);
        let outcome = run_steps_with_runtime_inner(steps, ctx, bindings, runtime);
        ctx.game.current_dna_origin = prev;
        return outcome;
    }
    run_steps_with_runtime_inner(steps, ctx, bindings, runtime)
}

fn run_steps_with_runtime_inner(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> RunOutcome {
    let mut i = 0;
    while i < steps.len() {
        // Cost-pay abort short-circuit (G-OPTIONAL-COST-DECLINE-ABORTS-CLAUSE).
        // When the player PASSes on an optional cost-pay prompt
        // (`select_hand`/`select_trash`/`select_union_zone` with
        // `optional: true`), the install's `on_decline` sets
        // `dsl_clause_aborted = true`. Every subsequent step in this clause
        // body — and any outer-tail draining that comes after — must skip,
        // so a "By trashing X, Y" cost does not run Y when X was declined.
        // The flag is scoped to the `on_decline` invocation by
        // `effect_queue::resolve_generic_selection` (save+restore).
        if ctx.game.dsl_clause_aborted {
            return RunOutcome::Synchronous;
        }
        let step = &steps[i];

        if let Some(outcome) = control_flow::try_run(step, ctx, bindings, runtime) {
            if matches!(outcome, RunOutcome::Parked) {
                park_outer_tail(ctx, bindings, steps, i, runtime);
                return RunOutcome::Parked;
            }
            i += 1;
            continue;
        }

        if let Some(outcome) = as_selecting_player::try_run(step, ctx, bindings, runtime) {
            if matches!(outcome, RunOutcome::Parked) {
                park_outer_tail(ctx, bindings, steps, i, runtime);
                return RunOutcome::Parked;
            }
            i += 1;
            continue;
        }

        if let Some(outcome) = iteration::try_run(step, ctx, bindings, runtime) {
            if matches!(outcome, RunOutcome::Parked) {
                park_outer_tail(ctx, bindings, steps, i, runtime);
                return RunOutcome::Parked;
            }
            i += 1;
            continue;
        }

        // `link_cards` (Gap 2) captures the remaining slice itself and drives a
        // pick loop of chained selections. It always either parks on a pending
        // selection or runs the captured tail synchronously — in both cases the
        // outer loop must stop (the tail was consumed inside the step).
        if link_cards::try_install(step, &steps[i + 1..], ctx, bindings.clone(), runtime) {
            return if ctx.game.pending_selection.is_some() {
                RunOutcome::Parked
            } else {
                RunOutcome::Synchronous
            };
        }

        // Selection steps either install the remainder as their callback, no-op
        // and continue, or complete their captured tail synchronously.
        match selections::try_install(step, &steps[i + 1..], ctx, bindings.clone(), runtime) {
            selections::InstallResult::NotSelection => {}
            selections::InstallResult::Continue => {
                i += 1;
                continue;
            }
            selections::InstallResult::TailAlreadyRan => {
                return RunOutcome::Synchronous;
            }
            selections::InstallResult::Parked => {
                return RunOutcome::Parked;
            }
        }

        // Synchronous families — execute and advance.
        run_step_with_runtime(step, ctx, bindings, runtime);
        if ctx.game.pending_selection.is_some() {
            park_pending_selection_tail(ctx, bindings, steps, i, runtime);
            return RunOutcome::Parked;
        }
        i += 1;
    }
    RunOutcome::Synchronous
}

/// Dispatch a compiled step to its family-specific handler. Unhandled
/// steps are silently skipped in Phase 2a; Phase 2b/c/d add more families.
pub fn run_step(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) {
    run_step_with_runtime(step, ctx, bindings, &StepRuntime::default());
}

pub fn run_step_with_runtime(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) {
    if let CompiledStep::RawRust { fn_name, .. } = step {
        if let Some(f) = runtime.raw().step_fn(fn_name) {
            f(ctx, bindings);
        }
        return;
    }
    if memory::try_run(step, ctx, bindings) {
        return;
    }
    if draw::try_run(step, ctx) {
        return;
    }
    if zone_moves::try_run(step, ctx, bindings, runtime) {
        return;
    }
    if permanent_mutations::try_run(step, ctx, bindings) {
        return;
    }
    if modifiers::try_run(step, ctx, bindings, runtime) {
        return;
    }
    if digixros_transaction::try_run(step, ctx, bindings) {
        return;
    }
    if grant_triggered::try_run(step, ctx, bindings) {
        return;
    }
    if play_digivolve::try_run(step, ctx, bindings) {
        return;
    }
    if dna_digivolve::try_run(step, ctx, bindings) {
        return;
    }
    if schedule_delayed::try_run(step, ctx, bindings, runtime) {
        return;
    }
    if matches!(step, CompiledStep::PlaceSelfAsDelayOption) {
        ctx.place_self_as_delay_option_permanent();
        return;
    }
    // G-OPT-REFUND-ON-DECLINE — DCGO `ActivateClass.RemoveUse()`. Refund the
    // running clause's once-per-turn use; the key was captured statically by
    // the lowering into the StepRuntime (and survives parked tails).
    if matches!(step, CompiledStep::RefundOpt) {
        if let (Some(opt_key), Some(perm)) = (runtime.opt_key(), ctx.source_permanent) {
            let source_card = ctx.source_card;
            ctx.game
                .unrecord_source_permanent_activation(perm, source_card, opt_key);
        }
        return;
    }
    // Phase 2 Track B — `CompiledStep::ActivationCost` is normally lifted
    // out of the body by `lower_triggered::lower_with_raw...` and bound to
    // `Effect::activation_cost(...)`. The compile-side validator rejects
    // mid-body uses, so reaching this arm at runtime indicates a
    // lowering bug; no-op silently. This arm also satisfies the
    // dsl_eval_arm_coverage lint which scans for a textual reference to
    // every `CompiledStep` variant.
    if matches!(step, CompiledStep::ActivationCost { .. }) {
        return;
    }
    if try_run_link_step(step, ctx) {
        return;
    }
    if link_cards::try_run_relink(step, ctx, bindings) {
        return;
    }
    if combat::try_run(step, ctx, bindings) {
        return;
    }
    if effects::try_run(step, ctx, bindings) {
        return;
    }
    if replacement_outcome::try_run(step, ctx, bindings) {
        return;
    }
    // Phase 2d+: other families.
}

fn try_run_link_step(step: &CompiledStep, ctx: &mut EffectContext<'_>) -> bool {
    let CompiledStep::LinkToOwnDigimon {
        optional,
        free: _,
        filter,
    } = step
    else {
        return false;
    };

    let Some(pending_snapshot) = ctx.game.pending_option.as_ref().cloned() else {
        return true;
    };
    let owner = pending_snapshot.owner;
    let source_card = pending_snapshot.card.handle();
    let read_ctx = EffectReadContext::new(ctx.game, source_card, None, owner);
    let candidates: Vec<PermanentHandle> = ctx
        .game
        .player(owner)
        .battle_area
        .iter()
        .enumerate()
        .filter_map(|(i, perm)| {
            if !perm.is_digimon(&ctx.game.card_data) {
                return None;
            }
            if !matches!(perm.option_state, crate::permanent::OptionState::Standard) {
                return None;
            }
            let handle = PermanentHandle {
                player: owner,
                index: i as u8,
            };
            eval_predicate(filter, &read_ctx, PredicateSubject::Permanent(handle)).then_some(handle)
        })
        .collect();

    if candidates.is_empty() {
        return true;
    }

    ctx.game.pending_option = Some(PendingOption {
        owner,
        card: pending_snapshot.card,
        source_kind: pending_snapshot.source_kind,
        resolution_phase: OptionResolutionPhase::LinkSelectHost,
        subtype: pending_snapshot.subtype,
    });
    ctx.game
        .install_link_host_selection(owner, source_card, candidates, *optional);
    true
}
