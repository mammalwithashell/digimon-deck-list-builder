//! Control-flow step lowering (Phase 2c: Optional + If; Phase 2d: returns
//! `Option<RunOutcome>` so a parked inner body suspends the outer slice).
//!
//! These live on the `run_steps` path (not `run_step`) because their inner
//! bodies may contain selection steps that need to park the continuation.
//! The dispatcher re-enters `run_steps` for each branch.

use digimon_dsl::compiled::CompiledStep;
use std::sync::Arc;

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use crate::dsl_cards::step::selections::select_hand_candidate_count;
use crate::dsl_cards::step::{
    drain_dsl_outer_tail, resolve_player, run_steps_with_runtime, RunOutcome, StepRuntime,
};
use crate::effect_context::EffectContext;
use crate::enums::GamePhase;
use crate::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};

/// Returns `Some(outcome)` if `step` is a control-flow verb whose body
/// has been dispatched. The outer `run_steps` propagates a `Parked`
/// upward (Task 7 captures the outer tail).
///
/// Returns `None` for any non-control-flow step, letting the caller
/// fall through to the next dispatcher.
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> Option<RunOutcome> {
    match step {
        CompiledStep::Optional(body) => {
            // G-SELECT-EMPTY-OUTER-TAIL — when an `- optional: [...]` substep
            // leads with a `select_hand` that has zero matching candidates,
            // the raw `EffectContext::select_hand` would early-return without
            // installing a `PendingSelection`, and the dispatcher would then
            // run the rest of the optional body (e.g. a mandatory
            // `select_dna_pair`) even though the prerequisite pick is
            // impossible. Treat that as a declined optional substep: skip the
            // whole body. This keeps an empty-hand leading `select_hand` from
            // forcing subsequent mandatory steps.
            if let Some(CompiledStep::SelectHand { of, filter, .. }) = body.first() {
                if select_hand_candidate_count(ctx, *of, filter, bindings) == 0 {
                    return Some(RunOutcome::Synchronous);
                }
            }
            Some(run_steps_with_runtime(body, ctx, bindings, runtime))
        }
        CompiledStep::If {
            condition,
            then,
            else_branch,
        } => {
            let cond_holds = {
                let rctx = ctx.as_read();
                let subject = if let Some(handle) = rctx.source_permanent {
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
                        PredicateSubject::None
                    } else {
                        PredicateSubject::Permanent(handle)
                    }
                } else {
                    PredicateSubject::None
                };
                eval_predicate_with_bindings(condition, &rctx, subject, Some(bindings))
            };
            let body = if cond_holds { then } else { else_branch };
            Some(run_steps_with_runtime(body, ctx, bindings, runtime))
        }
        CompiledStep::IfTrashTopSecurityCost { of, then } => {
            let player = resolve_player(ctx, *of);
            if ctx.trash_top_security(player) {
                Some(run_steps_with_runtime(then, ctx, bindings, runtime))
            } else {
                Some(RunOutcome::Synchronous)
            }
        }
        CompiledStep::IfTrashTopOrBottomSecurityCost { of, prompt, then } => {
            let player = resolve_player(ctx, *of);
            let security_count = ctx.game.player(player).security.len();
            if security_count == 0 {
                return Some(RunOutcome::Synchronous);
            }
            if security_count == 1 {
                return if ctx.trash_top_security(player) {
                    Some(run_steps_with_runtime(then, ctx, bindings, runtime))
                } else {
                    Some(RunOutcome::Synchronous)
                };
            }

            install_top_or_bottom_security_cost_choice(
                ctx,
                player,
                prompt,
                then.clone(),
                bindings.clone(),
                runtime.clone(),
            );
            Some(RunOutcome::Parked)
        }
        CompiledStep::IfAddTopSecurityToHandCost { of, prompt, then } => {
            let player = resolve_player(ctx, *of);
            if ctx.game.player(player).security.is_empty() {
                return Some(RunOutcome::Synchronous);
            }
            install_add_top_security_to_hand_cost_choice(
                ctx,
                player,
                prompt,
                then.clone(),
                bindings.clone(),
                runtime.clone(),
            );
            Some(RunOutcome::Parked)
        }
        CompiledStep::IfPlacePermanentOnOwnersSecurityCost {
            target,
            position,
            face_up,
            then,
        } => {
            let paid = match resolve_binding_ref(target, ctx, bindings) {
                Some(ResolvedBinding::Permanent(handle)) => {
                    let owner = handle.player;
                    let position = super::map_stack_position(*position);
                    ctx.place_permanent_on_security(owner, handle, position, *face_up)
                }
                _ => false,
            };
            if paid {
                Some(run_steps_with_runtime(then, ctx, bindings, runtime))
            } else {
                Some(RunOutcome::Synchronous)
            }
        }
        _ => None,
    }
}

fn install_top_or_bottom_security_cost_choice(
    ctx: &mut EffectContext<'_>,
    player: u8,
    prompt: &str,
    then: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    use crate::action::space::HAND_EFFECT_START;

    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();
    let selecting_player = override_pin.unwrap_or(controller);
    let previous_phase = ctx.game.current_phase;
    let trigger_context = ctx.game.current_trigger_context.clone();
    let then = Arc::new(then);

    let valid_action_ids = vec![HAND_EFFECT_START, HAND_EFFECT_START + 1];
    let choices = vec![
        EffectChoiceEntry {
            label: "Trash top security".to_string(),
            action_id: HAND_EFFECT_START,
            source_card: Some(source_card),
            source_kind: Some(source_kind),
            timing: None,
            is_optional: false,
            observation_metadata: Default::default(),
        },
        EffectChoiceEntry {
            label: "Trash bottom security".to_string(),
            action_id: HAND_EFFECT_START + 1,
            source_card: Some(source_card),
            source_kind: Some(source_kind),
            timing: None,
            is_optional: false,
            observation_metadata: Default::default(),
        },
    ];

    ctx.game.current_phase = GamePhase::EffectChoice;
    ctx.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: true,
        prompt: prompt.to_string(),
        effect_choices: Some(choices),
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, action_id| {
            let choice_index = action_id.saturating_sub(HAND_EFFECT_START) as usize;
            let mut cb_ctx = EffectContext::new_with_source_kind_and_override(
                game,
                source_card,
                source_permanent,
                source_kind,
                controller,
                override_pin,
            );
            let paid = match choice_index {
                0 => cb_ctx.trash_top_security(player),
                1 => cb_ctx.trash_bottom_security(player),
                _ => false,
            };
            if paid {
                let previous = cb_ctx.game.current_trigger_context.clone();
                cb_ctx.game.current_trigger_context = trigger_context;
                let mut b = bindings;
                run_steps_with_runtime(&then, &mut cb_ctx, &mut b, &runtime);
                drain_dsl_outer_tail(&mut cb_ctx);
                cb_ctx.game.current_trigger_context = previous;
            }
        }),
        on_decline: Some(Box::new(|_| {})),
    });
}

fn install_add_top_security_to_hand_cost_choice(
    ctx: &mut EffectContext<'_>,
    player: u8,
    prompt: &str,
    then: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    use crate::action::space::HAND_EFFECT_START;

    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();
    let selecting_player = override_pin.unwrap_or(controller);
    let previous_phase = ctx.game.current_phase;
    let trigger_context = ctx.game.current_trigger_context.clone();
    let then = Arc::new(then);

    ctx.game.current_phase = GamePhase::EffectChoice;
    ctx.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player,
        previous_phase,
        valid_action_ids: vec![HAND_EFFECT_START],
        is_optional: true,
        prompt: prompt.to_string(),
        effect_choices: Some(vec![EffectChoiceEntry {
            label: "Add top security to hand".to_string(),
            action_id: HAND_EFFECT_START,
            source_card: Some(source_card),
            source_kind: Some(source_kind),
            timing: None,
            is_optional: false,
            observation_metadata: Default::default(),
        }]),
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, action_id| {
            if action_id != HAND_EFFECT_START {
                return;
            }
            let mut cb_ctx = EffectContext::new_with_source_kind_and_override(
                game,
                source_card,
                source_permanent,
                source_kind,
                controller,
                override_pin,
            );
            cb_ctx.add_top_security_to_hand(player);
            let previous = cb_ctx.game.current_trigger_context.clone();
            cb_ctx.game.current_trigger_context = trigger_context;
            let mut b = bindings;
            run_steps_with_runtime(&then, &mut cb_ctx, &mut b, &runtime);
            drain_dsl_outer_tail(&mut cb_ctx);
            cb_ctx.game.current_trigger_context = previous;
        }),
        on_decline: Some(Box::new(|_| {})),
    });
}
