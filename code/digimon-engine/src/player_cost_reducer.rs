//! Player-scoped one-shot future-digivolve cost reducers.
//!
//! `G-COST-REDUCE-ALLY-DIGIVOLVE` (umbrella; also `G-COST-REDUCE-NEXT-SINGLE-FIRE`
//! and `G-PAY-COST-SELECT-ARBITRARY-SUSPEND`).
//!
//! Some `[Main]` effects install a turn-scoped cost reducer that has no
//! field permanent to host it (an Option that resolves and trashes itself
//! leaves the field on resolution). BT3-103 Hidden Potential Discovered! is
//! the canonical case:
//!
//! > "[Main] For the turn, when one of your green Digimon would next
//! > digivolve, by suspending 1 of your Digimon, reduce the digivolution
//! > cost by 5."
//!
//! This module provides a **closure-bearing, player-scoped reducer store**
//! that the digivolve cost paths consult BEFORE the synchronous
//! field-hosted `BeforePayCost` scan. Unlike the field-hosted scan (which
//! returns an `i32` synchronously and skips optional/paid reducers), a
//! player-scoped reducer can install an interactive accept/decline
//! `PendingSelection` plus a nested suspend-cost selection — surfacing
//! both choices through `pending_selection` per Working Rule §17.
//!
//! Lifecycle:
//! * **install** — `EffectContext::arm_player_digivolve_cost_reducer` pushes a
//!   `PlayerDigivolveCostReducer` onto `Game::player_digivolve_cost_reducers`.
//! * **fires** — at the next qualifying digivolution (target permanent
//!   passes the target-color filter), the digivolve path installs the
//!   accept/decline prompt.
//! * **single-fire** — a reducer with `single_fire == true` is removed on
//!   the first *successful* application (accept + cost paid). A declined
//!   prompt leaves it armed.
//! * **expiry** — `Expiry::EndOfTurn` reducers are dropped in
//!   `rotate_turn_player`.

use crate::card_source::CardHandle;
use crate::enums::{CardColor, PlayerId};
use crate::permanent::PermanentHandle;

/// Cost-kind discriminator. Player-scoped reducers currently only target the
/// digivolve cost path; the enum leaves room to generalize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCostReducerKind {
    /// Reduces the cost of a digivolution.
    Digivolve,
}

/// Turn-scoped expiry for a player-scoped reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCostReducerExpiry {
    /// Removed at the end of the installing player's turn ("For the turn").
    EndOfTurn,
}

/// A player-scoped, future-digivolve cost reducer.
///
/// Stored on `Game::player_digivolve_cost_reducers`. The `target_color`
/// gates which digivolutions qualify (BT3-103: "green Digimon"); the
/// `suspend_cost` flag declares that applying the reduction costs the
/// player a suspend of one of their own Digimon (an interactive,
/// player-visible cost surfaced through `pending_selection`).
#[derive(Debug, Clone)]
pub struct PlayerDigivolveCostReducer {
    /// The player who installed (and benefits from) the reducer. Only this
    /// player's digivolutions can trigger it.
    pub player: PlayerId,
    /// The card that installed the reducer (BT3-103). Used as the
    /// `source_card` for the accept/decline + suspend `PendingSelection`s so
    /// the prompts carry correct provenance for the RL observation tensor.
    pub source_card: CardHandle,
    pub kind: PlayerCostReducerKind,
    pub expiry: PlayerCostReducerExpiry,
    /// Reduction applied to the digivolution cost when the player accepts
    /// (and pays the suspend cost, if any).
    pub amount: i32,
    /// When `true`, the reducer is consumed on the first successful
    /// application ("next digivolve" — a one-shot).
    pub single_fire: bool,
    /// When `Some(color)`, the digivolving permanent's top card must
    /// include `color`. `None` = applies to any color.
    pub target_color: Option<CardColor>,
    /// When `true`, applying the reduction requires the player to suspend
    /// one of their own unsuspended Digimon. The suspend target is a
    /// player choice surfaced through `pending_selection`.
    pub suspend_cost: bool,
}

impl PlayerDigivolveCostReducer {
    /// Whether this reducer qualifies for a digivolution onto `target`
    /// performed by `acting_player`. Checks player scope and the
    /// target-color filter against the digivolving permanent's top-card
    /// colors.
    pub fn qualifies(
        &self,
        acting_player: PlayerId,
        target: PermanentHandle,
        top_card_colors: &[CardColor],
    ) -> bool {
        if self.player != acting_player {
            return false;
        }
        if self.kind != PlayerCostReducerKind::Digivolve {
            return false;
        }
        let _ = target;
        match self.target_color {
            Some(color) => top_card_colors.contains(&color),
            None => true,
        }
    }
}
