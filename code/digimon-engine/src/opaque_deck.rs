//! Opaque-opponent-deck mode primitives.
//!
//! In standard games the engine knows both decks fully (post-shuffle order).
//! In *opaque mode* — used by PvP recording replay (where we never observe
//! the opponent's deck order from one side) and by future RL inference
//! against unknown opponents — the engine knows only the **composition**
//! of one player's deck, not its order. When the engine would draw or mill
//! from that player's pile, it consults an externally-supplied
//! [`RevealSource`] for the next card identity.
//!
//! Structure split, intentional:
//!   - [`OpaqueDeckState`] (this module, `Clone`) is per-player composition
//!     accounting — multiset of remaining cards. Lives on [`Player`].
//!   - [`RevealSource`] (this module, NOT `Clone`) is the externally-owned
//!     supplier of revelations. Lives on [`Game`] (which is not `Clone`).
//!
//! That split keeps `Player` cloneable for snapshot/recording purposes
//! while letting the reveal source be stateful (e.g. a `VecDeque` queue).
//!
//! See `openspec/changes/add-dcgo-recording-parity-harness/specs/opaque-opponent-deck/spec.md`
//! for the requirement contract.
//!
//! [`Player`]: crate::player::Player
//! [`Game`]: crate::game::Game

use std::collections::{HashMap, VecDeque};

/// What kind of pile-touching action triggered a reveal request. The
/// supplier *may* validate alignment (e.g. a recording's reveal queue
/// could refuse if the next queued reveal has a different tag than the
/// engine is asking for); the engine itself enforces no semantics from
/// the tag — it's metadata the supplier uses for cross-checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevealKind {
    /// Standard draw into hand.
    Draw,
    /// Card moved from the opaque pile to security at setup, or as part
    /// of a security-replenishing effect.
    Security,
    /// Card sent from top of pile directly to trash (mill).
    Mill,
    /// Card revealed by an effect (peek, search, etc.) without an
    /// immediate zone destination — the effect resolution will route it.
    Effect,
}

impl RevealKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RevealKind::Draw => "draw",
            RevealKind::Security => "security",
            RevealKind::Mill => "mill",
            RevealKind::Effect => "effect",
        }
    }
}

/// Returned by [`RevealSource::next_reveal`] when the source has no more
/// reveals to supply (truncated recording, or supplier explicitly out of
/// data). The engine surfaces this as a structured error distinguishable
/// from other game-state errors — callers like the replay harness use it
/// to bucket "recording corrupted" separately from "engine misbehaved".
#[derive(Debug, Clone)]
pub struct RevealExhausted {
    pub requested_kind: RevealKind,
    pub message: String,
}

impl std::fmt::Display for RevealExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "reveal source exhausted while requesting {} reveal: {}",
            self.requested_kind.as_str(),
            self.message
        )
    }
}

impl std::error::Error for RevealExhausted {}

/// Supplier of opaque-pile revelations. The engine calls
/// [`RevealSource::next_reveal`] whenever it would draw or otherwise read
/// a card from an opaque pile; the supplier returns the card-ID string of
/// the next card to be revealed.
///
/// Implementations are typically backed by:
///   - a recording's reveal queue (for parity replay), OR
///   - a sampler over the multiset (for RL inference)
pub trait RevealSource: Send + std::fmt::Debug {
    fn next_reveal(&mut self, kind: RevealKind) -> Result<String, RevealExhausted>;
}

/// A FIFO queue-backed [`RevealSource`]. The canonical implementation for
/// replay — the harness preloads the queue from the recording's `reveal`
/// rows in order, then hands it to the engine.
///
/// Validation: if `strict_kind_check` is on, a reveal request with a
/// `kind` that doesn't match the next queued entry's `kind` returns
/// [`RevealExhausted`] with a descriptive message (helps catch
/// engine-vs-recording divergence in the parity oracle).
#[derive(Debug)]
pub struct RevealQueue {
    queue: VecDeque<(RevealKind, String)>,
    strict_kind_check: bool,
}

impl RevealQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            strict_kind_check: true,
        }
    }

    /// Construct a queue from a list of (kind, card_id) pairs to be served
    /// in order. Used by the replay harness to preload all known reveals
    /// from a recording before replay starts.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (RevealKind, String)>) -> Self {
        let queue: VecDeque<_> = pairs.into_iter().collect();
        Self {
            queue,
            strict_kind_check: true,
        }
    }

    /// Disable the kind-match check. Use when the supplier only knows
    /// card IDs in order (e.g. RL inference samplers) and doesn't care
    /// which engine path triggered the reveal.
    pub fn relaxed(mut self) -> Self {
        self.strict_kind_check = false;
        self
    }

    pub fn push(&mut self, kind: RevealKind, card_id: String) {
        self.queue.push_back((kind, card_id));
    }

    pub fn remaining(&self) -> usize {
        self.queue.len()
    }
}

impl Default for RevealQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl RevealSource for RevealQueue {
    fn next_reveal(&mut self, kind: RevealKind) -> Result<String, RevealExhausted> {
        let (queued_kind, card_id) = self.queue.pop_front().ok_or_else(|| RevealExhausted {
            requested_kind: kind,
            message: "queue empty".to_string(),
        })?;
        if self.strict_kind_check && queued_kind != kind {
            return Err(RevealExhausted {
                requested_kind: kind,
                message: format!(
                    "next queued reveal was tagged `{}` but engine requested `{}`",
                    queued_kind.as_str(),
                    kind.as_str()
                ),
            });
        }
        Ok(card_id)
    }
}

/// Per-player composition tracker for an opaque pile.
///
/// Records the multiset of card IDs remaining and the total count.
/// Updated by [`Self::consume`] whenever a reveal materializes a card.
/// Crucially: this does NOT contain the supplier — the supplier lives on
/// [`Game`] so [`OpaqueDeckState`] can stay [`Clone`] without forcing
/// constraint on the supplier.
///
/// [`Game`]: crate::game::Game
#[derive(Debug, Clone)]
pub struct OpaqueDeckState {
    /// Composition: card_id → count remaining.
    multiset: HashMap<String, usize>,
    /// Sum of multiset values. Cached to avoid repeated traversal.
    total_remaining: usize,
}

impl OpaqueDeckState {
    /// Build from a decklist (unordered Vec<card_id>). Duplicates fold into
    /// the multiset count.
    pub fn from_decklist(decklist: &[String]) -> Self {
        let mut multiset: HashMap<String, usize> = HashMap::new();
        for id in decklist {
            *multiset.entry(id.clone()).or_insert(0) += 1;
        }
        let total = decklist.len();
        Self {
            multiset,
            total_remaining: total,
        }
    }

    pub fn total_remaining(&self) -> usize {
        self.total_remaining
    }

    pub fn is_empty(&self) -> bool {
        self.total_remaining == 0
    }

    /// Returns the count of cards with this ID still in the pile.
    pub fn count_of(&self, card_id: &str) -> usize {
        self.multiset.get(card_id).copied().unwrap_or(0)
    }

    /// Borrow the composition map (read-only). Useful for tensor encoders
    /// and UI summaries that want to display "opp's known composition".
    pub fn composition(&self) -> &HashMap<String, usize> {
        &self.multiset
    }

    /// Mark one card of this ID as consumed. Returns an error if the
    /// multiset reports zero remaining of this ID (e.g. the supplier
    /// returned a card that wasn't in the decklist, or has now been
    /// over-supplied beyond its decklist count).
    ///
    /// The engine's invariant: a well-formed reveal stream from the
    /// recording NEVER over-supplies — every `card_id` returned by the
    /// supplier was originally in the decklist and has not been seen
    /// more times than its multiset count.
    pub fn consume(&mut self, card_id: &str) -> Result<(), RevealExhausted> {
        let count = self.multiset.get_mut(card_id);
        match count {
            Some(c) if *c > 0 => {
                *c -= 1;
                self.total_remaining -= 1;
                Ok(())
            }
            _ => Err(RevealExhausted {
                requested_kind: RevealKind::Draw, // best-effort tag; consume() doesn't know the real kind
                message: format!(
                    "supplier returned card `{}` but multiset has none of that ID remaining",
                    card_id
                ),
            }),
        }
    }

    /// Inverse of [`consume`]: increment the count of `card_id` in the
    /// multiset by one. Used when a card returns from a revealed zone
    /// back to the opaque pile — chiefly the mulligan path, where the
    /// opaque opponent's hand is folded back into the deck before
    /// re-drawing. Also useful for any future effect that returns a
    /// drawn card to the deck.
    ///
    /// This never errors: restoring an unknown card_id simply registers
    /// it in the multiset at count 1. (Such a thing should be impossible
    /// if the engine only restores cards previously consumed from this
    /// pile, but we don't enforce it here — the worst case is a
    /// composition that subtly diverges from the original decklist, and
    /// the next downstream `consume` will catch it.)
    pub fn restore(&mut self, card_id: &str) {
        *self.multiset.entry(card_id.to_string()).or_insert(0) += 1;
        self.total_remaining += 1;
    }

    /// Reserve `count` cards as "physically in some zone, identity not
    /// yet known" — debits `total_remaining` without decrementing any
    /// per-card multiset entry. Used by `Game::setup_security_for_player`
    /// in opaque mode: at setup time, N cards are physically in the
    /// security stack (so the pile has N fewer cards remaining) but we
    /// don't know which specific cards yet (so we can't update per-card
    /// counts). When a specific placeholder is later materialized to a
    /// known card_id, [`consume_per_card_only`] decrements that card's
    /// per-card count to balance the books.
    pub fn reserve_placeholders(&mut self, count: usize) {
        // Saturate at zero — defensive against over-reservation.
        self.total_remaining = self.total_remaining.saturating_sub(count);
    }

    /// Decrement only the per-card count of `card_id`, leaving
    /// `total_remaining` unchanged. Counterpart to
    /// [`reserve_placeholders`]: the total was debited up-front when the
    /// placeholder was created; this call balances the per-card multiset
    /// when the placeholder's specific identity is revealed.
    pub fn consume_per_card_only(&mut self, card_id: &str) -> Result<(), RevealExhausted> {
        let count = self.multiset.get_mut(card_id);
        match count {
            Some(c) if *c > 0 => {
                *c -= 1;
                Ok(())
            }
            _ => Err(RevealExhausted {
                requested_kind: RevealKind::Security,
                message: format!(
                    "supplier returned card `{}` for opaque security materialization, \
                     but multiset has none of that ID remaining",
                    card_id
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_queue_serves_in_order() {
        let mut q = RevealQueue::from_pairs(vec![
            (RevealKind::Draw, "BT1-001".to_string()),
            (RevealKind::Draw, "BT1-010".to_string()),
        ]);
        assert_eq!(q.next_reveal(RevealKind::Draw).unwrap(), "BT1-001");
        assert_eq!(q.next_reveal(RevealKind::Draw).unwrap(), "BT1-010");
        assert!(q.next_reveal(RevealKind::Draw).is_err());
    }

    #[test]
    fn reveal_queue_strict_kind_rejects_mismatch() {
        let mut q = RevealQueue::from_pairs(vec![(RevealKind::Draw, "BT1-001".into())]);
        let err = q.next_reveal(RevealKind::Security).unwrap_err();
        assert!(err.message.contains("draw"));
        assert!(err.message.contains("security"));
    }

    #[test]
    fn reveal_queue_relaxed_accepts_any_kind() {
        let mut q = RevealQueue::from_pairs(vec![(RevealKind::Draw, "BT1-001".into())]).relaxed();
        // No strict check → security request consumes the draw-tagged entry.
        assert_eq!(q.next_reveal(RevealKind::Security).unwrap(), "BT1-001");
    }

    #[test]
    fn opaque_deck_state_from_decklist_counts_duplicates() {
        let s = OpaqueDeckState::from_decklist(&[
            "BT1-001".into(),
            "BT1-001".into(),
            "BT1-010".into(),
        ]);
        assert_eq!(s.total_remaining(), 3);
        assert_eq!(s.count_of("BT1-001"), 2);
        assert_eq!(s.count_of("BT1-010"), 1);
        assert_eq!(s.count_of("BT99-999"), 0);
    }

    #[test]
    fn opaque_deck_state_consume_decrements() {
        let mut s = OpaqueDeckState::from_decklist(&[
            "BT1-001".into(),
            "BT1-001".into(),
            "BT1-010".into(),
        ]);
        s.consume("BT1-001").unwrap();
        assert_eq!(s.total_remaining(), 2);
        assert_eq!(s.count_of("BT1-001"), 1);
        s.consume("BT1-001").unwrap();
        assert_eq!(s.count_of("BT1-001"), 0);
        // Attempting to consume another BT1-001 now fails.
        assert!(s.consume("BT1-001").is_err());
        // Other card still consumable.
        s.consume("BT1-010").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn opaque_deck_state_consume_unknown_card_errors() {
        let mut s = OpaqueDeckState::from_decklist(&["BT1-001".into()]);
        let err = s.consume("BT99-999").unwrap_err();
        assert!(err.message.contains("BT99-999"));
    }
}
