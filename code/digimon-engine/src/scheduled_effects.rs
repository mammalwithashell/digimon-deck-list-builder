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
//!   `(controller, card_handle, source_permanent, source_kind)`.
//! - `fire_scheduled_for_timing` drains in FIFO order. Effects scheduled
//!   DURING a drain (from inside a firing body) land at the back of the queue
//!   and do NOT fire in the same pass. Re-entrancy is bounded by the outer
//!   caller's next observer-fire boundary.
//! - Non-matching effects remain queued.

use digimon_dsl::compiled::CompiledStep;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::{run_steps_with_runtime, RunOutcome, StepRuntime};
use crate::effect_context::EffectContext;
use crate::enums::{EffectSourceKind, EffectTiming, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::trigger_context::{EventSubject, ProvenanceToken};

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
    /// Rules-facing source classification at schedule time.
    pub source_kind: EffectSourceKind,
    /// Controller of the scheduling effect — becomes `ctx.player` at fire time.
    pub controller: PlayerId,
    /// Bindings captured at schedule time and replayed into the body.
    pub captured_bindings: Bindings,
    /// Turn number when this effect was scheduled.
    pub scheduled_at_turn: u16,
    /// Runtime registry captured at schedule time.
    pub runtime: StepRuntime,
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
            source_kind,
            controller,
            captured_bindings,
            runtime,
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
        let mut ctx = EffectContext::new_with_source_kind(
            game,
            source_card,
            source_permanent,
            source_kind,
            controller,
        );
        let outcome = run_steps_with_runtime(&body, &mut ctx, &mut bindings, &runtime);
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

// ─── PUPPETS-G003 — provenance-keyed turn-end self-deletion ──────────────────

/// A deletion scheduled for the current turn's end, keyed to a stable
/// [`ProvenanceToken`] rather than a battle-area index.
///
/// Used by cards whose text says "At turn end, delete the Digimon this effect
/// played" (EX11-022 Karakurumon, EX11-061 Mirai Kinosaki). The token is the
/// identity of the played card; it is resolved at drain time, so the deletion
/// hits the correct permanent even after other permanents enter or leave the
/// battle area (shifting indices). If the played permanent has already left
/// the battle area, the entry is a silent no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledProvenanceDeletion {
    /// Stable identity of the played card to delete.
    pub token: ProvenanceToken,
    /// The player whose effect played the card (the deletion is that
    /// player's own effect — cause [`crate::replacement::ReplacementCause::OwnEffect`]).
    pub controller: PlayerId,
}

/// Drain every [`ScheduledProvenanceDeletion`] queued this turn, deleting each
/// still-present permanent and discarding the queue.
///
/// Called from `fire_end_of_your_turn` after the printed `EndOfYourTurn`
/// observers and the `EndOfYourTurn` scheduled-effect drain — the played
/// Digimon's own end-of-turn effects (if any) have already had their window.
///
/// Each token is resolved against the live battle areas:
///   - resolves to a battle-area `Permanent` → delete it (cause `OwnEffect`,
///     so the deleted Digimon's "leave other than by your effects"
///     replacements do not fire);
///   - resolves to anything else, or fails to resolve → silent no-op (the
///     played permanent already left earlier in the turn).
///
/// The deletion routes through `delete_permanent_with_cause`, which honours
/// `WhenWouldBeDeleted` replacement windows (e.g. `<Scapegoat>`). If a
/// replacement parks a selection, the remaining entries are preserved on the
/// queue; re-draining them is guaranteed only via an end-turn continuation
/// resume (i.e. `resume_scheduled_drain` called from the end-turn path).
pub fn fire_scheduled_provenance_deletions(game: &mut Game) {
    use crate::replacement::ReplacementCause;

    let queued = std::mem::take(&mut game.scheduled_provenance_deletions);
    let mut iter = queued.into_iter();
    while let Some(entry) = iter.next() {
        let Some(EventSubject::Permanent(handle)) =
            game.resolve_provenance_token(entry.token)
        else {
            continue;
        };
        game.delete_permanent_with_cause(handle, ReplacementCause::OwnEffect);
        if game.pending_selection.is_some() {
            game.scheduled_provenance_deletions = iter.collect();
            return;
        }
    }
}

/// PUPPETS-G016 — drain provenance-keyed deletions scheduled for the end of
/// the **opponent's** turn. Called from `rotate_turn_player(ending_player)`
/// after `EndOfOpponentsTurn` observers and scheduled-effect drains.
///
/// `ending_player` is the player whose turn just ended. Only entries whose
/// `controller != ending_player` are drained in this pass — those are the
/// effects scheduled by non-ending players (i.e. the opponents), which fired
/// "at the end of *your opponent's* turn" relative to those controllers.
/// Entries belonging to the ending player itself are left queued for their
/// own opponent's turn (i.e. the next player's turn-end).
pub fn fire_scheduled_provenance_deletions_opp(game: &mut Game, ending_player: PlayerId) {
    use crate::replacement::ReplacementCause;

    let queued = std::mem::take(&mut game.scheduled_provenance_deletions_opp);
    let mut still_pending: Vec<ScheduledProvenanceDeletion> = Vec::new();
    let mut iter = queued.into_iter();
    while let Some(entry) = iter.next() {
        // Only fire for entries whose controller is an opponent of the ending
        // player — i.e. entries where ending_player is the "opponent" from
        // the controller's perspective.
        if entry.controller == ending_player {
            still_pending.push(entry);
            continue;
        }
        // Resolve the stable identity to a current battle-area permanent.
        // Anything else (card moved to trash/hand/etc., or unresolvable) is a
        // no-op — the played Digimon already left.
        let Some(EventSubject::Permanent(handle)) =
            game.resolve_provenance_token(entry.token)
        else {
            continue;
        };
        game.delete_permanent_with_cause(handle, ReplacementCause::OwnEffect);
        if game.pending_selection.is_some() {
            // A replacement parked a selection. Preserve not-yet-processed
            // entries (including those we deferred above) so the
            // selection-resolution path can re-drive them.
            still_pending.extend(iter);
            game.scheduled_provenance_deletions_opp = still_pending;
            return;
        }
    }
    // Restore entries that are waiting for a future opponent-turn boundary.
    game.scheduled_provenance_deletions_opp = still_pending;
}

