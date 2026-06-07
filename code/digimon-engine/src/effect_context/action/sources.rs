//! Digivolution-source / under-tamer placement on `EffectContext` — extracted by mechanic.

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
    pub(crate) fn remove_source_option_from_controller_zones(
        &mut self,
    ) -> Option<crate::card_source::CardSource> {
        if !matches!(
            self.game.card_kind_for_handle(self.source_card),
            Some(CardKind::Option)
        ) {
            return None;
        }

        if let Some(pos) = self
            .game
            .player(self.player)
            .hand
            .iter()
            .position(|card| card.handle() == self.source_card)
        {
            return Some(self.game.player_mut(self.player).hand.remove(pos));
        }

        let pos = self
            .game
            .player(self.player)
            .trash
            .iter()
            .position(|card| card.handle() == self.source_card)?;
        Some(self.game.player_mut(self.player).trash.remove(pos))
    }

    pub(crate) fn remove_source_card_from_permanent(
        &mut self,
        source_perm: PermanentHandle,
        source_card: CardHandle,
    ) -> Option<crate::card_source::CardSource> {
        let permanent = self
            .game
            .player_mut(source_perm.player)
            .battle_area
            .get_mut(source_perm.index as usize)?;
        let pos = permanent
            .card_sources
            .iter()
            .position(|card| card.handle() == source_card)?;
        if pos + 1 == permanent.card_sources.len() {
            return None;
        }
        Some(permanent.card_sources.remove(pos))
    }

    pub fn place_sourceless_permanent_bottom_security_and_cancel_current_replacement(
        &mut self,
        player: PlayerId,
        target: PermanentHandle,
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
            .place_sourceless_permanent_on_security_bottom(player, target, self.player)
        {
            if self.game.parked_replacement.is_some() {
                self.cancel_current_replacement();
            }
            true
        } else {
            false
        }
    }

    /// Insert a card at the bottom of `target`'s digivolution stack. See
    /// `Game::place_as_bottom_source`.
    ///
    /// **Phase B Progress gate — intentionally omitted.** DCGO's primitive
    /// `Permanent.AddDigivolutionCardsBottom` does not consult
    /// `CanNotBeAffected` on the receiving permanent, and DCGO scripts that
    /// place a source under an opponent's Digimon (e.g. EX10-059) do not
    /// gate the target on Progress either. Adding a card under a stack is
    /// not "affecting" the target in DCGO's semantics — the TopCard's
    /// status is unchanged. Gating here would over-restrict relative to
    /// DCGO and break parity with cards that intentionally route a source
    /// under an opponent's Progress attacker.
    ///
    /// `face_down: true` marks the inserted digivolution source face-down
    /// (Tamer-stash callers); `false` is ordinary face-up placement. Note
    /// that `face_down` is NOT honored for `CardSourceRef::Security` sources
    /// — those are always placed face-up (see the `Game::place_as_bottom_source`
    /// doc caveat).
    pub fn place_as_bottom_source(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        face_down: bool,
    ) -> bool {
        self.game
            .place_as_bottom_source_observed(source, target, self.player, face_down)
    }

    pub fn place_hand_card_under_tamer(
        &mut self,
        hand_index: usize,
        tamer: PermanentHandle,
        face_down: bool,
    ) -> Option<CardHandle> {
        if !self.own_tamer_target(tamer) {
            return None;
        }
        let card = self.game.player(self.player).hand.get(hand_index)?.handle();
        let moved = self.place_as_bottom_source(
            crate::enums::CardSourceRef::Hand(self.player, hand_index),
            tamer,
            face_down,
        );
        moved.then_some(card)
    }

    pub fn place_hand_card_under_source_tamer(
        &mut self,
        hand_index: usize,
        face_down: bool,
    ) -> Option<CardHandle> {
        let tamer = self.source_permanent?;
        self.place_hand_card_under_tamer(hand_index, tamer, face_down)
    }

    pub fn place_trash_card_under_tamer(
        &mut self,
        trash_index: usize,
        tamer: PermanentHandle,
        face_down: bool,
    ) -> Option<CardHandle> {
        if !self.own_tamer_target(tamer) {
            return None;
        }
        let card = self
            .game
            .player(self.player)
            .trash
            .get(trash_index)?
            .handle();
        let moved = self.place_as_bottom_source(
            crate::enums::CardSourceRef::Trash(self.player, trash_index),
            tamer,
            face_down,
        );
        moved.then_some(card)
    }

    pub fn place_trash_card_under_source_tamer(
        &mut self,
        trash_index: usize,
        face_down: bool,
    ) -> Option<CardHandle> {
        let tamer = self.source_permanent?;
        self.place_trash_card_under_tamer(trash_index, tamer, face_down)
    }

    pub fn place_union_card_under_tamer(
        &mut self,
        card: CardHandle,
        origin: crate::selection::UnionZoneOrigin,
        tamer: PermanentHandle,
        face_down: bool,
    ) -> Option<CardHandle> {
        match origin {
            crate::selection::UnionZoneOrigin::Hand => {
                let hand_index = self
                    .game
                    .player(self.player)
                    .hand
                    .iter()
                    .position(|candidate| candidate.handle() == card)?;
                self.place_hand_card_under_tamer(hand_index, tamer, face_down)
            }
            crate::selection::UnionZoneOrigin::Trash => {
                let trash_index = self
                    .game
                    .player(self.player)
                    .trash
                    .iter()
                    .position(|candidate| candidate.handle() == card)?;
                self.place_trash_card_under_tamer(trash_index, tamer, face_down)
            }
            crate::selection::UnionZoneOrigin::Material { .. } => None,
        }
    }

    pub fn place_union_card_under_source_tamer(
        &mut self,
        card: CardHandle,
        origin: crate::selection::UnionZoneOrigin,
        face_down: bool,
    ) -> Option<CardHandle> {
        let tamer = self.source_permanent?;
        self.place_union_card_under_tamer(card, origin, tamer, face_down)
    }

    pub fn place_permanent_as_bottom_sources(
        &mut self,
        source: PermanentHandle,
        target: PermanentHandle,
    ) -> bool {
        self.game.place_permanent_as_bottom_sources(source, target)
    }

    pub fn move_all_matching_sources_under_tamer<F>(
        &mut self,
        source: PermanentHandle,
        tamer: PermanentHandle,
        filter: F,
    ) -> usize
    where
        F: Fn(&Game, CardHandle) -> bool,
    {
        if source.player != self.player || source == tamer || !self.own_tamer_target(tamer) {
            return 0;
        }

        let source_indices = {
            let Some(permanent) = self
                .game
                .player(source.player)
                .battle_area
                .get(source.index as usize)
            else {
                return 0;
            };
            let source_count = permanent.card_sources.len().saturating_sub(1);
            (0..source_count)
                .filter(|idx| filter(self.game, permanent.card_sources[*idx].handle()))
                .collect::<Vec<_>>()
        };
        if source_indices.is_empty() {
            return 0;
        }

        let mut moved = Vec::with_capacity(source_indices.len());
        {
            let source_perm =
                &mut self.game.player_mut(source.player).battle_area[source.index as usize];
            for &idx in source_indices.iter().rev() {
                moved.push(source_perm.card_sources.remove(idx));
            }
        }
        moved.reverse();
        let moved_count = moved.len();

        let Some(target_perm) = self
            .game
            .player_mut(tamer.player)
            .battle_area
            .get_mut(tamer.index as usize)
        else {
            return 0;
        };
        target_perm.card_sources.splice(0..0, moved);
        moved_count
    }

    pub fn move_selected_sources_under_tamer(
        &mut self,
        selected: Vec<SourceSelectionRef>,
        tamer: PermanentHandle,
    ) -> usize {
        if !self.own_tamer_target(tamer) {
            return 0;
        }

        let mut moved = Vec::with_capacity(selected.len());
        for source_ref in selected {
            if source_ref.permanent.player != self.player {
                continue;
            }
            let Some(permanent) = self
                .game
                .player_mut(source_ref.permanent.player)
                .battle_area
                .get_mut(source_ref.permanent.index as usize)
            else {
                continue;
            };
            let Some(position) = permanent
                .card_sources
                .iter()
                .position(|candidate| candidate.handle() == source_ref.card)
            else {
                continue;
            };
            if position + 1 >= permanent.card_sources.len() {
                continue;
            }
            moved.push(permanent.card_sources.remove(position));
        }

        let moved_count = moved.len();
        if moved_count == 0 {
            return 0;
        }
        let Some(target_perm) = self
            .game
            .player_mut(tamer.player)
            .battle_area
            .get_mut(tamer.index as usize)
        else {
            return 0;
        };
        target_perm.card_sources.splice(0..0, moved);
        moved_count
    }

    /// Move `target`'s **top stacked card** (the card immediately beneath its
    /// active top card — `card_sources[len - 2]`) to the bottom of its own
    /// digivolution stack. Returns `false` if the target has no stacked card
    /// beneath the top (i.e. `card_sources.len() < 2`), in which case the
    /// printed-cost cannot be paid; callers should gate the activating step
    /// with a `materials_count_gte: 1` (or equivalent) predicate.
    ///
    /// Closes `G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM` for the deterministic
    /// "top stacked card → bottom" cost shape that DCGO encodes as
    /// `card.PermanentOfThisCard().TopCard` followed by
    /// `AddDigivolutionCardsBottom`. Per the no-approximations policy this
    /// path does NOT surface a player choice over which source moves — the
    /// printed text identifies a singular "top" source.
    pub fn place_top_source_as_bottom(&mut self, target: PermanentHandle) -> bool {
        let stack_size = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .map(|p| p.card_sources.len())
            .unwrap_or(0);
        if stack_size < 2 {
            return false;
        }
        let top_stacked_idx = stack_size - 2;
        self.place_as_bottom_source(
            crate::enums::CardSourceRef::Material(target, top_stacked_idx),
            target,
            false,
        )
    }

    /// Move `card` from whatever zone it currently occupies to the **bottom**
    /// of `target`'s digivolution stack (`card_sources[0]`).
    ///
    /// The card is located by scanning all zones of all players in the
    /// following priority order:
    ///   1. Each player's `hand`
    ///   2. Each player's `trash`
    ///   3. Each player's `deck`
    ///   4. Each player's `security`
    ///   5. Each player's `battle_area` card stacks (all permanents)
    ///   6. Each player's `breeding_area` card stack
    ///   7. The game-level `revealed_cards` transient pool
    ///
    /// This covers every realistic source for `<Save>` (self just moved to
    /// trash during deletion) and `<Material Save N>` (cards are in another
    /// permanent's `card_sources`). Opponent deck / security are included for
    /// completeness but Save/MaterialSave callers will never route through
    /// them in normal play. `revealed_cards` is included to handle cards that
    /// are mid-reveal when a Save effect resolves.
    ///
    /// `face_down: true` marks the placed source face-down; `false` is
    /// ordinary face-up placement.
    ///
    /// # Panics
    ///
    /// Panics if `card` cannot be located in any zone — this represents a
    /// programming error (passing an invalid or already-moved handle).
    ///
    /// Used by: `<Save>`, `<Material Save N>`.
    pub fn place_card_under_permanent_bottom(
        &mut self,
        card: CardHandle,
        target: PermanentHandle,
        face_down: bool,
    ) {
        let mut taken = self
            .game
            .remove_card_from_any_zone(card)
            .unwrap_or_else(|| {
                panic!(
                    "place_card_under_permanent_bottom: card {:?} not found in any zone",
                    card
                )
            });

        let target_player = self.game.player_mut(target.player);
        if (target.index as usize) >= target_player.battle_area.len() {
            // Safe-fail: target permanent no longer exists; route to its
            // controller's trash rather than dropping the card on the floor.
            // Trashed cards are not face-down sources, so `face_down` is not
            // applied on this path.
            target_player.trash.push(taken);
            return;
        }
        taken.face_down = face_down;
        target_player.battle_area[target.index as usize].push_under(taken);
    }

    pub fn place_cards_under_tamer_bottom_in_order(
        &mut self,
        cards: Vec<CardHandle>,
        tamer: PermanentHandle,
        face_down: bool,
    ) -> usize {
        if !self.own_tamer_target(tamer) {
            return 0;
        }

        let mut moved = Vec::with_capacity(cards.len());
        for card in cards {
            if let Some(mut taken) = self.game.remove_card_from_any_zone(card) {
                taken.face_down = face_down;
                moved.push(taken);
            }
        }
        let moved_count = moved.len();
        if moved_count == 0 {
            return 0;
        }

        let Some(target_perm) = self
            .game
            .player_mut(tamer.player)
            .battle_area
            .get_mut(tamer.index as usize)
        else {
            return 0;
        };
        target_perm.card_sources.splice(0..0, moved);
        moved_count
    }

    /// Place `tamer`'s top card at the bottom of `digimon`'s digivolution
    /// stack, replicating DCGO `MindLink.cs:71-79`:
    /// `IPlacePermanentToDigivolutionCards(new[] { tamer, selectedDigimon })`.
    ///
    /// The Tamer permanent itself is removed from battle area; its top
    /// CardSource becomes the new bottom of the target Digimon's stack.
    /// The face-down flag is NOT set (MindLink places face-up).
    ///
    /// ## DCGO parity (CardController.cs:3011-3061)
    ///
    /// `IPlacePermanentToDigivolutionCards.PlacePermanentToDigivolutionCards`
    /// takes `cardSource = DigivolutionPermanent.TopCard` (just the top),
    /// calls `DiscardEvoRoots()` on the rest of the stack (sending those
    /// sources to trash), removes the source permanent from field, then
    /// calls `getDigivolutionPermanent.AddDigivolutionCardsBottom(...)` to
    /// tuck the top card under the target.
    ///
    /// ## Index-stability strategy
    ///
    /// Removing `tamer` from `battle_area` shifts every higher-indexed
    /// permanent down by one. To keep the `digimon` handle valid we
    /// resolve `digimon`'s slot AFTER the removal (adjusting if it was
    /// past `tamer.index`). Both must share the same controller (DCGO
    /// `IsPermanentExistsOnOwnerBattleArea`); we assert that, since the
    /// MindLink filter already enforces it.
    ///
    /// ## Modifier cleanup
    ///
    /// `Game.modifiers.clear_permanent(tamer)` and
    /// `expire_player_on_permanent_leave(tamer)` are invoked before the
    /// removal, mirroring `Game::return_to_deck`'s and the deletion
    /// finalize's cleanup pattern. Player-scoped modifiers sourced from
    /// the Tamer (e.g., printed memory-gain effects) expire here.
    ///
    /// ## Source-stack handling
    ///
    /// Sources below the top of the Tamer's stack are routed to trash
    /// (mirroring DCGO `DiscardEvoRoots` — sources can't ride the move).
    /// Each such trash fires `OnDigivolutionCardTrashed` per player and
    /// drains the queue, mirroring `Game::return_to_deck`'s leave-field
    /// path. Linked cards on the Tamer (Tamers can host Option cards via
    /// `<Linked>`) are likewise trashed; if any were present, a single
    /// `OnLinkedCardTrashed` is fired per player and drained, mirroring
    /// `Game::finalize_permanent_deletion`'s linked-cascade pattern.
    ///
    /// Used by: `<Mind Link>` keyword auto-install (Phase F Task 5).
    pub fn attach_tamer_to_digimon(&mut self, tamer: PermanentHandle, digimon: PermanentHandle) {
        // Validate: shared controller (MindLink targets own permanents).
        // Promoted to `assert_eq!` so the precondition is enforced in
        // release builds too — this is a public primitive, and a release
        // caller violating it would silently misroute trash and target
        // lookup rather than panicking.
        assert_eq!(
            tamer.player, digimon.player,
            "attach_tamer_to_digimon: tamer and target must share a controller"
        );
        let controller = tamer.player;

        // Bounds check the tamer slot.
        let tamer_idx = tamer.index as usize;
        if tamer_idx >= self.game.player(controller).battle_area.len() {
            return;
        }

        // Cleanup tamer-scoped modifiers BEFORE removal (modifier registry
        // is keyed on PermanentHandle, which becomes invalid after the
        // index shift caused by `Vec::remove`).
        self.game.modifiers.clear_permanent(tamer);
        self.game.modifiers.expire_player_on_permanent_leave(tamer);

        // Remove the Tamer permanent from battle area. This shifts indices
        // for all higher-indexed permanents on the same player down by 1.
        let mut tamer_perm = self
            .game
            .player_mut(controller)
            .battle_area
            .remove(tamer_idx);

        // Trash any sources below the top (DCGO DiscardEvoRoots). The top
        // is the card that rides under the target.
        let Some(top) = tamer_perm.card_sources.pop() else {
            return;
        };
        // Below-top sources go to trash. Linked Option cards (Tamers can
        // host them via `<Linked>`) likewise go to trash — they cannot
        // ride the host into another permanent's digivolution stack.
        //
        // Mirror `Game::return_to_deck` (game_actions.rs:1497-1506): fire
        // `OnDigivolutionCardTrashed` per source, per player, draining
        // between each so observers see them one at a time.
        let top_handle = top.handle();
        for source in tamer_perm.card_sources.drain(..) {
            self.game.trash_source_and_fire_with_cause(
                controller,
                tamer,
                source,
                top_handle,
                crate::trigger_context::EventCause::Return,
            );
        }
        // Mirror `Game::finalize_permanent_deletion` (combat.rs:2429-2437):
        // route all linked cards to trash, then fire a single
        // `OnLinkedCardTrashed` per player if any were present.
        let had_linked = !tamer_perm.linked_cards.is_empty();
        for linked in tamer_perm.linked_cards.drain(..) {
            self.game.player_mut(controller).trash.push(linked);
        }
        if had_linked {
            for pid in 0..self.game.players.len() {
                self.game.enqueue_triggered(
                    crate::enums::EffectTiming::OnLinkedCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
            self.game.drain_effect_queue();
        }

        // Resolve the target's NEW slot after the index shift. If the
        // digimon was at a higher index than the removed tamer, its
        // slot dropped by 1; otherwise it's unchanged.
        let digimon_idx = if (digimon.index as usize) > tamer_idx {
            (digimon.index as usize) - 1
        } else {
            digimon.index as usize
        };

        // Bounds check the (possibly shifted) target slot. If the target
        // is gone, route the lifted top card to the controller's trash
        // (safe-fail — same shape as `place_card_under_permanent_bottom`).
        let target_player = self.game.player_mut(controller);
        if digimon_idx >= target_player.battle_area.len() {
            target_player.trash.push(top);
            return;
        }
        // Insert at the bottom of the target's stack. `face_down` stays
        // `false` (MindLink places face-up).
        target_player.battle_area[digimon_idx].push_under(top);
    }

    /// Trash the current top Digimon of `perm` and promote the next-highest
    /// digivolution source to become the new visible top. The remainder of the
    /// stack is preserved intact.
    ///
    /// **Caller MUST gate on `perm.card_sources.len() >= 2` before invoking.**
    /// This primitive `debug_assert!`s that constraint and will panic in debug
    /// builds if violated. Production callers (the ArmorPurge auto-install
    /// body) gate before calling.
    ///
    /// After the call:
    ///   - `perm.card_sources.len()` decreases by 1.
    ///   - The previous `card_sources[last - 1]` entry (previously the
    ///     next-highest source) is now `top_card()`.
    ///   - The trashed top card is appended to `players[controller].trash`.
    ///   - `EffectTiming::OnDigivolutionCardTrashed` is enqueued and drained
    ///     so observers (e.g. Rocks-archetype source-trash listeners) see the
    ///     trashed top. DCGO parity: `ArmorPurge.cs:65-78` —
    ///     `StackSkillInfos(hashtable, EffectTiming.WhenTopCardTrashed)`.
    ///
    /// **Modifier note:** Modifiers in this engine are keyed by
    /// `PermanentHandle` (the full permanent), not by individual
    /// `CardSource`. Therefore, any modifiers currently attached to this
    /// permanent handle remain valid for the new top card — no modifier
    /// cleanup is performed here. This deviates from DCGO
    /// `RemoveDigivolveRootEffect` (which removes inherited effects registered
    /// by the trashed card specifically), but is the correct behavior for this
    /// engine because inherited effects are script-driven, not stored as
    /// `ModifierEntry` values.
    ///
    /// DCGO parity: `ArmorPurge.cs:50-65`
    ///   `RemoveFromAllArea(topCard)` + `AddTrashCard(topCard)` +
    ///   `RemoveDigivolveRootEffect(topCard, _permanent)`.
    ///
    /// Used by: `<Armor Purge>` keyword auto-install (Phase D Task 5).
    pub fn armor_purge_top(&mut self, perm: PermanentHandle) {
        let permanent = self
            .game
            .player_mut(perm.player)
            .battle_area
            .get_mut(perm.index as usize)
            .expect("armor_purge_top: permanent handle is invalid");
        debug_assert!(
            permanent.card_sources.len() >= 2,
            "armor_purge_top requires >= 1 source under the top card (stack len = {})",
            permanent.card_sources.len()
        );
        let top = permanent
            .card_sources
            .pop()
            .expect("len >= 2 invariant asserted above");
        let top_handle = top.handle();
        // The new top is now `permanent.card_sources.last()` automatically —
        // no extra work needed (previous next-highest is now visible).
        let controller = perm.player;
        // Modifier cleanup: no per-card-source tracking exists in this engine;
        // permanent-handle modifiers remain valid for the promoted top card.
        //
        // Host = the promoted new top. The `top` was already popped above, and
        // pushing it to TRASH (inside the helper) does not touch the perm's
        // battle-area stack, so the host is identical computed here vs after
        // the push. Cause = `Cost` (matches DCGO `ArmorPurge.cs:65-78`, which
        // re-stacks `WhenTopCardTrashed`). Trash + fire via the shared Tier-2
        // primitive.
        let host_card = self
            .game
            .player(perm.player)
            .battle_area
            .get(perm.index as usize)
            .map(|permanent| permanent.top_card().handle())
            .unwrap_or(top_handle);
        self.game.trash_source_and_fire_with_cause(
            controller,
            perm,
            top,
            host_card,
            crate::trigger_context::EventCause::Cost,
        );
    }

    /// `<Training>` (Phase F §F4 / RULES_CONTEXT 16-40 / DCGO `Training.cs:30`)
    /// helper: pop the controller's deck top and append it at the BOTTOM of
    /// `perm`'s digivolution stack (`card_sources[0]`), marked face-down.
    ///
    /// Empty-deck case: silent no-op. The Rust port chooses safer behavior
    /// than DCGO's `LibraryCards[0]` raw indexing — DCGO never reaches the
    /// indexing line in practice because the `SetUpActivateClass` framework
    /// only calls in once activation is committed; the Rust version accepts
    /// the activation, pays the suspend cost in the calling effect, and
    /// silently no-ops the card move when there's nothing to draw. Mirrors
    /// the documented "no-op on empty source" pattern in `Player::draw`.
    ///
    /// `perm` may be either a battle-area or breeding-area permanent of
    /// the controller; this helper does not enforce zone — the caller's
    /// activation gate (carrier-not-suspended) handles eligibility. The
    /// breeding-area branch is a separate `as_mut()` lookup since
    /// `breeding_area: Option<Permanent>` is not in `battle_area`.
    ///
    /// The new source carries `face_down=true` (mirrors DCGO
    /// `isFacedown: true`); face-down sources are filtered out of the
    /// `<Mind Link>` "no Tamer source" gate (DCGO `MindLink.cs:25`
    /// `!cardSource.IsFlipped`).
    ///
    /// Used by: `<Training>` keyword auto-install (Phase F Task 6).
    pub fn training_place_deck_top_under_self_face_down(&mut self, perm: PermanentHandle) {
        // Pop the controller's deck top. Empty-deck case is a silent no-op.
        // Opaque-aware: opaque opponents materialize from RevealSource
        // tagged Effect (Training peeks at top and re-routes — not draw/mill/security).
        let owner = perm.player;
        let mut card = match self
            .game
            .take_from_deck_top_for_player(owner, crate::opaque_deck::RevealKind::Effect)
        {
            Ok(Some(c)) => c,
            Ok(None) => return,
            Err(e) => {
                eprintln!(
                    "[opaque-deck] training_place_deck_top_under_self_face_down \
                     error for player {}: {}",
                    owner, e
                );
                return;
            }
        };
        // Mark face-down — DCGO `AddDigivolutionCardsBottom(..., isFacedown: true)`.
        card.face_down = true;

        // Locate the carrier in battle area; if it's not there, look in
        // breeding area. (Breeding-area permanents never co-exist with a
        // same-handle battle-area slot — `move_from_breeding` takes the
        // Option, so the disjoint check holds.)
        let player = self.game.player_mut(owner);
        if let Some(p) = player.battle_area.get_mut(perm.index as usize) {
            // Insert at bottom of stack (index 0).
            p.card_sources.insert(0, card);
            return;
        }
        if let Some(ref mut breeding) = player.breeding_area {
            breeding.card_sources.insert(0, card);
            return;
        }
        // Carrier no longer exists in either zone (defensive — the calling
        // effect's `condition` gates on `source_permanent()`, which already
        // requires the carrier to be live). Drop the card on the floor
        // rather than misroute; in practice unreachable.
    }

    /// Place the top card of `target.player`'s deck as the bottom digivolution
    /// source of `target`. Generalizes `training_place_deck_top_under_self_face_down`
    /// to an arbitrary target permanent (Tamer or Digimon, in either player's
    /// battle area).
    ///
    /// Returns `Some(card_handle)` on success or `None` if the controller's
    /// deck is empty (silent no-op on empty deck, mirroring `Player::draw`).
    ///
    /// `face_down: true` marks the placed source face-down (Tamer-stash
    /// callers); `false` is ordinary face-up placement.
    ///
    /// Used by: ST-23 BEATBREAK / ST-24 DATA SQUAD Tamer-stash placement cards
    /// (e.g. ST23-13 Tomoro Tenma & Kyo Sawashiro, ST24-09 Sunflowmon).
    pub fn place_deck_top_under_permanent(
        &mut self,
        target: PermanentHandle,
        face_down: bool,
    ) -> Option<CardHandle> {
        let card_handle = self.game.player(target.player).deck.last()?.handle();
        let ok = self.game.place_as_bottom_source_observed(
            crate::enums::CardSourceRef::DeckTop(target.player),
            target,
            self.player,
            face_down,
        );
        if ok {
            Some(card_handle)
        } else {
            None
        }
    }

    /// Remove a single digivolution source card from `perm`'s stack and route
    /// it to its **owner's** hand. Mirrors `trash_card_source` but the
    /// destination is the owner's hand, not trash.
    ///
    /// Used by card effects that say "By returning N [card] from this
    /// Digimon's digivolution cards to its owner's hand" (e.g. BT12-031's
    /// Imperialdramon: Dragon Mode alt-cost).
    ///
    /// Owner-routing: the card is pushed to `removed.owner`'s hand (the
    /// `CardSource.owner` field), NOT the controller's — so a source owned by
    /// the opponent (rare, via control-transfer plays) returns to its true
    /// owner. `trash_card_source` reads `removed.owner` for the same reason.
    ///
    /// Stack invariant: the source is removed by `position(...)` (anywhere in
    /// the stack, not just the top), preserving the host permanent's top card
    /// and the ordering of the remaining sources.
    ///
    /// Observer dispatch: this is a *return-to-hand*, NOT a trash, so it does
    /// **not** fire `OnDigivolutionCardTrashed` (which would mis-attribute the
    /// move to source-trash listeners). No trash-specific observer fires.
    ///
    /// Returns `true` when the source handle was found and moved; `false` if
    /// the permanent slot is gone or the card is not in its stack.
    pub fn return_card_source_to_hand(&mut self, perm: PermanentHandle, card: CardHandle) -> bool {
        let removed = {
            let permanent = match self
                .game
                .player_mut(perm.player)
                .battle_area
                .get_mut(perm.index as usize)
            {
                Some(p) => p,
                None => return false,
            };
            let pos = match permanent
                .card_sources
                .iter()
                .position(|c| c.handle() == card)
            {
                Some(pos) => pos,
                None => return false,
            };
            permanent.card_sources.remove(pos)
        };
        let owner = removed.owner;
        self.game.player_mut(owner).hand.push(removed);
        let _ = self.cleanup_exposed_battle_area_digi_egg(perm);
        true
    }

    /// `Vec`-taking convenience wrapper over `return_card_source_to_hand`,
    /// keeping parity with `play_selected_sources_without_cost`. Each selected
    /// source ref is returned to its owner's hand. Returns `true` if every
    /// ref was successfully moved.
    pub fn return_selected_sources_to_hand(&mut self, selected: Vec<SourceSelectionRef>) -> bool {
        let mut all_ok = true;
        for source_ref in selected {
            if !self.return_card_source_to_hand(source_ref.permanent, source_ref.card) {
                all_ok = false;
            }
        }
        all_ok
    }
}
