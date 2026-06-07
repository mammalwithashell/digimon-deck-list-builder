//! Lower `CompiledDeclarativeClause::Replacement` into a Would* engine
//! `Effect`.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCostDelta, CompiledFieldSelector, CompiledPlayerRef,
    CompiledPredicate, CompiledScope, CompiledStep, CompiledZone,
};
use std::sync::Arc;

use crate::action::space::{HAND_EFFECT_START, HAND_MAIN_LIMIT};
use crate::card_source::{CardHandle, CardSource};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::formula_eval;
use crate::dsl_cards::predicate::{eval_predicate, eval_predicate_with_bindings, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{resolve_player, run_steps_with_runtime, StepRuntime};
use crate::dsl_cards::trigger_map::lookup_replacement_trigger;
use crate::effect::{Effect, EffectBuilder};
use crate::effect_context::{DelayCostStatus, EffectContext, EffectReadContext};
use crate::enums::{EffectTiming, GamePhase, ModifierType, PlayerId};
use crate::game::Game;
use crate::permanent::{OptionState, PermanentHandle};
use crate::replacement::ReplacementSubject;
use crate::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};

/// Build the base `EffectBuilder` for a "Would*" replacement timing.
/// Returns `None` only if `timing` is not one of the known replacement
/// variants (guard against future callers passing the wrong timing).
fn new_when_would_builder(card: CardHandle, timing: EffectTiming) -> Option<EffectBuilder> {
    match timing {
        EffectTiming::WhenWouldBeDeleted
        | EffectTiming::WhenWouldLeaveBattleArea
        | EffectTiming::WhenWouldBeReturnedToHand
        | EffectTiming::WhenWouldBeReturnedToDeck
        | EffectTiming::WhenWouldBeTrashed
        | EffectTiming::WhenWouldBeDeDigivolved
        | EffectTiming::WhenWouldLoseSecurity
        | EffectTiming::WhenWouldDraw
        | EffectTiming::WhenWouldPlaceInSecurity
        | EffectTiming::WhenPermanentWouldDigivolve
        | EffectTiming::WhenPermanentWouldPlay
        | EffectTiming::WhenWouldLink => Some(EffectBuilder::new(card, timing)),
        _ => None,
    }
}

/// Lower a `Replacement` declarative clause.
///
/// `_active_when` is accepted but ignored — Phase 2 gating.
///
/// Returns `None` for unknown trigger strings (caller silently skips).
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: &str,
    optional: bool,
    once_per_turn: bool,
    process: &[CompiledStep],
) -> Option<Effect> {
    lower_with_raw(
        card,
        scope,
        active_when,
        trigger,
        optional,
        once_per_turn,
        process,
        Arc::new(EngineRawRustRegistry::new()),
    )
}

pub fn lower_with_raw(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: &str,
    optional: bool,
    once_per_turn: bool,
    process: &[CompiledStep],
    raw: Arc<EngineRawRustRegistry>,
) -> Option<Effect> {
    let timing = lookup_replacement_trigger(trigger)?;
    let label = format!("Replacement: {trigger}");
    let process: Arc<[CompiledStep]> = Arc::from(process);
    let runtime = StepRuntime::new(raw);

    let mut builder = new_when_would_builder(card, timing)?;
    builder = builder.name(&label);

    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    if optional {
        builder = builder.optional();
    }
    if once_per_turn {
        builder = builder.once_per_turn();
    }

    let active_when = active_when.cloned();
    let can_match_cross_permanent_subject = active_when
        .as_ref()
        .is_some_and(predicate_reads_replacement_subject);
    let preflight_process = process.clone();
    builder = builder.replacement_condition(move |ctx, subject| {
        source_permanent_is_still_active(ctx)
            && (can_match_cross_permanent_subject || replacement_subject_is_source(ctx, *subject))
            && active_when.as_ref().is_none_or(|active_when| {
                eval_predicate(
                    active_when,
                    ctx,
                    predicate_subject_from_replacement_subject(*subject),
                )
            })
            && replacement_process_has_payable_required_selection(&preflight_process, ctx, *subject)
    });

    if DelaySelfCancelFlow::from_process(&process).is_some() {
        builder = builder.replacement_process(move |rctx| {
            if !source_is_delayed_option(rctx.effect) {
                return;
            }

            let Some(source) = rctx.effect.source_permanent else {
                return;
            };
            let Some(source_card) = rctx.effect.permanent_top_card_handle(source) else {
                return;
            };

            let continuation = DelayCancelContinuation {
                source_player: source.player,
                source_card,
            };

            match rctx.effect.trash_delay_source_status() {
                DelayCostStatus::Paid => {
                    rctx.cancel();
                }
                DelayCostStatus::Pending => {
                    arm_pending_delay_cancel_continuation(rctx.effect.game, continuation);
                }
                DelayCostStatus::Unpaid => {}
            };
        });
        return Some(builder.build());
    }

    if let Some(delay_flow) = DelayHandDigivolveFlow::from_process(&process) {
        builder = builder.replacement_process(move |rctx| {
            if !source_is_delayed_option(rctx.effect) {
                return;
            }

            let player = resolve_player(rctx.effect, delay_flow.of);
            let Some(source) = rctx.effect.source_permanent else {
                return;
            };
            let Some(source_card) = rctx.effect.permanent_top_card_handle(source) else {
                return;
            };
            let subject_card = stable_replacement_subject_card(rctx.effect, rctx.subject);

            let continuation = DelayCostContinuation {
                source_player: source.player,
                source_card,
                subject: rctx.subject,
                subject_card,
                player,
                prompt: delay_flow.prompt.clone(),
                filter: delay_flow.filter.clone(),
            };

            match rctx.effect.trash_delay_source_status() {
                DelayCostStatus::Paid => {
                    install_delay_hand_digivolve_after_paid(rctx.effect, &continuation);
                }
                DelayCostStatus::Pending => {
                    arm_pending_delay_cost_continuation(rctx.effect.game, continuation);
                }
                DelayCostStatus::Unpaid => {}
            };
        });
        return Some(builder.build());
    }

    if let Some(dna_flow) = DelayHandDnaDigivolveFlow::from_process(&process) {
        builder = builder.replacement_process(move |rctx| {
            if !source_is_delayed_option(rctx.effect) {
                return;
            }

            let player = resolve_player(rctx.effect, dna_flow.of);
            let Some(source) = rctx.effect.source_permanent else {
                return;
            };
            let Some(source_card) = rctx.effect.permanent_top_card_handle(source) else {
                return;
            };
            let subject_card = stable_replacement_subject_card(rctx.effect, rctx.subject);

            let continuation = DelayDnaCostContinuation {
                source_player: source.player,
                source_card,
                subject: rctx.subject,
                subject_card,
                player,
                result_prompt: dna_flow.result_prompt.clone(),
                result_filter: dna_flow.result_filter.clone(),
                partner_prompt: dna_flow.partner_prompt.clone(),
                partner_filter: dna_flow.partner_filter.clone(),
            };

            match rctx.effect.trash_delay_source_status() {
                DelayCostStatus::Paid => {
                    install_delay_dna_after_paid(rctx.effect, &continuation);
                }
                DelayCostStatus::Pending => {
                    arm_pending_delay_dna_continuation(rctx.effect.game, continuation);
                }
                DelayCostStatus::Unpaid => {}
            };
        });
        return Some(builder.build());
    }

    builder = builder.replacement_process(move |rctx| {
        let mut bindings = Bindings::new();
        if let Some(subject) = rctx.subject.permanent() {
            bindings.insert_permanent("replacement_subject", subject);
        }
        rctx.effect.game.dsl_replacement_outcome = None;
        let _ = run_steps_with_runtime(&process, rctx.effect, &mut bindings, &runtime);
        if let Some(outcome) = rctx.effect.game.dsl_replacement_outcome.take() {
            rctx.outcome = outcome;
        }
    });

    Some(builder.build())
}

fn predicate_reads_replacement_subject(pred: &CompiledPredicate) -> bool {
    let reads_direct_subject = pred.kind.is_some()
        || pred.level_eq.is_some()
        || pred.level_lte.is_some()
        || pred.level_gte.is_some()
        || pred.level_matches_aggregate.is_some()
        || pred.color_is.is_some()
        || pred.color_only.is_some()
        || pred.color_matches_any_field_digimon.is_some()
        || pred.trait_has.is_some()
        || pred.form_is.is_some()
        || pred.attribute_is.is_some()
        || pred.name_is.is_some()
        || pred.name_contains.is_some()
        || pred.name_in.is_some()
        || pred.card_number_is.is_some()
        || pred.play_cost_lte.is_some()
        || pred.dp_eq.is_some()
        || pred.dp_lte.is_some()
        || pred.dp_gte.is_some()
        || pred.stack_size_lte.is_some()
        || pred.stack_size_gte.is_some()
        || pred.materials_count_lte.is_some()
        || pred.materials_count_gte.is_some()
        || pred.has_inherited.is_some()
        || pred.is_suspended.is_some()
        || pred.is_unsuspended.is_some()
        || pred.has_keyword.is_some()
        || !pred.zone.is_empty()
        || pred.owner.is_some()
        || pred.other.is_some()
        || pred.of_permanent.is_some()
        || pred.not_in_binding.is_some()
        || pred.replacement_subject_is_mine.is_some();

    reads_direct_subject
        || pred.all_of.iter().any(predicate_reads_replacement_subject)
        || (!pred.any_of.is_empty() && pred.any_of.iter().all(predicate_reads_replacement_subject))
        || pred.none_of.iter().any(predicate_reads_replacement_subject)
        || pred
            .not
            .as_deref()
            .is_some_and(predicate_reads_replacement_subject)
}

fn replacement_process_has_payable_required_selection(
    process: &[CompiledStep],
    ctx: &EffectReadContext<'_>,
    subject: ReplacementSubject,
) -> bool {
    if process_places_permanent_in_security(process)
        && ctx
            .game
            .modifiers
            .player_has(ctx.player, ModifierType::CannotAddSecurityByEffect)
    {
        return false;
    }

    let Some(first) = process.first() else {
        return true;
    };
    let mut bindings = Bindings::new();
    if let Some(subject) = subject.permanent() {
        bindings.insert_permanent("replacement_subject", subject);
    }
    required_selection_step_has_candidate(first, ctx, &bindings)
}

fn process_places_permanent_in_security(process: &[CompiledStep]) -> bool {
    process.iter().any(step_places_permanent_in_security)
}

fn step_places_permanent_in_security(step: &CompiledStep) -> bool {
    match step {
        CompiledStep::PlacePermanentBottomSecurityAndCancelReplacement { .. }
        | CompiledStep::PlacePermanentOnSecurityAndHandleReplacement { .. } => true,
        CompiledStep::AsSelectingPlayer { body, .. }
        | CompiledStep::Optional(body)
        | CompiledStep::ScheduleDelayed { body, .. } => process_places_permanent_in_security(body),
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_places_permanent_in_security(then)
                || process_places_permanent_in_security(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_places_permanent_in_security(body)
        }
        CompiledStep::SelectOwnSources { then, .. }
        | CompiledStep::SelectOpponentDpBudget { then, .. }
        | CompiledStep::SelectOpponentPlayCostBudget { then, .. }
        | CompiledStep::SelectOwnBreedingPermanent { then, .. } => {
            process_places_permanent_in_security(then)
        }
        _ => false,
    }
}

fn required_selection_step_has_candidate(
    step: &CompiledStep,
    ctx: &EffectReadContext<'_>,
    bindings: &Bindings,
) -> bool {
    match step {
        CompiledStep::AsSelectingPlayer { of, body } => {
            let player = resolve_player_ref(ctx, *of);
            let scoped = EffectReadContext::new_with_source_kind(
                ctx.game,
                ctx.source_card,
                ctx.source_permanent,
                ctx.source_kind,
                player,
            );
            replacement_process_has_payable_required_selection_without_subject(
                body, &scoped, bindings,
            )
        }
        CompiledStep::SelectOwnPermanent {
            filter, selector, ..
        } => has_matching_permanent(ctx, Some(ctx.player), filter, *selector, bindings),
        CompiledStep::SelectOpponentPermanent {
            filter, selector, ..
        } => {
            let opponent = ctx.game.next_clockwise(ctx.player);
            has_matching_permanent(ctx, Some(opponent), filter, *selector, bindings)
        }
        CompiledStep::SelectAnyPermanent {
            filter, selector, ..
        } => has_matching_permanent(ctx, None, filter, *selector, bindings),
        CompiledStep::SelectHand { of, filter, .. } => {
            has_matching_card_in_zone(ctx, *of, CompiledZone::Hand, filter, bindings)
        }
        CompiledStep::SelectTrash { of, filter, .. } => {
            has_matching_card_in_zone(ctx, *of, CompiledZone::Trash, filter, bindings)
        }
        CompiledStep::SelectSecurity { of, .. } => !ctx
            .game
            .player(resolve_player_ref(ctx, *of))
            .security
            .is_empty(),
        CompiledStep::SelectCountCappedMulti {
            of,
            zone,
            max,
            min,
            filter,
            optional_zero,
            ..
        } => {
            // A zero-pick-acceptable step is always payable.
            if *optional_zero && *min == 0 {
                return true;
            }
            if matches!(max, digimon_dsl::compiled::CompiledCountBound::Literal(0)) {
                return *min == 0;
            }
            // When `min > 0` the required cost is payable only if at least
            // `min` cards in the zone match the filter. G-SELECT-MULTI-MIN.
            if *min > 0 {
                let player = resolve_player_ref(ctx, *of);
                let count = match zone {
                    CompiledZone::Hand => count_matching_zone_cards(
                        ctx,
                        &ctx.game.player(player).hand,
                        filter,
                        bindings,
                    ),
                    CompiledZone::Trash => count_matching_zone_cards(
                        ctx,
                        &ctx.game.player(player).trash,
                        filter,
                        bindings,
                    ),
                    _ => *min as usize,
                };
                return count >= *min as usize;
            }
            match zone {
                CompiledZone::Hand => !ctx
                    .game
                    .player(resolve_player_ref(ctx, *of))
                    .hand
                    .is_empty(),
                CompiledZone::Trash => !ctx
                    .game
                    .player(resolve_player_ref(ctx, *of))
                    .trash
                    .is_empty(),
                _ => true,
            }
        }
        CompiledStep::SelectMaterial {
            of_permanent,
            filter,
            ..
        } => resolve_permanent_binding(of_permanent, bindings)
            .is_some_and(|perm| permanent_has_matching_materials(ctx, perm, filter, bindings)),
        CompiledStep::SelectOwnSources { min, max, .. } => {
            *min == 0 || (*max >= *min && has_own_source_candidates(ctx))
        }
        CompiledStep::SelectOpponentDpBudget {
            dp_budget,
            min_picks,
            ..
        } => {
            let budget = formula_eval::evaluate_read_with_bindings(
                dp_budget,
                ctx,
                ctx.source_permanent.unwrap_or(PermanentHandle {
                    player: ctx.player,
                    index: u8::MAX,
                }),
                Some(bindings),
            );
            *min_picks == 0 || has_opponent_dp_budget_candidate(ctx, budget)
        }
        CompiledStep::SelectOpponentPlayCostBudget {
            play_cost_budget,
            min_picks,
            ..
        } => *min_picks == 0 || has_opponent_play_cost_budget_candidate(ctx, *play_cost_budget),
        CompiledStep::SelectOwnBreedingPermanent { filter, .. } => {
            if ctx.game.player(ctx.player).breeding_area.is_none() {
                false
            } else {
                eval_predicate_with_bindings(
                    filter,
                    ctx,
                    PredicateSubject::BreedingPermanent(ctx.player),
                    Some(bindings),
                )
            }
        }
        CompiledStep::SelectUnionZone { of, zones, .. } => zones.iter().any(|zone| match zone {
            CompiledZone::Hand => !ctx
                .game
                .player(resolve_player_ref(ctx, *of))
                .hand
                .is_empty(),
            CompiledZone::Trash => !ctx
                .game
                .player(resolve_player_ref(ctx, *of))
                .trash
                .is_empty(),
            _ => false,
        }),
        CompiledStep::SelectOrderedPermutation { items, .. } => {
            resolve_card_list_binding(items, bindings).is_some_and(|items| !items.is_empty())
        }
        // Gap 3a — the trash-own-link-card cost is payable only when the
        // leaving permanent (the `replacement_subject` binding) has ≥1 link
        // card. Gates the optional accept prompt so it is not offered with 0
        // link cards.
        CompiledStep::TrashOwnLinkCardAndCancelLeave => bindings
            .get_permanent("replacement_subject")
            .and_then(|h| {
                ctx.game
                    .player(h.player)
                    .battle_area
                    .get(h.index as usize)
            })
            .is_some_and(|perm| !perm.linked_cards.is_empty()),
        // Non-selection first steps may mutate state before a later selection,
        // so this read-only preflight deliberately does not speculate.
        _ => true,
    }
}

fn replacement_process_has_payable_required_selection_without_subject(
    process: &[CompiledStep],
    ctx: &EffectReadContext<'_>,
    bindings: &Bindings,
) -> bool {
    process
        .first()
        .is_none_or(|step| required_selection_step_has_candidate(step, ctx, bindings))
}

fn resolve_player_ref(ctx: &EffectReadContext<'_>, player: CompiledPlayerRef) -> PlayerId {
    match player {
        CompiledPlayerRef::You => ctx.player,
        CompiledPlayerRef::Opponent => ctx.game.next_clockwise(ctx.player),
        CompiledPlayerRef::Active => ctx.game.turn_player(),
        CompiledPlayerRef::Any => ctx.player,
    }
}

fn has_matching_permanent(
    ctx: &EffectReadContext<'_>,
    player: Option<PlayerId>,
    filter: &CompiledPredicate,
    selector: Option<CompiledFieldSelector>,
    bindings: &Bindings,
) -> bool {
    let mut handles = Vec::new();
    let players: Box<dyn Iterator<Item = PlayerId>> = if let Some(player) = player {
        Box::new(std::iter::once(player))
    } else {
        Box::new((0..ctx.game.players.len()).map(|p| p as PlayerId))
    };

    for player in players {
        for index in 0..ctx.game.player(player).battle_area.len() {
            let handle = PermanentHandle {
                player,
                index: index as u8,
            };
            if eval_predicate_with_bindings(
                filter,
                ctx,
                PredicateSubject::Permanent(handle),
                Some(bindings),
            ) {
                handles.push(handle);
            }
        }
    }

    let selected = selected_field_extreme(ctx, &handles, selector);
    handles
        .into_iter()
        .any(|handle| matches_selected_field(ctx, handle, selector, selected))
}

/// Result of resolving a `CompiledFieldSelector` over a candidate set.
#[derive(Clone, Copy)]
enum SelectedField {
    /// No selector — every candidate matches.
    Any,
    /// The selector's extreme value, in the selector's own unit
    /// (effective DP for DP selectors, printed play cost for the
    /// play-cost selector).
    Exact(i32),
    /// Selector present but the candidate set was empty.
    None,
}

/// Read a permanent's value in the unit of the given field selector:
/// effective DP for DP selectors, printed play cost for play-cost selectors.
fn field_value(
    ctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
    selector: CompiledFieldSelector,
) -> Option<i32> {
    match selector {
        CompiledFieldSelector::LowestDp | CompiledFieldSelector::HighestDp => {
            ctx.game.effective_dp(handle)
        }
        CompiledFieldSelector::LowestPlayCost | CompiledFieldSelector::HighestPlayCost => ctx
            .game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| i32::from(perm.top_card().play_cost(ctx.card_data()))),
        CompiledFieldSelector::LowestMaterialCount => ctx
            .game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| perm.card_sources.len().saturating_sub(1) as i32),
    }
}

fn selected_field_extreme(
    ctx: &EffectReadContext<'_>,
    handles: &[PermanentHandle],
    selector: Option<CompiledFieldSelector>,
) -> SelectedField {
    let Some(selector) = selector else {
        return SelectedField::Any;
    };
    let values = handles
        .iter()
        .filter_map(|handle| field_value(ctx, *handle, selector));
    let extreme = match selector {
        CompiledFieldSelector::LowestDp | CompiledFieldSelector::LowestPlayCost => values.min(),
        CompiledFieldSelector::HighestDp | CompiledFieldSelector::HighestPlayCost => values.max(),
        CompiledFieldSelector::LowestMaterialCount => values.min(),
    };
    extreme
        .map(SelectedField::Exact)
        .unwrap_or(SelectedField::None)
}

fn matches_selected_field(
    ctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
    selector: Option<CompiledFieldSelector>,
    selected: SelectedField,
) -> bool {
    match selected {
        SelectedField::Any => true,
        SelectedField::Exact(want) => selector
            .and_then(|selector| field_value(ctx, handle, selector))
            .is_some_and(|candidate| candidate == want),
        SelectedField::None => false,
    }
}

fn has_matching_card_in_zone(
    ctx: &EffectReadContext<'_>,
    of: CompiledPlayerRef,
    zone: CompiledZone,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> bool {
    let player = resolve_player_ref(ctx, of);
    let cards = match zone {
        CompiledZone::Hand => &ctx.game.player(player).hand,
        CompiledZone::Trash => &ctx.game.player(player).trash,
        _ => return true,
    };
    cards.iter().any(|card| {
        eval_predicate_with_bindings(
            filter,
            ctx,
            PredicateSubject::Card(card.handle()),
            Some(bindings),
        )
    })
}

/// Count the cards in `cards` that satisfy `filter`. Used by the replacement
/// preflight to verify a `select_count_capped_multi { min: N }` required cost
/// is payable. G-SELECT-MULTI-MIN.
fn count_matching_zone_cards(
    ctx: &EffectReadContext<'_>,
    cards: &[CardSource],
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> usize {
    cards
        .iter()
        .filter(|card| {
            eval_predicate_with_bindings(
                filter,
                ctx,
                PredicateSubject::Card(card.handle()),
                Some(bindings),
            )
        })
        .count()
}

fn resolve_permanent_binding(
    binding: &CompiledBindingRef,
    bindings: &Bindings,
) -> Option<PermanentHandle> {
    match binding {
        CompiledBindingRef::Named(name) | CompiledBindingRef::Binding(name) => {
            bindings.get_permanent(name)
        }
        _ => None,
    }
}

fn resolve_card_list_binding(
    binding: &CompiledBindingRef,
    bindings: &Bindings,
) -> Option<Vec<CardHandle>> {
    match binding {
        CompiledBindingRef::Named(name) | CompiledBindingRef::Binding(name) => {
            bindings.get_card_list(name)
        }
        _ => None,
    }
}

fn permanent_has_matching_materials(
    ctx: &EffectReadContext<'_>,
    perm: PermanentHandle,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> bool {
    ctx.game
        .player(perm.player)
        .battle_area
        .get(perm.index as usize)
        .is_some_and(|p| {
            p.card_sources
                .iter()
                .take(p.card_sources.len().saturating_sub(1))
                .any(|card| {
                    eval_predicate_with_bindings(
                        filter,
                        ctx,
                        PredicateSubject::Card(card.handle()),
                        Some(bindings),
                    )
                })
        })
}

fn has_own_source_candidates(ctx: &EffectReadContext<'_>) -> bool {
    ctx.game
        .player(ctx.player)
        .battle_area
        .iter()
        .any(|perm| perm.card_sources.len() > 1)
}

fn has_opponent_dp_budget_candidate(ctx: &EffectReadContext<'_>, dp_budget: i32) -> bool {
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

fn has_opponent_play_cost_budget_candidate(
    ctx: &EffectReadContext<'_>,
    play_cost_budget: i32,
) -> bool {
    let opponent = ctx.game.next_clockwise(ctx.player);
    ctx.game
        .player(opponent)
        .battle_area
        .iter()
        .any(|perm| i32::from(perm.top_card().play_cost(ctx.card_data())) <= play_cost_budget)
}

fn replacement_subject_is_source(ctx: &EffectReadContext<'_>, subject: ReplacementSubject) -> bool {
    subject
        .permanent()
        .is_some_and(|h| ctx.source_permanent == Some(h))
}

fn predicate_subject_from_replacement_subject(subject: ReplacementSubject) -> PredicateSubject {
    match subject {
        ReplacementSubject::Permanent(handle) => PredicateSubject::Permanent(handle),
        ReplacementSubject::Card(handle, _) => PredicateSubject::Card(handle),
        ReplacementSubject::Player(_) => PredicateSubject::None,
    }
}

fn source_permanent_is_still_active(ctx: &EffectReadContext<'_>) -> bool {
    let Some(handle) = ctx.source_permanent else {
        return false;
    };

    let permanent = if handle.index == crate::action::space::BREEDING_TARGET as u8 {
        ctx.game.player(handle.player).breeding_area.as_ref()
    } else {
        ctx.game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
    };

    permanent.is_some_and(|perm| {
        perm.card_sources
            .iter()
            .any(|source| source.handle() == ctx.source_card)
    })
}

struct DelayHandDigivolveFlow {
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    prompt: String,
}

struct DelaySelfCancelFlow;

#[derive(Clone)]
struct DelayCancelContinuation {
    source_player: PlayerId,
    source_card: CardHandle,
}

#[derive(Clone)]
struct DelayCostContinuation {
    source_player: PlayerId,
    source_card: CardHandle,
    subject: ReplacementSubject,
    subject_card: Option<(PlayerId, CardHandle)>,
    player: PlayerId,
    prompt: String,
    filter: CompiledPredicate,
}

impl DelaySelfCancelFlow {
    fn from_process(process: &[CompiledStep]) -> Option<Self> {
        let [CompiledStep::DeletePermanent { target }, CompiledStep::CancelReplacement] = process
        else {
            return None;
        };

        matches!(target, CompiledBindingRef::Source).then_some(Self)
    }
}

impl DelayHandDigivolveFlow {
    fn from_process(process: &[CompiledStep]) -> Option<Self> {
        let [CompiledStep::DeletePermanent { target }, CompiledStep::SelectHand {
            of,
            filter,
            bind_as,
            prompt,
            optional,
            ..
        }, CompiledStep::EffectInitiatedDigivolve {
            target: digivolve_target,
            from_hand,
            cost,
            ignore_requirements,
        }, CompiledStep::CancelReplacement] = process
        else {
            return None;
        };

        let bind_as = bind_as.as_deref()?;
        if !matches!(target, CompiledBindingRef::Source) {
            return None;
        }
        if !is_binding_ref_named(digivolve_target, "replacement_subject") {
            return None;
        }
        if !is_binding_ref_named(from_hand, bind_as) {
            return None;
        }
        if !matches!(
            cost,
            CompiledCostDelta::Free | CompiledCostDelta::Literal(0)
        ) {
            return None;
        }
        if !*ignore_requirements || !*optional {
            return None;
        }

        Some(Self {
            of: *of,
            filter: filter.clone(),
            prompt: prompt.clone(),
        })
    }
}

/// Recognised process shape for BT17-095 Clause B — the Delay-cost DNA
/// digivolve where one DNA material is the leaving subject (on field) and the
/// other is a card in hand, merging into an Omnimon-name card also in hand.
///
/// Recognised process:
/// ```text
/// [ delete_permanent { target: source },
///   select_hand { bind_as: <result>, optional: true },   // Omnimon result
///   select_hand { bind_as: <partner>, optional: true },  // L6 DNA partner
///   effect_initiated_dna_digivolve_hand_partner {
///       target: replacement_subject,
///       hand_partner: <partner>,
///       from_hand: <result>,
///   },
///   cancel_replacement ]
/// ```
struct DelayHandDnaDigivolveFlow {
    of: CompiledPlayerRef,
    result_filter: CompiledPredicate,
    result_prompt: String,
    partner_filter: CompiledPredicate,
    partner_prompt: String,
}

#[derive(Clone)]
struct DelayDnaCostContinuation {
    source_player: PlayerId,
    source_card: CardHandle,
    subject: ReplacementSubject,
    subject_card: Option<(PlayerId, CardHandle)>,
    player: PlayerId,
    result_prompt: String,
    result_filter: CompiledPredicate,
    partner_prompt: String,
    partner_filter: CompiledPredicate,
}

impl DelayHandDnaDigivolveFlow {
    fn from_process(process: &[CompiledStep]) -> Option<Self> {
        let [CompiledStep::DeletePermanent { target }, CompiledStep::SelectHand {
            of: result_of,
            filter: result_filter,
            bind_as: result_bind,
            prompt: result_prompt,
            optional: result_optional,
            ..
        }, CompiledStep::SelectHand {
            of: partner_of,
            filter: partner_filter,
            bind_as: partner_bind,
            prompt: partner_prompt,
            optional: partner_optional,
            ..
        }, CompiledStep::EffectInitiatedDnaDigivolveHandPartner {
            target: dna_target,
            hand_partner,
            from_hand,
            cost,
            ignore_requirements,
        }, CompiledStep::CancelReplacement] = process
        else {
            return None;
        };

        let result_bind = result_bind.as_deref()?;
        let partner_bind = partner_bind.as_deref()?;
        if !matches!(target, CompiledBindingRef::Source) {
            return None;
        }
        if result_of != partner_of {
            return None;
        }
        if !is_binding_ref_named(dna_target, "replacement_subject") {
            return None;
        }
        if !is_binding_ref_named(hand_partner, partner_bind) {
            return None;
        }
        if !is_binding_ref_named(from_hand, result_bind) {
            return None;
        }
        if !matches!(
            cost,
            CompiledCostDelta::Free | CompiledCostDelta::Literal(0)
        ) {
            return None;
        }
        if !*ignore_requirements || !*result_optional || !*partner_optional {
            return None;
        }

        Some(Self {
            of: *result_of,
            result_filter: result_filter.clone(),
            result_prompt: result_prompt.clone(),
            partner_filter: partner_filter.clone(),
            partner_prompt: partner_prompt.clone(),
        })
    }
}

fn is_binding_ref_named(binding: &CompiledBindingRef, name: &str) -> bool {
    match binding {
        CompiledBindingRef::Named(n) | CompiledBindingRef::Binding(n) => n == name,
        _ => false,
    }
}

fn source_is_delayed_option(ctx: &EffectContext<'_>) -> bool {
    let Some(source) = ctx.source_permanent else {
        return false;
    };
    ctx.game
        .player(source.player)
        .battle_area
        .get(source.index as usize)
        .is_some_and(|perm| matches!(perm.option_state, OptionState::Delayed { .. }))
}

fn stable_replacement_subject_card(
    ctx: &EffectContext<'_>,
    subject: ReplacementSubject,
) -> Option<(PlayerId, CardHandle)> {
    let handle = subject.permanent()?;
    ctx.permanent_top_card_handle(handle)
        .map(|card| (handle.player, card))
}

fn resolve_stable_replacement_subject(
    ctx: &EffectContext<'_>,
    subject: ReplacementSubject,
    stable_card: Option<(crate::enums::PlayerId, CardHandle)>,
) -> Option<ReplacementSubject> {
    if let Some((player, card)) = stable_card {
        return ctx
            .find_battle_permanent_containing_card(player, card)
            .map(ReplacementSubject::Permanent);
    }

    let handle = subject.permanent()?;
    ((handle.index as usize) < ctx.game.player(handle.player).battle_area.len())
        .then_some(ReplacementSubject::Permanent(handle))
}

fn matching_hand_candidates(
    ctx: &EffectContext<'_>,
    player: PlayerId,
    filter: &CompiledPredicate,
    limit: usize,
) -> Vec<CardHandle> {
    let read_ctx = EffectReadContext::new(ctx.game, ctx.source_card, ctx.source_permanent, player);
    ctx.game
        .player(player)
        .hand
        .iter()
        .take(limit)
        .filter_map(|source| {
            let card = source.handle();
            eval_predicate(filter, &read_ctx, PredicateSubject::Card(card)).then_some(card)
        })
        .collect()
}

fn install_delay_hand_digivolve_selection(
    ctx: &mut EffectContext<'_>,
    subject: ReplacementSubject,
    player: PlayerId,
    prompt: String,
    candidates: Vec<CardHandle>,
) {
    let previous_phase = ctx.game.current_phase;
    let valid_action_ids: Vec<u16> = candidates
        .iter()
        .enumerate()
        .map(|(idx, _)| HAND_EFFECT_START + idx as u16)
        .collect();
    let effect_choices: Vec<EffectChoiceEntry> = candidates
        .iter()
        .enumerate()
        .map(|(idx, card)| EffectChoiceEntry {
            label: ctx
                .game
                .card_data_for_handle(*card)
                .map(|data| data.card_name.clone())
                .unwrap_or_else(|| format!("Card {}", idx + 1)),
            action_id: HAND_EFFECT_START + idx as u16,
            source_card: Some(*card),
            source_kind: None,
            timing: None,
            is_optional: false,
            observation_metadata: Default::default(),
        })
        .collect();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind();

    ctx.game.current_phase = GamePhase::EffectChoice;
    ctx.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player: player,
        previous_phase,
        valid_action_ids,
        is_optional: true,
        prompt,
        effect_choices: Some(effect_choices),
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, action_id| {
            let Some(idx) = action_id.checked_sub(HAND_EFFECT_START).map(|i| i as usize) else {
                return;
            };
            let Some(card) = candidates.get(idx).copied() else {
                return;
            };
            let mut ctx = EffectContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            if ctx.digivolve_replacement_subject_without_cost(subject, card) {
                ctx.cancel_current_replacement();
            }
        }),
        on_decline: None,
    });
}

fn arm_pending_delay_cost_continuation(game: &mut Game, continuation: DelayCostContinuation) {
    let Some(mut selection) = game.pending_selection.take() else {
        continue_delay_cost_after_selection(game, continuation);
        return;
    };

    let original_callback = selection.callback;
    let callback_continuation = continuation.clone();
    selection.callback = Box::new(move |game, action_id| {
        original_callback(game, action_id);
        continue_delay_cost_after_selection(game, callback_continuation);
    });

    let original_decline = selection.on_decline.take();
    selection.on_decline = Some(Box::new(move |game| {
        if let Some(original_decline) = original_decline {
            original_decline(game);
        }
        continue_delay_cost_after_selection(game, continuation);
    }));

    game.pending_selection = Some(selection);
}

fn arm_pending_delay_cancel_continuation(game: &mut Game, continuation: DelayCancelContinuation) {
    let Some(mut selection) = game.pending_selection.take() else {
        continue_delay_cancel_after_selection(game, continuation);
        return;
    };

    let original_callback = selection.callback;
    let callback_continuation = continuation.clone();
    selection.callback = Box::new(move |game, action_id| {
        original_callback(game, action_id);
        continue_delay_cancel_after_selection(game, callback_continuation);
    });

    let original_decline = selection.on_decline.take();
    selection.on_decline = Some(Box::new(move |game| {
        if let Some(original_decline) = original_decline {
            original_decline(game);
        }
        continue_delay_cancel_after_selection(game, continuation);
    }));

    game.pending_selection = Some(selection);
}

fn continue_delay_cancel_after_selection(game: &mut Game, continuation: DelayCancelContinuation) {
    if game.pending_selection.is_some() {
        arm_pending_delay_cancel_continuation(game, continuation);
        return;
    }

    let paid = {
        let ctx = EffectContext::new(
            game,
            continuation.source_card,
            None,
            continuation.source_player,
        );
        ctx.delay_source_card_in_trash(continuation.source_player, continuation.source_card)
    };
    if !paid {
        return;
    }

    if let Some(parked) = game.parked_replacement.as_mut() {
        parked.outcome = crate::replacement::ReplacementOutcome::Cancelled;
    }
}

fn continue_delay_cost_after_selection(game: &mut Game, continuation: DelayCostContinuation) {
    if game.pending_selection.is_some() {
        arm_pending_delay_cost_continuation(game, continuation);
        return;
    }

    let mut ctx = EffectContext::new(game, continuation.source_card, None, continuation.player);
    install_delay_hand_digivolve_after_paid(&mut ctx, &continuation);
}

fn install_delay_hand_digivolve_after_paid(
    ctx: &mut EffectContext<'_>,
    continuation: &DelayCostContinuation,
) {
    if !ctx.delay_source_card_in_trash(continuation.source_player, continuation.source_card) {
        return;
    }

    let Some(subject) =
        resolve_stable_replacement_subject(ctx, continuation.subject, continuation.subject_card)
    else {
        return;
    };
    let candidates = matching_hand_candidates(
        ctx,
        continuation.player,
        &continuation.filter,
        HAND_MAIN_LIMIT,
    );
    if candidates.is_empty() {
        return;
    }

    install_delay_hand_digivolve_selection(
        ctx,
        subject,
        continuation.player,
        continuation.prompt.clone(),
        candidates,
    );
}

// ─── BT17-095 Clause B — Delay-cost DNA-with-hand-partner flow ─────────────────

fn arm_pending_delay_dna_continuation(game: &mut Game, continuation: DelayDnaCostContinuation) {
    let Some(mut selection) = game.pending_selection.take() else {
        continue_delay_dna_after_selection(game, continuation);
        return;
    };

    let original_callback = selection.callback;
    let callback_continuation = continuation.clone();
    selection.callback = Box::new(move |game, action_id| {
        original_callback(game, action_id);
        continue_delay_dna_after_selection(game, callback_continuation);
    });

    let original_decline = selection.on_decline.take();
    selection.on_decline = Some(Box::new(move |game| {
        if let Some(original_decline) = original_decline {
            original_decline(game);
        }
        continue_delay_dna_after_selection(game, continuation);
    }));

    game.pending_selection = Some(selection);
}

fn continue_delay_dna_after_selection(game: &mut Game, continuation: DelayDnaCostContinuation) {
    if game.pending_selection.is_some() {
        arm_pending_delay_dna_continuation(game, continuation);
        return;
    }

    let mut ctx = EffectContext::new(game, continuation.source_card, None, continuation.player);
    install_delay_dna_after_paid(&mut ctx, &continuation);
}

/// Stage 1: the Delay cost is paid. Install the selection for the Omnimon-name
/// result card (drawn from hand). Its callback chains into the partner-card
/// selection. If no result card qualifies the leaving subject simply proceeds
/// (the printed "may" — declining the DNA reward is legal).
fn install_delay_dna_after_paid(
    ctx: &mut EffectContext<'_>,
    continuation: &DelayDnaCostContinuation,
) {
    if !ctx.delay_source_card_in_trash(continuation.source_player, continuation.source_card) {
        return;
    }

    // Stage early-out: re-resolve the leaving subject and the candidate pools.
    if resolve_stable_replacement_subject(ctx, continuation.subject, continuation.subject_card)
        .is_none()
    {
        return;
    }
    let result_candidates = matching_hand_candidates(
        ctx,
        continuation.player,
        &continuation.result_filter,
        HAND_MAIN_LIMIT,
    );
    if result_candidates.is_empty() {
        return;
    }
    // The partner candidates exclude whichever card is later chosen as the
    // result; only require that at least one partner exists up front.
    let partner_candidates = matching_hand_candidates(
        ctx,
        continuation.player,
        &continuation.partner_filter,
        HAND_MAIN_LIMIT,
    );
    if partner_candidates.is_empty() {
        return;
    }

    install_delay_dna_card_selection(
        ctx,
        continuation.player,
        continuation.result_prompt.clone(),
        result_candidates,
        DnaSelectionStage::Result(continuation.clone()),
    );
}

/// Which selection stage a `DnaSelectionStage` callback represents.
enum DnaSelectionStage {
    /// Stage 1 — the chosen card is the Omnimon result. Chain into Stage 2.
    Result(DelayDnaCostContinuation),
    /// Stage 2 — the chosen card is the hand partner; `result` is the
    /// already-chosen Omnimon card. Execute the DNA merge.
    Partner {
        continuation: DelayDnaCostContinuation,
        result: CardHandle,
    },
}

/// Install a hand-card `EffectChoice` selection for one DNA stage. The chosen
/// card drives the next stage (Result → Partner) or the merge (Partner).
fn install_delay_dna_card_selection(
    ctx: &mut EffectContext<'_>,
    player: PlayerId,
    prompt: String,
    candidates: Vec<CardHandle>,
    stage: DnaSelectionStage,
) {
    let previous_phase = ctx.game.current_phase;
    let valid_action_ids: Vec<u16> = candidates
        .iter()
        .enumerate()
        .map(|(idx, _)| HAND_EFFECT_START + idx as u16)
        .collect();
    let effect_choices: Vec<EffectChoiceEntry> = candidates
        .iter()
        .enumerate()
        .map(|(idx, card)| EffectChoiceEntry {
            label: ctx
                .game
                .card_data_for_handle(*card)
                .map(|data| data.card_name.clone())
                .unwrap_or_else(|| format!("Card {}", idx + 1)),
            action_id: HAND_EFFECT_START + idx as u16,
            source_card: Some(*card),
            source_kind: None,
            timing: None,
            is_optional: false,
            observation_metadata: Default::default(),
        })
        .collect();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind();

    ctx.game.current_phase = GamePhase::EffectChoice;
    ctx.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player: player,
        previous_phase,
        valid_action_ids,
        is_optional: true,
        prompt,
        effect_choices: Some(effect_choices),
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, action_id| {
            let Some(idx) = action_id.checked_sub(HAND_EFFECT_START).map(|i| i as usize) else {
                return;
            };
            let Some(chosen) = candidates.get(idx).copied() else {
                return;
            };
            match &stage {
                DnaSelectionStage::Result(continuation) => {
                    // Stage 1 done — install the partner selection, excluding
                    // the card just chosen as the result.
                    let mut ctx = EffectContext::new_with_source_kind(
                        game,
                        source_card,
                        source_permanent,
                        source_kind,
                        player,
                    );
                    let partner_candidates: Vec<CardHandle> = matching_hand_candidates(
                        &ctx,
                        continuation.player,
                        &continuation.partner_filter,
                        HAND_MAIN_LIMIT,
                    )
                    .into_iter()
                    .filter(|c| *c != chosen)
                    .collect();
                    if partner_candidates.is_empty() {
                        return;
                    }
                    install_delay_dna_card_selection(
                        &mut ctx,
                        continuation.player,
                        continuation.partner_prompt.clone(),
                        partner_candidates,
                        DnaSelectionStage::Partner {
                            continuation: continuation.clone(),
                            result: chosen,
                        },
                    );
                }
                DnaSelectionStage::Partner {
                    continuation,
                    result,
                } => {
                    // Stage 2 done — re-resolve the leaving subject and run
                    // the DNA merge. On success the subject is consumed into
                    // the merged permanent, so cancel the leave.
                    let mut ctx = EffectContext::new_with_source_kind(
                        game,
                        source_card,
                        source_permanent,
                        source_kind,
                        player,
                    );
                    let Some(subject) = resolve_stable_replacement_subject(
                        &ctx,
                        continuation.subject,
                        continuation.subject_card,
                    ) else {
                        return;
                    };
                    let Some(target) = subject.permanent() else {
                        return;
                    };
                    let merged = ctx.effect_initiated_dna_digivolve_with_hand_partner(
                        target, chosen, *result, 0, true,
                    );
                    if merged.is_some() {
                        ctx.cancel_current_replacement();
                        if let Some(parked) = game.parked_replacement.as_mut() {
                            parked.outcome = crate::replacement::ReplacementOutcome::Cancelled;
                        }
                    }
                }
            }
        }),
        on_decline: None,
    });
}
