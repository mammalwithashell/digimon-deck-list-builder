//! Loop guard for a kept `[Main]` use.
//!
//! **Owner ruling 2026-11-24** ("Match dcgo on the main use"): a
//! `[Main] [Once Per Turn]` clause whose DCGO script calls `RemoveUse()` on a
//! path gives the use back on exactly that path (DSL `refund_opt`). CR 4.3
//! 15-14-1-4 and 15-8-4-1/-2 read the other way (a declared `[Main]` is the
//! activation and counts); the owner chose DCGO parity over that reading.
//!
//! **The loop.** A `[Main]` is a player action. When "declare, then pass the
//! pick" refunds the use and changes nothing else, the game is back in the
//! state it started from with the same action still legal: a free no-op the
//! RL policy and tree search can repeat forever (every legal action must
//! change the game state; `invariant_fuzz`, the training no-op tripwire).
//!
//! **The guard** (data on `Game`; clones, digests and determinization see it):
//! 1. *Declaration.* `activate_field_main`, for an effect lowered with a
//!    `refund_opt` (`Effect::main_refund_capable`), records an
//!    [`InFlightMainDeclare`]: the use key, the use count before, and the
//!    guard-free digest of the state before the declaration.
//! 2. *Settlement* ([`Game::settle_main_declaration`]), once the declaration's
//!    processing has fully finished (no prompt, no queue, outermost frame). If
//!    the use count is back where it was AND the guard-free digest equals the
//!    pre-declaration one, the declaration was a proven no-op and its key is
//!    parked at that digest. Otherwise every park is dropped. An unknown
//!    pre-state (a determinized world cut mid-prompt) is never a no-op.
//! 3. *Enforcement* ([`Game::main_declare_parked`]). A park withholds the
//!    `[Main]` with that key only while the guard-free digest still equals
//!    the digest it was proven at. The first match in
//!    `action::main_effect_select::field_main_match` skips it (the next live
//!    `[Main]` on the permanent, if any, takes the bit), and the decoder
//!    dispatches on that same match. A park left behind by a path that never
//!    clears it simply stops applying once anything changes.
//! 4. *Clearing.* Every other turn action ([`Game::note_turn_action`]), a
//!    non-refund-capable `[Main]` / `<Delay>` / `<Training>` / DigiLink /
//!    trash or hand `[Main]`, and the turn boundary (CR 4.3 15-14-1-5-1).
//!
//! Why nothing reachable is lost: a park applies only in a state digest-equal
//! to one where that exact declaration, followed by passing, came back to
//! it. Every outcome of the withheld re-declaration was available at the
//! first declaration; only the free repeat is removed. The use itself is
//! kept: the same `[Main]` is offered again as soon as the state changes.
//!
//! Nothing here changes the observation tensor or the action space.

use crate::card_source::CardHandle;
use crate::enums::PlayerId;
use crate::game::Game;

/// One `[Main]` use: the permanent, the source card and the OPT key that
/// `activate_field_main` / `refund_opt` count it under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainDeclareKey {
    pub player: PlayerId,
    pub permanent: u8,
    pub source_card: CardHandle,
    pub opt_key: u8,
}

impl MainDeclareKey {
    fn sort_key(&self) -> (u8, u8, u16, u8) {
        (self.player, self.permanent, self.source_card.0, self.opt_key)
    }
}

/// A declaration proven to be a no-op in the state whose guard-free digest is
/// `at_digest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Park {
    pub key: MainDeclareKey,
    pub at_digest: u64,
}

/// A refund-capable declaration whose processing has not finished yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InFlightMainDeclare {
    pub key: MainDeclareKey,
    pub uses_before: u8,
    /// Guard-free digest before the declaration; `None` when it cannot be
    /// known (a determinized world cut mid-prompt), which never parks.
    pub digest_before: Option<u64>,
}

/// See the module docs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MainDeclareGuard {
    /// Sorted by key, one entry per key.
    parked: Vec<Park>,
    in_flight: Option<InFlightMainDeclare>,
}

impl MainDeclareGuard {
    pub fn is_empty(&self) -> bool {
        self.parked.is_empty() && self.in_flight.is_none()
    }

    pub fn parked(&self) -> &[Park] {
        &self.parked
    }

    pub fn in_flight(&self) -> Option<&InFlightMainDeclare> {
        self.in_flight.as_ref()
    }

    fn park(&mut self, park: Park) {
        match self
            .parked
            .binary_search_by_key(&park.key.sort_key(), |p| p.key.sort_key())
        {
            Ok(i) => self.parked[i] = park,
            Err(i) => self.parked.insert(i, park),
        }
    }
}

impl Game {
    /// The guard (read-only), for tests and tools.
    pub fn main_declare_guard(&self) -> &MainDeclareGuard {
        &self.main_declare_guard
    }

    /// Step 1: a refund-capable `[Main]` passed every gate and is about to
    /// record its use.
    pub(crate) fn begin_main_declaration(&mut self, key: MainDeclareKey) {
        let uses_before = self.main_declare_use_count(&key).unwrap_or(0);
        let digest_before = Some(self.guard_free_verification_digest());
        self.main_declare_guard.in_flight = Some(InFlightMainDeclare {
            key,
            uses_before,
            digest_before,
        });
    }

    /// The use count of `key` on its permanent, or `None` when the permanent
    /// no longer carries that source card.
    fn main_declare_use_count(&self, key: &MainDeclareKey) -> Option<u8> {
        let perm = self
            .players
            .get(key.player as usize)?
            .battle_area
            .get(key.permanent as usize)?;
        perm.card_sources
            .iter()
            .any(|c| c.handle() == key.source_card)
            .then(|| perm.activation_count(key.source_card, key.opt_key))
    }

    /// Step 2: settle the in-flight declaration once its processing has fully
    /// finished. Idempotent; does nothing while anything of it is pending.
    pub(crate) fn settle_main_declaration(&mut self) {
        let Some(flight) = self.main_declare_guard.in_flight else {
            return;
        };
        if self.pending_selection.is_some()
            || self.pending_selection_resume.is_some()
            || !self.effect_queue.is_empty()
            || self.selection_resolution_depth != 0
            || self.draining_deferred != 0
        {
            return;
        }
        self.main_declare_guard.in_flight = None;
        if self.game_over {
            self.main_declare_guard.parked.clear();
            return;
        }
        let refunded = self.main_declare_use_count(&flight.key) == Some(flight.uses_before);
        let now = refunded.then(|| self.guard_free_verification_digest());
        match (flight.digest_before, now) {
            (Some(before), Some(now)) if before == now => self.main_declare_guard.park(Park {
                key: flight.key,
                at_digest: now,
            }),
            _ => self.main_declare_guard.parked.clear(),
        }
    }

    /// Step 3: is the `[Main]` with this key withheld right now?
    pub fn main_declare_parked(&self, key: &MainDeclareKey) -> bool {
        let Some(park) = self.main_declare_guard.parked.iter().find(|p| p.key == *key) else {
            return false;
        };
        park.at_digest == self.guard_free_verification_digest()
    }

    /// Step 4: a turn action other than a `[Main]` declaration. Drops every
    /// park (and any declaration left unsettled by a direct call).
    pub(crate) fn note_turn_action(&mut self) {
        if !self.main_declare_guard.is_empty() {
            self.main_declare_guard = MainDeclareGuard::default();
        }
    }

    /// Drop the parks only (a non-refund-capable `[Main]` declared).
    pub(crate) fn clear_main_declare_parks(&mut self) {
        self.main_declare_guard.parked.clear();
    }

    /// Determinization: `self` is a world materialized from `root`. A park
    /// live in `root` asserts that the state has not changed since it was
    /// proven, and only hidden zones differ between `root` and the world, so
    /// it is re-stamped with the world's own digest; a park already stale in
    /// `root` is dropped (re-stamping it would revive it). An in-flight
    /// declaration's pre-state cannot be rebuilt in the world, so it is
    /// forgotten (`None`: that declaration never parks in the world). Every
    /// world of one infoset therefore offers the same actions.
    pub(crate) fn rebase_main_declare_guard_for_world(&mut self, root: &Game) {
        if self.main_declare_guard.is_empty() {
            return;
        }
        let root_digest = root.guard_free_verification_digest();
        let world_digest = self.guard_free_verification_digest();
        let guard = &mut self.main_declare_guard;
        guard.parked.retain(|p| p.at_digest == root_digest);
        for park in &mut guard.parked {
            park.at_digest = world_digest;
        }
        if let Some(flight) = guard.in_flight.as_mut() {
            flight.digest_before = None;
        }
    }

    /// Replay only: DCGO's human UI lets a player declare a `[Main]`, cancel
    /// it and declare it again. When a recorded declaration is withheld only
    /// by a park, the replay lifts the parks on that permanent. Returns
    /// whether any park was lifted.
    pub fn unpark_main_declarations_for_replay(
        &mut self,
        player: PlayerId,
        permanent: usize,
    ) -> bool {
        let before = self.main_declare_guard.parked.len();
        self.main_declare_guard
            .parked
            .retain(|p| !(p.key.player == player && p.key.permanent as usize == permanent));
        self.main_declare_guard.parked.len() != before
    }
}
