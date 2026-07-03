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
