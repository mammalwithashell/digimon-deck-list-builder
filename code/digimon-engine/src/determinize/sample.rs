//! Task 1.3 — [`SamplingRevealSource`]: a seeded uniform sampler over a
//! card multiset, implementing [`crate::opaque_deck::RevealSource`].
//!
//! This is the "sampler over the multiset (for RL inference)" implementation
//! the `RevealSource` trait doc anticipated. Phase-1 determinized search
//! does NOT use it at search time (D1: worlds are materialized up front with
//! committed piles and `reveal_source = None`); it exists for the lazy /
//! live-sampling variants (IS-MCTS re-determinization, deferred
//! materialization) and as a standalone-testable sampling primitive.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use super::infoset::CardMultiset;
use crate::opaque_deck::{RevealExhausted, RevealKind, RevealSource};

/// Uniform without-replacement sampler over a card multiset.
///
/// Deterministic per seed: the internal pool is expanded in `BTreeMap`
/// (sorted) order and drawn with a private `StdRng`, so a given
/// (multiset, seed) pair always yields the same reveal sequence.
#[derive(Debug, Clone)]
pub struct SamplingRevealSource {
    /// Expanded multiset (one entry per physical card), unordered.
    remaining: Vec<String>,
    rng: StdRng,
}

impl SamplingRevealSource {
    pub fn new(multiset: &CardMultiset, seed: u64) -> Self {
        let mut remaining = Vec::with_capacity(multiset.values().sum());
        for (id, n) in multiset {
            for _ in 0..*n {
                remaining.push(id.clone());
            }
        }
        Self {
            remaining,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Convenience: build from an unordered decklist (duplicates fold in).
    pub fn from_decklist(ids: &[String], seed: u64) -> Self {
        let mut ms = CardMultiset::new();
        for id in ids {
            *ms.entry(id.clone()).or_insert(0) += 1;
        }
        Self::new(&ms, seed)
    }

    /// Cards left in the pool.
    pub fn remaining(&self) -> usize {
        self.remaining.len()
    }
}

impl RevealSource for SamplingRevealSource {
    /// Draw uniformly from the remaining multiset and consume the card.
    /// The `kind` tag is metadata only — a sampler doesn't care which
    /// engine path triggered the reveal.
    fn next_reveal(&mut self, kind: RevealKind) -> Result<String, RevealExhausted> {
        if self.remaining.is_empty() {
            return Err(RevealExhausted {
                requested_kind: kind,
                message: "sampling pool exhausted".to_string(),
            });
        }
        let i = self.rng.gen_range(0..self.remaining.len());
        Ok(self.remaining.swap_remove(i))
    }

    /// Verbatim clone — the fork shares this source's RNG *state*, so both
    /// copies replay IDENTICAL future draws. Per design D8 that is the
    /// intended **same-world lookahead** semantic: a fork evaluating "this
    /// world's line" must see the same future. A fork intended as an
    /// **alternative future** (averaging over worlds) must NOT rely on
    /// `clone`: construct a fresh `SamplingRevealSource` with a split seed
    /// instead.
    fn clone_box(&self) -> Box<dyn RevealSource> {
        Box::new(self.clone())
    }
}
