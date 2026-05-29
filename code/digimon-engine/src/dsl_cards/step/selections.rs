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
    drain_dsl_outer_tail, drain_or_rewrap_pending_tail, resolve_player, run_steps_with_runtime,
    StepRuntime,
};
use crate::effect_context::{
    CountCappedZone, DistinctByMode, EffectContext, EffectReadContext, RevealBucketSelection,
};
use crate::enums::{CardKind, GamePhase, PlayerId};
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

fn formula_value(
    formula: &digimon_dsl::compiled::CompiledFormula,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> i32 {
    let target = ctx.source_permanent.unwrap_or(PermanentHandle {
        player: ctx.player,
        index: u8::MAX,
    });
    formula_eval::evaluate_with_bindings(formula, ctx, target, Some(bindings))
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

/// Read a permanent's value in the unit of the given field selector:
/// effective DP for DP selectors, printed play cost for play-cost selectors.
fn field_value(
    game: &crate::game::Game,
    handle: PermanentHandle,
    selector: CompiledFieldSelector,
) -> Option<i32> {
    match selector {
        CompiledFieldSelector::LowestDp | CompiledFieldSelector::HighestDp => {
            game.effective_dp(handle)
        }
        CompiledFieldSelector::LowestPlayCost | CompiledFieldSelector::HighestPlayCost => game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| i32::from(perm.top_card().play_cost(&game.card_data))),
        CompiledFieldSelector::LowestMaterialCount => game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| perm.card_sources.len().saturating_sub(1) as i32),
    }
}

fn selected_field_extreme(
    game: &crate::game::Game,
    handles: &[PermanentHandle],
    selector: CompiledFieldSelector,
) -> Option<i32> {
    let values = handles
        .iter()
        .filter_map(|h| field_value(game, *h, selector));
    match selector {
        CompiledFieldSelector::LowestDp | CompiledFieldSelector::LowestPlayCost => values.min(),
        CompiledFieldSelector::HighestDp | CompiledFieldSelector::HighestPlayCost => values.max(),
        CompiledFieldSelector::LowestMaterialCount => values.min(),
    }
}

/// Result of resolving an optional `CompiledFieldSelector` over a
/// candidate set. `Exact` carries the extreme value in the selector's
/// own unit (DP or play cost).
#[derive(Clone, Copy)]
enum SelectedField {
    Any,
    Exact(i32),
    None,
}

fn select_field_extreme(
    game: &crate::game::Game,
    handles: &[PermanentHandle],
    selector: Option<CompiledFieldSelector>,
) -> SelectedField {
    match selector {
        None => SelectedField::Any,
        Some(selector) => match selected_field_extreme(game, handles, selector) {
            Some(value) => SelectedField::Exact(value),
            None => SelectedField::None,
        },
    }
}

fn matches_selected_field(
    game: &crate::game::Game,
    handle: PermanentHandle,
    selector: Option<CompiledFieldSelector>,
    selected: SelectedField,
) -> bool {
    match selected {
        SelectedField::Any => true,
        SelectedField::Exact(want) => selector
            .and_then(|selector| field_value(game, handle, selector))
            .is_some_and(|candidate| candidate == want),
        SelectedField::None => false,
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
            cost,
            ..
        } => {
            install_select_hand(
                ctx,
                *of,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                *cost,
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
            cost,
            ..
        } => {
            install_select_trash(
                ctx,
                *of,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                *cost,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::UseOptionFromHand {
            of,
            filter,
            use_cost_lte_opponent_memory,
            optional,
            prompt,
        } => {
            install_use_option_from_hand(
                ctx,
                *of,
                filter.clone(),
                *use_cost_lte_opponent_memory,
                *optional,
                prompt.clone(),
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
        CompiledStep::TrashBottomFaceDownSourceUnderTamer { of } => {
            // Bundled activation cost: "pick one of `of`'s Tamers that carries
            // a face-down stash → trash its bottom face-down source". Pre-filter
            // the controller's permanents with the fixed predicate
            // `{ kind: tamer, has_face_down_source: true }`.
            let player = resolve_player(ctx, *of);
            let filter = CompiledPredicate {
                kind: Some(digimon_dsl::compiled::CompiledCardKind::Tamer),
                has_face_down_source: Some(true),
                ..CompiledPredicate::default()
            };
            let candidates = collect_matching_permanents(ctx, player, &filter, Some(&bindings));
            if candidates.is_empty() {
                // The cost is unpayable: no Tamer has a face-down stash. Abort
                // the clause — the dispatcher must stop and the tail (the rest
                // of the process) must NOT run. `TailAlreadyRan` here means
                // "dispatcher, stop; do not run the remaining steps".
                return InstallResult::TailAlreadyRan;
            }
            install_trash_bottom_face_down_source_under_tamer(
                ctx,
                filter,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            // With ≥1 candidate, `select_own_permanent` always installs a
            // pending selection (even a single candidate is exposed as a
            // 1-option selection — no auto-resolve), so this parks.
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
            min,
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
            // When fewer candidates exist than the required minimum, the step
            // installs nothing AND does not run the captured tail — the
            // required cost is unpayable. G-SELECT-MULTI-MIN.
            let min_unpayable = *min > 0 && candidate_count < *min as usize;
            let completes_synchronously =
                !min_unpayable && (candidate_count == 0 || max_value == 0);
            install_select_count_capped_multi(
                ctx,
                *of,
                *zone,
                *min,
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
                // min_unpayable: the required cost cannot be paid. Nothing was
                // installed and the captured tail (e.g. a later
                // `cancel_replacement`) must NOT run — report TailAlreadyRan so
                // the dispatcher stops the slice atomically. This is the
                // cost-then-cancel guard: an unpayable cost aborts the whole
                // process rather than letting the cancel fire for free.
                debug_assert!(min_unpayable);
                InstallResult::TailAlreadyRan
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
            filter,
            bind_as,
            prompt,
            optional,
            ..
        } => {
            install_select_security(
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
        CompiledStep::SelectMaterials {
            of_permanent,
            max,
            filter,
            uniqueness,
            bind_as,
            prompt,
            optional_zero,
            ..
        } => {
            let perm = match resolve_binding_ref(of_permanent, ctx, &bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                // Missing binding or wrong type: silent no-op (2b/2c convention).
                _ => return InstallResult::Continue,
            };
            let max_value = resolve_count_bound(ctx, max, &bindings);
            // Whether the carrier yields any source matching the filter at
            // install time — drives the synchronous-completion accounting
            // (mirrors `SelectCountCappedMulti`). Battle-area AND breeding-area
            // (`BREEDING_TARGET` sentinel) carriers are both handled: the
            // engine multi-pick encodes breeding sources in the
            // `BREEDING_SOURCE_SELECT` action range (Task S1.3).
            let has_candidates = has_material_candidates(ctx, perm, filter, Some(&bindings));
            let completes_synchronously = !has_candidates || max_value == 0;
            install_select_materials(
                ctx,
                perm,
                max_value,
                filter.clone(),
                map_distinct_by(*uniqueness),
                bind_as.clone(),
                prompt.clone(),
                *optional_zero,
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
        CompiledStep::SelectOwnSources {
            target,
            filter,
            min,
            max,
            bind_as,
            prompt,
            then,
        } => {
            if min > max || *max == 0 {
                return InstallResult::Continue;
            }
            // When no digivolution sources exist at all (unfiltered), and the
            // clause requires at least one pick (min > 0), the cost cannot be
            // paid: abort the outer continuation silently. For min == 0, no
            // sources means the player implicitly picks 0 — the `then` body
            // with an empty binding is a no-op anyway, so advancing to the
            // outer tail (`Continue`) is correct for that case.
            if !has_own_source_candidates(ctx) {
                return if *min > 0 {
                    InstallResult::TailAlreadyRan
                } else {
                    InstallResult::Continue
                };
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
            // The outer tail was captured inside `inner_tail` and passed
            // entirely into `install_select_own_sources`. The outer loop must
            // NOT advance into those same steps again. If a selection was
            // installed, `Parked` stops the loop. If no selection was
            // installed (callback ran synchronously for min=0, or filtered
            // candidates = 0 for min>0 so the callback was dropped), the
            // outer tail already ran or was intentionally discarded —
            // `TailAlreadyRan` stops the loop in both sub-cases.
            if ctx.game.pending_selection.is_some() {
                InstallResult::Parked
            } else {
                InstallResult::TailAlreadyRan
            }
        }
        CompiledStep::SelectOpponentSources {
            target,
            filter,
            min,
            max,
            bind_as,
            prompt,
            then,
        } => {
            if min > max || *max == 0 {
                return InstallResult::Continue;
            }
            // Mirror of SelectOwnSources early-abort logic: when no opponent
            // sources exist and min > 0, the cost cannot be paid — abort.
            if !has_opponent_source_candidates(ctx) {
                return if *min > 0 {
                    InstallResult::TailAlreadyRan
                } else {
                    InstallResult::Continue
                };
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_opponent_sources(
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
            // Same tail-capture semantics as SelectOwnSources: the outer tail
            // is embedded in inner_tail, so the outer loop must not advance
            // into those steps regardless of how the install resolved.
            if ctx.game.pending_selection.is_some() {
                InstallResult::Parked
            } else {
                InstallResult::TailAlreadyRan
            }
        }
        CompiledStep::SelectOpponentDpBudget {
            dp_budget,
            min_picks,
            filter,
            bind_as,
            prompt,
            then,
        } => {
            let dp_budget = formula_value(dp_budget, ctx, &bindings);
            if !has_opponent_dp_budget_candidates(ctx, dp_budget, filter, &bindings) {
                return InstallResult::Continue;
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_opponent_dp_budget(
                ctx,
                dp_budget,
                *min_picks,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                inner_tail,
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectOpponentPlayCostBudget {
            play_cost_budget,
            min_picks,
            filter,
            bind_as,
            prompt,
            then,
        } => {
            if !has_opponent_play_cost_budget_candidates(ctx, *play_cost_budget, filter, &bindings)
            {
                return InstallResult::Continue;
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_opponent_play_cost_budget(
                ctx,
                *play_cost_budget,
                *min_picks,
                filter.clone(),
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
            filter,
            optional,
            then,
        } => {
            if !has_own_breeding_candidate_matching(ctx, filter, Some(&bindings)) {
                return InstallResult::Continue;
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_own_breeding_permanent(
                ctx,
                bind_as.clone(),
                prompt.clone(),
                filter.clone(),
                *optional,
                inner_tail,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectUnionZone {
            of,
            zones,
            material_of,
            filter,
            bind_as,
            prompt,
            optional,
            cost,
            then,
            ..
        } => {
            use crate::selection::UnionZoneSet;
            let mut zoneset = UnionZoneSet(0);
            for z in zones {
                match z {
                    CompiledZone::Hand => zoneset |= UnionZoneSet::HAND,
                    CompiledZone::Trash => zoneset |= UnionZoneSet::TRASH,
                    CompiledZone::Material => zoneset |= UnionZoneSet::MATERIAL,
                    // Other zones not yet exposed by UnionZoneSet bitfield.
                    // Silently skip — future tasks widen engine API as needed.
                    _ => {}
                }
            }
            if zoneset.0 == 0 {
                // No supported zones: silent no-op; tail runs synchronously.
                return InstallResult::Continue;
            }
            let mut success_tail = then.clone();
            success_tail.extend_from_slice(tail);
            install_select_union_zone(
                ctx,
                *of,
                zoneset,
                material_of.clone(),
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                *cost,
                success_tail,
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

/// Opponent-side mirror of `has_own_source_candidates`: true when at least one
/// of the controller's OPPONENT's battle-area permanents has a digivolution
/// source below its top card. G-SELECT-OPPONENT-SOURCES.
fn has_opponent_source_candidates(ctx: &EffectContext<'_>) -> bool {
    let opponent = ctx.game.next_clockwise(ctx.player);
    ctx.game
        .player(opponent)
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
    // material_carrier_permanent branches battle-area vs. breeding-area
    // (BREEDING_TARGET sentinel) carriers — so a King Drasil breeding
    // carrier yields its real digivolution sources here, keeping the
    // `completes_synchronously` accounting correct for breeding carriers.
    crate::effect_context::material_carrier_permanent(ctx.game, perm)
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

fn has_opponent_dp_budget_candidates(
    ctx: &EffectContext<'_>,
    dp_budget: i32,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> bool {
    let opponent = ctx.game.next_clockwise(ctx.player);
    let read = ctx.as_read();
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
            if ctx.game.effective_dp(handle).unwrap_or(0) > dp_budget {
                return false;
            }
            eval_predicate_with_bindings(
                filter,
                &read,
                PredicateSubject::Permanent(handle),
                Some(bindings),
            )
        })
}

/// True when at least one opponent permanent satisfies the filter AND has a
/// printed play cost within the budget — at least one pickable candidate
/// exists for a `select_opponent_play_cost_budget` step.
/// G-MULTI-SELECT-OPP-PLAY-COST-SUM.
fn has_opponent_play_cost_budget_candidates(
    ctx: &EffectContext<'_>,
    play_cost_budget: i32,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> bool {
    let opponent = ctx.game.next_clockwise(ctx.player);
    let read = ctx.as_read();
    ctx.game
        .player(opponent)
        .battle_area
        .iter()
        .enumerate()
        .any(|(index, perm)| {
            if i32::from(perm.top_card().play_cost(&ctx.game.card_data)) > play_cost_budget {
                return false;
            }
            let handle = PermanentHandle {
                player: opponent,
                index: index as u8,
            };
            eval_predicate_with_bindings(
                filter,
                &read,
                PredicateSubject::Permanent(handle),
                Some(bindings),
            )
        })
}

/// The breeding area must be non-empty AND the breeding permanent must
/// satisfy the predicate. An empty predicate passes everything,
/// matching the historical contract.
fn has_own_breeding_candidate_matching(
    ctx: &EffectContext<'_>,
    filter: &CompiledPredicate,
    bindings: Option<&Bindings>,
) -> bool {
    if ctx.game.player(ctx.player).breeding_area.is_none() {
        return false;
    }
    let read = ctx.as_read();
    eval_predicate_with_bindings(
        filter,
        &read,
        PredicateSubject::BreedingPermanent(ctx.player),
        bindings,
    )
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

#[allow(clippy::too_many_arguments)]
fn install_select_hand(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    cost: bool,
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
    let tail_for_decline = Arc::clone(&tail);
    let bindings_for_decline = bindings.clone();
    let runtime_for_decline = runtime.clone();
    let trigger_for_decline = trigger_context.clone();
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
    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                // Cost-pay selects (`cost: true` in YAML) abort the clause
                // on decline: set `dsl_clause_aborted = true` so the step
                // runner short-circuits the captured tail AND any parked
                // outer tail. See `G-OPTIONAL-COST-DECLINE-ABORTS-CLAUSE`
                // and DCGO `ActivateClass.SetUpICardEffect`. Non-cost
                // optional picks keep the historical "decline runs the
                // tail" semantic so mandatory housekeeping steps (e.g.
                // `add_this_option_to_hand` after a "you may" trash play)
                // execute regardless. The captured tail is always invoked
                // — when `cost: true` it short-circuits via the flag; when
                // not, it runs the tail directly.
                if cost {
                    game.dsl_clause_aborted = true;
                }
                let mut decline_ctx = EffectContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                let mut b = bindings_for_decline.clone();
                run_tail_preserving_trigger_context(
                    &mut decline_ctx,
                    trigger_for_decline.clone(),
                    &tail_for_decline,
                    &mut b,
                    &runtime_for_decline,
                );
            }));
        }
    }
}

fn player_visible_memory(game: &crate::game::Game, player: PlayerId) -> i16 {
    if game.turn_player() == player {
        game.memory.max(0)
    } else {
        (-game.memory).max(0)
    }
}

fn install_use_option_from_hand(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    use_cost_lte_opponent_memory: bool,
    optional: bool,
    prompt: Option<String>,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let prompt = prompt.unwrap_or_else(|| "Choose an Option card to use".to_string());
    let tail_for_accept = tail.clone();
    let tail_for_decline = tail;
    let runtime_for_accept = runtime.clone();
    let runtime_for_decline = runtime;
    let bindings_for_accept = bindings.clone();
    let bindings_for_decline = bindings.clone();
    let trigger_context = ctx.game.current_trigger_context.clone();
    let trigger_for_accept = trigger_context.clone();
    let trigger_for_decline = trigger_context;
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
            let Some(card) = game.player(target_player).hand.get(idx) else {
                return false;
            };
            let kind = card.card_kind(&game.card_data);
            if !matches!(kind, CardKind::Option | CardKind::Dual) {
                return false;
            }
            if use_cost_lte_opponent_memory {
                let opponent = game.next_clockwise(player);
                let ceiling = player_visible_memory(game, opponent);
                let use_cost = card
                    .option_use_cost(&game.card_data)
                    .unwrap_or_else(|| card.play_cost(&game.card_data))
                    as i16;
                if use_cost > ceiling {
                    return false;
                }
            }
            let read_ctx = EffectReadContext::new_with_source_kind(
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
        move |cb_ctx, idx| {
            let previous = cb_ctx.game.current_trigger_context.clone();
            cb_ctx.game.current_trigger_context = trigger_for_accept.clone();
            let result = cb_ctx
                .game
                .use_option_from_hand_without_paying_cost(target_player, idx);
            cb_ctx.game.current_trigger_context = previous;
            if matches!(result, crate::selection::OptionPlayResult::Invalid) {
                return;
            }
            drain_or_rewrap_pending_tail(
                cb_ctx.game,
                source_card,
                source_permanent,
                player,
                tail_for_accept,
                bindings_for_accept,
                runtime_for_accept,
            );
        },
    );

    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                let previous = game.current_trigger_context.clone();
                game.current_trigger_context = trigger_for_decline.clone();
                drain_or_rewrap_pending_tail(
                    game,
                    source_card,
                    source_permanent,
                    player,
                    tail_for_decline,
                    bindings_for_decline,
                    runtime_for_decline,
                );
                game.current_trigger_context = previous;
            }));
        }
    }
}

/// Count how many hand cards of `of`'s player satisfy `filter` under the
/// supplied bindings. Used by the `- optional: [...]` substep guard to
/// detect an empty-candidate leading `select_hand` (G-SELECT-EMPTY-OUTER-TAIL):
/// when zero candidates exist the entire optional substep body is skipped
/// rather than silently falling through to subsequent mandatory steps.
pub(crate) fn select_hand_candidate_count(
    ctx: &EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> usize {
    let target_player = resolve_player(ctx, of);
    let read_ctx = ctx.as_read();
    ctx.game
        .player(target_player)
        .hand
        .iter()
        .filter(|card| {
            eval_predicate_with_bindings(
                filter,
                &read_ctx,
                PredicateSubject::Card(card.handle()),
                Some(bindings),
            )
        })
        .count()
}

#[allow(clippy::too_many_arguments)]
fn install_select_trash(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    cost: bool,
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
    // Clone state for the decline path before the success callback consumes
    // them. G-OPTIONAL-SELECTION-CONTINUE-TAIL — declining an optional
    // trash selection must still run the outer tail so subsequent
    // mandatory steps (e.g. `add_this_option_to_hand`) execute.
    let tail_for_decline = Arc::clone(&tail);
    let bindings_for_decline = bindings.clone();
    let runtime_for_decline = runtime.clone();
    let trigger_for_decline = trigger_context.clone();
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
    // Attach decline-tail callback for optional selections.
    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                // See `install_select_hand` for the cost vs. continue-tail
                // distinction. `cost: true` sets the abort flag and the
                // captured tail short-circuits; otherwise the existing
                // mandatory-tail semantics for `G-OPTIONAL-SELECTION-CONTINUE-TAIL`
                // (e.g. `add_this_option_to_hand` after a "you may" trash
                // play) keep working.
                if cost {
                    game.dsl_clause_aborted = true;
                }
                let mut decline_ctx = EffectContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                let mut b = bindings_for_decline.clone();
                run_tail_preserving_trigger_context(
                    &mut decline_ctx,
                    trigger_for_decline.clone(),
                    &tail_for_decline,
                    &mut b,
                    &runtime_for_decline,
                );
            }));
        }
    }
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
    let selected_field = select_field_extreme(ctx.game, &candidates, selector);
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
            ) && matches_selected_field(game, handle, selector, selected_field)
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

/// Install the Tamer pick for `trash_bottom_face_down_source_under_tamer`.
///
/// Mirrors `install_select_own_permanent` but with a fixed predicate and no
/// `selector` / `bind_as` / `optional`. The empty-candidates short-circuit is
/// NOT here — the caller (`try_install`) already handled the no-eligible-Tamer
/// case by returning `TailAlreadyRan`. The pick is mandatory (`optional:
/// false`): once the player activates the clause, they must choose which
/// eligible Tamer pays the cost. Whether to activate/decline at all is the
/// clause's own `optional`, governed one level up.
///
/// The callback trashes the picked Tamer's bottom face-down source via
/// `EffectContext::trash_bottom_face_down_source`; the captured tail runs ONLY
/// if that trash succeeded. The eligibility filter (`has_face_down_source`)
/// matches a Tamer if ANY source is face-down, but `trash_bottom_face_down_source`
/// only succeeds when `card_sources[0]` (the BOTTOM) is face-down. In the
/// current ST-23/ST-24 pool these always agree (stashes insert at index 0), but
/// this substrate is reusable: guarding the tail on the trash's `bool` keeps a
/// future filter/action desync from running the effect without paying the cost
/// (a no-approximations violation). The `debug_assert!` makes such a desync
/// fail loudly in dev/test; production degrades gracefully by skipping the tail.
fn install_trash_bottom_face_down_source_under_tamer(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
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
    ctx.select_own_permanent(
        "Choose a Tamer to trash a face-down card from under",
        false,
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
            )
        },
        move |cb_ctx, handle: PermanentHandle| {
            let trashed = cb_ctx.trash_bottom_face_down_source(handle);
            debug_assert!(
                trashed,
                "trash_bottom_face_down_source_under_tamer: eligibility filter \
                 (has_face_down_source) offered a Tamer whose bottom source is not \
                 face-down — filter and action have desynced"
            );
            // No-approximations: the tail (the effect) runs ONLY if the cost was
            // actually paid. A `false` return means nothing was trashed, so the
            // tail must not run.
            if trashed {
                let mut b = bindings.clone();
                run_tail_preserving_trigger_context(
                    cb_ctx,
                    trigger_context,
                    &tail,
                    &mut b,
                    &runtime,
                );
            }
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
    let selected_field = select_field_extreme(ctx.game, &candidates, selector);
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
                && matches_selected_field(game, handle, selector, selected_field)
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
    let selected_field = select_field_extreme(ctx.game, &handles, selector);
    let candidates: Vec<(u16, PermanentHandle)> = handles
        .into_iter()
        .filter(|handle| matches_selected_field(ctx.game, *handle, selector, selected_field))
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
#[allow(clippy::too_many_arguments)]
fn install_select_count_capped_multi(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    zone: CompiledZone,
    min: u8,
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
            min,
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
    ctx.select_count_capped_multi_min(
        target_player,
        engine_zone,
        min,
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
    min: u8,
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
    // Fewer candidates than the required minimum → unpayable required cost;
    // silently no-op without running the tail. G-SELECT-MULTI-MIN.
    if min > 0 && candidates.len() < min as usize {
        return;
    }
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
        min,
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
#[allow(clippy::too_many_arguments)]
fn install_count_capped_permanent_step(
    game: &mut crate::game::Game,
    candidates: Vec<(u16, PermanentHandle)>,
    accum: Vec<PermanentHandle>,
    min: u8,
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
    // PASS gating: the player may commit early once `picked` reaches the
    // required floor. G-SELECT-MULTI-MIN.
    let effective_min = min.max(if optional_zero { 0 } else { 1 });
    let is_optional = picked >= effective_min;
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
                min,
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
    // Snapshot the source identity so the per-index filter closure can build
    // an `EffectReadContext` to evaluate the `CompiledPredicate` (mirrors
    // `install_select_material`). Without this the `filter` field on
    // `SelectSecurity` was silently ignored — every security card matched.
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    ctx.select_security(
        target_player,
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game.player(target_player).security.get(idx) else {
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
            // material_carrier_permanent branches battle vs. breeding carrier.
            let Some(card) = crate::effect_context::material_carrier_permanent(game, perm)
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
                // material_carrier_permanent branches battle vs. breeding.
                if let Some(card) =
                    crate::effect_context::material_carrier_permanent(cb_ctx.game, perm)
                        .and_then(|p| p.card_sources.get(src_idx))
                {
                    b.insert_card(name, card.handle());
                }
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

/// Install a count-capped / name-unique multi-pick over `perm`'s
/// digivolution-source stack. The batch sibling of `install_select_material`.
///
/// Lowers to `EffectContext::select_count_capped_multi` with
/// `CountCappedZone::Material(perm)` — REUSING the existing count-capped
/// selection mask/decoder (no `ACTION_SPACE_SIZE` change). `uniqueness`
/// is forwarded as the engine's `DistinctByMode`, which shapes the legal
/// action mask after each pick (no auto-selection — every pick is a
/// player choice, per CLAUDE.md §17).
///
/// The picked sources are bound as a `CardList`, so a follow-on
/// `play_from_materials` can consume the whole batch.
#[allow(clippy::too_many_arguments)]
fn install_select_materials(
    ctx: &mut EffectContext<'_>,
    perm: PermanentHandle,
    max: u8,
    filter: CompiledPredicate,
    uniqueness: Option<DistinctByMode>,
    bind_as: Option<String>,
    prompt: String,
    optional_zero: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    if max == 0 {
        // Zero-cap: bind an empty pick list and run the tail synchronously.
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            b.insert_card_list(name, Vec::new());
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

    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    ctx.select_count_capped_multi(
        // The carrier's owner — `Material` candidates come from
        // `perm.player`'s battle area regardless of `of_player`.
        perm.player,
        CountCappedZone::Material(perm),
        max,
        &prompt,
        optional_zero,
        uniqueness,
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
                PredicateSubject::Source(source),
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

/// Opponent-side mirror of `install_select_own_sources`. The candidate set is
/// drawn from the controller's OPPONENT's battle-area stacks via
/// `EffectContext::select_opponent_sources`; `target` (when present) restricts
/// the picker to a single opponent permanent binding. G-SELECT-OPPONENT-SOURCES.
#[allow(clippy::too_many_arguments)]
fn install_select_opponent_sources(
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
    ctx.select_opponent_sources(
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
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
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
    ctx.select_opponent_permanents_by_dp_budget(
        &prompt,
        dp_budget,
        min_picks,
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
            )
        },
        move |cb_ctx, handles| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent_list(name, handles);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn install_select_opponent_play_cost_budget(
    ctx: &mut EffectContext<'_>,
    play_cost_budget: i32,
    min_picks: u8,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
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
    ctx.select_opponent_permanents_by_play_cost_budget(
        &prompt,
        play_cost_budget,
        min_picks,
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
            )
        },
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
    filter: CompiledPredicate,
    optional: bool,
    success_tail: Vec<CompiledStep>,
    decline_tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let success_tail = Arc::new(success_tail);
    let decline_tail = Arc::new(decline_tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    let tail_for_decline = Arc::clone(&decline_tail);
    let bindings_for_decline = bindings.clone();
    let runtime_for_decline = runtime.clone();
    let trigger_for_decline = trigger_context.clone();
    ctx.select_own_breeding_permanent(
        &prompt,
        optional,
        move |game, target| {
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
                PredicateSubject::BreedingPermanent(target.player),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, target| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_breeding_permanent_ref(name, target);
            }
            run_tail_preserving_trigger_context(
                cb_ctx,
                trigger_context,
                &success_tail,
                &mut b,
                &runtime,
            );
        },
    );
    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                let mut decline_ctx = EffectContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                let mut b = bindings_for_decline.clone();
                run_tail_preserving_trigger_context(
                    &mut decline_ctx,
                    trigger_for_decline.clone(),
                    &tail_for_decline,
                    &mut b,
                    &runtime_for_decline,
                );
            }));
        }
    }
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
    material_of: Option<CompiledBindingRef>,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    cost: bool,
    success_tail: Vec<CompiledStep>,
    decline_tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let material_target = material_of.as_ref().and_then(|binding| {
        match resolve_binding_ref(binding, ctx, &bindings) {
            Some(ResolvedBinding::Permanent(handle)) => Some(handle),
            _ => None,
        }
    });
    let success_tail = Arc::new(success_tail);
    let decline_tail = Arc::new(decline_tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Clone state for the decline path before the success callback consumes
    // them. G-OPTIONAL-SELECTION-CONTINUE-TAIL — declining an optional
    // union-zone selection must still run the outer tail so subsequent
    // mandatory steps (e.g. `play_union_bound_free`'s neighboring steps, or
    // a mandatory `gain_memory`) execute. Mirrors `install_select_trash`.
    let tail_for_decline = Arc::clone(&decline_tail);
    let bindings_for_decline = bindings.clone();
    let runtime_for_decline = runtime.clone();
    let trigger_for_decline = trigger_context.clone();
    ctx.select_union_zone(
        target_player,
        zoneset,
        material_target,
        &prompt,
        optional,
        // PUPPETS-G021: evaluate the compiled predicate against the card's
        // CardData (via PredicateSubject::Card) so that DP constraints such
        // as `dp_lte: 4000` work for hidden-zone (hand/trash) candidates.
        move |game, card| {
            let handle = card.handle();
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
                PredicateSubject::Card(handle),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, handle, origin| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                // PUPPETS-G014: record the origin zone alongside the handle
                // so a `play_union_bound_free` tail step can replay the card
                // from its true zone (hand, trash, or material).
                b.insert_union_card(name, handle, origin, target_player);
            }
            run_tail_preserving_trigger_context(
                cb_ctx,
                trigger_context,
                &success_tail,
                &mut b,
                &runtime,
            );
        },
    );
    // Attach decline-tail callback for optional selections — carry-over from
    // Task 6 code review. Mirrors `install_select_trash`.
    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                // See `install_select_hand` for the cost vs. continue-tail
                // distinction. `cost: true` sets the abort flag and the
                // captured decline_tail short-circuits.
                if cost {
                    game.dsl_clause_aborted = true;
                }
                let mut decline_ctx = EffectContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                let mut b = bindings_for_decline.clone();
                run_tail_preserving_trigger_context(
                    &mut decline_ctx,
                    trigger_for_decline.clone(),
                    &tail_for_decline,
                    &mut b,
                    &runtime_for_decline,
                );
            }));
        }
    }
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
//
// AUTHOR NOTE — `optional: true` semantics:
// `choose_from_reveal { optional: true }` is permissible ONLY when the
// printed card text explicitly grants the player a "may" at THAT specific
// pick (e.g. "you may add 1 ... to your hand"). When the printed text says
// "Add 1 card ..." without "may", the pick is mandatory and `optional`
// MUST be `false` (or omitted — `false` is the default). The "no eligible
// candidates" case is handled by the engine's natural fizzle path
// (`install_choose_from_reveal` returns `false` and skips the pending
// install), NOT by a player-driven decline.
//
// For the canonical "Add 1 X and 1 Y" two-pick reveal-search pattern,
// prefer `select_reveal_buckets` (see BT24-031 Elecmon's YAML for the
// reference): one combined bucket prompt, `min: 1, max: 1` per bucket,
// `no_duplicate_cards: true`, mandatory by construction.
//
// See `openspec/specs/dsl-card-scripting-vocabulary/spec.md` for the
// requirement ("`choose_from_reveal { optional: true }` requires
// printed-text 'may'") added by `fix-qa-bugs-aura-tick-reveal-picks`.
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
        CompiledRevealDestination::PlayFree => {
            ctx.play_from_reveal_free(player, handle);
        }
        CompiledRevealDestination::BottomSourceOf(_) => {
            if let Some(perm) = target_permanent {
                let _ = ctx.place_as_bottom_source(CardSourceRef::Reveal(handle), perm, false);
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
    let prompt =
        prompt.unwrap_or_else(|| "Place remaining cards on top or bottom of the deck?".to_string());
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
                let placed =
                    ctx.game
                        .return_to_deck_from_reveal(player, *handle, StackPosition::Top);
                debug_assert!(
                    placed,
                    "place_remainder_in_order: handle {:?} not found in revealed_cards",
                    handle
                );
            }
        }
        StackPosition::Bottom => {
            for handle in ordered.iter() {
                let placed =
                    ctx.game
                        .return_to_deck_from_reveal(player, *handle, StackPosition::Bottom);
                debug_assert!(
                    placed,
                    "place_remainder_in_order: handle {:?} not found in revealed_cards",
                    handle
                );
            }
        }
        StackPosition::Random => {
            for handle in ordered.iter() {
                let placed =
                    ctx.game
                        .return_to_deck_from_reveal(player, *handle, StackPosition::Random);
                debug_assert!(
                    placed,
                    "place_remainder_in_order: handle {:?} not found in revealed_cards",
                    handle
                );
            }
        }
    }
}
