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
use crate::enums::{EffectSourceKind, EffectTiming, GamePhase, ModifierType, PlayerId};
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
    lower_with_raw_and_slot(card,scope,active_when,trigger,optional,once_per_turn,process,raw,None)
}

pub fn lower_with_raw_and_slot(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: &str,
    optional: bool,
    once_per_turn: bool,
    process: &[CompiledStep],
    raw: Arc<EngineRawRustRegistry>,
    effect_slot: Option<u8>,
) -> Option<Effect> {
    let timing = lookup_replacement_trigger(trigger)?;
    let label = format!("Replacement: {trigger}");
    let process: Arc<[CompiledStep]> = Arc::from(process);
    let mut runtime = StepRuntime::new(raw);

    let mut builder = new_when_would_builder(card, timing)?;
    builder = builder.name(&label);
    if process_has_paid_self_prevention(&process) {
        builder = if process_has_paid_all_prevention(&process) {
            builder.paid_self_prevention_all()
        } else { builder.paid_self_prevention() };
        builder = builder.paid_prevention_filter(active_when.cloned().unwrap_or_default());
    }

    let mut suspension_probe = PaidPreventionProbe::default();
    serde::Serialize::serialize(process.as_ref(), &mut suspension_probe).expect("typed prevention probe");
    if let Some(filter) = suspension_probe.suspend_filter {
        builder = builder.prevention_all_matching().paid_prevention_filter(filter);
        runtime = runtime.with_opt_key(if once_per_turn {effect_slot}else{None})
            .with_opt_key_inherited(matches!(scope,CompiledScope::Inherited|CompiledScope::Linked));
    }

    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    // A `scope: linked` replacement is a link card's Link-ESS — it applies to
    // the HOST while the card is attached as a link card. `.linked()` marks it
    // so the host-side `enqueue_from_permanent` linked-card scan consults it
    // (BT25-101 Divine Arms Version Ω: the leave-replacement is a linked
    // effect, DCGO `SetIsLinkedEffect(true)`).
    if matches!(scope, CompiledScope::Linked) {
        builder = builder.linked();
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
    // §15-7-4 (Ver. 4.3): an optional replacement's "by X" condition that
    // can't be executed can't be chosen, so the replacement is not offered
    // (e.g. "When this Digimon would leave the battle area, by trashing 2
    // cards in your hand, it doesn't leave" with 1 card in hand). Same
    // payability guard as the triggered path (`leading_cost_guard`).
    // G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED.
    let cost_guard = if optional {
        crate::dsl_cards::lower_triggered::leading_cost_guard(&process)
    } else {
        None
    };
    let field_dna_flow = DelayFieldDnaDigivolveFlow::from_process(&process);
    let requires_ready_delay = field_dna_flow.is_some();
    builder = builder.replacement_condition(move |ctx, subject| {
        source_permanent_is_still_active(ctx)
            && (!requires_ready_delay || source_delay_is_ready(ctx))
            && (can_match_cross_permanent_subject || replacement_subject_is_source(ctx, *subject))
            && active_when.as_ref().is_none_or(|active_when| {
                eval_predicate(
                    active_when,
                    ctx,
                    predicate_subject_from_replacement_subject(*subject),
                )
            })
            && replacement_process_has_payable_required_selection(&preflight_process, ctx, *subject)
            && cost_guard.as_ref().is_none_or(|guard| guard.holds(ctx))
    });

    if let Some(flow) = field_dna_flow {
        builder = builder.replacement_process(move |rctx| {
            if !source_delay_is_ready(&rctx.effect.as_read()) {
                return;
            }
            let Some(source) = rctx.effect.source_permanent else {
                return;
            };
            let Some(source_card) = rctx.effect.permanent_top_card_handle(source) else {
                return;
            };
            let continuation = DelayFieldDnaContinuation {
                source_player: source.player,
                source_card,
                subject_card: stable_replacement_subject_card(rctx.effect, rctx.subject),
                player: resolve_player(rctx.effect, flow.of),
                flow: flow.clone(),
            };
            match rctx.effect.trash_delay_source_status() {
                DelayCostStatus::Paid => install_delay_field_dna_stage(
                    rctx.effect,
                    continuation,
                    DelayFieldDnaStage::Result,
                ),
                DelayCostStatus::Pending => {
                    arm_delay_field_dna_payment(rctx.effect.game, continuation)
                }
                DelayCostStatus::Unpaid => {}
            }
        });
        return Some(builder.build());
    }

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

    // ST20-14 Our Courage United — Delay-cost trash-self then optionally PLAY a
    // card from hand (non-cancelling: the leaving Digimon still leaves). Same
    // Delay-cost shape as `DelayHandDigivolveFlow` but the reward is a hand PLAY
    // rather than a digivolve, and no `cancel_replacement` (Proceed). Recognised
    // as a data-driven flow (not the generic step runner) because the generic
    // path can't thread the reward's `select_hand` → `play_from_hand_free` tail
    // across the mid-process source self-trash — the nested Option deletion
    // clobbers the parked-replacement / outer-tail state. Clone-safe: the reward
    // selection is installed on the resumable VM (`ResumeFrame::Delay*`), never a
    // bespoke closure-only park (rule 28).
    if let Some(play_flow) = DelaySelfTrashPlayFromHandFlow::from_process(&process) {
        builder = builder.replacement_process(move |rctx| {
            if !source_is_delayed_option(rctx.effect) {
                return;
            }

            let player = resolve_player(rctx.effect, play_flow.of);
            let Some(source) = rctx.effect.source_permanent else {
                return;
            };
            let Some(source_card) = rctx.effect.permanent_top_card_handle(source) else {
                return;
            };

            // Capture the leaving subject's STABLE card identity. This flow is
            // non-cancelling (the leaver still leaves), but the reward PLAYS a
            // card into the battle area, which shifts indices and would make the
            // parked-replacement's index-based Proceed commit delete the WRONG
            // permanent (it would trash the freshly-played card instead of the
            // leaver). So we take ownership of the leave: mark the replacement
            // `handled()` (the parked drain becomes a no-op) and re-delete the
            // leaver by its stable card handle ourselves — AFTER the reward play
            // — inside the reward selection step. Faithful to DCGO: the leaving
            // Digimon still leaves; it is not cancelled.
            let subject_card = stable_replacement_subject_card(rctx.effect, rctx.subject);

            let continuation = DelayPlayFromHandContinuation {
                source_player: source.player,
                source_card,
                subject: rctx.subject,
                subject_card,
                player,
                prompt: play_flow.prompt.clone(),
                filter: play_flow.filter.clone(),
            };

            match rctx.effect.trash_delay_source_status() {
                DelayCostStatus::Paid => {
                    rctx.handled();
                    install_delay_play_from_hand_after_paid(rctx.effect, &continuation);
                }
                DelayCostStatus::Pending => {
                    rctx.handled();
                    arm_pending_delay_play_from_hand_continuation(rctx.effect.game, continuation);
                }
                DelayCostStatus::Unpaid => {}
            };
        });
        return Some(builder.build());
    }

    // BT19-099 The Wicked God Descends! — Delay-cost trash-self then optionally
    // PLAY a filtered card from HAND OR TRASH for free (union-zone reward),
    // non-cancelling. Union-zone sibling of `DelaySelfTrashPlayFromHandFlow`
    // with the same ownership contract: the generic step runner cannot thread
    // the reward tail across the mid-process source self-trash, and the parked
    // replacement's index-based Proceed commit deletes the WRONG permanent once
    // the self-trash + reward play shift battle-area indices — the freshly
    // played card got trashed and the leaving subject survived
    // (G-ENGINE-DELAY-SELF-TRASH-THEN-FREE-PLAY-IN-LEAVE-REPLACEMENT). So:
    // mark `handled()`, pay the self-trash, finish the OWNED leave by stable
    // card handle, THEN run the reward steps through the generic runner with
    // `replacement_subject` re-bound as the subject's stable TOP-CARD SNAPSHOT
    // (a card binding) so a `play_cost_eq_binding` reward filter reads the
    // pre-leave printed cost — DCGO BT19_099.cs:261 evaluates
    // `permanent.TopCard.GetCostItself + 1 == candidate.GetCostItself` against
    // the trigger-hashtable snapshot, not a live post-deletion lookup.
    // Clone-safe: the reward select is the ordinary `select_union_zone`
    // install (data-driven `RunTail` resume frame); the rare Pending case
    // parks the data-driven `DelayPlayFromUnionAfterSelection` frame.
    if let Some(union_flow) = DelaySelfTrashPlayFromUnionFlow::from_process(&process) {
        let runtime_for_union = runtime.clone();
        builder = builder.replacement_process(move |rctx| {
            if !source_is_delayed_option(rctx.effect) {
                return;
            }

            let player = resolve_player(rctx.effect, union_flow.of);
            let Some(source) = rctx.effect.source_permanent else {
                return;
            };
            let Some(source_card) = rctx.effect.permanent_top_card_handle(source) else {
                return;
            };
            let subject_card = stable_replacement_subject_card(rctx.effect, rctx.subject);

            let continuation = DelayPlayFromUnionContinuation {
                source_player: source.player,
                source_card,
                subject: rctx.subject,
                subject_card,
                player,
                reward_steps: union_flow.reward_steps.clone(),
                runtime: runtime_for_union.clone(),
            };

            match rctx.effect.trash_delay_source_status() {
                DelayCostStatus::Paid => {
                    rctx.handled();
                    install_delay_play_from_union_after_paid(rctx.effect, &continuation);
                }
                DelayCostStatus::Pending => {
                    rctx.handled();
                    arm_pending_delay_play_from_union_continuation(rctx.effect.game, continuation);
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
        || pred.source_count.is_some()
        || pred.link_card_count.is_some()
        || pred.has_inherited.is_some()
        || pred.is_suspended.is_some()
        || pred.is_unsuspended.is_some()
        || pred.can_change_orientation.is_some()
        || pred.can_unsuspend.is_some()
        || pred.can_attack.is_some()
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

    // ST20-14 shape (`DelaySelfTrashPlayFromHandFlow`): the first step is the
    // Delay COST (`delete_permanent {source}`), and the reward's `select_hand`
    // is the second step. The accept prompt is NOT gated on the hand having an
    // eligible candidate — DCGO ST20_14.cs:83-87 `CanUseCondition` is only
    // `CanTriggerWhenPermanentRemoveField(subject matches) &&
    // CanDeclareOptionDelayEffect(card)`, with no candidate check; the hand
    // scan (`card.Owner.HandCards.Count(CanPlayAdventureCondition) >= 1`,
    // ST20_14.cs:125) happens inside `ActivateCoroutine` AFTER the self-trash.
    // So the offer installs even with zero eligible hand candidates; accepting
    // pays the <Delay> cost (self-trash), the reward whiffs, and the leaver
    // still leaves — same ungated-offer contract as the union-zone sibling
    // `DelaySelfTrashPlayFromUnionFlow` (BT19-099). The generic first-step
    // check below returns `true` for the `DeletePermanent` cost step.

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
        | CompiledStep::PlacePermanentOnSecurityAndHandleReplacement { .. }
        | CompiledStep::SecurityPlaceTopStackedCard { .. } => true,
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
        // G-ENGINE-SAME-LEVEL-SOURCE-PAIR-SELECTION — a required multi-pick
        // over the carrier's digivolution cards is payable only when the
        // offerable pool (filter + `uniqueness` / `same_by` pool restriction)
        // reaches the floor. EX13-016 Omnimon: only offer the save when a
        // same-level PAIR exists.
        CompiledStep::SelectMaterials {
            named_groups,
            of_permanent,
            max,
            filter,
            uniqueness,
            optional_zero,
            min,
            same_by,
            ..
        } => {
            let floor = (*min).max(if *optional_zero { 0 } else { 1 });
            if floor == 0 {
                return true;
            }
            if let digimon_dsl::compiled::CompiledCountBound::Literal(m) = max {
                if *m == 0 {
                    return false;
                }
            }
            let Some(perm) = resolve_step_permanent(of_permanent, ctx, bindings) else {
                return false;
            };
            if !named_groups.is_empty() {
                return crate::dsl_cards::step::selections::named_material_candidate_count(
                    ctx,
                    perm,
                    filter,
                    bindings,
                    named_groups,
                ) >= floor as usize;
            }
            let mode = match same_by {
                Some(digimon_dsl::compiled::CompiledSameBy::Level) => {
                    Some(crate::effect_context::DistinctByMode::SameLevel)
                }
                None => uniqueness.map(|u| match u {
                    digimon_dsl::compiled::CompiledDistinctBy::CardNumber => {
                        crate::effect_context::DistinctByMode::CardNumber
                    }
                    digimon_dsl::compiled::CompiledDistinctBy::Level => {
                        crate::effect_context::DistinctByMode::Level
                    }
                    digimon_dsl::compiled::CompiledDistinctBy::Name => {
                        crate::effect_context::DistinctByMode::Name
                    }
                    digimon_dsl::compiled::CompiledDistinctBy::Color => {
                        crate::effect_context::DistinctByMode::Color
                    }
                }),
            };
            crate::dsl_cards::step::selections::count_capped_card_candidate_indices(
                ctx.game,
                perm.player,
                crate::effect_context::CountCappedZone::Material(perm),
                filter,
                bindings,
                ctx.source_card,
                ctx.source_permanent,
                ctx.source_kind,
                ctx.player,
                mode,
                floor as usize,
            )
            .len()
                >= floor as usize
        }
        // G-TOP-STACKED-CARD-TO-SECURITY — "by placing its top stacked card
        // as the top security card" is payable only while a digivolution
        // card sits under the top (the Digimon must survive the move).
        CompiledStep::SecurityPlaceTopStackedCard { carrier, .. } => {
            resolve_step_permanent(carrier, ctx, bindings)
                .and_then(|h| ctx.game.player(h.player).battle_area.get(h.index as usize))
                .is_some_and(|perm| perm.card_sources.len() >= 2)
        }
        CompiledStep::SelectOwnSources {
            target,
            filter,
            min,
            max,
            ..
        } => {
            if *min == 0 {
                return true;
            }
            if *max < *min {
                return false;
            }
            // Count sources matching `filter` under the `target` permanent (or,
            // when unscoped, across all the controller's stacks). Gap 2 pay-in-
            // full: the accept prompt is offered only when ≥ `min` such sources
            // exist, so BT21-062's "return 4 [Vemmon]" is not offered with < 4.
            matching_own_source_count(ctx, target.as_ref(), filter, bindings) >= *min as usize
        }
        CompiledStep::SelectOpponentDpBudget {
            dp_budget,
            min_picks,
            deletion_cap,
            ..
        } => {
            let bonus = if *deletion_cap {
                crate::dsl_cards::predicate::deletion_max_bonus_for_source(ctx)
            } else {
                0
            };
            let budget = bonus
                + formula_eval::evaluate_read_with_bindings(
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
        } => {
            // Evaluate the (possibly scaling) budget formula, mirroring the
            // `SelectOpponentDpBudget` arm above. Scalar users compile to a
            // `CompiledFormula::Literal` and evaluate to that constant.
            let budget = formula_eval::evaluate_read_with_bindings(
                play_cost_budget,
                ctx,
                ctx.source_permanent.unwrap_or(PermanentHandle {
                    player: ctx.player,
                    index: u8::MAX,
                }),
                Some(bindings),
            );
            *min_picks == 0 || has_opponent_play_cost_budget_candidate(ctx, budget)
        }
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
        // Both link-card leave-replacement costs (trash, or place-as-bottom-
        // source for EX11-027) are payable only when the leaving permanent has
        // ≥1 link card. Gates the optional accept prompt so it is not offered
        // with 0 link cards.
        CompiledStep::TrashOwnLinkCardAndCancelLeave
        | CompiledStep::PlaceLinkCardAsBottomSourceAndCancelLeave => bindings
            .get_permanent("replacement_subject")
            .and_then(|h| ctx.game.player(h.player).battle_area.get(h.index as usize))
            .is_some_and(|perm| !perm.linked_cards.is_empty()),
        // BT26 <Detach ([X] trait)> — payable only when the leaving permanent
        // has ≥1 link card matching the keyword's card condition.
        CompiledStep::TrashOwnLinkCardMatchingAndCancelLeave { filter } => bindings
            .get_permanent("replacement_subject")
            .is_some_and(|h| {
                !crate::dsl_cards::step::replacement_outcome::matching_link_cards_read(
                    ctx, bindings, h, filter,
                )
                .is_empty()
            }),
        // EX7-048 — the protect-OTHERS Option-trash cost is payable only when the
        // CARRIER (the effect's own `source_permanent`, NOT the leaving subject)
        // has ≥1 Option among its below-top digivolution sources. Gates the
        // optional accept prompt so it is not offered when the carrier cannot pay.
        CompiledStep::TrashOptionFromOwnDigivolutionCardsAndCancelLeave => ctx
            .source_permanent
            .and_then(|h| ctx.game.player(h.player).battle_area.get(h.index as usize))
            .is_some_and(|perm| {
                let n = perm.card_sources.len();
                n > 1
                    && perm.card_sources.iter().take(n - 1).any(|s| {
                        s.is_option(ctx.card_data())
                            && !ctx.game.source_card_trash_blocked(
                                ctx.source_permanent.unwrap(),
                                s.handle(),
                                ctx.player,
                            )
                    })
            }),
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

/// Resolve a step's permanent binding for the read-only preflight: named
/// bindings (incl. `replacement_subject`) plus the effect's own carrier.
fn resolve_step_permanent(
    binding: &CompiledBindingRef,
    ctx: &EffectReadContext<'_>,
    bindings: &Bindings,
) -> Option<PermanentHandle> {
    match binding {
        CompiledBindingRef::Source | CompiledBindingRef::SelfRef | CompiledBindingRef::Carrier => {
            ctx.source_permanent
        }
        other => resolve_permanent_binding(other, bindings),
    }
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

/// Count the controller's digivolution sources (cards BELOW each stack's top)
/// that match `filter`, restricted to the `target` permanent when one is given.
/// Mirrors `select_own_sources`'s execution-time candidate rule
/// (`source_multi_candidates_data`): candidates are the below-top sources,
/// filtered on `PredicateSubject::Source`, optionally scoped to a single
/// permanent. Used by the `SelectOwnSources` replacement-cost preflight so a
/// pay-in-full cost (BT21-062 "return 4 [Vemmon]") is not offered with < min
/// matching sources. `CompiledBindingRef::Source` resolves to the effect's own
/// `source_permanent` (the leaving Digimon for a leave-field replacement).
fn matching_own_source_count(
    ctx: &EffectReadContext<'_>,
    target: Option<&CompiledBindingRef>,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> usize {
    let target_permanent = match target {
        Some(CompiledBindingRef::Source) => ctx.source_permanent,
        Some(other) => resolve_permanent_binding(other, bindings),
        None => None,
    };
    // Any EXPLICIT target binding that fails to resolve means zero candidates
    // (the picker would set `target_resolution_failed` and reject everything).
    // `target: None` (unscoped) legitimately counts across all own stacks.
    if target.is_some() && target_permanent.is_none() {
        return 0;
    }
    let controller = ctx.player;
    let mut count = 0usize;
    for field_index in 0..ctx.game.player(controller).battle_area.len() {
        let handle = PermanentHandle {
            player: controller,
            index: field_index as u8,
        };
        if target_permanent.is_some_and(|t| t != handle) {
            continue;
        }
        let n = ctx.game.player(controller).battle_area[field_index]
            .card_sources
            .len();
        if n <= 1 {
            continue;
        }
        for source_index in 0..(n - 1) {
            let card = ctx.game.player(controller).battle_area[field_index].card_sources
                [source_index]
                .handle();
            let source = crate::selection::SourceSelectionRef {
                permanent: handle,
                field_index: field_index as u8,
                source_index: source_index as u8,
                card,
            };
            if eval_predicate_with_bindings(
                filter,
                ctx,
                PredicateSubject::Source(source),
                Some(bindings),
            ) {
                count += 1;
            }
        }
    }
    count
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

    let permanent = crate::replacement::replacement_carrier(ctx.game, handle);

    permanent.is_some_and(|perm| {
        // The source card may live in the digivolution stack (normal /
        // inherited effects) OR in `linked_cards` (a `.linked()` Link-ESS whose
        // host is this permanent — BT25-101 Divine Arms Version Ω leave-
        // replacement). Accept either zone.
        perm.card_sources
            .iter()
            .chain(perm.linked_cards.iter())
            .any(|source| source.handle() == ctx.source_card)
    })
}

struct DelayHandDigivolveFlow {
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    prompt: String,
}

struct DelaySelfCancelFlow;

/// ST20-14 Our Courage United — Delay-cost self-trash then optionally PLAY a
/// filtered card from hand, without cancelling the leave.
///
/// Recognised process:
/// ```text
/// [ delete_permanent { target: source },      // pay the <Delay> cost
///   select_hand { bind_as: <pick>, optional: true, filter: ... },
///   play_from_hand_free { of: <of>, hand_index: <pick> } ]
/// ```
struct DelaySelfTrashPlayFromHandFlow {
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    prompt: String,
}

#[derive(Clone, Debug)]
pub(crate) struct DelayPlayFromHandContinuation {
    source_player: PlayerId,
    source_card: CardHandle,
    /// The leaving subject (this flow `handled()` the replacement, so it owns
    /// the leaver's deletion — see the reward step).
    subject: ReplacementSubject,
    subject_card: Option<(PlayerId, CardHandle)>,
    player: PlayerId,
    prompt: String,
    filter: CompiledPredicate,
}

/// Resume state for the ST20-14 Delay-cost hand-PLAY reward prompt. Data-driven
/// (clone-safe) — resolving the prompt PLAYS the chosen hand card for free, then
/// finishes the (owned) leave by deleting the leaver via its stable card handle.
/// Candidates are stored as stable `CardHandle`s (not raw indices) so the play
/// is correct even if the hand shifts between install and resolution.
#[derive(Clone, Debug)]
pub(crate) struct DelayPlayFromHandSelectionState {
    player: PlayerId,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: EffectSourceKind,
    candidates: Vec<CardHandle>,
    /// The leaving subject to finish deleting after the (optional) reward play.
    subject: ReplacementSubject,
    subject_card: Option<(PlayerId, CardHandle)>,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
}

/// BT19-099 The Wicked God Descends! — Delay-cost self-trash then optionally
/// PLAY a filtered card from hand OR trash for free, without cancelling the
/// leave. Union-zone sibling of [`DelaySelfTrashPlayFromHandFlow`].
///
/// Recognised process:
/// ```text
/// [ delete_permanent { target: source },      // pay the <Delay> cost
///   select_union_zone { of, zones ⊆ [hand, trash], bind_as: <pick>,
///                       optional: true, filter: ... },
///   play_union_bound_free { binding: <pick> } ]
/// ```
struct DelaySelfTrashPlayFromUnionFlow {
    of: CompiledPlayerRef,
    /// The reward steps (`select_union_zone` + `play_union_bound_free`) —
    /// exactly `process[1..]`, run verbatim through the generic step runner
    /// after the cost + owned leave complete.
    reward_steps: Arc<[CompiledStep]>,
}

impl DelaySelfTrashPlayFromUnionFlow {
    fn from_process(process: &[CompiledStep]) -> Option<Self> {
        let [CompiledStep::DeletePermanent { target }, CompiledStep::SelectUnionZone {
            of,
            zones,
            material_of,
            zone_filters,
            material_carrier_filter,
            bind_as,
            optional,
            cost,
            then,
            ..
        }, CompiledStep::PlayUnionBoundFree { binding, .. }] = process
        else {
            return None;
        };

        let bind_as = bind_as.as_deref()?;
        // Cost trashes THIS Option; the reward pick is the printed "you may
        // play" (optional, not a clause-aborting cost); the play consumes the
        // same union-bound card the select bound.
        if !matches!(target, CompiledBindingRef::Source) {
            return None;
        }
        if !*optional || *cost {
            return None;
        }
        if binding != bind_as {
            return None;
        }
        // Hand/trash zones only (the DCGO BT19-099 reference surface); a
        // material zone, per-zone filter overrides, or a scoped `then:` tail
        // fall back to the generic path.
        if zones.is_empty()
            || !zones
                .iter()
                .all(|z| matches!(z, CompiledZone::Hand | CompiledZone::Trash))
        {
            return None;
        }
        if material_of.is_some()
            || material_carrier_filter.is_some()
            || zone_filters.hand.is_some()
            || zone_filters.trash.is_some()
            || zone_filters.material.is_some()
            || !then.is_empty()
        {
            return None;
        }

        Some(Self {
            of: *of,
            reward_steps: Arc::from(&process[1..]),
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DelayPlayFromUnionContinuation {
    source_player: PlayerId,
    source_card: CardHandle,
    /// The leaving subject (this flow `handled()` the replacement, so it owns
    /// the leaver's deletion — see `install_delay_play_from_union_after_paid`).
    subject: ReplacementSubject,
    subject_card: Option<(PlayerId, CardHandle)>,
    player: PlayerId,
    /// `select_union_zone` + `play_union_bound_free`, run verbatim through
    /// the generic step runner once the cost + owned leave are done.
    reward_steps: Arc<[CompiledStep]>,
    runtime: StepRuntime,
}

#[derive(Clone, Debug)]
pub(crate) struct DelayCancelContinuation {
    source_player: PlayerId,
    source_card: CardHandle,
}

#[derive(Clone, Debug)]
pub(crate) struct DelayCostContinuation {
    source_player: PlayerId,
    source_card: CardHandle,
    subject: ReplacementSubject,
    subject_card: Option<(PlayerId, CardHandle)>,
    player: PlayerId,
    prompt: String,
    filter: CompiledPredicate,
}

#[derive(Clone, Debug)]
pub(crate) struct DelayHandDigivolveSelectionState {
    subject: ReplacementSubject,
    player: PlayerId,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: EffectSourceKind,
    candidates: Vec<CardHandle>,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
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

impl DelaySelfTrashPlayFromHandFlow {
    fn from_process(process: &[CompiledStep]) -> Option<Self> {
        let [CompiledStep::DeletePermanent { target }, CompiledStep::SelectHand {
            of,
            filter,
            bind_as,
            prompt,
            optional,
            ..
        }, CompiledStep::PlayFromHandFree {
            of: play_of,
            hand_index,
            ..
        }] = process
        else {
            return None;
        };

        // Cost trashes THIS Option; the reward is optional ("you may play");
        // the play consumes the same hand card the `select_hand` bound; both
        // selection and play refer to the same player.
        let bind_as = bind_as.as_deref()?;
        if !matches!(target, CompiledBindingRef::Source) {
            return None;
        }
        if !*optional {
            return None;
        }
        if of != play_of {
            return None;
        }
        if !is_binding_ref_named(hand_index, bind_as) {
            return None;
        }

        Some(Self {
            of: *of,
            filter: filter.clone(),
            prompt: prompt.clone(),
        })
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
            ignore_level: false,
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
/// Paid Delay followed by an optional, recipe-checked pair of field materials.
/// Unlike hand-partner DNA, neither material is tied to the leave subject.
#[derive(Clone, Debug)]
struct DelayFieldDnaDigivolveFlow {
    of: CompiledPlayerRef,
    result_filter: CompiledPredicate,
    result_prompt: String,
    left_filter: CompiledPredicate,
    right_filter: CompiledPredicate,
    pair_prompt: String,
}

impl DelayFieldDnaDigivolveFlow {
    fn from_process(process: &[CompiledStep]) -> Option<Self> {
        let [CompiledStep::DeletePermanent { target }, CompiledStep::SelectHand {
            of,
            filter,
            bind_as,
            prompt,
            optional,
            cost: selection_cost,
            then,
            ..
        }, CompiledStep::SelectDnaPair {
            left_filter,
            right_filter,
            bind_left_as,
            bind_right_as,
            prompt: pair_prompt,
            optional: pair_optional,
            ..
        }, CompiledStep::EffectInitiatedDnaDigivolve {
            target_a,
            target_b,
            from_hand,
            cost,
            ignore_requirements,
        }] = process
        else {
            return None;
        };
        if !matches!(target, CompiledBindingRef::Source)
            || !matches!(of, CompiledPlayerRef::You)
            || !*optional
            || !*pair_optional
            || *selection_cost
            || !then.is_empty()
            || !matches!(cost, CompiledCostDelta::Printed)
            || *ignore_requirements
            || !is_binding_ref_named(target_a, bind_left_as)
            || !is_binding_ref_named(target_b, bind_right_as)
            || !is_binding_ref_named(from_hand, bind_as.as_deref()?)
        {
            return None;
        }
        Some(Self {
            of: *of,
            result_filter: filter.clone(),
            result_prompt: prompt.clone(),
            left_filter: left_filter.clone(),
            right_filter: right_filter.clone(),
            pair_prompt: pair_prompt.clone(),
        })
    }
}

fn source_delay_is_ready(ctx: &EffectReadContext<'_>) -> bool {
    ctx.source_permanent.and_then(|h| ctx.game.player(h.player).battle_area.get(h.index as usize))
        .is_some_and(|p| matches!(p.option_state, OptionState::Delayed { placed_on_turn, .. } if placed_on_turn < ctx.game.turn_count))
}

#[derive(Clone, Debug)]
pub(crate) struct DelayFieldDnaContinuation {
    source_player: PlayerId,
    source_card: CardHandle,
    subject_card: Option<(PlayerId, CardHandle)>,
    player: PlayerId,
    flow: DelayFieldDnaDigivolveFlow,
}

#[derive(Clone, Debug)]
enum DelayFieldDnaStage {
    Result,
    First {
        result: CardHandle,
    },
    Second {
        result: CardHandle,
        first: CardHandle,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct DelayFieldDnaSelectionState {
    continuation: DelayFieldDnaContinuation,
    stage: DelayFieldDnaStage,
    candidates: Vec<CardHandle>,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
}

/// A field material's top-card snapshot follows index compaction, but never
/// rebinds an already-consumed material to a new DNA stack containing it.
fn delay_field_material(
    ctx: &EffectContext<'_>,
    player: PlayerId,
    card: CardHandle,
) -> Option<PermanentHandle> {
    ctx.game
        .player(player)
        .battle_area
        .iter()
        .position(|p| p.top_card().handle() == card)
        .map(|index| PermanentHandle {
            player,
            index: index as u8,
        })
}

/// Pair selection permits either presentation order. Normalize using the
/// result's recipe, including the same result-scoped identity grants used by
/// the main-action DNA path, before passing stacks to the merge primitive.
fn delay_field_dna_material_order(
    ctx: &EffectContext<'_>,
    player: PlayerId,
    result: CardHandle,
    first: CardHandle,
    second: CardHandle,
) -> Option<(PermanentHandle, PermanentHandle)> {
    let a = delay_field_material(ctx, player, first)?;
    let b = delay_field_material(ctx, player, second)?;
    let game = &ctx.game;
    let result = game
        .player(player)
        .hand
        .iter()
        .find(|card| card.handle() == result)?;
    let extras_a = game.dna_material_extras(a, result);
    let extras_b = game.dna_material_extras(b, result);
    let (first_is_top, _) = crate::dna_digivolve::get_dna_stacking_order_ext(
        &game.card_data[result.data_index],
        (
            &game.player(player).battle_area[a.index as usize],
            &extras_a,
        ),
        (
            &game.player(player).battle_area[b.index as usize],
            &extras_b,
        ),
        &game.card_data,
    )?;
    Some(if first_is_top { (a, b) } else { (b, a) })
}

fn delay_field_dna_pairs(
    ctx: &EffectContext<'_>,
    cont: &DelayFieldDnaContinuation,
    result: CardHandle,
) -> Vec<(CardHandle, CardHandle, i32)> {
    let game = &ctx.game;
    if !game
        .player(cont.player)
        .hand
        .iter()
        .any(|c| c.handle() == result)
        || game
            .modifiers
            .player_has(cont.player, ModifierType::CannotDigivolveDigimonByEffect)
        || !eval_predicate(
            &cont.flow.result_filter,
            &ctx.as_read(),
            PredicateSubject::Card(result),
        )
    {
        return Vec::new();
    }
    let materials: Vec<_> = game
        .player(cont.player)
        .battle_area
        .iter()
        .enumerate()
        .filter_map(|(index, perm)| {
            let handle = PermanentHandle {
                player: cont.player,
                index: index as u8,
            };
            (perm.is_digimon_for_rules(&game.card_data, &game.modifiers, handle)
                && !game.modifiers.has(handle, ModifierType::CannotDigivolve)
                && ctx.can_affect_permanent(handle))
            .then_some((handle, perm.top_card().handle()))
        })
        .collect();
    let read = ctx.as_read();
    let mut pairs = Vec::new();
    for &(left, left_card) in &materials {
        if !eval_predicate(
            &cont.flow.left_filter,
            &read,
            PredicateSubject::Permanent(left),
        ) {
            continue;
        }
        for &(right, right_card) in &materials {
            if left == right
                || !eval_predicate(
                    &cont.flow.right_filter,
                    &read,
                    PredicateSubject::Permanent(right),
                )
            {
                continue;
            }
            let Some(cost) = ctx.printed_dna_cost_for_pair(left, right, result) else {
                continue;
            };
            let after = i32::from(game.memory)
                + if cont.player == game.turn_player() {
                    -cost
                } else {
                    cost
                };
            if after >= i32::from(game.rules.memory_range.0)
                && after <= i32::from(game.rules.memory_range.1)
            {
                pairs.push((left_card, right_card, cost));
            }
        }
    }
    pairs
}

fn arm_delay_field_dna_payment(game: &mut Game, continuation: DelayFieldDnaContinuation) {
    let Some(mut selection) = game.pending_selection.take() else {
        continue_delay_field_dna_payment(game, continuation);
        return;
    };
    if let Some(inner) = game.pending_selection_resume.take() {
        game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DelayFieldDnaAfterPayment {
                inner: Box::new(inner),
                continuation,
                outer_conts: Vec::new(),
            }],
        });
    } else {
        let callback = selection.callback;
        let after = continuation.clone();
        selection.callback = Box::new(move |game, action| {
            callback(game, action);
            continue_delay_field_dna_payment(game, after);
        });
        let decline = selection.on_decline.take();
        selection.on_decline = Some(Box::new(move |game| {
            if let Some(decline) = decline {
                decline(game);
            }
            continue_delay_field_dna_payment(game, continuation);
        }));
    }
    game.pending_selection = Some(selection);
}

pub(crate) fn continue_delay_field_dna_payment(
    game: &mut Game,
    continuation: DelayFieldDnaContinuation,
) {
    if game.pending_selection.is_some() {
        arm_delay_field_dna_payment(game, continuation);
        return;
    }
    let mut ctx = EffectContext::new(game, continuation.source_card, None, continuation.player);
    if ctx.delay_source_card_in_trash(continuation.source_player, continuation.source_card) {
        install_delay_field_dna_stage(&mut ctx, continuation, DelayFieldDnaStage::Result);
    }
}

fn install_delay_field_dna_stage(
    ctx: &mut EffectContext<'_>,
    continuation: DelayFieldDnaContinuation,
    stage: DelayFieldDnaStage,
) {
    let mut candidates = match stage {
        DelayFieldDnaStage::Result => matching_hand_candidates(
            ctx,
            continuation.player,
            &continuation.flow.result_filter,
            HAND_MAIN_LIMIT,
        )
        .into_iter()
        .filter(|result| !delay_field_dna_pairs(ctx, &continuation, *result).is_empty())
        .collect::<Vec<_>>(),
        DelayFieldDnaStage::First { result } => delay_field_dna_pairs(ctx, &continuation, result)
            .into_iter()
            .map(|(a, _, _)| a)
            .collect(),
        DelayFieldDnaStage::Second { result, first } => {
            delay_field_dna_pairs(ctx, &continuation, result)
                .into_iter()
                .filter(|(a, _, _)| *a == first)
                .map(|(_, b, _)| b)
                .collect()
        }
    };
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|card| seen.insert(*card));
    if candidates.is_empty() {
        return;
    }
    let prompt = match stage {
        DelayFieldDnaStage::Result => continuation.flow.result_prompt.clone(),
        _ => continuation.flow.pair_prompt.clone(),
    };
    let state = DelayFieldDnaSelectionState {
        continuation,
        stage,
        candidates,
        outer_conts: Vec::new(),
    };
    let choices: Vec<_> = state
        .candidates
        .iter()
        .enumerate()
        .map(|(i, card)| EffectChoiceEntry {
            count_choice: None,
            label: ctx
                .game
                .card_data_for_handle(*card)
                .map(|d| d.card_name.clone())
                .unwrap_or_default(),
            action_id: HAND_EFFECT_START + i as u16,
            source_card: Some(*card),
            source_kind: None,
            timing: None,
            is_optional: false,
            keyword: None,
            observation_metadata: Default::default(),
        })
        .collect();
    let callback_state = state.clone();
    let decline_state = state.clone();
    let previous_phase = ctx.game.current_phase;
    ctx.game.current_phase = GamePhase::EffectChoice;
    ctx.game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        kind: SelectionKind::EffectChoice,
        selecting_player: state.continuation.player,
        previous_phase,
        valid_action_ids: choices.iter().map(|c| c.action_id).collect(),
        is_optional: true,
        prompt,
        effect_choices: Some(choices),
        source_card: state.continuation.source_card,
        source_permanent: None,
        source_kind: ctx.source_kind(),
        callback: Box::new(move |game, action| {
            run_delay_field_dna_selection(game, callback_state, action, false)
        }),
        on_decline: Some(Box::new(move |game| {
            run_delay_field_dna_selection(game, decline_state, crate::action::space::PASS, true)
        })),
    });
    ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::DelayFieldDnaSelection(state)],
    });
}

pub(crate) fn run_delay_field_dna_selection(
    game: &mut Game,
    state: DelayFieldDnaSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    let DelayFieldDnaSelectionState {
        continuation,
        stage,
        candidates,
        outer_conts,
    } = state;
    if !is_pass {
        if let Some(chosen) = action_id
            .checked_sub(HAND_EFFECT_START)
            .and_then(|i| candidates.get(i as usize))
            .copied()
        {
            let mut ctx =
                EffectContext::new(game, continuation.source_card, None, continuation.player);
            match stage {
                DelayFieldDnaStage::Result => install_delay_field_dna_stage(
                    &mut ctx,
                    continuation,
                    DelayFieldDnaStage::First { result: chosen },
                ),
                DelayFieldDnaStage::First { result } => install_delay_field_dna_stage(
                    &mut ctx,
                    continuation,
                    DelayFieldDnaStage::Second {
                        result,
                        first: chosen,
                    },
                ),
                DelayFieldDnaStage::Second { result, first } => {
                    let pair = delay_field_dna_pairs(&ctx, &continuation, result)
                        .into_iter()
                        .find(|(a, b, _)| *a == first && *b == chosen);
                    if let Some((_, _, cost)) = pair {
                        if let Some((a, b)) = delay_field_dna_material_order(
                            &ctx,
                            continuation.player,
                            result,
                            first,
                            chosen,
                        ) {
                            let consumed_subject =
                                continuation.subject_card.is_some_and(|(player, card)| {
                                    player == continuation.player
                                        && (card == first || card == chosen)
                                });
                            if ctx
                                .effect_initiated_dna_digivolve(a, b, result, cost, false)
                                .is_some()
                                && consumed_subject
                            {
                                ctx.cancel_current_replacement();
                            }
                        }
                    }
                }
            }
        }
    }
    crate::dsl_cards::step::selections::run_outer_conts(game, outer_conts);
}

struct DelayHandDnaDigivolveFlow {
    of: CompiledPlayerRef,
    result_filter: CompiledPredicate,
    result_prompt: String,
    partner_filter: CompiledPredicate,
    partner_prompt: String,
}

#[derive(Clone, Debug)]
pub(crate) struct DelayDnaCostContinuation {
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

/// Which selection stage a `DnaSelectionStage` callback represents.
#[derive(Clone, Debug)]
pub(crate) enum DnaSelectionStage {
    /// Stage 1 -- the chosen card is the Omnimon result. Chain into Stage 2.
    Result(DelayDnaCostContinuation),
    /// Stage 2 -- the chosen card is the hand partner; `result` is the
    /// already-chosen Omnimon card. Execute the DNA merge.
    Partner {
        continuation: DelayDnaCostContinuation,
        result: CardHandle,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct DelayDnaCardSelectionState {
    player: PlayerId,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: EffectSourceKind,
    candidates: Vec<CardHandle>,
    stage: DnaSelectionStage,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
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
            // A card reserved by an in-flight DigiXros play (the host being
            // played, or a hand card already selected as a material) is committed
            // to that play and not a free hand card for intervening effects.
            if ctx.game.card_reserved_by_pending_digixros(card) {
                return None;
            }
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
            count_choice: None,
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
            keyword: None,
            observation_metadata: Default::default(),
        })
        .collect();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind();
    let resume_candidates = candidates.clone();

    ctx.game.current_phase = GamePhase::EffectChoice;
    ctx.game.pending_selection = Some(PendingSelection {
        zone_owner: None,
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
    ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::DelayHandDigivolveSelection(
            DelayHandDigivolveSelectionState {
                subject,
                player,
                source_card,
                source_permanent,
                source_kind,
                candidates: resume_candidates,
                outer_conts: Vec::new(),
            },
        )],
    });
}

pub(crate) fn run_delay_hand_digivolve_selection_step(
    game: &mut Game,
    state: DelayHandDigivolveSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        return;
    }
    let DelayHandDigivolveSelectionState {
        subject,
        player,
        source_card,
        source_permanent,
        source_kind,
        candidates,
        outer_conts,
    } = state;
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
    crate::dsl_cards::step::selections::run_outer_conts(ctx.game, outer_conts);
}

// ─── ST20-14 — Delay-cost trash-self then optionally PLAY 1 from hand ──────────

/// Arm the ST20-14 Delay-cost hand-PLAY reward onto a pending selection that
/// was opened while paying the <Delay> cost (the rare `Pending` case — e.g. an
/// interrupt around the self-trash). Composes onto the resumable VM when the
/// pending selection is already resume-driven, else onto its closure.
fn arm_pending_delay_play_from_hand_continuation(
    game: &mut Game,
    continuation: DelayPlayFromHandContinuation,
) {
    let Some(mut selection) = game.pending_selection.take() else {
        continue_delay_play_from_hand_after_selection(game, continuation);
        return;
    };

    if let Some(inner) = game.pending_selection_resume.take() {
        game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![
                crate::resume::ResumeFrame::DelayPlayFromHandAfterSelection {
                    inner: Box::new(inner),
                    continuation,
                    outer_conts: Vec::new(),
                },
            ],
        });
        game.pending_selection = Some(selection);
        return;
    }

    let original_callback = selection.callback;
    let callback_continuation = continuation.clone();
    selection.callback = Box::new(move |game, action_id| {
        original_callback(game, action_id);
        continue_delay_play_from_hand_after_selection(game, callback_continuation);
    });

    let original_decline = selection.on_decline.take();
    selection.on_decline = Some(Box::new(move |game| {
        if let Some(original_decline) = original_decline {
            original_decline(game);
        }
        continue_delay_play_from_hand_after_selection(game, continuation);
    }));

    game.pending_selection = Some(selection);
}

pub(crate) fn continue_delay_play_from_hand_after_selection(
    game: &mut Game,
    continuation: DelayPlayFromHandContinuation,
) {
    if game.pending_selection.is_some() {
        arm_pending_delay_play_from_hand_continuation(game, continuation);
        return;
    }

    let mut ctx = EffectContext::new(game, continuation.source_card, None, continuation.player);
    install_delay_play_from_hand_after_paid(&mut ctx, &continuation);
}

/// The <Delay> cost is paid (ST20-14 self-trashed). This flow `handled()` the
/// replacement, so it OWNS the leaver's deletion. Finish the leave first (delete
/// the leaver by its stable card handle — faithful to DCGO's "the Digimon still
/// leaves"), then install the optional hand-PLAY reward prompt for a Lv5-
/// [ADVENTURE] Digimon. If nothing qualifies the reward is skipped (the printed
/// "may" — declining or having no target is legal); the leave still happened.
fn install_delay_play_from_hand_after_paid(
    ctx: &mut EffectContext<'_>,
    continuation: &DelayPlayFromHandContinuation,
) {
    if !ctx.delay_source_card_in_trash(continuation.source_player, continuation.source_card) {
        return;
    }

    // Finish the (owned) leave: delete the leaving subject by its stable handle
    // BEFORE the reward play so the play cannot collide with the leaver's slot.
    finish_owned_leave(ctx, continuation.subject, continuation.subject_card);

    let candidates = matching_hand_candidates(
        ctx,
        continuation.player,
        &continuation.filter,
        HAND_MAIN_LIMIT,
    );
    if candidates.is_empty() {
        return;
    }

    install_delay_play_from_hand_selection(
        ctx,
        continuation.player,
        continuation.prompt.clone(),
        candidates,
    );
}

/// Delete the leaving subject via its stable card handle (re-resolved, so it is
/// robust to index shifts). No-op if the subject is already gone (e.g. deleted
/// by an intervening effect).
///
/// Cause note: the original leave was `handled()` at the replacement stage —
/// before any zone move — so no OnDeletion observers fired yet; this is the
/// FIRST and only deletion (no double-fire). We re-delete with `OwnEffect`
/// rather than replaying the original cause: replaying `Battle` outside the
/// resolved attack context would re-enter combat-deletion logic that assumes a
/// live `pending_attack`. `OwnEffect` is the safe generic re-deletion cause and
/// matches the printed intent ("the Digimon still leaves").
fn finish_owned_leave(
    ctx: &mut EffectContext<'_>,
    subject: ReplacementSubject,
    subject_card: Option<(PlayerId, CardHandle)>,
) {
    if let Some(handle) =
        resolve_stable_replacement_subject(ctx, subject, subject_card).and_then(|s| s.permanent())
    {
        ctx.game
            .delete_permanent_with_cause(handle, crate::replacement::ReplacementCause::OwnEffect);
    }
}

/// Install a data-driven (clone-safe) hand-card `EffectChoice` prompt; resolving
/// it plays the chosen card for free. The (owned) leave was already finished by
/// `install_delay_play_from_hand_after_paid`, so this only handles the reward.
fn install_delay_play_from_hand_selection(
    ctx: &mut EffectContext<'_>,
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
            count_choice: None,
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
            keyword: None,
            observation_metadata: Default::default(),
        })
        .collect();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind();
    let resume_candidates = candidates.clone();

    ctx.game.current_phase = GamePhase::EffectChoice;
    ctx.game.pending_selection = Some(PendingSelection {
        zone_owner: None,
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
            play_delay_hand_card(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
                card,
            );
        }),
        on_decline: None,
    });
    ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::DelayPlayFromHandSelection(
            DelayPlayFromHandSelectionState {
                player,
                source_card,
                source_permanent,
                source_kind,
                candidates: resume_candidates,
                subject: ReplacementSubject::Player(player),
                subject_card: None,
                outer_conts: Vec::new(),
            },
        )],
    });
}

/// Resolve `card` to its current hand index for `player`, then play it for
/// free. Index is re-resolved at play time so a hand shift after install is
/// handled correctly. No `cancel` — the original leave still proceeds.
fn play_delay_hand_card(
    game: &mut Game,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: EffectSourceKind,
    player: PlayerId,
    card: CardHandle,
) {
    let Some(hand_index) = game
        .player(player)
        .hand
        .iter()
        .position(|c| c.handle() == card)
    else {
        return;
    };
    let mut ctx = EffectContext::new_with_source_kind(
        game,
        source_card,
        source_permanent,
        source_kind,
        player,
    );
    let _ = ctx.play_from_hand_free(player, hand_index);
}

pub(crate) fn run_delay_play_from_hand_selection_step(
    game: &mut Game,
    state: DelayPlayFromHandSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    let DelayPlayFromHandSelectionState {
        player,
        source_card,
        source_permanent,
        source_kind,
        candidates,
        subject: _,
        subject_card: _,
        outer_conts,
    } = state;
    if is_pass {
        crate::dsl_cards::step::selections::run_outer_conts(game, outer_conts);
        return;
    }
    let Some(idx) = action_id.checked_sub(HAND_EFFECT_START).map(|i| i as usize) else {
        crate::dsl_cards::step::selections::run_outer_conts(game, outer_conts);
        return;
    };
    if let Some(card) = candidates.get(idx).copied() {
        play_delay_hand_card(
            game,
            source_card,
            source_permanent,
            source_kind,
            player,
            card,
        );
    }
    crate::dsl_cards::step::selections::run_outer_conts(game, outer_conts);
}

// ─── BT19-099 — Delay-cost trash-self then optionally PLAY 1 from hand/trash ──

/// Arm the BT19-099 Delay-cost union-PLAY reward onto a pending selection that
/// was opened while paying the <Delay> cost (the rare `Pending` case — e.g. an
/// interrupt around the self-trash) or while finishing the owned leave.
/// Composes onto the resumable VM when the pending selection is already
/// resume-driven, else onto its closure.
fn arm_pending_delay_play_from_union_continuation(
    game: &mut Game,
    continuation: DelayPlayFromUnionContinuation,
) {
    let Some(mut selection) = game.pending_selection.take() else {
        continue_delay_play_from_union_after_selection(game, continuation);
        return;
    };

    if let Some(inner) = game.pending_selection_resume.take() {
        game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![
                crate::resume::ResumeFrame::DelayPlayFromUnionAfterSelection {
                    inner: Box::new(inner),
                    continuation,
                    outer_conts: Vec::new(),
                },
            ],
        });
        game.pending_selection = Some(selection);
        return;
    }

    let original_callback = selection.callback;
    let callback_continuation = continuation.clone();
    selection.callback = Box::new(move |game, action_id| {
        original_callback(game, action_id);
        continue_delay_play_from_union_after_selection(game, callback_continuation);
    });

    let original_decline = selection.on_decline.take();
    selection.on_decline = Some(Box::new(move |game| {
        if let Some(original_decline) = original_decline {
            original_decline(game);
        }
        continue_delay_play_from_union_after_selection(game, continuation);
    }));

    game.pending_selection = Some(selection);
}

pub(crate) fn continue_delay_play_from_union_after_selection(
    game: &mut Game,
    continuation: DelayPlayFromUnionContinuation,
) {
    if game.pending_selection.is_some() {
        arm_pending_delay_play_from_union_continuation(game, continuation);
        return;
    }

    let mut ctx = EffectContext::new(game, continuation.source_card, None, continuation.player);
    install_delay_play_from_union_after_paid(&mut ctx, &continuation);
}

/// The <Delay> cost is paid (BT19-099 self-trashed). This flow `handled()` the
/// replacement, so it OWNS the leaver's deletion. Finish the leave FIRST —
/// the leaving Digimon is NOT prevented from leaving (DCGO BT19_099.cs places
/// no Cancel on the WhenRemoveField event), and resolving it before the reward
/// play means the play cannot collide with the leaver's battle-area slot.
/// Then run the reward steps (`select_union_zone` + `play_union_bound_free`)
/// through the generic step runner with `replacement_subject` re-bound as the
/// subject's stable top-card SNAPSHOT (a card binding): a
/// `play_cost_eq_binding` reward filter then reads the pre-leave printed cost
/// (DCGO BT19_099.cs:261 — `permanent.TopCard.GetCostItself + 1 ==
/// candidate.GetCostItself` against the trigger-hashtable snapshot). With
/// zero eligible candidates the reward select no-ops — the cost is already
/// paid and the leave already happened, matching DCGO's post-trash candidate
/// check (BT19_099.cs:275-277).
fn install_delay_play_from_union_after_paid(
    ctx: &mut EffectContext<'_>,
    continuation: &DelayPlayFromUnionContinuation,
) {
    // Finish the (owned) leave FIRST — even if the Delay-cost trash was itself
    // replaced/prevented (source not in trash below), the subject's leave must
    // still complete; only the reward is forfeit.
    finish_owned_leave(ctx, continuation.subject, continuation.subject_card);

    if !ctx.delay_source_card_in_trash(continuation.source_player, continuation.source_card) {
        return;
    }

    // The owned-leave deletion can itself park a selection (another watcher's
    // replacement window / trigger ordering). Re-arm and continue after it;
    // the re-entry's `finish_owned_leave` no-ops (the subject is gone).
    if ctx.game.pending_selection.is_some() {
        arm_pending_delay_play_from_union_continuation(ctx.game, continuation.clone());
        return;
    }

    let mut bindings = Bindings::new();
    if let Some((_, subject_card)) = continuation.subject_card {
        bindings.insert_card("replacement_subject", subject_card);
    }
    let _ = run_steps_with_runtime(
        &continuation.reward_steps,
        ctx,
        &mut bindings,
        &continuation.runtime,
    );
}

fn arm_pending_delay_cost_continuation(game: &mut Game, continuation: DelayCostContinuation) {
    let Some(mut selection) = game.pending_selection.take() else {
        continue_delay_cost_after_selection(game, continuation);
        return;
    };

    if let Some(inner) = game.pending_selection_resume.take() {
        game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![
                crate::resume::ResumeFrame::DelayHandDigivolveAfterSelection {
                    inner: Box::new(inner),
                    continuation,
                    outer_conts: Vec::new(),
                },
            ],
        });
        game.pending_selection = Some(selection);
        return;
    }

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

    if let Some(inner) = game.pending_selection_resume.take() {
        game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DelayCancelAfterSelection {
                inner: Box::new(inner),
                continuation,
                outer_conts: Vec::new(),
            }],
        });
        game.pending_selection = Some(selection);
        return;
    }

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

pub(crate) fn continue_delay_cancel_after_selection(
    game: &mut Game,
    continuation: DelayCancelContinuation,
) {
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

    if let Some(parked) = game.active_parked_replacement_mut() {
        parked.outcome = crate::replacement::ReplacementOutcome::Cancelled;
    }
}

pub(crate) fn continue_delay_cost_after_selection(
    game: &mut Game,
    continuation: DelayCostContinuation,
) {
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

    if let Some(inner) = game.pending_selection_resume.take() {
        game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DelayDnaAfterSelection {
                inner: Box::new(inner),
                continuation,
                outer_conts: Vec::new(),
            }],
        });
        game.pending_selection = Some(selection);
        return;
    }

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

pub(crate) fn continue_delay_dna_after_selection(
    game: &mut Game,
    continuation: DelayDnaCostContinuation,
) {
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
            count_choice: None,
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
            keyword: None,
            observation_metadata: Default::default(),
        })
        .collect();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind();
    let resume_candidates = candidates.clone();
    let resume_stage = stage.clone();

    ctx.game.current_phase = GamePhase::EffectChoice;
    ctx.game.pending_selection = Some(PendingSelection {
        zone_owner: None,
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
                        if let Some(parked) = game.active_parked_replacement_mut() {
                            parked.outcome = crate::replacement::ReplacementOutcome::Cancelled;
                        }
                    }
                }
            }
        }),
        on_decline: None,
    });
    ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::DelayDnaCardSelection(
            DelayDnaCardSelectionState {
                player,
                source_card,
                source_permanent,
                source_kind,
                candidates: resume_candidates,
                stage: resume_stage,
                outer_conts: Vec::new(),
            },
        )],
    });
}

pub(crate) fn run_delay_dna_card_selection_step(
    game: &mut Game,
    state: DelayDnaCardSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        return;
    }
    let DelayDnaCardSelectionState {
        player,
        source_card,
        source_permanent,
        source_kind,
        candidates,
        stage,
        outer_conts,
    } = state;
    let Some(idx) = action_id.checked_sub(HAND_EFFECT_START).map(|i| i as usize) else {
        return;
    };
    let Some(chosen) = candidates.get(idx).copied() else {
        return;
    };
    match &stage {
        DnaSelectionStage::Result(continuation) => {
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
            if !partner_candidates.is_empty() {
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
        }
        DnaSelectionStage::Partner {
            continuation,
            result,
        } => {
            let mut ctx = EffectContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            if let Some(subject) = resolve_stable_replacement_subject(
                &ctx,
                continuation.subject,
                continuation.subject_card,
            ) {
                if let Some(target) = subject.permanent() {
                    let merged = ctx.effect_initiated_dna_digivolve_with_hand_partner(
                        target, chosen, *result, 0, true,
                    );
                    if merged.is_some() {
                        ctx.cancel_current_replacement();
                        if let Some(parked) = ctx.game.active_parked_replacement_mut() {
                            parked.outcome = crate::replacement::ReplacementOutcome::Cancelled;
                        }
                    }
                }
            }
        }
    }
    crate::dsl_cards::step::selections::run_outer_conts(game, outer_conts);
}

fn process_has_paid_self_prevention(process: &[CompiledStep]) -> bool {
 let mut probe=PaidPreventionProbe::default();
 serde::Serialize::serialize(process,&mut probe).expect("infallible typed probe");
 probe.found
}
fn process_has_paid_all_prevention(process: &[CompiledStep]) -> bool {
 let mut probe=PaidPreventionProbe::default();
 serde::Serialize::serialize(process,&mut probe).expect("infallible typed probe");
 probe.all_matching
}
#[derive(Default)]
struct PaidPreventionProbe{found:bool,all_matching:bool,suspend_filter:Option<CompiledPredicate>,struct_names:Vec<&'static str>}
impl PaidPreventionProbe {
 fn variant(&mut self,name:&str,variant:&str){
  self.found |= name=="CompiledStep" && matches!(variant,"DeleteSelfAndCancelLeave"|"DeleteSelfAndCancelAllMatchingLeaves");
  self.all_matching |= name=="CompiledStep" && variant=="DeleteSelfAndCancelAllMatchingLeaves";
 }
}
macro_rules! primitive {($($method:ident($ty:ty)),*)=>{$(fn $method(self,_v:$ty)->Result<(),Self::Error>{Ok(())})*};}
impl<'a> serde::Serializer for &'a mut PaidPreventionProbe {
 type Ok=();type Error=serde_json::Error;
 type SerializeSeq=Self;type SerializeTuple=Self;type SerializeTupleStruct=Self;type SerializeTupleVariant=Self;
 type SerializeMap=Self;type SerializeStruct=Self;type SerializeStructVariant=Self;
 primitive!(serialize_bool(bool),serialize_i8(i8),serialize_i16(i16),serialize_i32(i32),serialize_i64(i64),serialize_u8(u8),serialize_u16(u16),serialize_u32(u32),serialize_u64(u64),serialize_f32(f32),serialize_f64(f64),serialize_char(char),serialize_str(&str),serialize_bytes(&[u8]));
 fn serialize_none(self)->Result<(),Self::Error>{Ok(())}
 fn serialize_some<T:serde::Serialize+?Sized>(self,v:&T)->Result<(),Self::Error>{v.serialize(self)}
 fn serialize_unit(self)->Result<(),Self::Error>{Ok(())}
 fn serialize_unit_struct(self,_n:&'static str)->Result<(),Self::Error>{Ok(())}
 fn serialize_unit_variant(self,n:&'static str,_i:u32,v:&'static str)->Result<(),Self::Error>{self.variant(n,v);Ok(())}
 fn serialize_newtype_struct<T:serde::Serialize+?Sized>(self,_n:&'static str,v:&T)->Result<(),Self::Error>{v.serialize(self)}
 fn serialize_newtype_variant<T:serde::Serialize+?Sized>(self,n:&'static str,_i:u32,v:&'static str,x:&T)->Result<(),Self::Error>{self.variant(n,v);x.serialize(self)}
 fn serialize_seq(self,_n:Option<usize>)->Result<Self,Self::Error>{Ok(self)}
 fn serialize_tuple(self,_n:usize)->Result<Self,Self::Error>{Ok(self)}
 fn serialize_tuple_struct(self,_s:&'static str,_n:usize)->Result<Self,Self::Error>{Ok(self)}
 fn serialize_tuple_variant(self,n:&'static str,_i:u32,v:&'static str,_len:usize)->Result<Self,Self::Error>{self.variant(n,v);Ok(self)}
 fn serialize_map(self,_n:Option<usize>)->Result<Self,Self::Error>{Ok(self)}
 fn serialize_struct(self,s:&'static str,_n:usize)->Result<Self,Self::Error>{self.struct_names.push(s);Ok(self)}
 fn serialize_struct_variant(self,n:&'static str,_i:u32,v:&'static str,_len:usize)->Result<Self,Self::Error>{self.variant(n,v);self.struct_names.push(if n=="CompiledStep" {v}else{n});Ok(self)}
}
macro_rules! sequence {($trait:ident,$method:ident)=>{impl serde::ser::$trait for &mut PaidPreventionProbe {
 type Ok=();type Error=serde_json::Error;
 fn $method<T:serde::Serialize+?Sized>(&mut self,v:&T)->Result<(),Self::Error>{v.serialize(&mut **self)}
 fn end(self)->Result<(),Self::Error>{Ok(())}
}};}
sequence!(SerializeSeq,serialize_element);sequence!(SerializeTuple,serialize_element);
sequence!(SerializeTupleStruct,serialize_field);sequence!(SerializeTupleVariant,serialize_field);
impl serde::ser::SerializeMap for &mut PaidPreventionProbe {
 type Ok=();type Error=serde_json::Error;
 fn serialize_key<T:serde::Serialize+?Sized>(&mut self,v:&T)->Result<(),Self::Error>{v.serialize(&mut **self)}
 fn serialize_value<T:serde::Serialize+?Sized>(&mut self,v:&T)->Result<(),Self::Error>{v.serialize(&mut **self)}
 fn end(self)->Result<(),Self::Error>{Ok(())}
}
macro_rules! structure {($trait:ident)=>{impl serde::ser::$trait for &mut PaidPreventionProbe {
 type Ok=();type Error=serde_json::Error;
 fn serialize_field<T:serde::Serialize+?Sized>(&mut self,_k:&'static str,v:&T)->Result<(),Self::Error>{
  if _k=="filter" && self.struct_names.last()==Some(&"SuspendAndCancelAllMatchingLeaves") {
      let filter:CompiledPredicate=serde_json::from_value(serde_json::to_value(v)?)?;
      if let Some(previous)=&self.suspend_filter { assert_eq!(previous,&filter,"one replacement must use one plural subject filter"); }
      self.suspend_filter=Some(filter);
  }
  v.serialize(&mut **self)
 }
 fn end(self)->Result<(),Self::Error>{self.struct_names.pop();Ok(())}
}};}

structure!(SerializeStruct);structure!(SerializeStructVariant);

#[cfg(test)]
mod paid_prevention_probe_tests {
 use super::*;
 #[test]
 fn probe_reads_typed_nested_steps_and_ignores_identical_prompt_text(){
  assert!(process_has_paid_self_prevention(&[CompiledStep::DeleteSelfAndCancelLeave]));
  assert!(process_has_paid_self_prevention(&[CompiledStep::If{condition:Default::default(),then:Vec::new(),else_branch:vec![CompiledStep::DeleteSelfAndCancelLeave]}]));
  assert!(!process_has_paid_self_prevention(&[CompiledStep::SelectEffectChoice{labels:vec!["DeleteSelfAndCancelLeave".into()],legal_when:None,bind_as:Some("DeleteSelfAndCancelLeave".into()),prompt:"DeleteSelfAndCancelLeave".into(),prompt_key:None}]));
 }
}
