//! Shared fixtures for **archetype interaction tests** — multi-card / combo
//! tests that exercise several real implemented cards working together as the
//! deck actually plays.
//!
//! These generalize the per-card `cards_behavioral/` helpers: instead of
//! pinning one card's effect in isolation, an interaction test builds a board
//! of several cards (often a digivolution stack plus a payoff), drives an
//! action sequence, and asserts the *combined* mechanical outcome a named
//! archetype combo claims.
//!
//! Authored by the `/archetype-interaction-test-author` skill; each test maps
//! 1:1 to a combo in the archetype's model (`qa/archetype-qa/<archetype>-model.md`).
//!
//! Fixture surface:
//! - [`dsl_builder`] — load N DSL cards into a fresh `DebugRunnerBuilder`.
//! - [`BoardSnapshot`] / [`snapshot`] — capture both players' zone sizes +
//!   memory for a before/after combined-outcome assertion.
//! - [`run_actions`] — drive a scripted `(player, action_id)` sequence,
//!   auto-resolving mandatory follow-ups between steps.
//!
//! The richer board setup (digivolution stacks, breeding, security/trash
//! seeding) already lives on `DebugRunnerBuilder` / `DebugRunner`
//! (`place_stack`, `place_field_stack`, `security`, `deck`, `hand`,
//! `place_in_breeding`); these fixtures only add the multi-card conveniences
//! the per-card helpers lack.

#![allow(dead_code)]

use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::PlayerId;

/// Load several DSL cards into a fresh builder. Panics — naming the offending
/// card id — if any id is absent from the embedded DSL pack, so a typo or an
/// un-migrated card fails loudly at fixture-build time rather than mid-test.
pub fn dsl_builder(card_ids: &[&str]) -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder();
    for id in card_ids {
        b = b
            .dsl_card(id)
            .unwrap_or_else(|e| panic!("DSL card {id} not loadable into the test fixture: {e}"));
    }
    b
}

/// A combined-outcome snapshot across both players' zones + the memory gauge.
/// Index `[0]` is player 0, `[1]` is player 1. Compare two snapshots to assert
/// the *net* board change a combo produces (cards deleted, drawn, milled,
/// memory swing) in one shot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardSnapshot {
    pub memory: i16,
    pub field: [usize; 2],
    pub hand: [usize; 2],
    pub security: [usize; 2],
    pub trash: [usize; 2],
    pub deck: [usize; 2],
}

/// Capture the current board state for both players.
pub fn snapshot(r: &DebugRunner) -> BoardSnapshot {
    BoardSnapshot {
        memory: r.memory(),
        field: [r.battle_area_size(0), r.battle_area_size(1)],
        hand: [r.hand_size(0), r.hand_size(1)],
        security: [r.security_count(0), r.security_count(1)],
        trash: [r.trash_size(0), r.trash_size(1)],
        deck: [r.deck_size(0), r.deck_size(1)],
    }
}

/// Drive a scripted sequence of `(player, action_id)` decisions, auto-resolving
/// any mandatory follow-up selections between steps. Returns the action results
/// (one per scripted action) so a caller can assert per-step success; mandatory
/// auto-resolutions are best-effort and their errors are swallowed (a combo
/// that parks an *optional* selection should script the explicit choice
/// instead).
pub fn run_actions(r: &mut DebugRunner, actions: &[(PlayerId, u16)]) {
    for &(player, action_id) in actions {
        let _ = r.execute_action(player, action_id);
        let _ = r.auto_resolve();
    }
}
