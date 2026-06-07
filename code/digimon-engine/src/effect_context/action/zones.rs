//! Draw / reveal / shuffle / hand-and-deck movement on `EffectContext` — extracted by mechanic.

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
    pub fn draw(&mut self, player: PlayerId, count: u8) -> u8 {
        use crate::enums::EffectTiming;
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Phase 6: if the drawing player has CannotDrawByEffect, suppress draw.
        // The flood gate fires FIRST (preserves Phase 6 semantics); if blocked,
        // no replacement window opens.
        if self
            .game
            .modifiers
            .player_has(player, ModifierType::CannotDrawByEffect)
        {
            return 0;
        }

        // Phase 7 Task 4: fire WhenWouldDraw once per draw call (not once
        // per card). Subject is the drawing player; no original_destination.
        let cause = self.game.infer_effect_cause(player);
        let subject = ReplacementSubject::Player(player);
        let outcome = self
            .game
            .try_replace(EffectTiming::WhenWouldDraw, subject, cause, None);
        if self.game.pending_selection.is_some() {
            return 0;
        }
        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return 0;
            }
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(
                    false,
                    "Redirected not meaningful for WhenWouldDraw (player-scoped)"
                );
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(false, "Substituted not supported for WhenWouldDraw v1");
            }
        }

        // Opaque-aware: replace draw_many with N calls to
        // draw_one_for_player so opaque opponents pull from RevealSource.
        // Errors fall through as "draw stopped" — same semantic as the
        // standard-mode draw_many returning fewer cards than requested.
        let mut drawn: u8 = 0;
        for _ in 0..count {
            match self.game.draw_one_for_player(player) {
                Ok(true) => drawn += 1,
                Ok(false) => break, // deck-out
                Err(e) => {
                    eprintln!(
                        "[opaque-deck] effect-driven draw error for player {}: {}",
                        player, e
                    );
                    break;
                }
            }
        }
        if drawn > 0 {
            self.game.mark_until_condition_dirty();
            self.game.reevaluate_until_condition_modifiers_if_dirty();
        }
        drawn
    }

    /// Move a specific card from `player`'s deck to their hand.
    pub fn add_to_hand_from_deck(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        self.game.add_to_hand_from_deck(player, card)
    }

    /// Move a specific card from `player`'s trash to their hand.
    pub fn add_to_hand_from_trash(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        self.game.add_to_hand_from_trash(player, card)
    }

    /// Reveal up to `n` cards from the top of `player`'s deck. See
    /// `Game::reveal_top_deck`.
    pub fn reveal_top_deck(
        &mut self,
        player: PlayerId,
        n: u8,
    ) -> Vec<crate::card_source::CardHandle> {
        self.game.reveal_top_deck(player, n)
    }

    pub fn reveal_top_digitama(
        &mut self,
        player: PlayerId,
        n: u8,
    ) -> Vec<crate::card_source::CardHandle> {
        self.game.reveal_top_digitama(player, n)
    }

    /// Move a specific revealed card into `player`'s hand.
    pub fn add_to_hand_from_reveal(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        self.game.add_to_hand_from_reveal(player, card)
    }

    /// Move a specific revealed card back to `player`'s deck at `position`.
    pub fn return_to_deck_from_reveal(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        self.game.return_to_deck_from_reveal(player, card, position)
    }

    /// Place all cards currently in `game.revealed_cards` back onto `player`'s
    /// deck at `position`, in a player-chosen order.
    ///
    /// **Contract**: `ordered_vec[0]` is drawn first among the placed cards.
    ///
    /// - **Empty pool** → silent no-op; no `PendingSelection` installed.
    /// - **1 card** → still installs a 1-choice `OrderedPermutation` so the RL
    ///   agent sees the (trivial) ordering decision (no-approximations policy).
    /// - **N cards** → installs `select_ordered_permutation` over the remainder;
    ///   the callback places cards at `position` using the correct iteration
    ///   direction so `ordered_vec[0]` ends up drawn first:
    ///   - `Top`:    iterate `rev()`, push each (`deck.push`) — last pushed lands
    ///               at Vec-end (= deck top = drawn first).
    ///   - `Bottom`: iterate forward, insert each at index 0 (`deck.insert(0)`) —
    ///               each subsequent insert pushes the previous card deeper; final
    ///               state has `ordered_vec[0]` at the highest index among the
    ///               placed group (closest to top of the bottom-placed set).
    ///   - `Random`: iterate forward, call `return_to_deck_from_reveal(Random)`
    ///               for each — placement order is semantically irrelevant but the
    ///               permutation selection is still surfaced to the RL agent.
    pub fn place_remainder_on_deck(&mut self, player: PlayerId, position: StackPosition) {
        // Snapshot handles of every card currently in the reveal pool.
        let remainder: Vec<CardHandle> = self
            .game
            .revealed_cards
            .iter()
            .map(|cs| cs.handle())
            .collect();

        // Empty pool → silent no-op.
        if remainder.is_empty() {
            return;
        }

        debug_assert!(
            remainder.len() <= 10,
            "place_remainder_on_deck: reveal pool has {} cards; select_ordered_permutation is capped at 10",
            remainder.len()
        );

        self.select_ordered_permutation(
            remainder,
            "Place remaining cards on deck in any order",
            move |ctx, ordered_vec| {
                match position {
                    StackPosition::Top => {
                        // Reverse-iterate: last item is pushed first, so ordered_vec[0]
                        // is pushed last → lands at Vec-end (deck top) → drawn first.
                        for handle in ordered_vec.iter().rev() {
                            let placed = ctx.game.return_to_deck_from_reveal(player, *handle, StackPosition::Top);
                            debug_assert!(placed, "place_remainder_on_deck: handle {:?} not found in revealed_cards at placement time", handle);
                        }
                    }
                    StackPosition::Bottom => {
                        // Forward-iterate with insert(0): ordered_vec[0] is inserted
                        // first at index 0; each subsequent insert pushes it one step
                        // further from index 0. Final: ordered_vec[0] is at the highest
                        // index among the placed group (closest to top within the
                        // bottom-placed set) → drawn first among them.
                        for handle in ordered_vec.iter() {
                            let placed = ctx.game.return_to_deck_from_reveal(player, *handle, StackPosition::Bottom);
                            debug_assert!(placed, "place_remainder_on_deck: handle {:?} not found in revealed_cards at placement time", handle);
                        }
                    }
                    StackPosition::Random => {
                        // Each card is placed at a random position. The permutation
                        // selection is still surfaced — the ordering is strategically
                        // irrelevant but the RL action space must see it (§17).
                        for handle in ordered_vec.iter() {
                            let placed = ctx.game.return_to_deck_from_reveal(player, *handle, StackPosition::Random);
                            debug_assert!(placed, "place_remainder_on_deck: handle {:?} not found in revealed_cards at placement time", handle);
                        }
                    }
                }
            },
        );
    }

    /// Shuffle `player`'s deck. Pair with `add_to_hand_from_deck` for
    /// "search and shuffle" effects.
    pub fn shuffle_deck(&mut self, player: PlayerId) {
        self.game.shuffle_deck(player);
    }

    /// Move `player`'s real breeding permanent into the battle area by effect.
    pub fn move_from_breeding_by_effect(&mut self, player: PlayerId) -> bool {
        self.game.move_from_breeding_by_effect(player)
    }

    /// Bounce a permanent to its owner's hand. See `Game::return_to_hand`.
    pub fn return_to_hand(
        &mut self,
        target: PermanentHandle,
    ) -> Option<crate::card_source::CardHandle> {
        if !self.can_affect_permanent(target) {
            return None;
        }
        self.game.return_to_hand(target)
    }

    /// Return the resolving effect's own permanent (`self.source_permanent`)
    /// to its owner's hand. Sugar over `return_to_hand` for printed text like
    /// "return this Digimon to your hand". Returns the moved card's handle
    /// on success, `None` if the effect has no source permanent (e.g. an
    /// Option-card effect or a rule-source effect) or if the bounce is
    /// blocked by `CannotBeReturnedToHand` / `CannotBeAffected` modifiers.
    ///
    /// Owner-routed: `Game::return_to_hand` reads the moved card's `owner`
    /// field, so a permanent owned by player A but currently controlled by
    /// player B (e.g. via a control-transfer effect) returns to A's hand.
    pub fn bounce_self(&mut self) -> Option<crate::card_source::CardHandle> {
        let handle = self.source_permanent?;
        self.return_to_hand(handle)
    }

    /// Return a permanent's top card to its owner's deck. See `Game::return_to_deck`.
    pub fn return_to_deck(
        &mut self,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        if !self.can_affect_permanent(target) {
            return false;
        }
        self.game.return_to_deck(target, position)
    }

    /// Return a permanent's full stack to its owner's deck. See
    /// `Game::return_stack_to_deck`.
    pub fn return_stack_to_deck(
        &mut self,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        if !self.can_affect_permanent(target) {
            return false;
        }
        self.game.return_stack_to_deck(target, position)
    }

    /// Move a SELECTED LIST of cards out of `player`'s trash to the bottom of
    /// the deck, in the given order (the first handle ends up deepest). Unlike
    /// `return_all_trash_to_deck_bottom`, this targets exactly the cards in
    /// `cards` (e.g. a `select_count_capped_multi` pick set) and leaves the
    /// rest of the trash untouched. Returns the handles actually moved (a
    /// handle not found in the trash is silently skipped).
    /// G-ZONE-TRASH-TO-DECK.
    pub fn return_trash_cards_to_deck_bottom(
        &mut self,
        player: PlayerId,
        cards: &[CardHandle],
    ) -> Vec<CardHandle> {
        let mut moved = Vec::with_capacity(cards.len());
        for &handle in cards {
            let Some(pos) = self
                .game
                .player(player)
                .trash
                .iter()
                .position(|c| c.handle() == handle)
            else {
                continue;
            };
            let card = self.game.player_mut(player).trash.remove(pos);
            let owner = card.owner;
            self.game.player_mut(owner).deck.insert(0, card);
            moved.push(handle);
        }
        moved
    }

    /// Move a SELECTED LIST of cards out of `player`'s trash to the **top** of
    /// the deck — the position `draw` pops first. The deck-bottom sibling is
    /// `return_trash_cards_to_deck_bottom`; by deck convention bottom is index
    /// 0 and top is the `Vec` end. The first handle in `cards` ends up on top
    /// (drawn first), the rest sit just beneath it, so selection order becomes
    /// draw order. Returns the handles actually moved, in selection order (a
    /// handle not found in the trash is silently skipped).
    /// G-ZONE-SELECTED-TRASH-TO-DECK-TOP.
    pub fn return_trash_cards_to_deck_top(
        &mut self,
        player: PlayerId,
        cards: &[CardHandle],
    ) -> Vec<CardHandle> {
        let mut moved = Vec::with_capacity(cards.len());
        // Iterate in reverse and `push`: the first handle in `cards` is pushed
        // last, landing at the `Vec` end (= deck top, drawn first).
        for &handle in cards.iter().rev() {
            let Some(pos) = self
                .game
                .player(player)
                .trash
                .iter()
                .position(|c| c.handle() == handle)
            else {
                continue;
            };
            let card = self.game.player_mut(player).trash.remove(pos);
            let owner = card.owner;
            self.game.player_mut(owner).deck.push(card);
            moved.push(handle);
        }
        // `moved` was built in reverse; restore selection order for callers.
        moved.reverse();
        moved
    }

    /// Move a single SELECTED card out of `player`'s trash to the TOP of the
    /// deck (the position drawn from first). The card returns to its OWNER's
    /// deck — `player` only identifies whose trash zone currently holds it.
    /// Selected-trash analog of `return_trash_cards_to_deck_bottom`, but
    /// single-card and deck-TOP. Returns true if the card was found and moved.
    /// A handle not present in `player`'s trash is a silent no-op.
    /// G-ZONE-SELECTED-TRASH-TO-DECK-TOP.
    pub fn move_trash_card_to_deck_top(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(pos) = self
            .game
            .player(player)
            .trash
            .iter()
            .position(|c| c.handle() == card)
        else {
            return false;
        };
        let removed = self.game.player_mut(player).trash.remove(pos);
        let owner = removed.owner;
        // Deck top = Vec end (drawn first) per engine convention.
        self.game.player_mut(owner).deck.push(removed);
        true
    }

    /// Recover up to `count` cards from `player`'s deck to the top of security.
    pub fn recover_from_deck(&mut self, player: PlayerId, count: u8) -> u8 {
        let mut recovered = 0;
        for _ in 0..count {
            if self.place_on_security(
                player,
                crate::enums::CardSourceRef::DeckTop(player),
                crate::enums::StackPosition::Top,
                false,
            ) {
                recovered += 1;
            } else {
                break;
            }
        }
        recovered
    }
}
