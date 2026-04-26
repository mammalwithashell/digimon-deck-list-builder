//! Phase 2f4 Task 1 — `ScheduledEffect` queue. One-shot delayed effects
//! scheduled by `EffectContext::schedule_delayed`; drained at observer-fire
//! boundaries that match the scheduled `when:` timing (Task 2 will wire the
//! drain into game-phase transitions).
//!
//! Design notes:
//!
//! - The queue lives on `Game::scheduled_effects` so it survives across
//!   `EffectContext` lifetimes.
//! - Each entry stores a `CardHandle` (Copy) instead of a `CardSource`
//!   (heavier clone). At fire time we reconstruct an `EffectContext` from
//!   `(controller, card_handle, source_permanent)` — same parameter shape as
//!   `EffectContext::new`.
//! - `fire_scheduled_for_timing` drains in FIFO order. Effects scheduled
//!   DURING a drain (from inside a firing body) land at the back of the queue
//!   and do NOT fire in the same pass. Re-entrancy is bounded by the outer
//!   caller's next observer-fire boundary.
//! - Non-matching effects remain queued.

use digimon_dsl::compiled::CompiledStep;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::{run_steps, RunOutcome};
use crate::effect_context::EffectContext;
use crate::enums::{EffectTiming, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;

/// A one-shot delayed effect waiting on a future timing boundary.
#[derive(Debug, Clone)]
pub struct ScheduledEffect {
    /// Timing that drains and fires this entry.
    pub when: EffectTiming,
    /// Compiled body executed via `run_steps` against a fresh `EffectContext`.
    pub body: Vec<CompiledStep>,
    /// Source card handle — reconstituted into the fire-time `EffectContext`.
    pub source_card: CardHandle,
    /// Source permanent at schedule time, if any.
    pub source_permanent: Option<PermanentHandle>,
    /// Controller of the scheduling effect — becomes `ctx.player` at fire time.
    pub controller: PlayerId,
    /// Bindings captured at schedule time and replayed into the body.
    pub captured_bindings: Bindings,
    /// Turn number when this effect was scheduled.
    pub scheduled_at_turn: u16,
}

#[derive(Debug, Clone)]
pub struct ScheduledDrainTail {
    pub timing: EffectTiming,
    pub remaining: Vec<ScheduledEffect>,
}

/// Drain every `ScheduledEffect` whose `when` matches `t`, running each body
/// via `run_steps` against a fresh `EffectContext`.
///
/// Effects are processed in FIFO order. Effects scheduled DURING this drain
/// (from inside a firing body) land at the back of the queue and do NOT
/// fire in the same pass — re-entrancy is bounded by the outer caller's
/// next observer-fire boundary.
///
/// Non-matching effects remain queued.
pub fn fire_scheduled_for_timing(game: &mut Game, t: EffectTiming) {
    let queued = std::mem::take(&mut game.scheduled_effects);
    drain_scheduled_for_timing(game, t, queued);
}

pub fn resume_scheduled_drain(game: &mut Game) {
    let Some(tail) = game.scheduled_drain_tail.take() else {
        return;
    };
    drain_scheduled_for_timing(game, tail.timing, tail.remaining);
}

fn can_fire_scheduled(effect: &ScheduledEffect, current_turn: u16) -> bool {
    match effect.when {
        EffectTiming::EndOfYourNextTurn
        | EffectTiming::EndOfOpponentsNextTurn
        | EffectTiming::UntilNextUnsuspend => current_turn > effect.scheduled_at_turn,
        _ => true,
    }
}

fn timing_controller_matches(game: &Game, effect: &ScheduledEffect) -> bool {
    match effect.when {
        EffectTiming::EndOfYourNextTurn | EffectTiming::UntilNextUnsuspend => {
            effect.controller == game.turn_player()
        }
        EffectTiming::EndOfOpponentsNextTurn => effect.controller != game.turn_player(),
        _ => true,
    }
}

fn drain_scheduled_for_timing(game: &mut Game, t: EffectTiming, queued: Vec<ScheduledEffect>) {
    let mut still_pending: Vec<ScheduledEffect> = Vec::new();
    let mut iter = queued.into_iter().peekable();
    while let Some(eff) = iter.next() {
        if eff.when != t
            || !can_fire_scheduled(&eff, game.turn_count)
            || !timing_controller_matches(game, &eff)
        {
            still_pending.push(eff);
            continue;
        }
        let ScheduledEffect {
            body,
            source_card,
            source_permanent,
            controller,
            captured_bindings,
            ..
        } = eff;
        let mut bindings = captured_bindings;
        // Phase 2f4 Task 2: parked-selection guard. With `dsl_outer_tail`'s
        // single-outstanding invariant, two scheduled bodies parking in the
        // same drain pass would clobber each other's outer-tail slot. In
        // practice scheduled bodies are end-of-turn housekeeping
        // (gain_memory / draw / plain modifier application) that don't
        // park, so we assert rather than break-on-Parked. If a printed
        // card needs a multi-parking drain in the future, Phase 3 should
        // replace this with break-and-resume retry logic.
        debug_assert!(
            game.dsl_outer_tail.is_none(),
            "scheduled effect fired while a previous parked selection is still outstanding"
        );
        let mut ctx = EffectContext::new(game, source_card, source_permanent, controller);
        let outcome = run_steps(&body, &mut ctx, &mut bindings);
        if matches!(outcome, RunOutcome::Parked) {
            let mut remaining = still_pending;
            remaining.extend(iter);
            ctx.game.scheduled_drain_tail = Some(ScheduledDrainTail {
                timing: t,
                remaining,
            });
            return;
        }
    }
    // Restore non-matching effects, then append any newly-scheduled effects
    // that landed during firing (these go to the back of the queue).
    let mut new_queue = still_pending;
    new_queue.append(&mut game.scheduled_effects);
    game.scheduled_effects = new_queue;
}
