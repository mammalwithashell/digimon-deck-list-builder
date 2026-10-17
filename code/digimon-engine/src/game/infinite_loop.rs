//! §18-3 infinite loops (Comprehensive Rules Ver. 4.3).
//!
//! - §18-3-1: "An infinite loop refers to a state where a set of processing
//!   will continue infinitely or must be continued infinitely as a result of
//!   processing for a rule or effect, and game progress is no longer
//!   possible."
//! - §18-3-2: "If an infinite loop occurs and neither player has the ability
//!   to stop it, that game ends in a draw."
//! - §18-3-3 / §18-3-3-3: a loop a player can stop runs at least the declared
//!   number of times, "then the player stops the processing when possible."
//!
//! **Representation (controller ruling W10).** A player's ability to stop a
//! loop is an optional decision inside it ("you may ..."), which the engine
//! already offers as a real `pending_selection` on every repetition: the
//! player keeps accepting for as many repetitions as they want, then
//! declines. That satisfies §18-3-3-3; the §18-3-3-1/-2 repeat-count
//! declaration is not modelled as a separate action.
//!
//! **Detection.** Before each resolution step (from `LOOP_WATCH_START` steps
//! on) the drainer records a loop signature: the pending effect entries
//! (source card, controller, timing, slot) plus the board (every zone's card
//! handles, suspension, effective DP, per-turn activation counts, memory) --
//! the state a repeating set of processing keeps returning to. Resolving a
//! DECLINABLE player decision (`selection_can_stop_a_loop`) clears the watch,
//! and so does the outermost drain settling with nothing parked; a forced
//! decision (a mandatory pick, a trigger order) does not, since it gives no
//! ability to stop the loop. A signature seen [`LOOP_SIGNATURE_REPEATS`]
//! times therefore means the same state recurred with no optional decision
//! offered within the cycle: neither player can stop it, and the game ends in
//! the §18-3-2 draw (`GameEvent::InfiniteLoop`,
//! then `GameOver { winner: None, reason: InfiniteLoopDraw }`). A chain that
//! makes progress (consumes a queued entry, moves a card, changes DP or memory)
//! never repeats a signature, so a long finite chain runs to its end.
//! `effect_queue::MAX_CHAIN_DEPTH` stays as a far backstop for a loop whose
//! state never repeats; it ends in the same draw. Nothing is ever cleared
//! with the game going on.
//!
//! The signature hashes card handles and plain values only (no pointer
//! addresses, no RNG state, no batch ids), with a fixed-key hasher, so a
//! clone computes the same signatures as its original.

use std::hash::{Hash, Hasher};

use super::*;
use crate::card_source::CardSource;
use crate::permanent::{Permanent, PermanentHandle};

/// How many times the same loop signature must be seen, with no player
/// decision in between, before the processing is an infinite loop nobody can
/// stop (§18-3-2).
pub const LOOP_SIGNATURE_REPEATS: u8 = 3;

/// Steps (drain resolutions and flush passes) since the watch last started
/// over before signatures are taken at all. Ordinary processing never pays for
/// the hash; a loop is still caught after `LOOP_SIGNATURE_REPEATS` identical
/// repeats, only a few steps later.
pub const LOOP_WATCH_START: u16 = 20;

/// Where the drainer took a loop signature. Part of the signature, so a
/// rule-processing flush pass never matches a queue-resolution step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LoopSite {
    /// One resolution step of the effect-queue drainer.
    Queue,
    /// One pass of the outermost drain's `OnDiscardHand` flush.
    DiscardFlush,
    /// One pass of the outermost drain's `OnAddToDeck` flush.
    DeckAddFlush,
    /// One pass of the outermost drain's `OnAddDigivolutionCards` flush.
    AddedSourcesFlush,
    /// One pass of the outermost drain's state-based rules-check fixpoint.
    RulesCheck,
}

fn hash_cards<H: Hasher>(cards: &[CardSource], h: &mut H) {
    cards.len().hash(h);
    for card in cards {
        card.card_index.hash(h);
    }
}

fn hash_permanent<H: Hasher>(permanent: &Permanent, h: &mut H) {
    hash_cards(&permanent.card_sources, h);
    hash_cards(&permanent.linked_cards, h);
    permanent.is_suspended.hash(h);
    // Per-turn activation counts (only effects with a per-turn cap record
    // one), sorted: `HashMap` order is not part of the state.
    let mut activations: Vec<_> = permanent
        .effect_activations
        .iter()
        .map(|(&(card, slot), &count)| (card.0, slot, count))
        .collect();
    activations.sort_unstable();
    activations.hash(h);
}

/// Whether resolving `sel` gives its player the ability to stop a loop
/// (§18-3-3-3: "the player stops the processing when possible"):
/// - a prompt that can be declined (`is_optional`, PASS legal); or
/// - the DSL's effect-text choose-one (`select_effect_choice`, how "you may X
///   instead" is authored), recognised by the resume frame that resolves it:
///   `RunTail { select_kind: EffectChoice { post: None } }`. Its illegal
///   branches are already trimmed at install, and a single legal branch is
///   never prompted.
///
/// Everything else is a forced decision that cannot stop the loop, so the
/// loop watch survives it: mandatory picks (however many options), trigger
/// and immediate-replacement ORDER prompts, count choices, the refire prompt,
/// and the engine's other effect choices (deck top/bottom destination,
/// remainder order: `post: Some(..)`). Residual: a choose-one whose every
/// branch leads back into the loop is still counted, since telling that apart
/// needs lookahead.
pub(crate) fn selection_can_stop_a_loop(
    sel: &crate::selection::PendingSelection,
    resume: Option<&crate::resume::ResumeStack>,
) -> bool {
    if sel.is_optional {
        return true;
    }
    sel.kind == crate::selection::SelectionKind::EffectChoice
        && matches!(
            resume.and_then(|stack| stack.frames.last()),
            Some(crate::resume::ResumeFrame::RunTail {
                select_kind: crate::resume::ResumeSelectKind::EffectChoice { post: None },
                ..
            })
        )
}

impl Game {
    /// The §18-3 loop signature at `site`: the pending effect entries plus
    /// the board (zones' card handles, suspension, memory). Clone-stable.
    pub(crate) fn loop_signature(&self, site: LoopSite) -> u64 {
        // `DefaultHasher::new()` is SipHash with fixed zero keys: deterministic
        // across clones and runs (unlike a `RandomState`-built hasher).
        let mut h = std::collections::hash_map::DefaultHasher::new();
        site.hash(&mut h);
        self.memory.hash(&mut h);
        for player in &self.players {
            hash_cards(&player.hand, &mut h);
            hash_cards(&player.deck, &mut h);
            hash_cards(&player.digitama_deck, &mut h);
            hash_cards(&player.security, &mut h);
            hash_cards(&player.trash, &mut h);
            player.battle_area.len().hash(&mut h);
            for (index, permanent) in player.battle_area.iter().enumerate() {
                hash_permanent(permanent, &mut h);
                // Effective DP: a chain whose only progress is DP ("+1000
                // DP; then, if it has less than 6000 DP, ...") is finite.
                self.effective_dp(PermanentHandle {
                    player: player.id,
                    index: index as u8,
                })
                .hash(&mut h);
            }
            match &player.breeding_area {
                Some(permanent) => {
                    true.hash(&mut h);
                    hash_permanent(permanent, &mut h);
                }
                None => false.hash(&mut h),
            }
        }
        self.effect_queue.len().hash(&mut h);
        for qe in &self.effect_queue {
            qe.source_card.hash(&mut h);
            qe.source_permanent.hash(&mut h);
            qe.controller.hash(&mut h);
            qe.timing.hash(&mut h);
            qe.effect_slot.hash(&mut h);
            qe.granted_effect_id.hash(&mut h);
            qe.keyword_effect.hash(&mut h);
            qe.pending_processing.is_some().hash(&mut h);
        }
        h.finish()
    }

    /// Record one step of processing at `site`. Returns `true` when the step
    /// completed an infinite loop nobody can stop: the game has then ended in
    /// the §18-3-2 draw and the caller must stop processing.
    pub(crate) fn watch_for_infinite_loop(&mut self, site: LoopSite) -> bool {
        if self.game_over {
            return self.halted_by_infinite_loop();
        }
        self.loop_watch_steps = self.loop_watch_steps.saturating_add(1);
        if self.loop_watch_steps < LOOP_WATCH_START {
            return false;
        }
        let signature = self.loop_signature(site);
        let seen = self.loop_watch.entry(signature).or_insert(0);
        *seen = seen.saturating_add(1);
        let seen = *seen;
        if seen >= LOOP_SIGNATURE_REPEATS {
            self.declare_infinite_loop_draw(u16::from(seen), false);
            return true;
        }
        false
    }

    /// A declinable player decision resolved, or the outermost drain settled
    /// with nothing parked: whatever repeats from here on had an optional
    /// decision (or fresh processing) in between, so it starts a new watch.
    pub(crate) fn reset_loop_watch(&mut self) {
        self.loop_watch.clear();
        self.loop_watch_steps = 0;
    }

    /// True once the game has ended in the §18-3-2 infinite-loop draw.
    pub(crate) fn halted_by_infinite_loop(&self) -> bool {
        self.game_over
            && self.terminal_outcome_reason == Some(TerminalOutcomeReason::InfiniteLoopDraw)
    }

    /// §18-3-2: "If an infinite loop occurs and neither player has the
    /// ability to stop it, that game ends in a draw." Emits
    /// `GameEvent::InfiniteLoop` naming the newest pending effect of the loop,
    /// then ends the game with no winner. The pending effects belong to a
    /// game that is over, so the queue is dropped with it (as `concede` does).
    pub(crate) fn declare_infinite_loop_draw(&mut self, repetitions: u16, backstop: bool) {
        if self.game_over {
            return;
        }
        let (player, card_id, card_name) = match self.effect_queue.back() {
            Some(qe) => {
                let name = self
                    .card_data_for_handle(qe.source_card)
                    .map(|data| data.card_name.clone());
                (Some(qe.controller), Some(qe.card_id.clone()), name)
            }
            None => (None, None, None),
        };
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::InfiniteLoop {
            seq,
            player,
            card_id,
            card_name,
            repetitions,
            backstop,
        });
        self.effect_queue.clear();
        self.reset_loop_watch();
        self.effect_chain_depth = 0;
        self.declare_draw_with_reason(TerminalOutcomeReason::InfiniteLoopDraw);
    }
}
