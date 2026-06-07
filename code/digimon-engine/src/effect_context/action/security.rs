//! Security-stack mutations on `EffectContext` — extracted by mechanic.

#![allow(unused_imports)]
use crate::action::mask::*;
use crate::action::space::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::effect::*;
use crate::effect_context::*;
use crate::enums::*;
use crate::game::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::scheduled_effects::*;
use crate::selection::*;
use crate::token_registry::*;
use crate::trigger_context::*;

impl<'a> EffectContext<'a> {
    /// (Track A) Variant of `place_sourceless_permanent_bottom_security_…` that
    /// targets any permanent at any position, then handles the current
    /// replacement window via `handle_replacement` rather than `cancel_leave`.
    pub fn place_permanent_on_security_and_handle_current_replacement(
        &mut self,
        player: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        if self
            .game
            .modifiers
            .player_has(self.player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }
        if self
            .game
            .place_permanent_on_security_without_leave_replacement(
                player,
                target,
                position,
                face_up,
                self.player,
            )
        {
            if self.game.parked_replacement.is_some() {
                self.handle_replacement();
            }
            true
        } else {
            false
        }
    }

    /// (Track E) Move the resolving effect's source permanent
    /// (`self.source_permanent`) into its owner's security stack at
    /// `position` with the requested orientation. Sugar over
    /// `Game::place_permanent_on_security_observed`.
    ///
    /// Sources below the top card are routed to each source's owner's trash
    /// (firing `OnDigivolutionCardTrashed` per source). Linked cards likewise
    /// go to trash with a single `OnLinkedCardTrashed` dispatch. The top card
    /// becomes a single security slot at the requested end of the stack;
    /// `face_up=true` adds the slot to `face_up_security`.
    ///
    /// Used by printed text "place this Digimon at the bottom of your
    /// security stack face down" (EX4-060), "place this Digimon as your top
    /// security card" (EX9-021), and similar self-placement riders. DCGO
    /// parity: `IPutSecurityPermanent(card.PermanentOfThisCard(), …)`.
    ///
    /// Returns `false` if there is no source permanent (e.g. an Option-card
    /// effect or rule-source effect), if the controller has
    /// `CannotAddSecurityByEffect`, or if either of the
    /// `WhenWouldLeaveBattleArea` / `WhenWouldPlaceInSecurity` replacement
    /// outcomes is non-`None` (cancelled / redirected / etc.).
    ///
    /// **Engine divergence:** DCGO bundles the entire permanent (top +
    /// sources + linked) under a single security slot. The Rust security
    /// model is flat; sources/linked go to trash. See
    /// `Game::place_permanent_on_security_observed` for the divergence note.
    pub fn place_self_at_security(
        &mut self,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        let Some(handle) = self.source_permanent else {
            return false;
        };
        let owner = handle.player;
        self.game.place_permanent_on_security_observed(
            owner,
            handle,
            position,
            face_up,
            self.player,
        )
    }

    /// Flip the topmost still-face-down card in `player`'s security stack
    /// face-up without opening a player choice. If every security card is
    /// already face-up, this is a no-op.
    pub fn flip_security_face_up(&mut self, player: PlayerId) -> bool {
        let Some(card) = self.game.player(player).security.iter().rev().find(|card| {
            !self
                .game
                .player(player)
                .face_up_security
                .contains(&card.card_index)
        }) else {
            return false;
        };
        let card_index = card.card_index;
        self.game
            .player_mut(player)
            .face_up_security
            .insert(card_index);
        true
    }

    /// (Track E) Replacement-aware sibling of `place_self_at_security`. When
    /// invoked inside a parked replacement (e.g. a "would leave" replacement
    /// whose subject is the source permanent), runs the move and then
    /// cancels the parked replacement so the original event does not also
    /// fire.
    ///
    /// Mirrors the shape of
    /// `place_sourceless_permanent_bottom_security_and_cancel_current_replacement`.
    /// Used by EX4-060 (place self at security bottom face-down on
    /// "would leave battle area other than by your effects" replacement).
    pub fn place_self_at_security_and_cancel_current_replacement(
        &mut self,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        if self.place_self_at_security(position, face_up) {
            if self.game.parked_replacement.is_some() {
                self.cancel_current_replacement();
            }
            true
        } else {
            false
        }
    }

    /// (Track E) Option-card flavor of `place_self_at_security`. Consumes
    /// the `Game.pending_option` transient that carries the in-flight
    /// Option card mid-resolution (between pay-cost and dispose), routing
    /// it into its owner's security stack at `position` with the requested
    /// orientation. Used by ST20-15 Island of Adventure's [Main] tail
    /// "Then, place this card face up as the top security card."
    ///
    /// Distinguishing factors vs `place_self_at_security` (Digimon flavor):
    /// - The Option card has no source permanent during `OptionMain`
    ///   resolution; `self.source_permanent` is `None`. The card lives in
    ///   the `Game.pending_option` transient instead.
    /// - There is no source stack to bundle or trash — Options are single
    ///   cards.
    /// - Consuming `pending_option` automatically suppresses the post-
    ///   `OptionMain` dispose-trash: `advance_pending_option` short-circuits
    ///   on `pending_option.is_none()`, so the card lands in security
    ///   instead of routing through the standard Option dispose path.
    ///
    /// Routes through `WhenWouldPlaceInSecurity` replacement; bails (and
    /// restores `pending_option`) on any non-`None` outcome or installed
    /// pending selection. Gated by `CannotAddSecurityByEffect` (player-
    /// scoped against the acting player).
    ///
    /// DCGO parity: `ReplaceTopSecurityWithFaceUpOptionMainEffect` family
    /// (ST20-15 and similar Option-card self-placement riders).
    pub fn place_self_option_at_security(
        &mut self,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        use crate::enums::Zone;
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Gate on `CannotAddSecurityByEffect` BEFORE consuming pending_option,
        // so a gated call leaves the resolution flow intact.
        if self
            .game
            .modifiers
            .player_has(self.player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }

        // Take pending_option. If absent, this method was invoked outside an
        // Option-card resolution and is a clean no-op.
        let Some(pending) = self.game.pending_option.take() else {
            return false;
        };

        // Snapshot fields needed across the replacement-call mutable borrow.
        let owner = pending.owner;
        let card_handle = pending.card.handle();
        let face_up_key = pending.card.card_index;
        let source_zone = pending.source_kind.zone();

        // Route through WhenWouldPlaceInSecurity. If a selection is installed
        // or the replacement returns a non-None outcome, restore pending and
        // bail — the original Option flow will then continue normally.
        if !self.game.would_replacement_is_clear(
            crate::enums::EffectTiming::WhenWouldPlaceInSecurity,
            ReplacementSubject::Card(card_handle, source_zone),
            self.player,
            Some(Zone::Security),
        ) {
            // Restore so the dispose flow can complete normally.
            self.game.pending_option = Some(pending);
            return false;
        }

        // Commit the move. Move pending.card into security at `position`.
        let card = pending.card;
        match position {
            crate::enums::StackPosition::Top => {
                self.game.player_mut(owner).security.push(card);
            }
            crate::enums::StackPosition::Bottom => {
                self.game.player_mut(owner).security.insert(0, card);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let sec_len = self.game.player(owner).security.len();
                let idx = if sec_len == 0 {
                    0
                } else {
                    self.game.rng.gen_range(0..=sec_len)
                };
                self.game.player_mut(owner).security.insert(idx, card);
            }
        }
        if face_up {
            self.game
                .player_mut(owner)
                .face_up_security
                .insert(face_up_key);
        }
        true
    }

    /// Move a specific card from `player`'s security stack to their hand.
    pub fn add_to_hand_from_security(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(idx) = self
            .game
            .player(player)
            .security
            .iter()
            .position(|c| c.handle() == card)
        else {
            return false;
        };
        // Opaque-aware: materialize before moving to hand — the card's
        // identity must be real to land in hand sensibly (subsequent
        // hand reads, plays, etc. expect a real data_index).
        self.game.ensure_security_materialized(player, idx);
        let removed = self.game.player_mut(player).security.remove(idx);
        let owner = removed.owner;
        self.game
            .player_mut(player)
            .face_up_security
            .remove(&removed.card_index);
        self.game.fire_security_removed_observers(
            player,
            self.player,
            removed,
            crate::selection::SecurityRemovalDestination::Hand(owner),
        );
        true
    }

    /// Move a specific card from `player`'s security stack to that player's
    /// deck (top or bottom; Digi-Eggs route to the digitama deck). Fires the
    /// `OnLoseSecurity` observer chain via `fire_security_removed_observers`
    /// with a `Deck` destination. Mirrors `add_to_hand_from_security` but the
    /// final placement is the deck. Used by LM-020 Quantumon's [When
    /// Digivolving] clause. Returns `false` if the card is not in the stack.
    pub fn return_security_card_to_deck(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
        to_bottom: bool,
    ) -> bool {
        let Some(idx) = self
            .game
            .player(player)
            .security
            .iter()
            .position(|c| c.handle() == card)
        else {
            return false;
        };
        // Materialize before moving — the card's identity must be real to land
        // in the deck sensibly (it will later be drawn / revealed).
        self.game.ensure_security_materialized(player, idx);
        let removed = self.game.player_mut(player).security.remove(idx);
        let owner = removed.owner;
        self.game
            .player_mut(player)
            .face_up_security
            .remove(&removed.card_index);
        self.game.fire_security_removed_observers(
            player,
            self.player,
            removed,
            crate::selection::SecurityRemovalDestination::Deck { owner, to_bottom },
        );
        true
    }

    /// Move the top card of `player`'s security stack to its owner's hand.
    pub fn add_top_security_to_hand(&mut self, player: PlayerId) -> bool {
        let Some(card) = self
            .game
            .player(player)
            .security
            .last()
            .map(|card| card.handle())
        else {
            return false;
        };
        self.add_to_hand_from_security(player, card)
    }

    /// Move the bottom card of `player`'s security stack to its owner's hand.
    pub fn add_bottom_security_to_hand(&mut self, player: PlayerId) -> bool {
        let Some(card) = self
            .game
            .player(player)
            .security
            .first()
            .map(|card| card.handle())
        else {
            return false;
        };
        self.add_to_hand_from_security(player, card)
    }

    /// Move the card currently resolving from security into its defender's hand.
    ///
    /// This consumes `Game.pending_security`, so the security dispose phase
    /// cannot also trash the card. If the card was already played from
    /// security, the pending state is restored and this is a no-op.
    pub fn add_pending_security_to_hand(&mut self) -> bool {
        let Some(pending) = self.game.pending_security.take() else {
            return false;
        };

        if pending.played {
            self.game.pending_security = Some(pending);
            return false;
        }

        let owner = pending.card.owner;
        self.game.player_mut(owner).hand.push(pending.card);
        self.game.fire_on_add_to_hand_by_effect(owner);
        true
    }

    /// Shuffle `player`'s security stack.
    pub fn shuffle_security(&mut self, player: PlayerId) {
        self.game.shuffle_security(player);
    }

    /// Move a card from `source` to `player`'s security stack. Does not
    /// fire `OnLoseSecurity` observers. See `Game::place_on_security`.
    ///
    /// Phase 6: gated by `CannotAddSecurityByEffect`. The gate checks the
    /// ACTING player (the effect owner, `self.player`), not the target —
    /// consistent with DCGO's per-player restriction semantics.
    pub fn place_on_security(
        &mut self,
        player: PlayerId,
        source: crate::enums::CardSourceRef,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        // Phase 6: if the acting player has CannotAddSecurityByEffect, suppress.
        if self
            .game
            .modifiers
            .player_has(self.player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }
        // Track C / D consult site (2026-05-08): permanent-scoped
        // `CannotAddSecurity` — sibling of player-scoped
        // `CannotAddSecurityByEffect`. Anchored to a specific permanent in
        // the acting player's battle area, mirroring the printed-text
        // shape "while this Digimon is in play, your effects can't add
        // security cards."
        let battle_area_len = self.game.player(self.player).battle_area.len();
        for i in 0..battle_area_len {
            let h = PermanentHandle {
                player: self.player,
                index: i as u8,
            };
            if self.game.modifiers.has(h, ModifierType::CannotAddSecurity) {
                return false;
            }
        }
        self.game
            .place_on_security_observed(player, source, position, face_up, self.player)
    }

    /// (Track E) Extract a digivolution source identified by its stable `CardHandle`
    /// from `carrier`'s stack and place it into `target_player`'s security
    /// stack at `position` with the requested orientation. Track E Tier 2
    /// Task 6 — sugar over `select_own_sources` -> `place_on_security` for
    /// printed text like "place 1 of this Digimon's digivolution cards on
    /// top of your security stack" (Puppets G027 shape).
    ///
    /// Looks up the source's current index from its stable `CardHandle` —
    /// resilient to intervening battle-area shifts that would invalidate a
    /// raw `usize` index. Routes through `place_on_security_observed`, so
    /// `WhenWouldPlaceInSecurity` replacements and `CannotAddSecurityByEffect`
    /// gates apply identically to a hand/trash placement.
    ///
    /// Returns `false` if the carrier handle is invalid or the source card
    /// is not present in the carrier's stack at call time.
    pub fn security_place_stacked_card(
        &mut self,
        carrier: PermanentHandle,
        source_card: CardHandle,
        target_player: PlayerId,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        // Resolve the stable CardHandle to its current index in the carrier's
        // stack. Bail clean if the carrier was deleted or the source was
        // already moved by an intervening effect.
        let source_index = {
            let Some(perm) = self
                .game
                .player(carrier.player)
                .battle_area
                .get(carrier.index as usize)
            else {
                return false;
            };
            match perm
                .card_sources
                .iter()
                .position(|c| c.handle() == source_card)
            {
                Some(idx) => idx,
                None => return false,
            }
        };
        self.place_on_security(
            target_player,
            crate::enums::CardSourceRef::Material(carrier, source_index),
            position,
            face_up,
        )
    }

    /// Convenience: extract the **top stacked card** (the source one below
    /// the visible top, i.e. `card_sources[len - 2]`) and place it in
    /// `target_player`'s security at `position` / `face_up`. Mirrors
    /// printed text like Puppets G027 "move the top stacked card to top
    /// security card." When the carrier has fewer than 2 card_sources
    /// (no stacked card below the top), returns `false` without mutating
    /// state.
    pub fn security_place_top_stacked_card(
        &mut self,
        carrier: PermanentHandle,
        target_player: PlayerId,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        let source_card = {
            let Some(perm) = self
                .game
                .player(carrier.player)
                .battle_area
                .get(carrier.index as usize)
            else {
                return false;
            };
            let len = perm.card_sources.len();
            if len < 2 {
                // No stacked card below the top.
                return false;
            }
            perm.card_sources[len - 2].handle()
        };
        self.security_place_stacked_card(carrier, source_card, target_player, position, face_up)
    }

    /// (Track A) Move a battle-area permanent to a player's security stack
    /// through the normal leave-field replacement window. This is for
    /// effects that initiate a new move to security, not replacement bodies
    /// already handling an in-flight leave event.
    pub fn place_permanent_on_security(
        &mut self,
        player: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        if self
            .game
            .modifiers
            .player_has(self.player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }
        self.game
            .place_permanent_on_security(player, target, position, face_up, self.player)
    }
}
