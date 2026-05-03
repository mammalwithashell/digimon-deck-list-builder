//! Lower `CompiledDeclarativeClause::Replacement` into a Would* engine
//! `Effect`.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCostDelta, CompiledPlayerRef, CompiledPredicate, CompiledScope,
    CompiledStep,
};
use std::sync::Arc;

use crate::action::space::{HAND_EFFECT_START, HAND_MAIN_LIMIT};
use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{resolve_player, run_steps_with_runtime, StepRuntime};
use crate::dsl_cards::trigger_map::lookup_replacement_trigger;
use crate::effect::{Effect, EffectBuilder};
use crate::effect_context::{DelayCostStatus, EffectContext, EffectReadContext};
use crate::enums::{EffectTiming, GamePhase, PlayerId};
use crate::game::Game;
use crate::permanent::OptionState;
use crate::replacement::ReplacementSubject;
use crate::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};

/// Build the base `EffectBuilder` for a "Would*" replacement timing.
/// Returns `None` only if `timing` is not one of the nine `WhenWould*`
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
        | EffectTiming::WhenWouldPlaceInSecurity => Some(EffectBuilder::new(card, timing)),
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
    process: &[CompiledStep],
) -> Option<Effect> {
    lower_with_raw(
        card,
        scope,
        active_when,
        trigger,
        process,
        Arc::new(EngineRawRustRegistry::new()),
    )
}

pub fn lower_with_raw(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: &str,
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

    let active_when = active_when.cloned();
    let can_match_cross_permanent_subject = active_when
        .as_ref()
        .is_some_and(predicate_requires_replacement_subject);
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
    });

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

fn predicate_requires_replacement_subject(pred: &CompiledPredicate) -> bool {
    pred.replacement_subject_is_mine.is_some()
        || pred
            .all_of
            .iter()
            .any(predicate_requires_replacement_subject)
        || (!pred.any_of.is_empty()
            && pred
                .any_of
                .iter()
                .all(predicate_requires_replacement_subject))
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
