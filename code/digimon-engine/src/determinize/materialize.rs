//! Task 1.4 — the world materializer: sample one concrete, fully-known,
//! cloneable, playable `Game` consistent with a viewer's [`Infoset`].
//!
//! Strategy (design D1/D2): clone the true game, then RE-DEAL every
//! hidden-from-viewer collection:
//!
//! - the viewer's own joint unseen pool (deck ∪ face-down security) is
//!   re-partitioned and re-ordered — own security identities are hidden
//!   from the owner too, so the partition itself is resampled;
//! - the opponent's unseen pool (hand ∪ deck ∪ face-down security) is
//!   re-partitioned into hand / face-down security slots / deck order;
//! - both digitama decks are re-shuffled (composition known, order not);
//! - face-up (pinned) security cards keep their exact slot and identity;
//! - `reveal_source` is cleared and the world's `rng` is re-seeded from the
//!   world seed (the true game's RNG stream predicts future shuffles — a
//!   covert channel search must not inherit).
//!
//! **No-X-ray argument.** The re-deal is a uniformly random permutation
//! (Fisher-Yates over the pooled cards) of exactly the cards whose location
//! is hidden. The multiset of that pool is precisely what the infoset
//! entitles the viewer to know in the self-play regime (`DeckPrior::Exact`
//! = decklist minus observed), and a uniform shuffle's output distribution
//! is independent of the input arrangement — so the sampled world depends
//! only on (infoset, seed), never on the true hidden assignment.
//! Re-dealing the physical `CardSource` instances (rather than fabricating
//! new ones) reuses the engine's own card-instance plumbing — `data_index`
//! stays coherent with the shared card store, `card_index` stays unique —
//! and makes per-zone counts and copy limits hold by construction.
//! Distinct physical copies of the same card id are indistinguishable in
//! the infoset, so permuting instances is equivalent to sampling
//! identities.
//!
//! **Scope (v1, self-play regime only — task 1.6):**
//! - 2-player, non-opaque games only (asserted).
//! - Pins include face-up security and live public security reveals; other
//!   per-card revelation state remains untracked
//!   (no "seen top of deck", no "revealed hand card"), so
//!   knowledge from past peeks/reveals is not preserved — the documented
//!   PIMC belief-staleness limitation.
//! - Materializing is defined at any state (including mid-selection: the
//!   parked prompt and resume stack are cloned verbatim). Search should
//!   root at the *viewer's* decision points; stepping THROUGH a cloned
//!   opponent-owned selection whose valid indices referenced that
//!   opponent's re-dealt hand is not meaningful and is out of Phase-1
//!   scope.

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use crate::card_source::CardSource;
use crate::enums::PlayerId;
use crate::game::Game;

/// Materialize one determinized world for `viewer`, deterministic in
/// `seed`. The result is a fully-known `Game`: every pile concrete and
/// ordered, `reveal_source = None`.
///
/// Panics on non-2-player or opaque-deck games (Phase-1 scope; see module
/// docs).
pub fn materialize(game: &Game, viewer: PlayerId, seed: u64) -> Game {
    assert_eq!(
        game.players.len(),
        2,
        "materialize supports 2-player games only"
    );
    assert!((viewer as usize) < 2, "viewer out of range");
    assert!(
        game.players.iter().all(|p| p.opaque_deck_state.is_none()),
        "Phase-1 determinization supports self-play (non-opaque) games only"
    );

    let mut world = game.clone();
    // D1: chance is frozen by world choice — no live reveal source.
    world.reveal_source = None;

    let mut rng = StdRng::seed_from_u64(seed);
    // Viewer: own hand is KNOWN (kept verbatim); re-deal deck ∪ face-down
    // security.
    let inspected_security = game.pending_selection.as_ref().and_then(|s| s.card_choice.as_ref())
        .and_then(|choice| choice.inspected_security_owner(viewer));
    redeal_hidden(&mut world, viewer, false, inspected_security == Some(viewer), &mut rng);
    // Opponent: hand is hidden too — re-deal hand ∪ deck ∪ face-down
    // security.
    redeal_hidden(&mut world, 1 - viewer, true, inspected_security == Some(1 - viewer), &mut rng);

    if let Some(mut selection) = world.pending_selection.take() {
        if let Some(choice) = &mut selection.card_choice {
            choice.rebind_sampled_security(game, &world);
        }
        world.pending_selection = Some(selection);
    }

    // Fresh RNG stream for the world's own future randomness (shuffles
    // triggered by effects, etc.), derived from the world seed only.
    world.rng = StdRng::seed_from_u64(rng.gen());
    world
}

/// Pool `pid`'s hidden collections, shuffle, and re-deal:
/// hand (if `resample_hand`) → face-down security slots → deck (remaining,
/// in shuffled order). Public security identities stay pinned at their slots.
/// The digitama deck is re-shuffled in place (order-only uncertainty).
pub(super) fn redeal_hidden(
   world: &mut Game,
   pid: PlayerId,
   resample_hand: bool,
   security_inspected: bool,
   rng: &mut StdRng,
) {
   let public: std::collections::HashSet<_> = world
       .player(pid)
       .security
       .iter()
       .filter(|c| world.security_card_is_public(pid, c.handle()))
       .map(|c| c.card_index)
       .collect();
    let p = &mut world.players[pid as usize];

    let mut pool: Vec<CardSource> = std::mem::take(&mut p.deck);
    let hand_count = p.hand.len();
    if resample_hand {
        pool.append(&mut p.hand);
    }

    // Partition security into pinned slots (kept) and face-down slots
    // (pooled), preserving slot positions.
    let old_security = std::mem::take(&mut p.security);
    let mut slots: Vec<Option<CardSource>> = Vec::with_capacity(old_security.len());
    for c in old_security {
        if security_inspected || public.contains(&c.card_index) {
            slots.push(Some(c));
        } else {
            pool.push(c);
            slots.push(None);
        }
    }

    // One uniform permutation drives the whole re-deal: partition AND order.
    pool.shuffle(rng);

    if resample_hand {
        for _ in 0..hand_count {
            p.put_in_hand(pool.pop().expect("re-deal pool underflow (hand)"));
        }
    }
    for slot in slots.iter_mut() {
        if slot.is_none() {
            *slot = Some(pool.pop().expect("re-deal pool underflow (security)"));
        }
    }
    p.security = slots
        .into_iter()
        .map(|s| s.expect("all security slots filled"))
        .collect();
    p.deck = pool;

    p.digitama_deck.shuffle(rng);
}
