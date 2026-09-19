//! Lower `CompiledDeclarativeClause::Delay` into an engine `Effect` with
//! `timing == DelayEffect`.
//!
//! Trigger mapping:
//! - `CompiledTiming::Delayed` → `DelayTrigger::MainPhaseActivated`. This is
//!   the standard printed `<Delay>` (RULES_CONTEXT 16-16) — a player-visible
//!   `[Main]`-phase activation action. The body never auto-fires; the
//!   controller trashes the Option to activate it on a later main phase.
//! - `CompiledTiming::EndOfYourTurn` → `DelayTrigger::EndOfThisTurn`.
//! - `CompiledTiming::StartOfYourTurn` → `DelayTrigger::StartOfYourNextTurn`.
//! - `CompiledTiming::EndOfYourNextTurn` → `DelayTrigger::EndOfYourNextTurn`
//!   (engine-scheduled auto-fire; retained for cards not yet migrated to the
//!   `delayed` Main-phase trigger).
//! - event timings (`on_suspend` / `on_unsuspend` / `on_ally_played`) →
//!   `DelayTrigger::OnEvent`. `on_ally_played` event-gated Delay Options
//!   (P-229) park indefinitely and fire when a matching card is played after
//!   the placing turn (PUPPETS-G004).
//!
//! Body `active_when` predicates are evaluated when the delayed effect fires.
//! Body steps run through `run_step` (Phase 2a dispatcher).

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::dsl_cards::timing_map::compiled_timing_to_engine;
use crate::effect::{Effect, EffectBuilder};
use crate::effect_context::{DelayCostStatus, EffectContext};
use crate::enums::{DelayTrigger, EffectTiming, PlayerId};
use crate::game::Game;
use crate::permanent::OptionState;
use crate::trigger_context::TriggerContext;

/// Lower a `Delay` declarative clause.
///
/// Returns an `Effect` with `timing == DelayEffect` and a `delay_trigger`
/// mapped from `compiled_trigger`. The `process` closure iterates the
/// `process_steps` via `run_step`.
///
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: CompiledTiming,
    process_steps: &[CompiledStep],
) -> Effect {
    lower_with_raw(
        card,
        scope,
        active_when,
        trigger,
        process_steps,
        Arc::new(EngineRawRustRegistry::new()),
    )
}

pub fn lower_with_raw(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<&CompiledPredicate>,
    trigger: CompiledTiming,
    process_steps: &[CompiledStep],
    raw: Arc<EngineRawRustRegistry>,
) -> Effect {
    let delay_trigger = match trigger {
        // Standard printed `<Delay>` — player-visible `[Main]`-phase
        // activation action (PUPPETS-G009, RULES_CONTEXT 16-16).
        CompiledTiming::Delayed => DelayTrigger::MainPhaseActivated,
        CompiledTiming::EndOfYourTurn => DelayTrigger::EndOfThisTurn,
        CompiledTiming::StartOfYourTurn => DelayTrigger::StartOfYourNextTurn,
        CompiledTiming::EndOfYourNextTurn => DelayTrigger::EndOfYourNextTurn,
        // EVERY other timing is an event-gated `<Delay>`: park indefinitely and
        // fire when the gating event is observed after the placing turn.
        // `on_ally_played` closes the engine half of PUPPETS-G004 (see
        // `effect_queue.rs` `enqueue_triggered` for the `EnteredField` dispatch
        // fan-out).
        //
        // This used to be a HARDCODED LIST (OnSuspend | OnUnsuspend |
        // OnAllyPlayed | OnAttack | OnAllyAttack | OnOpponentAttack) with a
        // `_ => EndOfYourNextTurn` catch-all, and that default was silent and
        // wrong: any event timing outside the list was lowered into a
        // turn-scheduled auto-fire at the end of the controller's next turn —
        // a different trigger, at a different time, with (before 2026-08-24)
        // no §16-16-2 decline. It caught BT21-093, whose printed trigger is
        // "[All Turns] When your opponent's security stack is removed from"
        // and whose `on_opponent_security_removed` timing has existed in
        // `CompiledTiming` and `timing_map` all along, and LM-055
        // (`trigger: on_play`).
        //
        // The turn-scheduled forms are enumerated ABOVE and are the only ones
        // that mean "a scan fires this"; anything else that maps to an engine
        // timing is by definition event-gated. The remaining fallback covers
        // only a `CompiledTiming` with no engine mapping at all.
        other => compiled_timing_to_engine(other)
            .map(DelayTrigger::OnEvent)
            .unwrap_or(DelayTrigger::EndOfYourNextTurn),
    };
    let process_arc: Arc<[CompiledStep]> = Arc::from(process_steps);
    let runtime = StepRuntime::new(raw);
    let active_when = active_when.cloned();
    // §16-16-1: "While a card with this effect is in the battle area, BY
    // TRASHING THAT CARD, the effect specified in <Delay> will activate." The
    // trash is the processing condition (§15-7), so it is paid FIRST and the
    // body runs only if the card actually reached the trash. DCGO does the
    // same in every Delay card's `ActivateCoroutine`
    // (`DeletePeremanentAndProcessAccordingToResult(... successProcess ...)`
    // then `if (deleted)` -- e.g. LM_032.cs:146-158). The turn scan / event /
    // `[Main]` lifecycles used to run the body and trash the carrier AFTER it
    // (exam LM-032#effect#1, G-ENGINE-DELAY-BODY-BEFORE-TRASH); their post-body
    // delete now re-finds nothing and no-ops. A marker-only Delay (empty
    // process -- ST23-15 / ST24-15) is not an activation and pays nothing here.
    let pays_own_cost = !process_steps.is_empty();
    let mut builder = EffectBuilder::new(card, EffectTiming::DelayEffect)
        .delay(delay_trigger)
        .process(move |ctx| {
            if pays_own_cost {
                if let Some((source_player, source_card)) = delay_carrier(ctx) {
                    match ctx.trash_delay_source_status() {
                        DelayCostStatus::Paid => {}
                        DelayCostStatus::Unpaid => return,
                        DelayCostStatus::Pending => {
                            let continuation = DelayBodyContinuation {
                                source_player,
                                source_card,
                                player: ctx.player,
                                steps: process_arc.clone(),
                                runtime: runtime.clone(),
                                trigger_context: ctx.game.current_trigger_context.clone(),
                            };
                            arm_pending_delay_body_continuation(ctx.game, continuation);
                            return;
                        }
                    }
                }
            }
            let mut bindings = Bindings::new();
            let _ = run_steps_with_runtime(&process_arc, ctx, &mut bindings, &runtime);
        });
    // Rules manual 16-16-2: "The processing from <Delay> is optional." An
    // event-gated `<Delay>` (P-228 / P-229 / BT22-098 / BT23-096 / BT24-089 /
    // EX10-069) reaches its timing when the gating event is observed
    // — at that point the CONTROLLER chooses whether to activate (trash the
    // Option + run the body) or decline (the Option stays parked and can fire
    // on a later matching event). DCGO registers these triggers with
    // `SetUpActivateClass(..., isOptional: true, ...)` and shows a bool
    // prompt. Mark the effect optional and force the outer accept/decline
    // prompt so the choice surfaces through `pending_selection` (the
    // no-approximations policy forbids the previous mandatory auto-fire).
    // The prompt is unconditional (no first-step candidate guard): DCGO
    // prompts whenever the event condition matches, and accepting trashes
    // the Option even when the body then finds no legal target — declining
    // the trash is itself the meaningful choice.
    //
    // `MainPhaseActivated` needs no flag (activation IS an explicit player
    // `[Main]` action — flagging it would double-prompt).
    //
    // The TURN-SCHEDULED triggers need the same treatment as `OnEvent`, and
    // used to be excluded on the grounds that they "fire through
    // `resolve_delayed_options_matching` (game_phases.rs), which bypasses the
    // queue's optional machinery" — a description of the bug, not a reason for
    // it. §16-16-2 makes `<Delay>` optional however its window arrives, so the
    // scan auto-paid the trash-this-card cost and auto-resolved the body with
    // no decline anywhere in the action space (rule 17 violation). The scan
    // now honours the outer prompt and, on a decline, leaves the Option on the
    // field and RESCHEDULES it to its next window — §16-16-1 keeps the Delay
    // available "while a card with this effect is in the battle area", and
    // these are recurring printed windows ([Start of Your Turn] on LM-027/029/
    // 030/031/032, [End of Your Turn] on BT21-097), not one-shot forfeits.
    //
    // An EMPTY process is a structural marker, not an activatable Delay:
    // ST23-15 (e-Pulse) and ST24-15 (DNA Charge) print no `<Delay>` at all and
    // carry `kind: delay` with `process: []` purely so `classify_option_modes`
    // keeps the Option in the battle area for their real
    // `[Start of Your Main Phase]` clause. Prompting "activate this?" for a
    // body that does nothing would be a prompt the printed card never offers.
    let is_marker_only = process_steps.is_empty();
    if !is_marker_only
        && matches!(
            delay_trigger,
            DelayTrigger::OnEvent(_)
                | DelayTrigger::EndOfThisTurn
                | DelayTrigger::EndOfYourNextTurn
                | DelayTrigger::StartOfYourNextTurn
        )
    {
        builder = builder.optional().needs_outer_optional_prompt();
    }
    if let Some(predicate) = active_when {
        builder =
            builder.condition(move |ctx| eval_predicate(&predicate, ctx, PredicateSubject::None));
    }
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    builder.build()
}

/// The resolving effect's source when it is a `<Delay>` Option carrier in the
/// battle area whose top card is this effect's card: `(owner, card)`.
fn delay_carrier(ctx: &EffectContext<'_>) -> Option<(PlayerId, CardHandle)> {
    let source = ctx.source_permanent?;
    let perm = ctx
        .game
        .player(source.player)
        .battle_area
        .get(source.index as usize)?;
    if !matches!(perm.option_state, OptionState::Delayed { .. }) {
        return None;
    }
    let top = ctx.permanent_top_card_handle(source)?;
    (top == ctx.source_card).then_some((source.player, top))
}

/// A `<Delay>` body whose trash-this-card cost parked a selection (a
/// replacement window around the trash). Plain data -- resumed through
/// `ResumeFrame::DelayBodyAfterCost` (rule 28 clone-safety).
#[derive(Clone, Debug)]
pub(crate) struct DelayBodyContinuation {
    source_player: PlayerId,
    source_card: CardHandle,
    player: PlayerId,
    steps: Arc<[CompiledStep]>,
    runtime: StepRuntime,
    trigger_context: Option<TriggerContext>,
}

fn arm_pending_delay_body_continuation(game: &mut Game, continuation: DelayBodyContinuation) {
    let Some(mut selection) = game.pending_selection.take() else {
        continue_delay_body_after_selection(game, continuation);
        return;
    };

    if let Some(inner) = game.pending_selection_resume.take() {
        game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DelayBodyAfterCost {
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
        continue_delay_body_after_selection(game, callback_continuation);
    });

    let original_decline = selection.on_decline.take();
    selection.on_decline = Some(Box::new(move |game| {
        if let Some(original_decline) = original_decline {
            original_decline(game);
        }
        continue_delay_body_after_selection(game, continuation);
    }));

    game.pending_selection = Some(selection);
}

/// The `<Delay>` cost's parked selection resolved. Run the body only if the
/// carrier actually reached the trash (§16-16-1: the body activates BY
/// trashing the card; a replaced/prevented trash activates nothing).
pub(crate) fn continue_delay_body_after_selection(
    game: &mut Game,
    continuation: DelayBodyContinuation,
) {
    if game.pending_selection.is_some() {
        arm_pending_delay_body_continuation(game, continuation);
        return;
    }
    let paid = {
        let ctx = EffectContext::new(game, continuation.source_card, None, continuation.player);
        ctx.delay_source_card_in_trash(continuation.source_player, continuation.source_card)
    };
    if !paid {
        return;
    }
    crate::dsl_cards::step::drain_or_rewrap_pending_tail(
        game,
        continuation.source_card,
        None,
        continuation.player,
        continuation.steps.to_vec(),
        Bindings::new(),
        continuation.runtime,
        continuation.trigger_context,
    );
}
