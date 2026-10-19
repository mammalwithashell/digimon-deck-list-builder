//! Trash mutations (security, sources, hand) on `EffectContext` — extracted by mechanic.

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

impl Game {
    /// Source-specific effect protection. Rules cleanup and whole-stack departure
    /// deliberately do not consult this effect-trash gate.
    pub fn source_card_trash_blocked(
        &self,
        host: PermanentHandle,
        card: CardHandle,
        controller: PlayerId,
    ) -> bool {
        self.modifiers.blocks_effect_from(host, ModifierType::ImmuneFromStackTrashing, controller)
            || (self.player(host.player).battle_area.get(host.index as usize).is_some_and(|p| p.card_sources.iter().take(p.card_sources.len().saturating_sub(1)).any(|c| c.handle() == card))
            && self.modifiers.get(host, ModifierType::CannotTrash).iter().any(|entry| {
                matches!(&entry.payload, ModifierPayload::ProtectedSourceCards(cards) if cards.contains(&card))
            }))
    }
}

impl<'a> EffectContext<'a> {
    pub fn source_card_trash_blocked(&self, host: PermanentHandle, card: CardHandle) -> bool {
        self.game.source_card_trash_blocked(host, card, self.player)
    }

    /// Whether `target`'s stacked cards are protected from being trashed by
    /// THIS effect (its controller `self.player`), per
    /// `ModifierType::ImmuneFromStackTrashing` and each entry's controller
    /// scope. G-ENGINE-STACKED-CARD-RETURN-PROTECTION.
    pub fn stack_trash_blocked(&self, target: PermanentHandle) -> bool {
        self.game.modifiers.blocks_effect_from(
            target,
            ModifierType::ImmuneFromStackTrashing,
            self.player,
        )
    }

    /// Whether `target`'s stacked cards are protected from being returned to
    /// the hand or deck by THIS effect (`ModifierType::ImmuneFromStackReturn`).
    /// G-ENGINE-STACKED-CARD-RETURN-PROTECTION.
    pub fn stack_return_blocked(&self, target: PermanentHandle) -> bool {
        self.game.modifiers.blocks_effect_from(
            target,
            ModifierType::ImmuneFromStackReturn,
            self.player,
        )
    }

    pub fn select_deleted_self_digisources_from_trash<F, C>(
        &mut self,
        max: u8,
        prompt: &str,
        is_optional_zero: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, CardHandle) -> bool + Send + Sync + 'static,
        C: FnOnce(&mut EffectContext<'_>, Vec<CardHandle>) + Send + Sync + 'static,
    {
        let Some(snapshot) = self.deleted_object_snapshot().cloned() else {
            return;
        };
        let owner = snapshot.former_controller;
        let snapshot_sources = snapshot.digisources_just_before;
        self.select_snapshot_digisources_from_trash(
            owner,
            snapshot_sources,
            max,
            prompt,
            is_optional_zero,
            filter,
            callback,
        );
    }

    pub fn select_snapshot_digisources_from_trash<F, C>(
        &mut self,
        owner: PlayerId,
        snapshot_sources: Vec<CardHandle>,
        max: u8,
        prompt: &str,
        is_optional_zero: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, CardHandle) -> bool + Send + Sync + 'static,
        C: FnOnce(&mut EffectContext<'_>, Vec<CardHandle>) + Send + Sync + 'static,
    {
        if snapshot_sources.is_empty() {
            return;
        }
        self.select_count_capped_multi(
            owner,
            CountCappedZone::Trash,
            max,
            prompt,
            is_optional_zero,
            None,
            move |game, card| {
                let handle = card.handle();
                snapshot_sources.contains(&handle) && filter(game, handle)
            },
            callback,
        );
    }

    pub fn trash_top_security_and_cancel_current_replacement(&mut self, player: PlayerId) -> bool {
        if self.trash_top_security(player) {
            if self.game.has_active_parked_replacement() {
                self.cancel_current_replacement();
            }
            true
        } else {
            false
        }
    }

    pub fn trash_delay_source(&mut self) -> bool {
        matches!(self.trash_delay_source_status(), DelayCostStatus::Paid)
    }

    pub fn trash_delay_source_status(&mut self) -> DelayCostStatus {
        let Some(source) = self.source_permanent else {
            return DelayCostStatus::Unpaid;
        };
        let Some(source_card) = self.permanent_top_card_handle(source) else {
            return DelayCostStatus::Unpaid;
        };
        self.game
            .delete_permanent_with_cause(source, ReplacementCause::Cost);
        if self.game.pending_selection.is_some() {
            return DelayCostStatus::Pending;
        }
        if self.delay_source_card_in_trash(source.player, source_card) {
            DelayCostStatus::Paid
        } else {
            DelayCostStatus::Unpaid
        }
    }

    /// Trash the top N cards of a player's deck (mill effect).
    pub fn trash_from_top(&mut self, player: PlayerId, count: u8) -> u8 {
        let mut trashed = 0;
        for _ in 0..count {
            // Opaque-aware: opaque opponents materialize from RevealSource
            // tagged Mill; standard players pop from their ordered deck.
            let card = match self
                .game
                .take_from_deck_top_for_player(player, crate::opaque_deck::RevealKind::Mill)
            {
                Ok(Some(c)) => c,
                Ok(None) => break,
                Err(e) => {
                    eprintln!(
                        "[opaque-deck] trash_from_top error for player {}: {}",
                        player, e
                    );
                    break;
                }
            };
            let card_id = card.card_id(&self.game.card_data).to_string();
            let card_name = card.card_name(&self.game.card_data).to_string();
            let seq = self.game.next_event_seq();
            self.game.events.push(crate::events::GameEvent::Reveal {
                seq,
                player,
                card_id,
                card_name,
                source_zone: crate::events::RevealZone::TrashFromDeckTop,
            });
            // Route the deck→trash movement through the canonical trash path so
            // a `GameEvent::Trash` also fires (event-stream completeness per the
            // engine-event-emission "every card-to-trash emits Trash" rule). The
            // preceding `Reveal{TrashFromDeckTop}` is what the match log renders;
            // the frontend suppresses the paired `Trash` line to avoid a dup.
            self.game.trash_card(player, card);
            trashed += 1;
        }
        trashed
    }

    /// Move the top card of `player`'s security stack to their trash.
    /// No-op if the stack is empty. Returns true if a card was moved.
    ///
    /// Phase 7 Task 4: fires `WhenWouldBeTrashed` at entry. Subject carries
    /// the top security card's handle; cause inferred.
    pub fn trash_top_security(&mut self, player: PlayerId) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Opaque-aware: if top of security is a placeholder, materialize
        // before the replacement window so the WhenWouldBeTrashed lookup
        // sees real card identity (replacement effects are often keyed
        // on color/type).
        if let Some(top_idx) = self.game.player(player).security.len().checked_sub(1) {
            self.game.ensure_security_materialized(player, top_idx);
        }
        // Snapshot the top-of-security card handle before any state change.
        let top_handle = match self.game.player(player).security.last() {
            Some(c) => c.handle(),
            None => return false,
        };
        if !self.game.would_replacement_proceeds(
            EffectTiming::WhenWouldBeTrashed,
            ReplacementSubject::Card(top_handle, Zone::Security),
            player,
            Some(Zone::Trash),
        ) {
            return false;
        }

        let p = self.game.player_mut(player);
        if let Some(card) = p.security.pop() {
            p.face_up_security.remove(&card.card_index);
            self.game.fire_security_removed_observers(
                player,
                self.player,
                card,
                crate::selection::SecurityRemovalDestination::Trash,
            );
            self.game.mark_until_condition_dirty();
            self.game.reevaluate_until_condition_modifiers_if_dirty();
            true
        } else {
            false
        }
    }

    /// Trash the bottom card of `player`'s security stack (index 0).
    ///
    /// Mirrors `trash_top_security` but removes `security[0]` instead of
    /// the last element. Replacement effects and observers fire identically
    /// to the top-trash path.
    pub fn trash_bottom_security(&mut self, player: PlayerId) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Opaque-aware: materialize the bottom security card if it's a
        // placeholder, so subsequent WhenWouldBeTrashed lookups and
        // observer firings see real card identity.
        if !self.game.player(player).security.is_empty() {
            self.game.ensure_security_materialized(player, 0);
        }
        // Snapshot the bottom-of-security card handle before any state change.
        let bottom_handle = match self.game.player(player).security.first() {
            Some(c) => c.handle(),
            None => return false,
        };
        if !self.game.would_replacement_proceeds(
            EffectTiming::WhenWouldBeTrashed,
            ReplacementSubject::Card(bottom_handle, Zone::Security),
            player,
            Some(Zone::Trash),
        ) {
            return false;
        }

        let p = self.game.player_mut(player);
        if p.security.is_empty() {
            return false;
        }
        let card = p.security.remove(0);
        p.face_up_security.remove(&card.card_index);
        self.game.fire_security_removed_observers(
            player,
            self.player,
            card,
            crate::selection::SecurityRemovalDestination::Trash,
        );
        self.game.mark_until_condition_dirty();
        self.game.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    /// Trash a specific card — identified by the stable `handle` — from
    /// `player`'s security stack. Used by effects that let a player trash
    /// "any 1" of a security stack: the card is chosen via a `select_security`
    /// binding, which yields a `CardHandle`. Addressing the card by handle
    /// (rather than a positional index) means an intervening security-stack
    /// mutation cannot cause the wrong card to be trashed.
    ///
    /// No-op (returns false) if `handle` is not currently in `player`'s
    /// security stack. Mirrors `trash_top_security` / `trash_bottom_security`:
    /// same `WhenWouldBeTrashed` replacement window and security-removed
    /// observer fan-out, but addresses an arbitrary chosen card.
    /// G-TRASH-SELECTED-SECURITY.
    pub fn trash_security_card(&mut self, player: PlayerId, handle: CardHandle) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // The handle must currently be in `player`'s security stack.
        // Opaque-aware: placeholder card_index values are preserved
        // across materialization, so the handle iteration finds the
        // right slot whether or not it's still a placeholder. We then
        // materialize at that position so identity-dependent effects
        // (replacement lookups, observer firings) see real card data.
        let Some(initial_pos) = self
            .game
            .player(player)
            .security
            .iter()
            .position(|c| c.handle() == handle)
        else {
            return false;
        };
        self.game.ensure_security_materialized(player, initial_pos);
        if !self.game.would_replacement_proceeds(
            EffectTiming::WhenWouldBeTrashed,
            ReplacementSubject::Card(handle, Zone::Security),
            player,
            Some(Zone::Trash),
        ) {
            return false;
        }

        // Re-find the card by handle — the replacement window may have mutated
        // the security stack.
        let p = self.game.player_mut(player);
        let Some(pos) = p.security.iter().position(|c| c.handle() == handle) else {
            return false;
        };
        let card = p.security.remove(pos);
        p.face_up_security.remove(&card.card_index);
        self.game.fire_security_removed_observers(
            player,
            self.player,
            card,
            crate::selection::SecurityRemovalDestination::Trash,
        );
        self.game.mark_until_condition_dirty();
        self.game.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    pub fn trash_breeding_permanent(&mut self, target: PermanentHandle) -> bool {
        if target.index != crate::action::space::BREEDING_TARGET as u8 {
            return false;
        }
        if !self.can_affect_permanent(target) {
            return false;
        }

        let Some(perm) = self.game.player_mut(target.player).breeding_area.take() else {
            return false;
        };

        if !perm.card_sources.is_empty() && !perm.top_card().is_token {
            for card in perm.card_sources {
                let owner = card.owner;
                self.game.player_mut(owner).put_in_trash(card);
            }
        }
        for card in perm.linked_cards {
            let owner = card.owner;
            self.game.player_mut(owner).put_in_trash(card);
        }

        self.game.clear_permanent_full(target);
        self.game.modifiers.expire_player_on_permanent_leave(target);
        self.game.mark_until_condition_dirty();
        true
    }

    /// Pay the source permanent's trash-self activation cost.
    ///
    /// Used as the closure body for [`crate::effect::EffectBuilder::activation_cost`]
    /// on `<Delay>` abilities whose printed cost is "by trashing this card"
    /// (BT21-093 Raging Serpentine family). Deletes the source permanent —
    /// for a `<Delay>` Option this moves the card to its owner's trash —
    /// routing through `delete_permanent` so `OnDeletion` observers and
    /// `WhenWouldBeDeleted` replacements fire identically to the
    /// `delete_permanent` step. Returns `false` if the source permanent has
    /// already left the field.
    ///
    /// No-approximations note: per Comprehensive Rules 16-17-2 the processing
    /// of a `<Delay>` is optional; the player's accept/decline prompt belongs
    /// to [`crate::effect::EffectBuilder::optional`] and runs BEFORE this cost.
    pub fn trash_self_as_cost(&mut self) -> bool {
        let Some(handle) = self.source_permanent else {
            return false;
        };
        if self.source_permanent().is_none() {
            return false;
        }
        self.delete_permanent(handle);
        true
    }

    /// Trash a specific hand card by index.
    ///
    /// Phase 7 Task 4: fires `WhenWouldBeTrashed` at entry. Subject is the
    /// hand card handle; cause inferred.
    pub fn trash_from_hand_by_index(
        &mut self,
        player: PlayerId,
        hand_index: usize,
    ) -> Option<crate::card_source::CardHandle> {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Snapshot the card handle before any mutation. Return early if
        // the index is invalid.
        let card_handle = {
            let p = self.game.player(player);
            if hand_index >= p.hand.len() {
                return None;
            }
            p.hand[hand_index].handle()
        };
        if !self.game.would_replacement_proceeds(
            EffectTiming::WhenWouldBeTrashed,
            ReplacementSubject::Card(card_handle, Zone::Hand),
            player,
            Some(Zone::Trash),
        ) {
            return None;
        }

        let trashed = self.game.trash_from_hand_by_index(player, hand_index);
        if trashed.is_some() {
            // Effect-caused hand→trash: record into the OnDiscardHand batch
            // window keyed on the causing effect's controller (this context's
            // player). Fires once when the effect body completes
            // (G-ENGINE-ON-DISCARD-HAND). `self.player` is the controller of
            // the effect performing the discard.
            let cause_controller = self.player;
            self.game.note_effect_hand_discard(cause_controller, player);
            self.game.note_effect_hand_discard_card(player, card_handle);
        }
        trashed
    }

    /// Move a specific revealed card into `player`'s trash.
    pub fn trash_from_reveal(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        self.game.trash_from_reveal(player, card)
    }

    /// Trash a specific digivolution source from a permanent.
    ///
    /// Used by:
    /// - `<Fragment (N)>` keyword auto-install (Phase D Task 4) — picked sources
    ///   are trashed as the cancel-deletion cost.
    /// - `<Partition>` keyword auto-install (Phase D Task 9) — sources are
    ///   moved out of the deleted permanent's stack.
    /// - Hand-authored card scripts that "trash a digivolution card" as a cost
    ///   or effect.
    ///
    /// The card is removed from `perm.card_sources` (anywhere in the stack —
    /// not just the top — see `armor_purge_top` for the top-card-only variant)
    /// and pushed to the card owner's trash.
    ///
    /// Returns `true` iff the card was actually trashed. Returns `false` (no
    /// mutation, no observer dispatch, no `soft_remove` side-effects) for any
    /// rules-natural fizzle:
    ///   - the carrier slot is gone (soft-removed or deleted by a sibling
    ///     effect between handle capture and this call);
    ///   - the carrier exists but `card_sources` is empty (zombie slot
    ///     mid-cleanup — also avoids the `top_card()` panic on empty stack);
    ///   - the carrier's `card_sources` does not contain `card` (the captured
    ///     handle was invalidated by an intervening observer that trashed,
    ///     returned, or extracted the source).
    ///
    /// This soft-fail contract mirrors DCGO
    /// `ITrashDigivolutionCards.TrashDigivolutionCards()` (see
    /// `DCGO/Assets/Scripts/Script/CardController.cs:5181`): the trash
    /// primitive is declarative ("trash these if possible") and the outcome
    /// is observable from the return value. Callers that need to branch on
    /// the actually-trashed set check this bool; callers that don't care
    /// (e.g., the DSL `TrashSelectedSources` loop and the engine-side
    /// pre-validated helpers `<Fragment>` install, `trash_all_sources`,
    /// `trash_top_n_digivolution_cards_of_each`) discard it with `let _ =`.
    /// See change `fix-trash-card-source-stale-handle` and panic family
    /// `G-DSL-TRASH-SOURCES-STALE-HANDLE` in
    /// `qa/archetype-qa/panic-families.json`.
    ///
    /// Token cards (`is_token == true`) are still pushed to trash; the
    /// caller's gate is responsible for any token-aware filtering.
    pub fn trash_card_source(&mut self, perm: PermanentHandle, card: CardHandle) -> bool {
        // G-ENGINE-STACKED-CARD-RETURN-PROTECTION: stacked-card trash
        // protection (DCGO `TrashDigivolutionCards` checks
        // `ImmuneFromStackTrashing` before any removal).
        if self.source_card_trash_blocked(perm, card) {
            return false;
        }
        let (removed, host_card) = {
            // Soft-fail: carrier missing (DCGO: `if (_permanent == null) yield break;`).
            let permanent = match self
                .game
                .player_mut(perm.player)
                .battle_area
                .get_mut(perm.index as usize)
            {
                Some(p) => p,
                None => return false,
            };
            // Soft-fail: empty stack (DCGO: `HasNoDigivolutionCards` yield-break).
            // Guards the `top_card()` call below from panicking on an empty stack.
            if permanent.card_sources.is_empty() {
                return false;
            }
            let host_card = permanent.top_card().handle();
            // Soft-fail: card not in stack (DCGO: target dropped by
            // `_trashTargetCards.Filter(c => _permanent.DigivolutionCards.Contains(c))`).
            let pos = match permanent
                .card_sources
                .iter()
                .position(|c| c.handle() == card)
            {
                Some(p) => p,
                None => return false,
            };
            (permanent.card_sources.remove(pos), host_card)
        };
        self.game
            .trash_source_and_fire(removed.owner, perm, removed, host_card);
        // Soft-remove the carrier slot if the trash emptied it. Sibling
        // of the digivolve-from-material fix landed in PR #533. The
        // `fire_digivolution_card_trashed` observer enqueue above captures
        // the trigger context BEFORE the slot removal, so observers see the
        // source-trash event attributed to the correct host. See
        // `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
        // `qa/archetype-qa/engine-gaps.md`.
        if !self.game.soft_remove_if_emptied(perm) {
            let _ = self.cleanup_exposed_battle_area_digi_egg(perm);
        }
        true
    }

    /// Trash every digivolution source below `target`'s top card, preserving
    /// the live permanent. All of them are trashed at the same time: ONE
    /// source-trash event, so each observer triggers once and only after the
    /// whole trash (§15-5-2, §15-8-3-2).
    pub fn trash_all_sources(&mut self, target: PermanentHandle) -> bool {
        if self.player != target.player && self.stack_trash_blocked(target) {
            return false;
        }
        let Some(permanent) = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return false;
        };
        let source_handles: Vec<CardHandle> = permanent
            .card_sources
            .iter()
            .take(permanent.card_sources.len().saturating_sub(1))
            .map(|source| source.handle())
            .collect();

        // Every source is trashed at the same time: one event (§15-5-2).
        let event = self.game.begin_source_trash_event();
        for source in source_handles {
            let Some(permanent) = self
                .game
                .player(target.player)
                .battle_area
                .get(target.index as usize)
            else {
                break;
            };
            let still_below_top = permanent
                .card_sources
                .iter()
                .take(permanent.card_sources.len().saturating_sub(1))
                .any(|candidate| candidate.handle() == source);
            if still_below_top {
                // Bool discarded: pre-validated above; no caller branches
                // on outcome (return is `true` regardless of per-source success).
                let _ = self.trash_card_source(target, source);
            }
        }
        self.game.end_source_trash_event(event);
        true
    }

    /// Strip the top digivolution source from `target`'s stack and route the
    /// underlying card to its owner's trash. Returns `true` on
    /// success; `false` if the target handle is invalid or the stack is empty.
    ///
    /// Used by card effects that say "trash the top digivolution source of
    /// this Digimon" — the bool-returning, gate-friendly counterpart to
    /// `armor_purge_top` (which is reserved for the `<Armor Purge>` keyword
    /// auto-install body and panics on insufficient stack).
    ///
    /// The trashed card is the visible TOP card, not a digivolution card
    /// (§4-8-1), so this enqueues `OnTopCardTrashed` (the timing
    /// `armor_purge_top` and <De-Digivolve> fire; DCGO
    /// `EffectTiming.WhenTopCardTrashed`) and never
    /// `OnDigivolutionCardTrashed`. Enqueue-only: the observers activate once
    /// the causing effect has finished (§15-4-3-2), like every other
    /// top-card trash. G-ENGINE-TRASH-TOP-SOURCE-FIRES-DIGIVOLUTION-CARD-
    /// TRASHED. The trashed card moves to its owner's trash.
    ///
    /// §8-3-2-1 (the Burst Digivolve turn-end trash, this primitive's only
    /// authored user): "if there are no stacked cards or that card is not a
    /// Digimon, no card is trashed", and §4-7-2 "Only 1 card isn't considered
    /// 'stacked cards'": a 1-card stack, or one whose top card is not a
    /// Digimon card, trashes nothing and the permanent stays (DCGO
    /// `SelectBurstDigivolutionEffect` requires >= 1 digivolution card).
    ///
    /// Reject-before-mutate discipline: invalid handle, empty or 1-card stack
    /// and a non-Digimon top all return `false` before any state change.
    pub fn trash_top_source(&mut self, target: PermanentHandle) -> bool {
        // §8-3-2-1 / §4-7-2 (the burst turn-end trash's own rule): a lone
        // card is not a stacked card, and a non-Digimon top trashes nothing.
        // Checked HERE, not in the shared pop: the generic "trash the top N
        // stacked cards" (`trash_top_n_stacked_sources`) keeps going past a
        // Tamer / Option / Digi-Egg it exposes (DCGO `ITrashStack` has no
        // type check; BT21-030 Q&A "until there are no more stacked cards").
        let eligible = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .is_some_and(|p| {
                p.card_sources.len() > 1
                    && matches!(
                        p.top_card().card_kind(&self.game.card_data),
                        crate::enums::CardKind::Digimon | crate::enums::CardKind::Dual
                    )
            });
        if !eligible {
            return false;
        }
        let Some(removed) = self.pop_top_source_for_trash(target) else {
            return false;
        };
        self.trash_popped_top_cards(target, vec![removed]);
        true
    }

    /// Guards + pop shared by [`Self::trash_top_source`] and the batched
    /// `trash_top_n_stacked_sources`: `None` (no mutation) when the slot is
    /// missing or empty, or a protection blocks the trash
    /// (`ImmuneFromStackTrashing` on the host, or the top card's own
    /// source-trash protection). Type- and length-agnostic: the §8-3-2-1 /
    /// §4-7-2 guards belong to `trash_top_source` alone, and the batch has its
    /// own `len > 1` stop.
    fn pop_top_source_for_trash(
        &mut self,
        target: PermanentHandle,
    ) -> Option<crate::card_source::CardSource> {
        if let Some(card) = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .and_then(|p| p.card_sources.last())
        {
            if self.source_card_trash_blocked(target, card.handle()) {
                return None;
            }
        }
        // Track C / D consult site (2026-05-08): `ImmuneFromStackTrashing`
        // on the host permanent blocks the inherited stack-peel mutation.
        // Distinct from `CannotBeDestroyed` (which protects the live top
        // card from deletion) — this protects the digivolution sources
        // sitting beneath the top from being peeled off and trashed.
        if self.stack_trash_blocked(target) {
            return None;
        }
        self.game
            .player_mut(target.player)
            .battle_area
            .get_mut(target.index as usize)?
            .card_sources
            .pop()
    }

    /// Move top cards popped from `target` (in pop order) to their owners'
    /// trash and enqueue ONE `OnTopCardTrashed` for the batch (§16-12-5 /
    /// DCGO `ITrashStack`: every card trashed by one event in one timing).
    /// The host is the card now on top (the last popped card when the stack
    /// is empty); an emptied permanent leaves the battle area.
    fn trash_popped_top_cards(
        &mut self,
        target: PermanentHandle,
        removed: Vec<crate::card_source::CardSource>,
    ) {
        let Some(last) = removed.last() else {
            return;
        };
        let host_card = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .and_then(|p| p.card_sources.last())
            .map(|card| card.handle())
            .unwrap_or_else(|| last.handle());
        let cause = crate::trigger_context::EventCause::from(
            self.game.infer_effect_cause(target.player),
        );
        let handles: Vec<_> = removed.iter().map(|card| card.handle()).collect();
        for card in removed {
            self.game.player_mut(card.owner).put_in_trash(card);
        }
        self.game
            .enqueue_top_cards_trashed(target.player, target, host_card, handles, cause);
        self.game.mark_until_condition_dirty();
        self.game.reevaluate_until_condition_modifiers_if_dirty();
        if !self.game.soft_remove_if_emptied(target) {
            let _ = self.cleanup_exposed_battle_area_digi_egg(target);
        }
    }

    /// Trash up to `count` bottom-most digivolution sources from `target`.
    ///
    /// This is the no-choice source-peel primitive for early blue effects
    /// like ST2-03 / ST2-06 / ST2-09. It never removes the visible top card:
    /// when fewer source cards exist than requested, it trashes the available
    /// sources and then stops. Missing/stale targets and zero-source stacks
    /// soft-fail with no mutation.
    pub fn trash_bottom_sources(&mut self, target: PermanentHandle, count: u8) -> u8 {
        if count == 0 {
            return 0;
        }
        if self.stack_trash_blocked(target) {
            return 0;
        }

        let cards: Vec<_> = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .map(|p| {
                p.card_sources
                    .iter()
                    .take(p.card_sources.len().saturating_sub(1))
                    .take(count as usize)
                    .map(|c| c.handle())
                    .collect()
            })
            .unwrap_or_default();
        // The N cards are trashed at the same time: one event (§15-5-2).
        let event = self.game.begin_source_trash_event();
        let trashed = cards
            .into_iter()
            .filter(|card| self.trash_card_source(target, *card))
            .count() as u8;
        self.game.end_source_trash_event(event);
        trashed
    }

    /// Trash up to `count` cards from the top of `target`'s stack, one at a
    /// time, stopping once a single card remains.
    ///
    /// `include_top_card: false` — "trash the top N **digivolution cards**":
    /// each pass removes the topmost digivolution card (`card_sources
    /// [len - 2]`); the top card stays (BT13-030, EX7-016, EX7-020).
    ///
    /// `include_top_card: true` — "trash the top N **stacked cards**": each
    /// pass removes the visible TOP card (`card_sources[len - 1]`, DCGO
    /// `ITrashStack` → `_permanent.TopCard`), promoting the card under it;
    /// the last remaining card is the original bottom card (BT21-030 official
    /// Q&A: "The top card is trashed until there are no more stacked cards").
    /// An exposed Digi-Egg top deletes the permanent and stops.
    /// G-TOP-STACKED-CARD-TRASH (2026-10-01).
    pub fn trash_top_n_stacked_sources(
        &mut self,
        target: PermanentHandle,
        count: u8,
        include_top_card: bool,
    ) -> usize {
        if count == 0 || !self.can_affect_permanent(target) {
            return 0;
        }
        if self.stack_trash_blocked(target) {
            return 0;
        }

        if include_top_card {
            // One event: every popped top card is trashed together and
            // `OnTopCardTrashed` fires ONCE for the batch (DCGO `ITrashStack`).
            let mut removed = Vec::new();
            for _ in 0..count {
                if !self
                    .game
                    .player(target.player)
                    .battle_area
                    .get(target.index as usize)
                    .is_some_and(|p| p.card_sources.len() > 1)
                {
                    break;
                }
                let Some(card) = self.pop_top_source_for_trash(target) else {
                    break;
                };
                removed.push(card);
            }
            let n = removed.len();
            self.trash_popped_top_cards(target, removed);
            return n;
        }
        let cards: Vec<_> = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .map(|p| {
                let len = p.card_sources.len();
                let end = if include_top_card {
                    len
                } else {
                    len.saturating_sub(1)
                };
                p.card_sources[..end]
                    .iter()
                    .rev()
                    .take((count as usize).min(len.saturating_sub(1)))
                    .map(|c| c.handle())
                    .collect()
            })
            .unwrap_or_default();
        // One effect step: one event (§15-5-2).
        let event = self.game.begin_source_trash_event();
        let trashed = cards
            .into_iter()
            .filter(|card| self.trash_card_source(target, *card))
            .count();
        self.game.end_source_trash_event(event);
        trashed
    }

    /// Trash the bottom-most face-down digivolution source from `target`,
    /// fire `OnDigivolutionCardTrashed`, and drain the observer queue. Returns
    /// `true` iff a face-down source was found at `card_sources[0]` and
    /// trashed; returns `false` with no mutation otherwise (no face-down
    /// bottom source, or `target` missing).
    ///
    /// This is the cost-form trash primitive for ST-23 BEATBREAK and ST-24
    /// DATA SQUAD cards whose printed text reads "by trashing the bottom
    /// face-down card from under any of your Tamers, ...".
    ///
    /// The trashed source routes to the source's own `owner` trash, matching
    /// the standard `OnDigivolutionCardTrashed` ownership semantics (mirrors
    /// `trash_card_source` / `trash_top_source`).
    ///
    /// Effect costs obey source-card trash protection too (BT9-109 Q1920).
    /// Controller-scoped stack protection still distinguishes opponent effects.
    ///
    /// Used by: ST23-01 Kekkomon, ST23-03 Cougarmon, ST23-04 Murasamemon,
    /// ST23-08 Monarchlizamon, ST23-11 Wolvermon, ST23-12 Chiropmon,
    /// ST24-01 Koromon, ST24-06 RizeGreymon, ST24-10 Lilamon, ST24-11 Rosemon,
    /// ST24-12 Falcomon.
    pub fn trash_bottom_face_down_source(&mut self, target: PermanentHandle) -> bool {
        if let Some(card) = self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .and_then(|p| p.card_sources.first())
        {
            if self.source_card_trash_blocked(target, card.handle()) {
                return false;
            }
        }

        // Inspect `card_sources[0]`: it must exist AND be face-down. Reject
        // before any mutation otherwise (missing target, empty stack, or a
        // face-up bottom source — e.g. an un-stashed Tamer whose only source
        // is its own face-up card).
        let removed = {
            let permanent = match self
                .game
                .player_mut(target.player)
                .battle_area
                .get_mut(target.index as usize)
            {
                Some(p) => p,
                None => return false,
            };
            match permanent.card_sources.first() {
                Some(bottom) if bottom.face_down => {}
                _ => return false,
            }
            permanent.card_sources.remove(0)
        };
        // Host = the permanent's top card. The below-top source was already
        // removed above; the perm still exists with its top intact (a Tamer
        // always retains its own card as the top), so the host is unchanged by
        // the helper's push-to-trash. Direct index is infallible by
        // construction (validated present at the top of this function).
        let host_card = self.game.player(target.player).battle_area[target.index as usize]
            .top_card()
            .handle();
        self.game
            .trash_source_and_fire(removed.owner, target, removed, host_card);
        true
    }

    /// Force `opponent`'s hand size down to `target_count` by surfacing a
    /// multi-pick selection on their hand. The opponent is the selecting
    /// player — they choose which cards to trash, per the no-approximations
    /// policy (mass forced-trash effects with player choice on the affected
    /// side, e.g. BT19-075 MoonMillenniummon "your opponent trashes cards
    /// from their hand until they have N").
    ///
    /// Returns `true` if a selection was installed, `false` if the
    /// opponent's hand is already at or below `target_count` (no-op).
    ///
    /// **Selection semantics:** uses `select_count_capped_multi` with
    /// `max = current - target_count` and `is_optional_zero = false`
    /// (forcing the opponent to pick at least one card). The engine's
    /// existing count-capped multi-select allows the player to PASS once
    /// they've picked at least 1, so for cards that strictly require an
    /// exact count (vs. up-to count), callers should chain a
    /// `trash_opponent_hand_to_count` call as a continuation until the
    /// hand size meets the target. Most printed cards are forgiving here —
    /// "until they have N" cards typically allow the opponent to control
    /// the cadence as long as the floor is reached.
    pub fn trash_opponent_hand_to_count(&mut self, opponent: PlayerId, target_count: u8) -> bool {
        self.trash_opponent_hand_to_count_bound(opponent, target_count, None)
    }

    /// As [`Self::trash_opponent_hand_to_count`], but tags the resumable
    /// selection's terminal with `bind_count_as` so that, once the opponent
    /// finishes trashing, the number of cards actually trashed is published
    /// into the resolving DSL tail's bindings (a `Literal`) for a downstream
    /// `binding_value` formula to read. Returns `false` (no selection
    /// installed) when the opponent's hand is already ≤ `target_count`; the
    /// caller is responsible for binding 0 on that synchronous no-op path.
    /// G-DSL-TRASH-COUNT-RESULT-BINDING.
    pub fn trash_opponent_hand_to_count_bound(
        &mut self,
        opponent: PlayerId,
        target_count: u8,
        bind_count_as: Option<String>,
    ) -> bool {
        let current = self.game.player(opponent).hand.len();
        let target = target_count as usize;
        if current <= target {
            return false;
        }
        let to_trash = (current - target).min(u8::MAX as usize) as u8;
        let prov = crate::resume::ResumeProvenance {
            source_card: self.source_card,
            source_permanent: self.source_permanent,
            source_kind: self.source_kind,
            controller: self.player,
            override_pin: Some(opponent),
        };

        self.as_selecting_player(opponent)
            .select_count_capped_multi(
                opponent,
                CountCappedZone::Hand,
                to_trash,
                "Choose cards to trash from your hand",
                /* is_optional_zero */ false,
                /* distinct_by */ None,
                /* filter */ |_g, _c| true,
                move |ctx, picks| {
                    // Trash each chosen card by stable handle. Hand indices
                    // shift after each trash, so re-resolve per pick.
                    for card_handle in picks {
                        let idx = ctx
                            .game
                            .player(opponent)
                            .hand
                            .iter()
                            .position(|c| c.handle() == card_handle);
                        if let Some(i) = idx {
                            ctx.trash_from_hand_by_index(opponent, i);
                        }
                    }
                },
            );
        if let Some(pending) = self.game.pending_selection.as_ref() {
            self.game.pending_selection_resume = Some(crate::resume::ResumeStack {
                frames: vec![crate::resume::ResumeFrame::NonDslCountCappedStep(
                    crate::resume::NonDslCountCappedState {
                        prov,
                        of_player: opponent,
                        zone: CountCappedZone::Hand,
                        min: 0,
                        max: to_trash,
                        is_optional_zero: false,
                        distinct_by: None,
                        candidate_actions: pending.valid_action_ids.clone(),
                        accum: Vec::new(),
                        prompt: pending.prompt.clone(),
                        previous_phase: pending.previous_phase,
                        terminal:
                            crate::resume::NonDslCountCappedTerminal::TrashOpponentHandToCount {
                                opponent,
                                bind_count_as,
                            },
                        outer_conts: Vec::new(),
                    },
                )],
            });
        }
        true
    }

    /// Trim up to `n` digivolution-source cards (the cards beneath the
    /// visible top) from every battle-area permanent of `target_player`.
    /// Track E Tier 2 Task 8 — bulk stack-peel primitive for printed text
    /// like BT12-028 "trash the top digivolution card of each of your
    /// opponent's Digimon" and generalisations.
    ///
    /// Semantic note: "top digivolution card" in TCG parlance means the
    /// topmost source **below** the visible top (`card_sources[len-2]`),
    /// NOT the visible top itself (which is the Digimon, not a digivolution
    /// card). This helper trims sources from the top of the stack-below-top
    /// and never deletes the visible Digimon. Permanents with stack size
    /// less than 2 (no digivolution cards under the top) are skipped.
    ///
    /// Per source: routes through `trash_card_source`, which fires
    /// `OnDigivolutionCardTrashed` with the proper `SourceTrashedFromStack`
    /// trigger context (host permanent + host_card + extracted source
    /// card). Owner-routed: `trash_card_source` pushes each card to its
    /// owner's trash.
    ///
    /// Iterates by stable `PermanentHandle` snapshots re-resolved per
    /// pass so intervening battle-area shifts (caused by observer fan-out)
    /// don't skip permanents.
    pub fn trash_top_n_digivolution_cards_of_each(
        &mut self,
        target_player: PlayerId,
        n: u8,
    ) -> usize {
        if n == 0 {
            return 0;
        }
        let targets: Vec<_> = self
            .game
            .player(target_player)
            .battle_area
            .iter()
            .enumerate()
            .flat_map(|(index, p)| {
                let h = PermanentHandle {
                    player: target_player,
                    index: index as u8,
                };
                p.card_sources
                    .iter()
                    .take(p.card_sources.len().saturating_sub(1))
                    .rev()
                    .take(n as usize)
                    .map(move |c| (h, c.handle()))
            })
            .collect();
        // Every Digimon's cards are trashed at the same time: one event
        // (§15-5-2).
        let event = self.game.begin_source_trash_event();
        let trashed = targets
            .into_iter()
            .filter(|(host, card)| self.trash_card_source(*host, *card))
            .count();
        self.game.end_source_trash_event(event);
        trashed
    }
}
