//! Phase 2f4 Task 1 — `ScheduledEffect` queue. One-shot delayed effects
//! ("pending processing", §18-1-1) scheduled by
//! `EffectContext::schedule_delayed`; processed at the timing boundary that
//! matches the scheduled `when:`.
//!
//! Design notes:
//!
//! - The queue lives on `Game::scheduled_effects` so it survives across
//!   `EffectContext` lifetimes.
//! - Each entry stores a `CardHandle` (Copy) instead of a `CardSource`
//!   (heavier clone). At fire time we reconstruct an `EffectContext` from
//!   `(controller, card_handle, source_permanent, source_kind)`.
//! - §18-1-2: pending processing performed at the same time as other
//!   processing is performed like simultaneously triggered effects
//!   (§15-4-3). Each timing site therefore calls
//!   [`Game::enqueue_scheduled_for_timing`] BEFORE it drains the timing's
//!   printed triggers: the due entries become `QueuedEffect`s (carrying the
//!   `ScheduledEffect` in `QueuedEffect::scheduled`) in the same trigger
//!   batch, so the drainer's turn-player / `TriggerOrder` logic orders them
//!   (§15-4-3-5-1), and a body that parks resumes through the ordinary
//!   selection path. DCGO agrees: player effects
//!   (`AddEffectToPlayer(..., EffectTiming.OnEndTurn, ...)`) are stacked in
//!   the same `AutoProcessing.StackSkillInfos(null, OnEndTurn)` batch as the
//!   card skills.
//! - The due set is taken once, when the timing occurs. An effect scheduled
//!   WHILE that pool drains (from inside a resolving body) stays on
//!   `Game::scheduled_effects` and is processed at the next matching
//!   boundary, never in the current pool.
//! - Non-matching effects remain queued.

use digimon_dsl::compiled::CompiledStep;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::effect_context::EffectContext;
use crate::enums::{EffectSourceKind, EffectTiming, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::selection::QueuedEffect;
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

/// Process every due `ScheduledEffect` for timing `t` on its own: enqueue
/// them ([`Game::enqueue_scheduled_for_timing`]) and drain the effect queue.
///
/// Engine timing sites do NOT use this: they enqueue the scheduled entries
/// into the same batch as the timing's printed triggers before their own
/// drain (§18-1-2). It remains the entry point for exercising scheduled
/// bodies in isolation (tests, tools). Several due entries still form one
/// simultaneous pool, ordered by the turn player.
pub fn fire_scheduled_for_timing(game: &mut Game, t: EffectTiming) {
    if game.enqueue_scheduled_for_timing(t) > 0 {
        game.drain_effect_queue();
    }
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

impl Game {
    /// §18-1-2: materialise every `ScheduledEffect` due at timing `t` as a
    /// `QueuedEffect` (`QueuedEffect::scheduled`), appended to the effect
    /// queue WITHOUT draining. Returns how many were enqueued.
    ///
    /// Timing sites call this before draining the timing's printed triggers
    /// (in the same deferred-drain scope), so the pending processing and the
    /// triggered effects form one simultaneous pool (§15-4-3) that the turn
    /// player orders (§15-4-3-5-1). Entries not yet due (another timing, a
    /// "next turn" window that has not come, the wrong turn player) stay on
    /// `Game::scheduled_effects`; so does anything scheduled later, while
    /// this pool drains (it is processed at the next matching boundary).
    pub fn enqueue_scheduled_for_timing(&mut self, t: EffectTiming) -> usize {
        let turn = self.turn_count;
        let queued = std::mem::take(&mut self.scheduled_effects);
        let (due, not_due): (Vec<ScheduledEffect>, Vec<ScheduledEffect>) =
            queued.into_iter().partition(|eff| {
                eff.when == t
                    && can_fire_scheduled(eff, turn)
                    && timing_controller_matches(self, eff)
            });
        self.scheduled_effects = not_due;
        let turn_player = self.turn_player();
        let count = due.len();
        for eff in due {
            let card_id = self
                .card_data_for_handle(eff.source_card)
                .map(|data| data.card_id.clone())
                .unwrap_or_default();
            self.effect_queue.push_back(QueuedEffect {
                source_card: eff.source_card,
                source_permanent: eff.source_permanent,
                source_kind: eff.source_kind,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: true,
                controller: eff.controller,
                timing: t,
                trigger_context: None,
                effect_slot: 0,
                // Pending processing is performed at its timing (§18-1-1);
                // it is not a "you may" effect.
                is_optional: false,
                is_turn_player: eff.controller == turn_player,
                card_id,
                allow_below_top_liveness: false,
                dna_origin_context: None,
                granted_effect_id: None,
                keyword_effect: None,
                trigger_batch: 0,
                scheduled: Some(Box::new(eff)),
            });
        }
        count
    }
}

/// Run one scheduled body (a `QueuedEffect::scheduled` payload) through
/// `run_steps` with its captured bindings and runtime, against a fresh
/// `EffectContext` whose controller is the scheduling effect's controller.
/// `source_permanent` is the queue entry's (battle-area removals re-key it
/// while it waits). A body that parks on a selection resumes through the
/// ordinary selection path; the drainer then continues the pool.
pub(crate) fn run_scheduled_body(
    game: &mut Game,
    effect: ScheduledEffect,
    source_permanent: Option<PermanentHandle>,
) {
    let ScheduledEffect {
        body,
        source_card,
        source_kind,
        controller,
        captured_bindings,
        runtime,
        ..
    } = effect;
    let mut bindings = captured_bindings;
    let mut ctx = EffectContext::new_with_source_kind(
        game,
        source_card,
        source_permanent,
        source_kind,
        controller,
    );
    let _ = run_steps_with_runtime(&body, &mut ctx, &mut bindings, &runtime);
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
/// Called from `fire_end_of_your_turn` after the `EndOfYourTurn` pool
/// (printed observers + due scheduled effects) has drained — the played
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
/// queue (`Game::scheduled_provenance_deletions`); no resume hook re-drives
/// them after that selection resolves.
pub fn fire_scheduled_provenance_deletions(game: &mut Game) {
    use crate::replacement::ReplacementCause;

    let queued = std::mem::take(&mut game.scheduled_provenance_deletions);
    let mut iter = queued.into_iter();
    while let Some(entry) = iter.next() {
        let Some(EventSubject::Permanent(handle)) = game.resolve_provenance_token(entry.token)
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
        let Some(EventSubject::Permanent(handle)) = game.resolve_provenance_token(entry.token)
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
