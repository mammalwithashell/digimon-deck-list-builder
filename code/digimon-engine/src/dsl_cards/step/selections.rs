//! Selection-step lowering: install a `PendingSelection` with the
//! remainder of the process-step slice as its callback.
//!
//! Phase 2b: `SelectHand`, `SelectTrash`, `SelectOwnPermanent`,
//! `SelectOpponentPermanent`.
//!
//! **Known limitation (Phase 2b):** the `EffectContext::select_*` filter
//! closure is `Fn(&Game, ...) -> bool`, not `Fn(&EffectReadContext, ...)`.
//! Evaluating a `CompiledPredicate` needs the full read-context tuple
//! (`source_card`, `source_permanent`, `player`), so Phase 2b accepts
//! all candidates at install time. Phase 2c widens the filter signature.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCountBound, CompiledFieldSelector, CompiledPlayerRef,
    CompiledPredicate, CompiledRemainderDestination, CompiledRevealDestination, CompiledStep,
    CompiledZone,
};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::formula_eval;
use crate::dsl_cards::predicate::{eval_predicate, eval_predicate_with_bindings, PredicateSubject};
use crate::dsl_cards::step::{
    drain_dsl_outer_tail, resolve_player, run_steps_with_runtime, StepRuntime,
};
use crate::effect_context::{
    CountCappedZone, DistinctByMode, EffectContext, EffectReadContext, RevealBucketSelection,
};
use crate::enums::GamePhase;
use crate::permanent::PermanentHandle;
use crate::selection::{PendingSelection, SelectionKind};
use crate::trigger_context::TriggerContext;

fn map_distinct_by(d: Option<digimon_dsl::compiled::CompiledDistinctBy>) -> Option<DistinctByMode> {
    use digimon_dsl::compiled::CompiledDistinctBy;
    d.map(|c| match c {
        CompiledDistinctBy::CardNumber => DistinctByMode::CardNumber,
        CompiledDistinctBy::Level => DistinctByMode::Level,
        CompiledDistinctBy::Name => DistinctByMode::Name,
    })
}

fn collect_matching_permanents(
    ctx: &EffectContext<'_>,
    player: u8,
    filter: &CompiledPredicate,
    bindings: Option<&Bindings>,
) -> Vec<PermanentHandle> {
    let read = ctx.as_read();
    let mut handles = Vec::new();
    for index in 0..read.game.player(player).battle_area.len() {
        let handle = PermanentHandle {
            player,
            index: index as u8,
        };
        if eval_predicate_with_bindings(
            filter,
            &read,
            PredicateSubject::Permanent(handle),
            bindings,
        ) {
            handles.push(handle);
        }
    }
    handles
}

fn collect_matching_any_permanents(
    ctx: &EffectContext<'_>,
    excluded: Option<PermanentHandle>,
    filter: &CompiledPredicate,
    bindings: Option<&Bindings>,
) -> Vec<PermanentHandle> {
    let read = ctx.as_read();
    let mut handles = Vec::new();
    for player in 0..read.game.players.len() {
        let player = player as u8;
        for index in 0..read.game.player(player).battle_area.len() {
            let handle = PermanentHandle {
                player,
                index: index as u8,
            };
            if Some(handle) == excluded {
                continue;
            }
            if eval_predicate_with_bindings(
                filter,
                &read,
                PredicateSubject::Permanent(handle),
                bindings,
            ) {
                handles.push(handle);
            }
        }
    }
    handles
}

fn selected_dp_extreme(
    game: &crate::game::Game,
    handles: &[PermanentHandle],
    selector: CompiledFieldSelector,
) -> Option<i32> {
    let values = handles.iter().filter_map(|h| game.effective_dp(*h));
    match selector {
        CompiledFieldSelector::LowestDp => values.min(),
        CompiledFieldSelector::HighestDp => values.max(),
    }
}

#[derive(Clone, Copy)]
enum SelectedDp {
    Any,
    Exact(i32),
    None,
}

fn select_dp_extreme(
    game: &crate::game::Game,
    handles: &[PermanentHandle],
    selector: Option<CompiledFieldSelector>,
) -> SelectedDp {
    match selector {
        None => SelectedDp::Any,
        Some(selector) => match selected_dp_extreme(game, handles, selector) {
            Some(dp) => SelectedDp::Exact(dp),
            None => SelectedDp::None,
        },
    }
}

fn matches_selected_dp(
    game: &crate::game::Game,
    handle: PermanentHandle,
    selected_dp: SelectedDp,
) -> bool {
    match selected_dp {
        SelectedDp::Any => true,
        SelectedDp::Exact(dp) => game
            .effective_dp(handle)
            .is_some_and(|candidate_dp| candidate_dp == dp),
        SelectedDp::None => false,
    }
}

fn run_tail_preserving_trigger_context(
    cb_ctx: &mut EffectContext<'_>,
    trigger_context: Option<TriggerContext>,
    tail: &[CompiledStep],
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) {
    let previous = cb_ctx.game.current_trigger_context.clone();
    cb_ctx.game.current_trigger_context = trigger_context;
    run_steps_with_runtime(tail, cb_ctx, bindings, runtime);
    drain_dsl_outer_tail(cb_ctx);
    cb_ctx.game.current_trigger_context = previous;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallResult {
    NotSelection,
    Continue,
    TailAlreadyRan,
    Parked,
}

fn selection_result(ctx: &EffectContext<'_>) -> InstallResult {
    if ctx.game.pending_selection.is_some() {
        InstallResult::Parked
    } else {
        InstallResult::Continue
    }
}

/// Installs a selection step or reports how the dispatcher should advance.
///
/// Most selection steps park by installing the remainder as their callback.
/// Some unsupported or empty selections no-op and let the dispatcher continue
/// with the next step. Reveal-bucket selections can complete synchronously and
/// run the captured tail from inside their callback.
pub fn try_install(
    step: &CompiledStep,
    tail: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: Bindings,
    runtime: &StepRuntime,
) -> InstallResult {
    match step {
        CompiledStep::SelectHand {
            of,
            filter,
            bind_as,
            prompt,
            optional,
            ..
        } => {
            install_select_hand(
                ctx,
                *of,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectTrash {
            of,
            filter,
            bind_as,
            prompt,
            optional,
            ..
        } => {
            install_select_trash(
                ctx,
                *of,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectOwnPermanent {
            filter,
            bind_as,
            selector,
            prompt,
            optional,
            ..
        } => {
            install_select_own_permanent(
                ctx,
                filter.clone(),
                *selector,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectOpponentPermanent {
            filter,
            bind_as,
            selector,
            prompt,
            optional,
            ..
        } => {
            install_select_opponent_permanent(
                ctx,
                filter.clone(),
                *selector,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectAnyPermanent {
            filter,
            bind_as,
            selector,
            prompt,
            optional,
            ..
        } => {
            install_select_any_permanent(
                ctx,
                filter.clone(),
                None,
                *selector,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectDnaPair {
            left_filter,
            right_filter,
            bind_left_as,
            bind_right_as,
            prompt,
            optional,
            ..
        } => {
            install_select_dna_pair(
                ctx,
                left_filter.clone(),
                right_filter.clone(),
                bind_left_as.clone(),
                bind_right_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectCountCappedMulti {
            of,
            zone,
            max,
            bind_as,
            prompt,
            optional_zero,
            distinct_by,
            filter,
            ..
        } => {
            let target_player = resolve_player(ctx, *of);
            let max_value = resolve_count_bound(ctx, max, &bindings);
            let Some(candidate_count) =
                count_capped_candidate_count(ctx, target_player, *zone, filter, Some(&bindings))
            else {
                return InstallResult::Continue;
            };
            let completes_synchronously = candidate_count == 0 || max_value == 0;
            install_select_count_capped_multi(
                ctx,
                *of,
                *zone,
                max_value,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional_zero,
                map_distinct_by(*distinct_by),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            if ctx.game.pending_selection.is_some() {
                InstallResult::Parked
            } else if completes_synchronously {
                InstallResult::TailAlreadyRan
            } else {
                InstallResult::Continue
            }
        }
        CompiledStep::SelectEffectChoice {
            labels,
            bind_as,
            prompt,
            ..
        } => {
            install_select_effect_choice(
                ctx,
                labels.clone(),
                bind_as.clone(),
                prompt.clone(),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectReveal {
            of,
            filter,
            bind_as,
            prompt,
            optional,
            ..
        } => {
            return if install_select_reveal(
                ctx,
                *of,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            ) {
                InstallResult::Parked
            } else {
                InstallResult::Continue
            };
        }
        CompiledStep::SelectRevealBuckets {
            from,
            buckets,
            no_duplicate_cards,
            prompt,
        } => {
            return install_select_reveal_buckets(
                ctx,
                from,
                buckets,
                *no_duplicate_cards,
                prompt.clone(),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
        }
        CompiledStep::SelectSecurity {
            of,
            bind_as,
            prompt,
            optional,
            ..
        } => {
            install_select_security(
                ctx,
                *of,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectMaterial {
            of_permanent,
            filter,
            bind_as,
            prompt,
            optional,
            ..
        } => {
            use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
            let perm = match resolve_binding_ref(of_permanent, ctx, &bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                // Missing binding or wrong type: silent no-op (2b/2c convention).
                _ => return InstallResult::Continue,
            };
            if !has_material_candidates(ctx, perm, filter, Some(&bindings)) {
                return InstallResult::Continue;
            }
            install_select_material(
                ctx,
                perm,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectOwnSources {
            target,
            filter,
            min,
            max,
            bind_as,
            prompt,
            then,
        } => {
            if min > max || *max == 0 || !has_own_source_candidates(ctx) {
                return InstallResult::Continue;
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_own_sources(
                ctx,
                filter.clone(),
                *min,
                *max,
                target.clone(),
                bind_as.clone(),
                prompt.clone(),
                inner_tail,
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectOpponentDpBudget {
            dp_budget,
            min_picks,
            bind_as,
            prompt,
            then,
        } => {
            if !has_opponent_dp_budget_candidates(ctx, *dp_budget) {
                return InstallResult::Continue;
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_opponent_dp_budget(
                ctx,
                *dp_budget,
                *min_picks,
                bind_as.clone(),
                prompt.clone(),
                inner_tail,
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectOwnBreedingPermanent {
            bind_as,
            prompt,
            then,
        } => {
            if !has_own_breeding_candidate(ctx) {
                return InstallResult::Continue;
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_own_breeding_permanent(
                ctx,
                bind_as.clone(),
                prompt.clone(),
                inner_tail,
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectUnionZone {
            of,
            zones,
            bind_as,
            prompt,
            optional,
            ..
        } => {
            use crate::selection::UnionZoneSet;
            let mut zoneset = UnionZoneSet(0);
            for z in zones {
                match z {
                    CompiledZone::Hand => zoneset |= UnionZoneSet::HAND,
                    CompiledZone::Trash => zoneset |= UnionZoneSet::TRASH,
                    // Other zones not yet exposed by UnionZoneSet bitfield.
                    // Silently skip — Phase 2f+ widens engine API as needed.
                    _ => {}
                }
            }
            if zoneset.0 == 0 {
                // No supported zones: silent no-op; tail runs synchronously.
                return InstallResult::Continue;
            }
            install_select_union_zone(
                ctx,
                *of,
                zoneset,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectOrderedPermutation {
            items,
            bind_as,
            prompt,
            ..
        } => {
            use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
            let item_list = match resolve_binding_ref(items, ctx, &bindings) {
                Some(ResolvedBinding::CardList(v)) => v,
                // Missing binding or wrong type: silent no-op.
                _ => return InstallResult::Continue,
            };
            let completes_synchronously = item_list.is_empty();
            install_select_ordered_permutation(
                ctx,
                item_list,
                bind_as.clone(),
                prompt.clone(),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            if ctx.game.pending_selection.is_some() {
                InstallResult::Parked
            } else if completes_synchronously {
                InstallResult::TailAlreadyRan
            } else {
                InstallResult::Continue
            }
        }
        // Phase 2 Track E (2026-05-17): pick one revealed card, route to a
        // typed destination. Lowers as a single `select_reveal` install whose
        // callback dispatches to the destination's engine helper.
        CompiledStep::ChooseFromReveal {
            of,
            filter,
            destination,
            bind_as,
            prompt,
            optional,
            ..
        } => {
            if install_choose_from_reveal(
                ctx,
                *of,
                filter.clone(),
                destination.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            ) {
                InstallResult::Parked
            } else {
                InstallResult::Continue
            }
        }
        // Phase 2 Track E (2026-05-17): place reveal pool back onto deck. When
        // multiple destinations are listed, prompt the player to choose; the
        // permutation is always exposed (no auto-determinism, Working Rule §17).
        CompiledStep::OrderRemainder {
            of,
            destinations,
            prompt,
            ..
        } => install_order_remainder(
            ctx,
            *of,
            destinations.clone(),
            prompt.clone(),
            tail.to_vec(),
            bindings,
            runtime.clone(),
        ),
        _ => InstallResult::NotSelection,
    }
}

fn has_own_source_candidates(ctx: &EffectContext<'_>) -> bool {
    ctx.game
        .player(ctx.player)
        .battle_area
        .iter()
        .any(|perm| perm.card_sources.len() > 1)
}

fn has_material_candidates(
    ctx: &EffectContext<'_>,
    perm: PermanentHandle,
    filter: &CompiledPredicate,
    bindings: Option<&Bindings>,
) -> bool {
    let read = ctx.as_read();
    ctx.game
        .player(perm.player)
        .battle_area
        .get(perm.index as usize)
        .map(|p| {
            p.card_sources
                .iter()
                .take(p.card_sources.len().saturating_sub(1))
                .any(|card| {
                    eval_predicate_with_bindings(
                        filter,
                        &read,
                        PredicateSubject::Card(card.handle()),
                        bindings,
                    )
                })
        })
        .unwrap_or(false)
}

fn has_opponent_dp_budget_candidates(ctx: &EffectContext<'_>, dp_budget: i32) -> bool {
    let opponent = ctx.game.next_clockwise(ctx.player);
    ctx.game
        .player(opponent)
        .battle_area
        .iter()
        .enumerate()
        .any(|(index, _)| {
            let handle = PermanentHandle {
                player: opponent,
                index: index as u8,
            };
            ctx.game.effective_dp(handle).unwrap_or(0) <= dp_budget
        })
}

fn has_own_breeding_candidate(ctx: &EffectContext<'_>) -> bool {
    ctx.game.player(ctx.player).breeding_area.is_some()
}

fn resolve_count_bound(
    ctx: &EffectContext<'_>,
    max: &CompiledCountBound,
    bindings: &Bindings,
) -> u8 {
    let value = match max {
        CompiledCountBound::Literal(n) => i32::from(*n),
        CompiledCountBound::Formula(formula) => ctx
            .source_permanent
            .map(|target| {
                formula_eval::evaluate_with_bindings(formula, ctx, target, Some(bindings))
            })
            .unwrap_or(0),
    };
    value.clamp(0, 10) as u8
}

fn count_capped_candidate_count(
    ctx: &EffectContext<'_>,
    of_player: u8,
    zone: CompiledZone,
    filter: &CompiledPredicate,
    bindings: Option<&Bindings>,
) -> Option<usize> {
    match zone {
        CompiledZone::Hand => {
            let read = ctx.as_read();
            Some(
                ctx.game
                    .player(of_player)
                    .hand
                    .iter()
                    .filter(|card| {
                        eval_predicate_with_bindings(
                            filter,
                            &read,
                            PredicateSubject::Card(card.handle()),
                            bindings,
                        )
                    })
                    .count(),
            )
        }
        CompiledZone::Trash => {
            let read = ctx.as_read();
            Some(
                ctx.game
                    .player(of_player)
                    .trash
                    .iter()
                    .filter(|card| {
                        eval_predicate_with_bindings(
                            filter,
                            &read,
                            PredicateSubject::Card(card.handle()),
                            bindings,
                        )
                    })
                    .count(),
            )
        }
        CompiledZone::BattleArea => {
            Some(collect_matching_permanents(ctx, of_player, filter, bindings).len())
        }
        _ => None,
    }
}

fn install_select_hand(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    ctx.select_hand(
        target_player,
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game.player(target_player).hand.get(idx).map(|c| c.handle()) else {
                return false;
            };
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_hand_index(name, target_player, idx as u16);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

fn install_select_trash(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    ctx.select_trash(
        target_player,
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game
                .player(target_player)
                .trash
                .get(idx)
                .map(|c| c.handle())
            else {
                return false;
            };
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_trash_index(name, target_player, idx as u16);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

fn install_select_own_permanent(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
    selector: Option<CompiledFieldSelector>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    // Pre-filter candidates using the compiled predicate so that an empty
    // result (e.g. "kind: token" with no tokens on field) short-circuits
    // without installing a PendingSelection. Mirrors install_select_any_permanent.
    let target_player = ctx.player;
    let candidates = collect_matching_permanents(ctx, target_player, &filter, Some(&bindings));
    let selected_dp = select_dp_extreme(ctx.game, &candidates, selector);
    if candidates.is_empty() {
        return;
    }

    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    ctx.select_own_permanent(
        &prompt,
        optional,
        move |game, handle| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Permanent(handle),
                Some(&filter_bindings),
            ) && matches_selected_dp(game, handle, selected_dp)
        },
        move |cb_ctx, handle: PermanentHandle| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent(name, handle);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

fn install_select_opponent_permanent(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
    selector: Option<CompiledFieldSelector>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    // Pre-filter candidates using the compiled predicate so that an empty
    // result short-circuits without installing a PendingSelection.
    let opponent = ctx.game.next_clockwise(ctx.player);
    let candidates: Vec<_> = collect_matching_permanents(ctx, opponent, &filter, Some(&bindings))
        .into_iter()
        .filter(|handle| !ctx.game.progress_excludes(*handle, Some(ctx.player)))
        .collect();
    let selected_dp = select_dp_extreme(ctx.game, &candidates, selector);
    if candidates.is_empty() {
        return;
    }

    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    ctx.select_opponent_permanent(
        &prompt,
        optional,
        move |game, handle| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            !game.progress_excludes(handle, Some(player))
                && eval_predicate_with_bindings(
                    &filter,
                    &read_ctx,
                    PredicateSubject::Permanent(handle),
                    Some(&filter_bindings),
                )
                && matches_selected_dp(game, handle, selected_dp)
        },
        move |cb_ctx, handle: PermanentHandle| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent(name, handle);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn install_select_any_permanent(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
    excluded: Option<PermanentHandle>,
    selector: Option<CompiledFieldSelector>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    use crate::action::space::encode_attack;

    let handles = collect_matching_any_permanents(ctx, excluded, &filter, Some(&bindings));
    let selected_dp = select_dp_extreme(ctx.game, &handles, selector);
    let candidates: Vec<(u16, PermanentHandle)> = handles
        .into_iter()
        .filter(|handle| matches_selected_dp(ctx.game, *handle, selected_dp))
        .map(|handle| {
            (
                encode_attack(handle.player as u16, handle.index as u16),
                handle,
            )
        })
        .collect();

    if candidates.is_empty() {
        return;
    }

    let valid_action_ids = candidates.iter().map(|(action, _)| *action).collect();
    let selecting_player = ctx.override_selecting_player().unwrap_or(ctx.player);
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();

    let previous_phase = ctx.game.current_phase;
    ctx.game.current_phase = GamePhase::SelectTarget;
    ctx.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::Target,
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: optional,
        prompt,
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, action_id| {
            let Some((_, handle)) = candidates
                .iter()
                .find(|(candidate_action, _)| *candidate_action == action_id)
                .copied()
            else {
                return;
            };

            let mut cb_ctx = EffectContext::new_with_source_kind_and_override(
                game,
                source_card,
                source_permanent,
                source_kind,
                controller,
                override_pin,
            );
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent(name, handle);
            }
            run_tail_preserving_trigger_context(
                &mut cb_ctx,
                trigger_context,
                &tail,
                &mut b,
                &runtime,
            );
        }),
        on_decline: None,
    });
}

#[allow(clippy::too_many_arguments)]
fn install_select_dna_pair(
    ctx: &mut EffectContext<'_>,
    left_filter: CompiledPredicate,
    right_filter: CompiledPredicate,
    bind_left_as: String,
    bind_right_as: String,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    use crate::action::space::encode_attack;

    let candidates: Vec<(u16, PermanentHandle)> = {
        let read = ctx.as_read();
        let mut candidates = Vec::new();
        for player in 0..read.game.players.len() {
            let player = player as u8;
            for index in 0..read.game.player(player).battle_area.len() {
                let handle = PermanentHandle {
                    player,
                    index: index as u8,
                };
                if eval_predicate(&left_filter, &read, PredicateSubject::Permanent(handle)) {
                    candidates.push((encode_attack(player as u16, index as u16), handle));
                }
            }
        }
        candidates
    };

    if candidates.is_empty() {
        return;
    }

    let valid_action_ids = candidates.iter().map(|(action, _)| *action).collect();
    let selecting_player = ctx.override_selecting_player().unwrap_or(ctx.player);
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let previous_phase = ctx.game.current_phase;

    ctx.game.current_phase = GamePhase::SelectTarget;
    ctx.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::Target,
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: optional,
        prompt: prompt.clone(),
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, action_id| {
            let Some((_, left)) = candidates
                .iter()
                .find(|(candidate_action, _)| *candidate_action == action_id)
                .copied()
            else {
                return;
            };

            let mut cb_ctx = EffectContext::new_with_source_kind_and_override(
                game,
                source_card,
                source_permanent,
                source_kind,
                controller,
                override_pin,
            );
            let mut b = bindings.clone();
            b.insert_permanent(&bind_left_as, left);
            install_select_any_permanent(
                &mut cb_ctx,
                right_filter,
                Some(left),
                None,
                Some(bind_right_as),
                prompt,
                optional,
                tail,
                b,
                runtime,
            );
        }),
        on_decline: None,
    });
}

#[allow(clippy::too_many_arguments)]
fn install_select_count_capped_multi(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    zone: CompiledZone,
    max: u8,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional_zero: bool,
    distinct_by: Option<DistinctByMode>,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    if max == 0 {
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            match zone {
                CompiledZone::BattleArea => b.insert_permanent_list(name, Vec::new()),
                _ => b.insert_card_list(name, Vec::new()),
            }
        }
        run_tail_preserving_trigger_context(
            ctx,
            ctx.game.current_trigger_context.clone(),
            &tail,
            &mut b,
            &runtime,
        );
        return;
    }
    if matches!(zone, CompiledZone::BattleArea) {
        install_select_count_capped_permanents(
            ctx,
            target_player,
            max,
            filter,
            bind_as,
            prompt,
            optional_zero,
            tail,
            bindings,
            runtime,
        );
        return;
    }
    let engine_zone = match zone {
        CompiledZone::Hand => CountCappedZone::Hand,
        CompiledZone::Trash => CountCappedZone::Trash,
        // Phase 2d scope: only Hand/Trash supported. Other zones (Materials,
        // Security, Reveal, Source, Field, Deck, Breeding) silently no-op
        // here; Phase 2e+ adds the missing engine API hooks.
        _ => return,
    };
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    ctx.select_count_capped_multi(
        target_player,
        engine_zone,
        max,
        &prompt,
        optional_zero,
        distinct_by,
        move |game, card| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, picks| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_card_list(name, picks);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn install_select_count_capped_permanents(
    ctx: &mut EffectContext<'_>,
    target_player: u8,
    max: u8,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional_zero: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    use crate::action::space::encode_attack;

    let candidates: Vec<(u16, PermanentHandle)> =
        collect_matching_permanents(ctx, target_player, &filter, Some(&bindings))
            .into_iter()
            .map(|handle| {
                (
                    encode_attack(handle.player as u16, handle.index as u16),
                    handle,
                )
            })
            .collect();
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    if candidates.is_empty() {
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            b.insert_permanent_list(name, Vec::new());
        }
        run_tail_preserving_trigger_context(ctx, trigger_context, &tail, &mut b, &runtime);
        return;
    }

    let selecting_player = ctx.override_selecting_player().unwrap_or(ctx.player);
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let previous_phase = ctx.game.current_phase;
    let final_callback: Box<
        dyn FnOnce(&mut crate::game::Game, Vec<PermanentHandle>) + Send + Sync,
    > = Box::new(move |game, picks| {
        let mut cb_ctx = EffectContext::new_with_source_kind_and_override(
            game,
            source_card,
            source_permanent,
            source_kind,
            controller,
            override_pin,
        );
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            b.insert_permanent_list(name, picks);
        }
        run_tail_preserving_trigger_context(&mut cb_ctx, trigger_context, &tail, &mut b, &runtime);
    });

    install_count_capped_permanent_step(
        ctx.game,
        candidates,
        Vec::new(),
        max,
        optional_zero,
        prompt,
        source_card,
        source_permanent,
        source_kind,
        selecting_player,
        previous_phase,
        final_callback,
    );
}

#[allow(clippy::too_many_arguments)]
fn install_count_capped_permanent_step(
    game: &mut crate::game::Game,
    candidates: Vec<(u16, PermanentHandle)>,
    accum: Vec<PermanentHandle>,
    max: u8,
    optional_zero: bool,
    prompt: String,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
    selecting_player: u8,
    previous_phase: GamePhase,
    final_callback: Box<dyn FnOnce(&mut crate::game::Game, Vec<PermanentHandle>) + Send + Sync>,
) {
    use std::sync::{Arc, Mutex};

    let picked = accum.len() as u8;
    let is_optional = optional_zero || picked >= 1;
    let valid_action_ids = candidates.iter().map(|(action, _)| *action).collect();
    let shared_cb: Arc<
        Mutex<Option<Box<dyn FnOnce(&mut crate::game::Game, Vec<PermanentHandle>) + Send + Sync>>>,
    > = Arc::new(Mutex::new(Some(final_callback)));
    let shared_cb_decline = Arc::clone(&shared_cb);
    let accum_for_decline = accum.clone();

    game.current_phase = GamePhase::SelectBudgeted;
    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::CountCappedMultiSelect { max, picked },
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional,
        prompt: prompt.clone(),
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, action_id| {
            let Some((_, handle)) = candidates
                .iter()
                .find(|(candidate_action, _)| *candidate_action == action_id)
                .copied()
            else {
                return;
            };

            let mut new_accum = accum;
            new_accum.push(handle);
            if new_accum.len() == max as usize {
                if let Some(cb) = shared_cb.lock().unwrap().take() {
                    cb(game, new_accum);
                }
                return;
            }

            let new_candidates: Vec<(u16, PermanentHandle)> = candidates
                .into_iter()
                .filter(|(candidate_action, _)| *candidate_action != action_id)
                .collect();
            if new_candidates.is_empty() {
                if let Some(cb) = shared_cb.lock().unwrap().take() {
                    cb(game, new_accum);
                }
                return;
            }

            let next_cb: Box<
                dyn FnOnce(&mut crate::game::Game, Vec<PermanentHandle>) + Send + Sync,
            > = Box::new(move |game, picks| {
                if let Some(cb) = shared_cb.lock().unwrap().take() {
                    cb(game, picks);
                }
            });
            install_count_capped_permanent_step(
                game,
                new_candidates,
                new_accum,
                max,
                optional_zero,
                prompt,
                source_card,
                source_permanent,
                source_kind,
                selecting_player,
                previous_phase,
                next_cb,
            );
        }),
        on_decline: Some(Box::new(move |game| {
            if let Some(cb) = shared_cb_decline.lock().unwrap().take() {
                cb(game, accum_for_decline);
            }
        })),
    });
}

fn install_select_effect_choice(
    ctx: &mut EffectContext<'_>,
    labels: Vec<String>,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    ctx.select_effect_choice(&prompt, labels, move |cb_ctx, idx| {
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            b.insert_literal(name, idx as i64);
        }
        run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
    });
}

fn install_select_reveal(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) -> bool {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    ctx.select_reveal(
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game.revealed_cards.get(idx) else {
                return false;
            };
            if !revealed_owner_matches(of, card.owner, player, game) {
                return false;
            }
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::RevealedCard(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                // Resolve the picked reveal index into a stable CardHandle.
                if let Some(card) = cb_ctx.game.revealed_cards.get(idx) {
                    b.insert_card(name, card.handle());
                }
                // If the index has gone stale (the reveal pile mutated mid-
                // resolution — currently impossible but defensive), silently
                // skip the binding; downstream verbs that consume it no-op
                // per the 2b/2c missing-binding convention.
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    )
}

fn install_select_reveal_buckets(
    ctx: &mut EffectContext<'_>,
    from: &str,
    buckets: &[digimon_dsl::compiled::CompiledRevealBucket],
    no_duplicate_cards: bool,
    prompt: Option<String>,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) -> InstallResult {
    let source_handles = if let Some(list) = bindings.get_card_list(from) {
        list
    } else if let Some(card) = bindings.get_card(from) {
        vec![card]
    } else {
        return InstallResult::Continue;
    };

    let engine_buckets = {
        let read_ctx = ctx.as_read();
        buckets
            .iter()
            .map(|bucket| {
                let candidates = source_handles
                    .iter()
                    .copied()
                    .filter(|handle| {
                        ctx.game
                            .revealed_cards
                            .iter()
                            .any(|card| card.handle() == *handle)
                    })
                    .filter(|handle| {
                        bucket.filter.as_ref().is_none_or(|filter| {
                            eval_predicate_with_bindings(
                                filter,
                                &read_ctx,
                                PredicateSubject::RevealedCard(*handle),
                                Some(&bindings),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                RevealBucketSelection {
                    bind_as: bucket.bind_as.clone(),
                    min: bucket.min,
                    max: bucket.max,
                    candidates,
                }
            })
            .collect::<Vec<_>>()
    };

    let prompt = prompt.unwrap_or_else(|| "Choose revealed cards".to_string());
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    if ctx.select_reveal_buckets(
        engine_buckets,
        &prompt,
        no_duplicate_cards,
        move |cb_ctx, picked| {
            let mut b = bindings.clone();
            for (name, cards) in picked {
                b.insert_card_list(&name, cards);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    ) {
        InstallResult::Parked
    } else {
        InstallResult::TailAlreadyRan
    }
}

fn revealed_owner_matches(
    of: CompiledPlayerRef,
    owner: u8,
    player: u8,
    game: &crate::game::Game,
) -> bool {
    match of {
        CompiledPlayerRef::You => owner == player,
        CompiledPlayerRef::Opponent => owner == game.next_clockwise(player),
        CompiledPlayerRef::Active => owner == game.turn_player(),
        CompiledPlayerRef::Any => true,
    }
}

fn install_select_security(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    ctx.select_security(
        target_player,
        &prompt,
        optional,
        |_game, _idx| true,
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                if let Some(card) = cb_ctx.game.player(target_player).security.get(idx) {
                    b.insert_card(name, card.handle());
                }
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

fn install_select_material(
    ctx: &mut EffectContext<'_>,
    perm: PermanentHandle,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Top-card exclusion is enforced by EffectContext::select_material itself
    // (matches CountCappedZone::Material).
    ctx.select_material(
        perm,
        &prompt,
        optional,
        move |game, src_idx| {
            let Some(card) = game
                .player(perm.player)
                .battle_area
                .get(perm.index as usize)
                .and_then(|p| p.card_sources.get(src_idx))
            else {
                return false;
            };
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, src_idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                let perm_owner = perm.player;
                let perm_index = perm.index as usize;
                if let Some(card) = cb_ctx
                    .game
                    .player(perm_owner)
                    .battle_area
                    .get(perm_index)
                    .and_then(|p| p.card_sources.get(src_idx))
                {
                    b.insert_card(name, card.handle());
                }
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

fn install_select_own_sources(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
    min: u8,
    max: u8,
    target: Option<CompiledBindingRef>,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    if min > max || max == 0 {
        return;
    }

    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let target_permanent =
        target
            .as_ref()
            .and_then(|target| match resolve_binding_ref(target, ctx, &bindings) {
                Some(ResolvedBinding::Permanent(handle)) => Some(handle),
                _ => None,
            });
    let target_resolution_failed = target.is_some() && target_permanent.is_none();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    ctx.select_own_sources(
        &prompt,
        min,
        max,
        move |game, source| {
            if target_resolution_failed {
                return false;
            }
            if target_permanent.is_some_and(|handle| source.permanent != handle) {
                return false;
            }
            let read = EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read,
                PredicateSubject::Card(source.card),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, source_refs| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_source_refs(name, source_refs);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn install_select_opponent_dp_budget(
    ctx: &mut EffectContext<'_>,
    dp_budget: i32,
    min_picks: u8,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    ctx.select_opponent_permanents_by_dp_budget(
        &prompt,
        dp_budget,
        min_picks,
        |_game, _handle| true,
        move |cb_ctx, handles| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent_list(name, handles);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

fn install_select_own_breeding_permanent(
    ctx: &mut EffectContext<'_>,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    ctx.select_own_breeding_permanent(
        &prompt,
        |_game, _target| true,
        move |cb_ctx, target| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_breeding_permanent_ref(name, target);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

fn install_select_ordered_permutation(
    ctx: &mut EffectContext<'_>,
    items: Vec<crate::card_source::CardHandle>,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    ctx.select_ordered_permutation(items, &prompt, move |cb_ctx, ordered| {
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            b.insert_card_list(name, ordered);
        }
        run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
    });
}

#[allow(clippy::too_many_arguments)]
fn install_select_union_zone(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    zoneset: crate::selection::UnionZoneSet,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    ctx.select_union_zone(
        target_player,
        zoneset,
        &prompt,
        optional,
        |_game, _card| true, // Phase 2e: accept-all filter.
        move |cb_ctx, handle| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_card(name, handle);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

// ───────────────────────────────────────────────────────────────────────────
// Phase 2 Track E (2026-05-17): reveal-ordering DSL verbs.
//
// `choose_from_reveal` and `order_remainder` are the author-facing combo for
// the "reveal N, pick one matching to <destination>, place rest top-or-bottom
// in any order" pattern that recurs across Rocks searchers (P-167 et al) and
// every Training / Memory Boost / search effect. Both lower as selection
// installs that consume the existing `select_reveal` / `select_effect_choice`
// / `select_ordered_permutation` engine surface — no new substrate hooks.
// ───────────────────────────────────────────────────────────────────────────

fn revealed_owner_matches_for_choose(
    of: CompiledPlayerRef,
    owner: u8,
    player: u8,
    game: &crate::game::Game,
) -> bool {
    revealed_owner_matches(of, owner, player, game)
}

/// Install a `select_reveal`-style pick that routes the chosen revealed card
/// to a typed destination on success.
///
/// Returns `true` iff a `PendingSelection` was installed (i.e. the install
/// did NOT short-circuit because no candidate revealed cards matched the
/// filter / `of`-owner constraint).
#[allow(clippy::too_many_arguments)]
fn install_choose_from_reveal(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    destination: CompiledRevealDestination,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) -> bool {
    let target_player = resolve_player(ctx, of);
    // For BottomSourceOf, resolve the target permanent now so we can short-
    // circuit later. If resolution fails (binding missing or wrong kind),
    // routing silently no-ops on the destination — matching the 2b/2c missing-
    // binding convention used elsewhere in this module.
    let target_permanent = match &destination {
        CompiledRevealDestination::BottomSourceOf(binding_ref) => {
            match resolve_binding_ref(binding_ref, ctx, &bindings) {
                Some(ResolvedBinding::Permanent(h)) => Some(h),
                _ => None,
            }
        }
        _ => None,
    };
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    let destination_for_callback = destination;
    ctx.select_reveal(
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game.revealed_cards.get(idx) else {
                return false;
            };
            if !revealed_owner_matches_for_choose(of, card.owner, player, game) {
                return false;
            }
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::RevealedCard(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            // Resolve the picked reveal index to a stable CardHandle and
            // route it to the destination. If the index has gone stale
            // (reveal pool mutated mid-resolution), silently skip routing —
            // bindings are best-effort per the 2b/2c convention.
            let picked = cb_ctx.game.revealed_cards.get(idx).map(|c| c.handle());
            if let Some(handle) = picked {
                if let Some(name) = &bind_as {
                    b.insert_card(name, handle);
                }
                route_chosen_reveal(
                    cb_ctx,
                    target_player,
                    handle,
                    &destination_for_callback,
                    target_permanent,
                );
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    )
}

/// Route a single chosen revealed card to its destination. Pure dispatch —
/// the engine helpers handle the actual zone movement and event emission.
fn route_chosen_reveal(
    ctx: &mut EffectContext<'_>,
    player: crate::enums::PlayerId,
    handle: crate::card_source::CardHandle,
    destination: &CompiledRevealDestination,
    target_permanent: Option<PermanentHandle>,
) {
    use crate::enums::StackPosition;
    use crate::CardSourceRef;
    match destination {
        CompiledRevealDestination::Hand => {
            ctx.add_to_hand_from_reveal(player, handle);
        }
        CompiledRevealDestination::DeckTop => {
            ctx.return_to_deck_from_reveal(player, handle, StackPosition::Top);
        }
        CompiledRevealDestination::DeckBottom => {
            ctx.return_to_deck_from_reveal(player, handle, StackPosition::Bottom);
        }
        CompiledRevealDestination::BottomSourceOf(_) => {
            if let Some(perm) = target_permanent {
                let _ = ctx.place_as_bottom_source(CardSourceRef::Reveal(handle), perm);
            }
        }
    }
}

/// Install `order_remainder` — drives placement of the entire reveal pool back
/// onto the controller's deck, surfacing an ordered-permutation selection for
/// the cards and (when multiple destinations are listed) a destination
/// effect-choice. Empty reveal pool → silent no-op (no selection installed).
fn install_order_remainder(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    destinations: Vec<CompiledRemainderDestination>,
    prompt: Option<String>,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) -> InstallResult {
    let target_player = resolve_player(ctx, of);
    // Empty reveal pool → silent no-op; tail proceeds inline.
    if ctx.game.revealed_cards.is_empty() {
        return InstallResult::Continue;
    }
    // Defensive: compile validated 1..=2 destinations, but treat empty as no-op.
    if destinations.is_empty() {
        return InstallResult::Continue;
    }

    // Single destination → equivalent to `place_remainder_on_deck` (which
    // surfaces ordering directly via select_ordered_permutation).
    if destinations.len() == 1 {
        let position = remainder_destination_position(destinations[0]);
        // Snapshot remainder + install ordered permutation with the placement
        // closure AND tail-run. Cannot delegate to `place_remainder_on_deck`
        // because we need to chain the tail onto the same selection callback.
        return install_remainder_permutation_with_tail(
            ctx,
            target_player,
            position,
            tail,
            bindings,
            runtime,
        );
    }

    // Multi-destination → prompt for top vs bottom, then install ordered
    // permutation with chosen position. Both selections are surfaced as
    // separate action-mask windows per Working Rule §17.
    let labels: Vec<String> = destinations
        .iter()
        .map(|d| remainder_destination_label(*d).to_string())
        .collect();
    let prompt = prompt.unwrap_or_else(|| {
        "Place remaining cards on top or bottom of the deck?".to_string()
    });
    let destinations_capture: Vec<CompiledRemainderDestination> = destinations;
    let tail_arc = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    ctx.select_effect_choice(&prompt, labels, move |cb_ctx, idx| {
        let position = remainder_destination_position(destinations_capture[idx]);
        // The select_effect_choice callback runs in a fresh selection scope
        // (pending_selection is now None). Install the ordered permutation
        // with the tail, exactly as the single-destination path does.
        let _ = install_remainder_permutation_with_tail(
            cb_ctx,
            target_player,
            position,
            (*tail_arc).clone(),
            bindings.clone(),
            runtime.clone(),
        );
        // Preserve trigger context across the inner install + tail.
        cb_ctx.game.current_trigger_context = trigger_context.clone();
    });
    if ctx.game.pending_selection.is_some() {
        InstallResult::Parked
    } else {
        InstallResult::Continue
    }
}

fn remainder_destination_position(d: CompiledRemainderDestination) -> crate::enums::StackPosition {
    match d {
        CompiledRemainderDestination::DeckTop => crate::enums::StackPosition::Top,
        CompiledRemainderDestination::DeckBottom => crate::enums::StackPosition::Bottom,
    }
}

fn remainder_destination_label(d: CompiledRemainderDestination) -> &'static str {
    match d {
        CompiledRemainderDestination::DeckTop => "Top of deck",
        CompiledRemainderDestination::DeckBottom => "Bottom of deck",
    }
}

/// Install an ordered-permutation selection over the current reveal pool that,
/// on resolution, places each card at `position` (top or bottom of `player`'s
/// deck) per the `place_remainder_on_deck` order rules, then runs `tail`.
fn install_remainder_permutation_with_tail(
    ctx: &mut EffectContext<'_>,
    player: crate::enums::PlayerId,
    position: crate::enums::StackPosition,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) -> InstallResult {
    use crate::card_source::CardHandle;
    let remainder: Vec<CardHandle> = ctx
        .game
        .revealed_cards
        .iter()
        .map(|cs| cs.handle())
        .collect();
    if remainder.is_empty() {
        return InstallResult::Continue;
    }
    debug_assert!(
        remainder.len() <= 10,
        "order_remainder: reveal pool has {} cards; select_ordered_permutation is capped at 10",
        remainder.len()
    );
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    ctx.select_ordered_permutation(
        remainder,
        "Place remaining cards on deck in any order",
        move |cb_ctx, ordered_vec| {
            place_remainder_in_order(cb_ctx, player, &ordered_vec, position);
            let mut b = bindings.clone();
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    if ctx.game.pending_selection.is_some() {
        InstallResult::Parked
    } else {
        // Empty / capped — selection completed synchronously; tail already ran.
        InstallResult::TailAlreadyRan
    }
}

/// Replicate `EffectContext::place_remainder_on_deck`'s placement loop
/// directly (we control the surrounding selection install ourselves).
///
/// - `Top`: reverse-iterate so `ordered_vec[0]` lands at the highest deck
///   index (drawn first).
/// - `Bottom`: forward-iterate with bottom inserts so `ordered_vec[0]` ends up
///   at the highest index among the placed group (drawn first of the bottom
///   group).
/// - `Random`: forward-iterate placing each at a random position. The order
///   selection is still surfaced (Working Rule §17) even though placement is
///   non-deterministic.
fn place_remainder_in_order(
    ctx: &mut EffectContext<'_>,
    player: crate::enums::PlayerId,
    ordered: &[crate::card_source::CardHandle],
    position: crate::enums::StackPosition,
) {
    use crate::enums::StackPosition;
    match position {
        StackPosition::Top => {
            for handle in ordered.iter().rev() {
                let placed = ctx.game.return_to_deck_from_reveal(player, *handle, StackPosition::Top);
                debug_assert!(
                    placed,
                    "place_remainder_in_order: handle {:?} not found in revealed_cards",
                    handle
                );
            }
        }
        StackPosition::Bottom => {
            for handle in ordered.iter() {
                let placed = ctx.game.return_to_deck_from_reveal(player, *handle, StackPosition::Bottom);
                debug_assert!(
                    placed,
                    "place_remainder_in_order: handle {:?} not found in revealed_cards",
                    handle
                );
            }
        }
        StackPosition::Random => {
            for handle in ordered.iter() {
                let placed = ctx.game.return_to_deck_from_reveal(player, *handle, StackPosition::Random);
                debug_assert!(
                    placed,
                    "place_remainder_in_order: handle {:?} not found in revealed_cards",
                    handle
                );
            }
        }
    }
}
