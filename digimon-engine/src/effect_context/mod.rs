//! EffectContext — the curated API surface for card effect scripts.
//!
//! Card scripts mutate the game through this context (never directly).
//! `EffectContext` wraps `&mut Game` for `process` closures; `EffectReadContext`
//! wraps `&Game` for `condition` closures and tensor-time effect inspection.
//! Both expose the same read-only query surface.
//!
//! **File layout.** Selection-prompt helpers (`select_*`, `play_from_security`,
//! `mark_security_face_up`, plus the private `install_field_selection`
//! shared implementation) live in `selections.rs` — they are numerous and
//! will grow substantially as the gap-closing roadmap adds multi-select,
//! ordered-permutation, cross-player, and budgeted-multi-select primitives.
//! Core mutations (memory, draw, trash, suspend, modifier grants) stay here.

mod selections;

pub use selections::{CountCappedZone, EffectContextSelectorScope};

use crate::card_data::CardData;
use crate::card_source::CardHandle;
use crate::enums::{Expiry, Keyword, ModifierType, PlayerId, PlaySource, StackPosition};
use crate::game::Game;
use crate::modifiers::ModifierEntry;
use crate::permanent::{Permanent, PermanentHandle};
use crate::player::Player;
use crate::rules::Rules;

/// Read-only view of game state for effect condition closures.
///
/// Wraps `&Game` so conditions can be evaluated without a mutable borrow —
/// which is required at tensor-build time (§3.1 / §3.2 parity fixes) to
/// decide whether a conditional DP modifier currently contributes.
pub struct EffectReadContext<'a> {
    pub game: &'a Game,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub player: PlayerId,
}

impl<'a> EffectReadContext<'a> {
    pub fn new(
        game: &'a Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        player: PlayerId,
    ) -> Self {
        Self {
            game,
            source_card,
            source_permanent,
            player,
        }
    }

    pub fn memory(&self) -> i16 {
        self.game.memory
    }

    pub fn turn_count(&self) -> u16 {
        self.game.turn_count
    }

    pub fn rules(&self) -> &Rules {
        &self.game.rules
    }

    pub fn card_data(&self) -> &[CardData] {
        &self.game.card_data
    }

    pub fn player(&self, id: PlayerId) -> &Player {
        self.game.player(id)
    }

    pub fn my_player(&self) -> &Player {
        self.game.player(self.player)
    }

    pub fn opponent_id(&self) -> PlayerId {
        self.game.next_clockwise(self.player)
    }

    pub fn opponent(&self) -> &Player {
        self.game.player(self.opponent_id())
    }

    pub fn opponents(&self) -> Vec<PlayerId> {
        self.game.opponents(self.player)
    }

    pub fn battle_area(&self, id: PlayerId) -> &[Permanent] {
        &self.game.player(id).battle_area
    }

    pub fn hand(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).hand
    }

    pub fn trash(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).trash
    }

    pub fn security_count(&self, id: PlayerId) -> usize {
        self.game.player(id).security.len()
    }

    pub fn source_permanent(&self) -> Option<&Permanent> {
        let h = self.source_permanent?;
        let player = self.game.player(h.player);
        player.battle_area.get(h.index as usize)
    }

    /// Returns `true` if this effect's source card is a Tamer.
    ///
    /// Used by flood-gate discriminators like `CannotGainMemoryExceptFromTamers`
    /// that allow Tamer-sourced effects but block Digimon/Option-sourced ones.
    /// Matches DCGO's `ICardEffect.IsTamerEffect` property.
    pub fn source_is_tamer(&self) -> bool {
        // Fast path: if we know the source permanent, check its top card directly.
        if let Some(h) = self.source_permanent {
            if let Some(perm) = self.game.player(h.player).battle_area.get(h.index as usize) {
                return perm.is_tamer(&self.game.card_data);
            }
        }
        // Slow path: source_permanent is None (e.g. effect from hand/trash/security).
        self.game
            .card_kind_for_handle(self.source_card)
            .map(|k| k == crate::enums::CardKind::Tamer)
            .unwrap_or(false)
    }
}

/// The context passed to every effect's `process` closure.
/// For `condition` closures see `EffectReadContext`.
pub struct EffectContext<'a> {
    pub game: &'a mut Game,
    /// Card whose effect is being resolved.
    pub source_card: CardHandle,
    /// The permanent containing the source card, if applicable.
    pub source_permanent: Option<PermanentHandle>,
    /// Player who controls the source.
    pub player: PlayerId,
    /// Temporary override for `selecting_player` inside `as_selecting_player`
    /// scope methods. `None` at all times except during the body of an
    /// `EffectContextSelectorScope::select_*` call, where it is set to the
    /// desired selector and cleared again before the method returns.
    pub(super) override_selecting_player: Option<PlayerId>,
}

impl<'a> EffectContext<'a> {
    pub fn new(
        game: &'a mut Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        player: PlayerId,
    ) -> Self {
        Self {
            game,
            source_card,
            source_permanent,
            player,
            override_selecting_player: None,
        }
    }

    // ─── Read-only queries ────────────────────────────────────────────

    pub fn memory(&self) -> i16 {
        self.game.memory
    }

    pub fn turn_count(&self) -> u16 {
        self.game.turn_count
    }

    pub fn rules(&self) -> &Rules {
        &self.game.rules
    }

    pub fn card_data(&self) -> &[CardData] {
        &self.game.card_data
    }

    pub fn player(&self, id: PlayerId) -> &Player {
        self.game.player(id)
    }

    pub fn my_player(&self) -> &Player {
        self.game.player(self.player)
    }

    /// First clockwise opponent (sugar for `opponents()[0]`).
    pub fn opponent_id(&self) -> PlayerId {
        self.game.next_clockwise(self.player)
    }

    pub fn opponent(&self) -> &Player {
        self.game.player(self.opponent_id())
    }

    pub fn opponents(&self) -> Vec<PlayerId> {
        self.game.opponents(self.player)
    }

    pub fn battle_area(&self, id: PlayerId) -> &[Permanent] {
        &self.game.player(id).battle_area
    }

    pub fn hand(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).hand
    }

    pub fn trash(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).trash
    }

    pub fn security_count(&self, id: PlayerId) -> usize {
        self.game.player(id).security.len()
    }

    pub fn source_permanent(&self) -> Option<&Permanent> {
        let h = self.source_permanent?;
        let player = self.game.player(h.player);
        player.battle_area.get(h.index as usize)
    }

    /// Returns `true` if this effect's source card is a Tamer.
    ///
    /// Used by flood-gate discriminators like `CannotGainMemoryExceptFromTamers`
    /// that allow Tamer-sourced effects but block Digimon/Option-sourced ones.
    /// Matches DCGO's `ICardEffect.IsTamerEffect` property.
    pub fn source_is_tamer(&self) -> bool {
        // Fast path: if we know the source permanent, check its top card directly.
        if let Some(h) = self.source_permanent {
            if let Some(perm) = self.game.player(h.player).battle_area.get(h.index as usize) {
                return perm.is_tamer(&self.game.card_data);
            }
        }
        // Slow path: source_permanent is None (e.g. effect from hand/trash/security).
        self.game
            .card_kind_for_handle(self.source_card)
            .map(|k| k == crate::enums::CardKind::Tamer)
            .unwrap_or(false)
    }

    /// Reborrow this mut context as a read-only context — for condition
    /// closures, which take `&EffectReadContext`.
    pub fn as_read(&self) -> EffectReadContext<'_> {
        EffectReadContext {
            game: self.game,
            source_card: self.source_card,
            source_permanent: self.source_permanent,
            player: self.player,
        }
    }

    // ─── Memory mutations ─────────────────────────────────────────────

    pub fn gain_memory(&mut self, amount: i16) {
        let target = self.player;
        // Phase 6: CannotGainMemoryByEffect — suppress all memory gains by effect.
        if self.game.modifiers.player_has(target, ModifierType::CannotGainMemoryByEffect) {
            return;
        }
        // Phase 6: CannotGainMemoryExceptFromTamers — only Tamer-sourced gains are
        // allowed; block Digimon/Option-sourced gains.
        if self.game.modifiers.player_has(target, ModifierType::CannotGainMemoryExceptFromTamers)
            && !self.source_is_tamer()
        {
            return;
        }
        self.game.gain_memory(amount);
    }

    pub fn lose_memory(&mut self, amount: i16) {
        let new_memory = self.game.memory - amount;
        self.game.set_memory(new_memory);
    }

    pub fn set_memory(&mut self, value: i16) {
        self.game.set_memory(value);
    }

    // ─── Card draw / trash ────────────────────────────────────────────

    pub fn draw(&mut self, player: PlayerId, count: u8) -> u8 {
        // Phase 6: if the drawing player has CannotDrawByEffect, suppress draw.
        if self.game.modifiers.player_has(player, ModifierType::CannotDrawByEffect) {
            return 0;
        }
        self.game.player_mut(player).draw_many(count)
    }

    /// Trash the top N cards of a player's deck.
    pub fn trash_from_top(&mut self, player: PlayerId, count: u8) -> u8 {
        let p = self.game.player_mut(player);
        let mut trashed = 0;
        for _ in 0..count {
            if let Some(card) = p.deck.pop() {
                p.trash.push(card);
                trashed += 1;
            } else {
                break;
            }
        }
        trashed
    }

    /// Move the top card of `player`'s security stack to their trash.
    /// No-op if the stack is empty. Returns true if a card was moved.
    pub fn trash_top_security(&mut self, player: PlayerId) -> bool {
        let p = self.game.player_mut(player);
        if let Some(card) = p.security.pop() {
            p.trash.push(card);
            true
        } else {
            false
        }
    }

    // ─── Field mutations ──────────────────────────────────────────────

    pub fn delete_permanent(&mut self, target: PermanentHandle) {
        let player = self.game.player_mut(target.player);
        if (target.index as usize) < player.battle_area.len() {
            player.delete_permanent(target.index as usize);
            self.game.modifiers.clear_permanent(target);
            // Phase 6: expire any player-scoped modifiers sourced from this permanent.
            self.game.modifiers.expire_player_on_permanent_leave(target);
        }
    }

    /// Pop up to `amount` cards off `target`'s digivolution stack,
    /// trashing each popped source into the target owner's trash.
    ///
    /// Rules:
    ///   * Never pops the base card — `Permanent` must always retain at
    ///     least one `CardSource`.
    ///   * `stop_at_level = Some(L)` — stop early if popping would leave
    ///     a top whose level is strictly less than `L`. For standard
    ///     De-Digivolve N use `Some(3)` (card text: "You can't trash
    ///     past level 3 cards").
    ///   * `stop_at_level = None` — no level floor; pop until the base.
    ///   * `amount = Some(N)` — cap pops at N.
    ///   * `amount = None` — unbounded (equivalent to `Some(u8::MAX)`).
    ///
    /// Returns the actual number of cards popped.
    pub fn de_digivolve(
        &mut self,
        target: PermanentHandle,
        stop_at_level: Option<u8>,
        amount: Option<u8>,
    ) -> u8 {
        let max = amount.unwrap_or(u8::MAX);
        let mut popped: u8 = 0;

        while popped < max {
            let perm = match self
                .game
                .player(target.player)
                .battle_area
                .get(target.index as usize)
            {
                Some(p) => p,
                None => break,
            };

            if perm.stack_size() <= 1 {
                break;
            }

            let next_top_level = {
                let stack = perm.digivolution_cards();
                let next_top = &stack[stack.len() - 2];
                next_top.level(&self.game.card_data)
            };

            if let (Some(floor), Some(nt_level)) = (stop_at_level, next_top_level) {
                if nt_level < floor {
                    break;
                }
            }

            let owner = target.player;
            let p = self.game.player_mut(owner);
            let stack = &mut p.battle_area[target.index as usize].card_sources;
            debug_assert!(stack.len() >= 2, "stack_size-guard failed");
            let popped_card = stack.pop().expect("stack_size-guarded pop");
            p.trash.push(popped_card);
            popped += 1;
        }

        popped
    }

    /// Materialize a token on `controller`'s battle area.
    ///
    /// Looks up `token_name` in `game.token_registry`, synthesizes a
    /// `CardSource` with `is_token = true`, wraps it in a `Permanent`, and
    /// pushes onto `controller.battle_area`. No play cost, no OnPlay
    /// observer fan-out (tokens enter via effect, not via `play_from_hand`).
    ///
    /// Returns the spawned permanent's handle, or `None` if the token name
    /// is unknown or the field is full.
    pub fn play_token(
        &mut self,
        controller: crate::enums::PlayerId,
        token_name: &str,
    ) -> Option<crate::permanent::PermanentHandle> {
        use crate::card_source::CardSource;
        use crate::permanent::{Permanent, PermanentHandle};

        let def = self.game.token_registry.get(token_name)?;
        let target_card_id = def.card_id.clone();
        let data_index = self
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == target_card_id)?;
        debug_assert_eq!(
            self.game.card_data[data_index].card_kind,
            crate::enums::CardKind::Token,
            "token_registry entry must map to a CardKind::Token CardData row"
        );

        let slots = self.game.rules.field_slots as usize;
        if self.game.player(controller).battle_area.len() >= slots {
            return None;
        }

        let card_index = self.game.next_card_index();
        let mut card = CardSource::new_token(data_index, controller, card_index);
        card.card_index = card_index;
        let turn = self.game.turn_count;
        let perm = Permanent::new(card, turn);

        let player = self.game.player_mut(controller);
        player.battle_area.push(perm);
        let idx = player.battle_area.len() - 1;
        Some(PermanentHandle {
            player: controller,
            index: idx as u8,
        })
    }

    /// Suspend a permanent and fire `OnSuspend` observers.
    /// Delegates to `Game::suspend` — the canonical single-target chokepoint.
    pub fn suspend(&mut self, target: PermanentHandle) {
        self.game.suspend(target);
    }

    /// Unsuspend a permanent and fire `OnUnsuspend` observers.
    /// Delegates to `Game::unsuspend` — the canonical single-target chokepoint.
    pub fn unsuspend(&mut self, target: PermanentHandle) {
        self.game.unsuspend(target);
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

    /// Snapshot of the current reveal pool. Scripts inspect this to decide
    /// follow-up moves.
    pub fn revealed(&self) -> &[crate::card_source::CardSource] {
        &self.game.revealed_cards
    }

    /// Trash a specific hand card by index.
    pub fn trash_from_hand_by_index(
        &mut self,
        player: PlayerId,
        hand_index: usize,
    ) -> Option<crate::card_source::CardHandle> {
        self.game.trash_from_hand_by_index(player, hand_index)
    }

    /// Move a specific revealed card into `player`'s hand.
    pub fn add_to_hand_from_reveal(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        self.game.add_to_hand_from_reveal(player, card)
    }

    /// Move a specific revealed card into `player`'s trash.
    pub fn trash_from_reveal(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        self.game.trash_from_reveal(player, card)
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
    pub fn place_remainder_on_deck(
        &mut self,
        player: PlayerId,
        position: StackPosition,
    ) {
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

    /// Play a card from `player`'s hand at `hand_index`, deducting memory
    /// according to `cost_delta`. OnPlay effects fire.
    ///
    /// Returns the `PermanentHandle` of the new field permanent, or `None`
    /// if the hand index is invalid, the battle area is full, or memory is
    /// insufficient.
    pub fn play_from_hand_with_cost(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
    ) -> Option<PermanentHandle> {
        let field_index =
            self.game
                .play_from_hand_with_cost(player, hand_index, cost_delta, PlaySource::ByEffect)?;
        Some(PermanentHandle {
            player,
            index: field_index as u8,
        })
    }

    /// Play a card from `player`'s trash at `trash_index`, deducting memory
    /// according to `cost_delta`. OnPlay effects fire.
    pub fn play_from_trash_with_cost(
        &mut self,
        player: PlayerId,
        trash_index: usize,
        cost_delta: crate::enums::CostDelta,
    ) -> Option<PermanentHandle> {
        let field_index = self
            .game
            .play_from_trash_with_cost(player, trash_index, cost_delta, PlaySource::ByEffect)?;
        Some(PermanentHandle {
            player,
            index: field_index as u8,
        })
    }

    /// Insert a card at the bottom of `target`'s digivolution stack. See
    /// `Game::place_as_bottom_source`.
    pub fn place_as_bottom_source(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
    ) -> bool {
        self.game.place_as_bottom_source(source, target)
    }

    /// Bounce a permanent to its owner's hand. See `Game::return_to_hand`.
    pub fn return_to_hand(
        &mut self,
        target: PermanentHandle,
    ) -> Option<crate::card_source::CardHandle> {
        self.game.return_to_hand(target)
    }

    /// Return a permanent's top card to its owner's deck. See `Game::return_to_deck`.
    pub fn return_to_deck(
        &mut self,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        self.game.return_to_deck(target, position)
    }

    /// Digivolve a card from `player`'s hand at `hand_index` onto `target`
    /// by effect. Bypasses the Main-phase check; optionally ignores color
    /// requirements (`ignore_color=true`); pays memory via `cost_delta`.
    ///
    /// Returns `true` on success. See `Game::effect_initiated_digivolve`.
    pub fn effect_initiated_digivolve(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
    ) -> bool {
        self.game.effect_initiated_digivolve(
            player,
            hand_index,
            target,
            cost_delta,
            ignore_color,
            PlaySource::ByEffect,
        )
    }

    // ─── Modifier registration ────────────────────────────────────────

    pub fn add_dp_modifier(&mut self, target: PermanentHandle, value: i32, expiry: Expiry) {
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(
                ModifierType::ChangeDp,
                value,
                expiry,
                self.player,
            ),
        );
    }

    pub fn add_modifier(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
    ) {
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(
                modifier,
                value,
                expiry,
                self.player,
            ),
        );
    }

    pub fn grant_keyword(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        expiry: Expiry,
    ) {
        self.game
            .modifiers
            .grant_keyword(target, keyword, expiry, self.player);
    }

    // ─── Breeding-area mutations ──────────────────────────────────────

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
        if self.game.modifiers.player_has(self.player, ModifierType::CannotAddSecurityByEffect) {
            return false;
        }
        self.game.place_on_security(player, source, position, face_up)
    }

    /// Move the top of `player`'s digitama deck into the breeding area.
    ///
    /// Returns `true` if a hatch occurred — i.e. the breeding slot was
    /// empty and the digitama deck had at least one card.  Returns `false`
    /// if the breeding slot was already occupied or the digitama deck was
    /// empty.
    ///
    /// No `PermanentHandle` is returned: breeding-area permanents are
    /// addressed separately from battle-area permanents and do not use
    /// the same handle type.
    pub fn hatch(&mut self, player: PlayerId) -> bool {
        self.game.hatch(player)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card_data::CardData;
    use crate::rules::Rules;
    use std::collections::HashMap;

    fn min_db() -> HashMap<String, CardData> {
        let json = r#"{
            "BT1-001": {
                "card_id": "BT1-001", "card_name_eng": "Koromon",
                "card_effect_class_name": "BT1_001", "play_cost": 0, "dp": -1,
                "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
                "type_eng": [], "form_eng": [], "attribute_eng": [],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            }
        }"#;
        CardData::load_from_str(json).unwrap()
    }

    #[test]
    fn memory_mutations() {
        let db = min_db();
        let deck = vec!["BT1-001".to_string(); 10];
        let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(1)).unwrap();
        let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, 0);
        ctx.set_memory(0);
        ctx.gain_memory(3);
        assert_eq!(ctx.memory(), 3);
        ctx.lose_memory(2);
        assert_eq!(ctx.memory(), 1);
    }

    #[test]
    fn play_token_unknown_name_returns_none() {
        let db = min_db();
        let deck = vec!["BT1-001".to_string(); 10];
        let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(1)).unwrap();
        let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, 0);
        assert!(ctx.play_token(0, "no-such-token-lol").is_none());
    }
}
