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
//!   printed triggers: the due entries become `QueuedEffect`s (carrying a
//!   [`PendingProcessing`] in `QueuedEffect::pending_processing`) in the same trigger
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
use crate::effect::{EffectCategoryFlags, EffectObservationMetadata};
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
        // "At the end of your opponent's turn": the controller's opponent is
        // the turn player whose turn is ending.
        EffectTiming::EndOfOpponentsTurn | EffectTiming::EndOfOpponentsNextTurn => {
            effect.controller != game.turn_player()
        }
        _ => true,
    }
}

/// The payload of a `QueuedEffect` that is PENDING PROCESSING (§18-1-1)
/// rather than a triggered effect: it joined its timing's simultaneous pool
/// (§18-1-2) and is run by [`run_pending_processing`] instead of looking up
/// `effects_for_card(card_id)[effect_slot]`. Plain data, so `Game: Clone`
/// stays faithful.
#[derive(Debug, Clone)]
pub enum PendingProcessing {
    /// A `schedule_delayed` body (`EffectContext::schedule_delayed`).
    Scheduled(Box<ScheduledEffect>),
    /// "Delete the Digimon this effect played" at a turn's end
    /// (`EffectContext::schedule_delete_at_end_of_turn` /
    /// `…_of_opponents_turn`).
    ProvenanceDeletion(ScheduledProvenanceDeletion),
}

impl PendingProcessing {
    /// What the `TriggerOrder` branch says it is, after the source card id:
    /// it must not read as the source card's printed effect.
    pub(crate) fn describe(&self, game: &Game) -> String {
        match self {
            PendingProcessing::Scheduled(effect) => {
                format!("pending processing at {:?}", effect.when)
            }
            PendingProcessing::ProvenanceDeletion(entry) => {
                let target = game
                    .card_data_for_handle(CardHandle(entry.token.0 as u16))
                    .map(|data| data.card_id.clone())
                    .unwrap_or_default();
                format!("pending processing: delete {target}")
            }
        }
    }

    /// Observation categories of the branch (the tensor's pending-choice
    /// rows read `EffectChoiceEntry::observation_metadata`).
    pub(crate) fn observation_metadata(&self) -> EffectObservationMetadata {
        let mut metadata = EffectObservationMetadata::default();
        match self {
            PendingProcessing::Scheduled(effect) => {
                add_step_categories(&mut metadata.categories, &effect.body);
            }
            PendingProcessing::ProvenanceDeletion(_) => {
                metadata.categories.delete = true;
            }
        }
        metadata
    }
}

/// Top-level step categories of a scheduled body (a coarse summary for the
/// observation; branches and nested bodies are not inspected).
fn add_step_categories(flags: &mut EffectCategoryFlags, body: &[CompiledStep]) {
    for step in body {
        match step {
            CompiledStep::GainMemory { .. }
            | CompiledStep::LoseMemory { .. }
            | CompiledStep::SetMemory { .. }
            | CompiledStep::GainMemoryFn { .. }
            | CompiledStep::LoseMemoryFn { .. } => flags.memory = true,
            CompiledStep::DeletePermanent { .. }
            | CompiledStep::DeleteBoundPermanents { .. }
            | CompiledStep::DeletePermanents { .. }
            | CompiledStep::DeleteAllPermanents { .. } => flags.delete = true,
            CompiledStep::ReturnToHand { .. }
            | CompiledStep::BounceSelf { .. }
            | CompiledStep::ReturnAllToHand { .. } => flags.bounce = true,
            CompiledStep::ReturnToDeck { .. } | CompiledStep::ReturnAllToDeck { .. } => {
                flags.bottom_deck = true
            }
            CompiledStep::Draw { .. } | CompiledStep::DrawFn { .. } => flags.draw_search = true,
            CompiledStep::Suspend { .. } => flags.suspend = true,
            CompiledStep::Unsuspend { .. } => flags.unsuspend = true,
            CompiledStep::AddDpModifier { .. } => flags.dp_change = true,
            CompiledStep::GrantKeyword { .. } | CompiledStep::GrantKeywordContinuous { .. } => {
                flags.grant_keyword = true
            }
            CompiledStep::Recover { .. } => flags.recover = true,
            CompiledStep::TrashTopSecurity { .. } | CompiledStep::TrashBottomSecurity { .. } => {
                flags.trash_security = true
            }
            CompiledStep::PlayFromHandFree { .. }
            | CompiledStep::PlayFromTrashFree { .. }
            | CompiledStep::PlayToken { .. } => flags.play = true,
            _ => {}
        }
    }
}

impl Game {
    /// §18-1-2: materialise every `ScheduledEffect` due at timing `t` as a
    /// `QueuedEffect` (`PendingProcessing::Scheduled`), appended to the
    /// effect queue WITHOUT draining. Returns how many were enqueued.
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
        let count = due.len();
        for eff in due {
            self.enqueue_pending_processing(
                t,
                eff.source_card,
                eff.source_permanent,
                eff.source_kind,
                eff.controller,
                PendingProcessing::Scheduled(Box::new(eff)),
            );
        }
        count
    }

    /// §18-1-1 / §18-1-2: enqueue the provenance-keyed deletions due at the
    /// end of the turn ("At turn end, delete the Digimon this effect
    /// played", EX11-022 / EX11-061) into the `EndOfYourTurn` pool. Every
    /// queued entry is due (the queue holds only this turn's deletions).
    pub(crate) fn enqueue_provenance_deletions_at_turn_end(&mut self) -> usize {
        let due = std::mem::take(&mut self.scheduled_provenance_deletions);
        let count = due.len();
        for entry in due {
            self.enqueue_provenance_deletion(EffectTiming::EndOfYourTurn, entry);
        }
        count
    }

    /// PUPPETS-G016 — the same for deletions scheduled for the end of the
    /// **opponent's** turn (P-165 ShoeShoemon), into the
    /// `EndOfOpponentsTurn` pool. `ending_player` is the player whose turn is
    /// ending: only entries whose controller is one of their opponents are
    /// due ("at the end of *your opponent's* turn" for that controller);
    /// the ending player's own entries wait for their opponent's turn end.
    pub(crate) fn enqueue_provenance_deletions_at_opponents_turn_end(
        &mut self,
        ending_player: PlayerId,
    ) -> usize {
        let queued = std::mem::take(&mut self.scheduled_provenance_deletions_opp);
        let (due, not_due): (Vec<_>, Vec<_>) = queued
            .into_iter()
            .partition(|entry| entry.controller != ending_player);
        self.scheduled_provenance_deletions_opp = not_due;
        let count = due.len();
        for entry in due {
            self.enqueue_provenance_deletion(EffectTiming::EndOfOpponentsTurn, entry);
        }
        count
    }

    fn enqueue_provenance_deletion(&mut self, t: EffectTiming, entry: ScheduledProvenanceDeletion) {
        self.enqueue_pending_processing(
            t,
            entry.source_card,
            None,
            entry.source_kind,
            entry.controller,
            PendingProcessing::ProvenanceDeletion(entry),
        );
    }

    fn enqueue_pending_processing(
        &mut self,
        t: EffectTiming,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        source_kind: EffectSourceKind,
        controller: PlayerId,
        payload: PendingProcessing,
    ) {
        let card_id = self
            .card_data_for_handle(source_card)
            .map(|data| data.card_id.clone())
            .unwrap_or_default();
        let is_turn_player = controller == self.turn_player();
        self.push_effect_with_identity(QueuedEffect {
                source_identity: None,
            source_card,
            source_permanent,
            source_kind,
            attribution_source_card: None,
            attribution_source_kind: None,
            bypass_once_per_turn: true,
            controller,
            timing: t,
            trigger_context: None,
            effect_slot: 0,
            // Pending processing is performed at its timing (§18-1-1); it is
            // not a "you may" effect.
            is_optional: false,
            is_turn_player,
            card_id,
            allow_below_top_liveness: false,
            dna_origin_context: None,
            granted_effect_id: None,
            keyword_effect: None,
            trigger_batch: 0,
            pending_processing: Some(payload),
        });
    }
}

/// Perform one queued pending processing (the drainer's single gate for
/// `QueuedEffect::pending_processing`). `source_permanent` is the queue
/// entry's (battle-area removals re-key it while it waits). Processing that
/// parks on a selection resumes through the ordinary selection path; the
/// drainer then continues the pool.
pub(crate) fn run_pending_processing(
    game: &mut Game,
    payload: PendingProcessing,
    source_permanent: Option<PermanentHandle>,
) {
    match payload {
        PendingProcessing::Scheduled(effect) => {
            run_scheduled_body(game, *effect, source_permanent)
        }
        PendingProcessing::ProvenanceDeletion(entry) => run_provenance_deletion(game, entry),
    }
}

/// Run one scheduled body through `run_steps` with its captured bindings and
/// runtime, against a fresh `EffectContext` whose controller is the
/// scheduling effect's controller.
fn run_scheduled_body(
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
    // A `Parked` outcome needs nothing here: the parked selection's resume
    // finishes the body, and the drainer continues the pool after it.
    let _ = run_steps_with_runtime(&body, &mut ctx, &mut bindings, &runtime);
}

/// Delete the permanent a provenance deletion names, if it is still in the
/// battle area: resolves to a battle-area `Permanent` → delete it (cause
/// `OwnEffect`, so the deleted Digimon's "leave other than by your effects"
/// replacements do not fire); anything else (it already left) → a silent
/// no-op. Routes through `delete_permanent_with_cause`, so
/// `WhenWouldBeDeleted` replacement windows (e.g. `<Scapegoat>`) apply; a
/// parked window resumes through the ordinary selection path.
fn run_provenance_deletion(game: &mut Game, entry: ScheduledProvenanceDeletion) {
    use crate::replacement::ReplacementCause;
    let Some(EventSubject::Permanent(handle)) = game.resolve_provenance_token(entry.token) else {
        return;
    };
    game.delete_permanent_with_cause(handle, ReplacementCause::OwnEffect);
}

// ─── PUPPETS-G003 — provenance-keyed turn-end self-deletion ──────────────────

/// A deletion scheduled for a turn's end, keyed to a stable
/// [`ProvenanceToken`] rather than a battle-area index.
///
/// Used by cards whose text says "At turn end, delete the Digimon this effect
/// played" (EX11-022 Karakurumon, EX11-061 Mirai Kinosaki; P-165 for the
/// opponent's turn end). The token is the identity of the played card; it is
/// resolved when the deletion is performed, so it hits the correct permanent
/// even after other permanents enter or leave the battle area (shifting
/// indices). If the played permanent has already left the battle area, the
/// entry is a silent no-op. It is pending processing (§18-1-1): at the turn
/// end it joins that timing's pool (§18-1-2) via
/// `Game::enqueue_provenance_deletions_at_turn_end` /
/// `…_at_opponents_turn_end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledProvenanceDeletion {
    /// Stable identity of the played card to delete.
    pub token: ProvenanceToken,
    /// The player whose effect played the card (the deletion is that
    /// player's own effect — cause [`crate::replacement::ReplacementCause::OwnEffect`]).
    pub controller: PlayerId,
    /// The card whose effect scheduled the deletion (labels its
    /// `TriggerOrder` branch and attributes the effect).
    pub source_card: CardHandle,
    /// Rules-facing source classification of that effect.
    pub source_kind: EffectSourceKind,
}
