//! Game-state determinization (add-determinized-search Phase 1).
//!
//! Extracts a viewer-relative [`Infoset`] (the inverse of the PvP state
//! filter's redaction) and materializes concrete, fully-known, cloneable
//! `Game` worlds consistent with it — the world-sampling seam determinized
//! search (PIMC / IS-MCTS) runs on. See the change's `design.md` D1/D2/D8
//! and `specs/game-state-determinization/spec.md` for the contract:
//!
//! - The infoset must never expose hidden information (opponent concealed
//!   identities appear only as counts + pins + a deck prior).
//! - The viewer's own hidden model is one JOINT unseen pool (own deck ∪
//!   own face-down security) plus a count partition — own security is
//!   hidden from its owner too.
//! - Sampled worlds honor per-zone counts, copy limits, and pins; they
//!   carry no live `RevealSource` (chance is frozen by world choice).
//! - Round-trip law: re-extracting the infoset from a sampled world (for
//!   the same viewer) equals the source infoset.
//!
//! Submodules:
//! - [`infoset`] — the [`Infoset`] data type + extractor (tasks 1.1/1.2).
//! - [`sample`] — [`SamplingRevealSource`], a seeded uniform sampler over a
//!   multiset implementing `opaque_deck::RevealSource` (task 1.3).
//! - [`materialize`] — the world materializer (task 1.4).
//! - [`invariants`] — the sampler invariant checker + opaque-book
//!   reconciliation (task 1.5).
//!
//! Phase-1 scope is the SELF-PLAY regime only (task 1.6): both decklists
//! known, so the opponent's `DeckPrior` is exact. PvP priors land with the
//! Phase-6 `opponent-deck-prior` capability.

mod infoset;
mod invariants;
mod materialize;
mod sample;

pub use infoset::{
    CardMultiset, DeckPrior, HiddenModel, Infoset, OwnHiddenModel, PermanentView,
    PublicPlayerView,
};
pub use invariants::{check_world_invariants, reconcile_opaque_books};
pub use materialize::materialize;
pub use sample::SamplingRevealSource;
