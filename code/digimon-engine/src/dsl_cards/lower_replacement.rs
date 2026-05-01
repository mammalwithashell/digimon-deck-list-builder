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
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{EffectTiming, GamePhase};
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

    if let Some(active_when) = active_when {
        let active_when = active_when.clone();
        builder = builder.replacement_condition(move |ctx, subject| {
            let subject = match *subject {
                ReplacementSubject::Permanent(handle) => PredicateSubject::Permanent(handle),
                ReplacementSubject::Card(handle, _) => PredicateSubject::Card(handle),
                ReplacementSubject::Player(_) => PredicateSubject::None,
            };
            eval_predicate(&active_when, ctx, subject)
        });
    }

    if let Some(delay_flow) = DelayHandDigivolveFlow::from_process(&process) {
        builder = builder.replacement_process(move |rctx| {
            if !source_is_delayed_option(rctx.effect) {
                return;
            }

            let player = resolve_player(rctx.effect, delay_flow.of);
            let candidates =
                matching_hand_candidates(rctx.effect, player, &delay_flow.filter, HAND_MAIN_LIMIT);
            if candidates.is_empty() {
                return;
            }
            let subject_card = stable_replacement_subject_card(rctx.effect, rctx.subject);
            if !rctx.effect.trash_delay_source() {
                return;
            }
            let Some(subject) =
                resolve_stable_replacement_subject(rctx.effect, rctx.subject, subject_card)
            else {
                return;
            };

            install_delay_hand_digivolve_selection(
                rctx.effect,
                subject,
                player,
                delay_flow.prompt.clone(),
                candidates,
            );
        });
        return Some(builder.build());
    }

    builder = builder.replacement_process(move |rctx| {
        // DSL replacements scope to the carrier permanent by default:
        // only fire when the SUBJECT is the same permanent that carries
        // this effect. This mirrors the "When THIS Digimon would leave"
        // semantics on all printed self-replacement clauses. Inherited
        // clauses (scope == Inherited) can propagate from any card in the
        // digivolution stack; for now they still bind to the top-card perm.
        let subject_matches = match rctx.subject {
            ReplacementSubject::Permanent(subject_h) => {
                rctx.effect.source_permanent == Some(subject_h)
            }
            _ => false,
        };
        if !subject_matches {
            return;
        }

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

struct DelayHandDigivolveFlow {
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    prompt: String,
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
) -> Option<(crate::enums::PlayerId, CardHandle)> {
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
    player: crate::enums::PlayerId,
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
    player: crate::enums::PlayerId,
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
        })
        .collect();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;

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
        callback: Box::new(move |game, action_id| {
            let Some(idx) = action_id.checked_sub(HAND_EFFECT_START).map(|i| i as usize) else {
                return;
            };
            let Some(card) = candidates.get(idx).copied() else {
                return;
            };
            let mut ctx = EffectContext::new(game, source_card, source_permanent, player);
            if ctx.digivolve_replacement_subject_without_cost(subject, card) {
                ctx.cancel_current_replacement();
            }
        }),
        on_decline: None,
    });
}
