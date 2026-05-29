//! Iteration steps (Phase 2d): ForEach + PerSelected.
//!
//! These live on the `run_steps` path (not `run_step`) because their
//! per-iteration bodies may park selections. The dispatcher re-enters
//! `run_steps` once per iteration and propagates a `RunOutcome::Parked`
//! upward via Task 7's continuation plumbing.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::permanent_scan::scan;
use crate::dsl_cards::step::{run_steps_with_runtime, RunOutcome, StepRuntime};
use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

/// The stable identity of the permanent referenced by `handle`: the
/// `CardHandle` of its top card (`card_index` is unique per game). Returns
/// `None` if the handle does not currently point at a battle-area permanent.
fn top_card_handle(ctx: &EffectContext<'_>, handle: PermanentHandle) -> Option<CardHandle> {
    ctx.game
        .players
        .get(handle.player as usize)?
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.top_card().handle())
}

/// Re-resolve a positional `PermanentHandle` from a permanent's stable
/// top-card identity. Searches every battle area for the permanent whose top
/// card has `stable_id`. Returns `None` if no such permanent is on the field
/// (it left since the snapshot was taken).
fn resolve_by_top_card(ctx: &EffectContext<'_>, stable_id: CardHandle) -> Option<PermanentHandle> {
    for (player_idx, player) in ctx.game.players.iter().enumerate() {
        for (index, perm) in player.battle_area.iter().enumerate() {
            if perm.top_card().handle() == stable_id {
                return Some(PermanentHandle {
                    player: player_idx as u8,
                    index: index as u8,
                });
            }
        }
    }
    None
}

/// Returns `Some(RunOutcome)` if `step` is an iteration verb. Returns
/// `None` if `step` is not an iteration verb (the caller continues
/// dispatching).
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> Option<RunOutcome> {
    match step {
        CompiledStep::ForEach {
            over,
            bind_as,
            body,
        } => {
            // `scan` returns positional `PermanentHandle`s ({player, index}).
            // Those indices are invalidated the moment a body step removes a
            // permanent — every higher-indexed permanent shifts down by one,
            // so a later snapshotted handle becomes stale (points at a
            // different permanent, or out of bounds). To make `ForEach`
            // correct for ALL bodies (delete, return-to-hand, bounce, etc.),
            // snapshot each matched permanent's STABLE identity — the top
            // card's `CardHandle` (`card_index` is unique per game instance)
            // — and re-resolve the positional handle at the START of each
            // iteration. If the permanent has left the field since the
            // snapshot, its iteration is skipped (the match no longer
            // exists). See G-FOR-EACH-DELETE-INDEX-SHIFT.
            let matches = scan(ctx, over, Some(bindings));
            let snapshot: Vec<CardHandle> = matches
                .iter()
                .filter_map(|h| top_card_handle(ctx, *h))
                .collect();
            for stable_id in snapshot {
                let Some(handle) = resolve_by_top_card(ctx, stable_id) else {
                    // Permanent left the field (e.g. an earlier iteration's
                    // body deleted it). Nothing to iterate over — skip.
                    continue;
                };
                let mut iter_bindings = bindings.clone();
                let result_cursor = iter_bindings.result_log_cursor();
                iter_bindings.insert_permanent(bind_as, handle);
                let outcome = run_steps_with_runtime(body, ctx, &mut iter_bindings, runtime);
                bindings.merge_result_log_delta_from(&iter_bindings, result_cursor);
                if matches!(outcome, RunOutcome::Parked) {
                    // v1 semantics: a parked iteration aborts remaining
                    // iterations. Faithful per-iteration resumption is
                    // a future-phase enhancement.
                    return Some(RunOutcome::Parked);
                }
            }
            Some(RunOutcome::Synchronous)
        }
        CompiledStep::PerSelected {
            selection,
            bind_as,
            body,
        } => {
            let bref = CompiledBindingRef::Named(selection.clone());
            match resolve_binding_ref(&bref, ctx, bindings) {
                Some(ResolvedBinding::PermanentList(v)) => {
                    for h in v {
                        let mut iter_bindings = bindings.clone();
                        let result_cursor = iter_bindings.result_log_cursor();
                        iter_bindings.insert_permanent(bind_as, h);
                        let outcome =
                            run_steps_with_runtime(body, ctx, &mut iter_bindings, runtime);
                        bindings.merge_result_log_delta_from(&iter_bindings, result_cursor);
                        if matches!(outcome, RunOutcome::Parked) {
                            return Some(RunOutcome::Parked);
                        }
                    }
                }
                Some(ResolvedBinding::CardList(v)) => {
                    for c in v {
                        let mut iter_bindings = bindings.clone();
                        let result_cursor = iter_bindings.result_log_cursor();
                        iter_bindings.insert_card(bind_as, c);
                        let outcome =
                            run_steps_with_runtime(body, ctx, &mut iter_bindings, runtime);
                        bindings.merge_result_log_delta_from(&iter_bindings, result_cursor);
                        if matches!(outcome, RunOutcome::Parked) {
                            return Some(RunOutcome::Parked);
                        }
                    }
                }
                Some(ResolvedBinding::SourceRefs(v)) => {
                    // `per_selected` over a `select_own_sources` binding — runs
                    // the body once per selected/trashed source. Each source's
                    // top card is bound as a Card so the body can reference it
                    // if needed (EX4-073 clause C's body does not — it only
                    // needs the iteration count). Used for "for each card
                    // trashed, activate the effect below".
                    for src in v {
                        let mut iter_bindings = bindings.clone();
                        let result_cursor = iter_bindings.result_log_cursor();
                        iter_bindings.insert_card(bind_as, src.card);
                        let outcome =
                            run_steps_with_runtime(body, ctx, &mut iter_bindings, runtime);
                        bindings.merge_result_log_delta_from(&iter_bindings, result_cursor);
                        if matches!(outcome, RunOutcome::Parked) {
                            return Some(RunOutcome::Parked);
                        }
                    }
                }
                _ => {} // Missing or wrong-typed binding → silent no-op (2b/2c convention).
            }
            Some(RunOutcome::Synchronous)
        }
        _ => None,
    }
}
