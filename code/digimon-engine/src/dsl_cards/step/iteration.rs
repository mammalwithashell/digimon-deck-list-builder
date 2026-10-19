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
use crate::dsl_cards::formula_eval;
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

fn repeat_formula_target(ctx: &EffectContext<'_>) -> PermanentHandle {
    ctx.source_permanent.unwrap_or(PermanentHandle {
        player: ctx.player,
        index: 0,
    })
}

fn run_repeated_effect_choice(
    remaining: u8,
    labels: &[String],
    bind_as: &Option<String>,
    prompt: &str,
    prompt_key: &Option<String>,
    body: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> RunOutcome {
    if remaining == 0 {
        return RunOutcome::Synchronous;
    }

    let mut steps = Vec::with_capacity(body.len() + 2);
    steps.push(CompiledStep::SelectEffectChoice {
        labels: labels.to_vec(),
        legal_when: None,
        bind_as: bind_as.clone(),
        prompt: prompt.to_string(),
        prompt_key: prompt_key.clone(),
    });
    steps.extend_from_slice(body);
    if remaining > 1 {
        steps.push(CompiledStep::RepeatEffectChoiceRemaining {
            remaining: remaining - 1,
            labels: labels.to_vec(),
            bind_as: bind_as.clone(),
            prompt: prompt.to_string(),
            prompt_key: prompt_key.clone(),
            body: body.to_vec(),
        });
    }
    run_steps_with_runtime(&steps, ctx, bindings, runtime)
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
            let members=scan(ctx,over,Some(bindings)).into_iter().map(|h|(h.player,ctx.game.capture_digivolve_host_identity(h))).collect::<Vec<_>>();
            Some(run_for_each_remaining(&members,bind_as,body,ctx,bindings,runtime))
        }
        CompiledStep::ForEachRemaining { members,bind_as,body } => Some(run_for_each_remaining(members,bind_as,body,ctx,bindings,runtime)),
        CompiledStep::PerSelected {
            selection,
            bind_as,
            body,
        } => {
            let bref = CompiledBindingRef::Named(selection.clone());
            match resolve_binding_ref(&bref, ctx, bindings) {
                Some(ResolvedBinding::PermanentList(v)) => {
                    // Positional `PermanentHandle`s are invalidated the moment a
                    // body step removes a permanent (e.g. `per_selected {
                    // delete_permanent }` from `select_count_capped_multi`):
                    // every higher-indexed permanent shifts down by one, so a
                    // later handle in `v` points at the wrong permanent (or out
                    // of bounds). Snapshot each selected permanent's STABLE
                    // identity (its top card's `CardHandle`) up front, then
                    // re-resolve the positional handle at the START of each
                    // iteration, skipping any that already left play. Mirrors
                    // the `ForEach` machinery above. See
                    // G-PER-SELECTED-DELETE-INDEX-SHIFT.
                    let snapshot: Vec<CardHandle> =
                        v.iter().filter_map(|h| top_card_handle(ctx, *h)).collect();
                    for stable_id in snapshot {
                        let Some(h) = resolve_by_top_card(ctx, stable_id) else {
                            // Permanent left the field (e.g. an earlier
                            // iteration's body deleted it). Skip it.
                            continue;
                        };
                        let mut iter_bindings = bindings.clone();
                        let cursor = iter_bindings.result_log_cursor();
                        iter_bindings.insert_permanent(bind_as, h);
                        let outcome =
                            run_steps_with_runtime(body, ctx, &mut iter_bindings, runtime);
                        bindings.merge_result_log_from_since(&iter_bindings, cursor);
                        if matches!(outcome, RunOutcome::Parked) {
                            return Some(RunOutcome::Parked);
                        }
                    }
                }
                Some(ResolvedBinding::CardList(v)) => {
                    for c in v {
                        let mut iter_bindings = bindings.clone();
                        let cursor = iter_bindings.result_log_cursor();
                        iter_bindings.insert_card(bind_as, c);
                        let outcome =
                            run_steps_with_runtime(body, ctx, &mut iter_bindings, runtime);
                        bindings.merge_result_log_from_since(&iter_bindings, cursor);
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
                        let cursor = iter_bindings.result_log_cursor();
                        iter_bindings.insert_card(bind_as, src.card);
                        let outcome =
                            run_steps_with_runtime(body, ctx, &mut iter_bindings, runtime);
                        bindings.merge_result_log_from_since(&iter_bindings, cursor);
                        if matches!(outcome, RunOutcome::Parked) {
                            return Some(RunOutcome::Parked);
                        }
                    }
                }
                _ => {} // Missing or wrong-typed binding → silent no-op (2b/2c convention).
            }
            Some(RunOutcome::Synchronous)
        }
        CompiledStep::RepeatEffectChoice {
            count,
            labels,
            bind_as,
            prompt,
            prompt_key,
            body,
        } => {
            let target = repeat_formula_target(ctx);
            let n = formula_eval::evaluate_with_bindings(count, ctx, target, Some(bindings))
                .clamp(0, u8::MAX as i32) as u8;
            Some(run_repeated_effect_choice(
                n, labels, bind_as, prompt, prompt_key, body, ctx, bindings, runtime,
            ))
        }
        CompiledStep::RepeatEffectChoiceRemaining {
            remaining,
            labels,
            bind_as,
            prompt,
            prompt_key,
            body,
        } => Some(run_repeated_effect_choice(
            *remaining, labels, bind_as, prompt, prompt_key, body, ctx, bindings, runtime,
        )),
        _ => None,
    }
}

/// A parked body retains each later physical incarnation and its original lexical
/// bindings. It remains one separate operation per member, never a group event.
fn run_for_each_remaining(members:&[(u8,u64)],bind_as:&str,body:&[CompiledStep],ctx:&mut EffectContext<'_>,bindings:&mut Bindings,runtime:&StepRuntime)->RunOutcome {
    for (index,(player,id)) in members.iter().enumerate() {
        let Some(handle)=ctx.game.resolve_digivolve_host_identity(*player,*id)else{continue;};
        let mut local=bindings.clone();let cursor=local.result_log_cursor();local.insert_permanent(bind_as,handle);
        let outcome=run_steps_with_runtime(body,ctx,&mut local,runtime);
        bindings.merge_result_log_from_since(&local,cursor);
        if matches!(outcome,RunOutcome::Parked) {
            if index+1<members.len() {
                let tail=vec![CompiledStep::ForEachRemaining{members:members[index+1..].to_vec(),bind_as:bind_as.into(),body:body.to_vec()}];
                let trigger=ctx.game.current_trigger_context.clone();
                super::wrap_pending_selection_with_tail(ctx.game,ctx.source_card,ctx.source_permanent,ctx.player,tail,bindings.clone(),runtime.clone(),trigger);
            }
            return RunOutcome::Parked;
        }
    }
    RunOutcome::Synchronous
}
