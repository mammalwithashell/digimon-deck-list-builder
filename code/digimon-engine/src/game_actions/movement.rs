//! Return-to-hand / return-to-deck operations (Tier 2) — `impl Game`.

#![allow(unused_imports)]
use super::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
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
use rand::seq::SliceRandom;

impl Game {
    /// Move a specific revealed card back to `player`'s deck at `position`.
    /// Returns false if the handle is not in the reveal pool.
    pub fn return_to_deck_from_reveal(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        let Some(pos_idx) = self.revealed_cards.iter().position(|c| c.handle() == card) else {
            return false;
        };
        let mut taken = self.revealed_cards.remove(pos_idx);
        taken.clear_reveal_overlay();
        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).deck.push(taken);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).deck.insert(0, taken);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let deck_len = self.player(player_id).deck.len();
                let idx = if deck_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=deck_len)
                };
                self.player_mut(player_id).deck.insert(idx, taken);
            }
        }
        true
    }

    /// Bounce a permanent to its owner's hand: the top card moves to hand,
    /// every card beneath it goes to the owner's trash (per DCGO leave-field
    /// rules). Linked cards go to trash. Returns the handle of the card that
    /// ended up in hand.
    ///
    /// Does not fire OnLeaveField observers — that's Phase 1 timing-dispatch
    /// infrastructure. Modifiers targeting the returned permanent are cleared.
    ///
    /// Phase 7 Task 4: fires `WhenWouldLeaveBattleArea` + `WhenWouldBeReturnedToHand`
    /// replacement windows before committing. See spec §4.1, §7.
    pub fn return_to_hand(
        &mut self,
        handle: PermanentHandle,
    ) -> Option<crate::card_source::CardHandle> {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        {
            let player = self.player_mut(handle.player);
            if (handle.index as usize) >= player.battle_area.len() {
                return None;
            }
        }

        let cause = self.infer_effect_cause(handle.player);
        let subject = ReplacementSubject::Permanent(handle);

        // Super-timing first, then route-specific would.
        let leave_outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            subject,
            cause,
            Some(Zone::Hand),
        );
        if self.pending_selection.is_some() {
            return None;
        }
        let outcome = match leave_outcome {
            ReplacementOutcome::None => self.try_replace(
                EffectTiming::WhenWouldBeReturnedToHand,
                subject,
                cause,
                Some(Zone::Hand),
            ),
            other => other,
        };
        if self.pending_selection.is_some() {
            return None;
        }

        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return None;
            }
            ReplacementOutcome::Redirected(Zone::Deck) => {
                self.return_to_deck(handle, crate::enums::StackPosition::Bottom);
                return None;
            }
            ReplacementOutcome::Redirected(Zone::Trash) => {
                self.delete_permanent_with_cause(handle, cause);
                return None;
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected redirect destination for WhenWouldBeReturnedToHand: {:?}",
                    other
                );
                // Fall through and commit the original return-to-hand.
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)) => {
                return self.return_to_hand(other);
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "non-Permanent substitute subject for WhenWouldBeReturnedToHand"
                );
                // Fall through and commit the original.
            }
        }

        // Snapshot the leaving permanent's identity BEFORE removal so the
        // OnLeaveField observer's `event_target_*` predicates resolve.
        let leave_snapshot = self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .and_then(|p| {
                let top_handle = p.top_card().handle();
                let data = self.card_data_for_handle(top_handle)?;
                let mut digisources: Vec<crate::card_source::CardHandle> = Vec::new();
                for src in p.card_sources.iter() {
                    let h = src.handle();
                    if h != top_handle {
                        digisources.push(h);
                    }
                }
                let source_count = digisources.len();
                let dp_now = self.effective_dp(handle);
                let is_token = p.top_card().is_token;
                let identity = p.synth_identity(&self.card_data, &self.modifiers, handle);
                Some(crate::trigger_context::DeletedObjectSnapshot {
                    former_controller: handle.player,
                    top_card: top_handle,
                    card_kind: identity.kind,
                    retained_kind: identity.retained_kind,
                    traits: identity.traits.clone(),
                    level: identity.level,
                    dp: dp_now,
                    cause: crate::trigger_context::EventCause::Return,
                    dp_just_before: dp_now,
                    level_just_before: identity.level,
                    cost_just_before: Some(data.play_cost),
                    names_just_before: identity.card_names.clone(),
                    traits_just_before: identity.traits.clone(),
                    source_count_just_before: source_count,
                    digisources_just_before: digisources,
                    is_token,
                })
            });

        let perm = self
            .player_mut(handle.player)
            .battle_area
            .remove(handle.index as usize);
        // `battle_area` is a Vec, so the remove above COMPACTS it and every
        // queued effect whose carrier sat at a LATER slot now addresses the
        // wrong permanent. Re-key them, as the deletion and link paths already
        // do. Without this a SURVIVING carrier reads as dead in
        // `queued_effect_source_is_live` and its MANDATORY trigger is silently
        // dropped -- measured on EX12-036 Ryugumon, where the standing carrier
        // reported index 1 while the battle area held one permanent at index 0.
        // Clear the departing identity and compact every surviving handle
        // BEFORE overflow/source-trash callbacks can rebuild declarative state.
        // A late clear at the former slot would erase the shifted survivor.
        self.clear_permanent_full(handle);
        self.modifiers.expire_player_on_permanent_leave(handle);
        self.modifiers
            .shift_after_battle_area_remove(handle.player, handle.index);
        self.shift_pending_attack_after_battle_area_remove(handle.player, handle.index);
        self.shift_effect_queue_after_battle_area_remove(handle.player, handle.index);

        let mut sources = perm.card_sources;
        let Some(top) = sources.pop() else {
            return None;
        };
        let top_handle = top.handle();
        let top_owner = top.owner;
        let mut leaving_sources = sources.clone();
        leaving_sources.push(top.clone());
        self.apply_ace_overflow_for_sources(&leaving_sources);
        // Owner-routed: top card returns to its owner's hand, not the
        // controller's. Track E correctness rule. Identical when
        // `top_owner == handle.player` (the common case today).
        let top_is_egg = top.card_kind(&self.card_data) == CardKind::DigiEgg;
        if top_is_egg {
            // Rule 3-1-3-9-2: a Digi-Egg cannot enter hand.
            self.player_mut(top_owner).digitama_deck.insert(0, top);
        } else {
            self.player_mut(top_owner).hand.push(top);
        }

        // Sources below the top go to each source's owner's trash and fire
        // OnDigivolutionCardTrashed (digivolution stack sources only — not
        // linked_cards which are Tamer equipment and separate semantic
        // category). Owner-routed.
        // All of them are trashed at the same time: ONE event (§15-5-2),
        // enqueued without draining — the leave-field observers below belong
        // to the same return and nothing activates mid-process (§15-8-3-2).
        let mut trashed = Vec::new();
        for card in sources {
            let source_card = card.handle();
            // Owner-routed: each source returns to its OWN owner's trash,
            // not the controller's (Track E correctness rule). The trigger
            // attribution still uses `handle.player` (the host's controller)
            // for event-source-player binding.
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
            trashed.push(super::TrashedSource {
                player: handle.player,
                host: handle,
                host_card: top_handle,
                card: source_card,
            });
        }
        self.enqueue_digivolution_cards_trashed(
            trashed,
            crate::trigger_context::EventCause::Rule,
        );
        let had_linked = !perm.linked_cards.is_empty();
        for card in perm.linked_cards {
            // Owner-routed: linked cards return to their own owner's trash.
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
        }
        // Phase 8 Task 4: fire OnLinkedCardTrashed if the returning host was
        // carrying any linked cards (they cannot ride the host back to hand).
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnLinkedCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
        }

        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        // OnLeaveField: the permanent left the battle area by return-to-hand.
        if let Some(snapshot) = leave_snapshot {
            self.fire_on_leave_field(handle, snapshot);
        } else {
            self.maybe_drain_effect_queue();
        }
        // OnAddToHand: the returned top card entered its owner's hand by effect.
        // (Digivolution sources went to trash, not hand, so only the top card
        // counts as a hand gain.) See G-ON-ADD-TO-HAND-OBSERVER.
        if top_is_egg {
            None
        } else {
            self.fire_on_add_to_hand_by_effect(top_owner);
            Some(top_handle)
        }
    }

    /// Low-level source-attribution helper for tests and engine internals.
    ///
    /// The underlying movement still routes through `return_to_hand`, including
    /// replacement windows and source-disposition triggers; this wrapper only
    /// supplies effect source attribution so opponent-only protection can
    /// distinguish own effects from opponent effects. Production card effects
    /// should prefer `EffectContext::return_to_hand`, which also enforces
    /// `can_affect_permanent` gates and uses real source metadata.
    #[doc(hidden)]
    pub fn return_to_hand_from_effect(
        &mut self,
        handle: PermanentHandle,
        effect_player: PlayerId,
    ) -> bool {
        let previous = self.effect_source_player;
        self.effect_source_player = Some(effect_player);
        let moved = self.return_to_hand(handle).is_some();
        self.effect_source_player = previous;
        moved
    }

    /// Return a permanent's top card to its owner's deck at `position`.
    /// Sources under the top go to trash; linked_cards go to trash.
    /// Modifiers targeting the permanent are cleared. Returns true on
    /// success, false if the handle is invalid or the stack is empty.
    ///
    /// Does not fire OnLeaveField observers.
    ///
    /// Phase 7 Task 4: fires `WhenWouldLeaveBattleArea` +
    /// `WhenWouldBeReturnedToDeck` replacement windows before committing.
    pub fn return_to_deck(
        &mut self,
        handle: PermanentHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        self.return_to_deck_inner(handle, position, false)
    }

    /// Low-level source-attribution helper for tests and engine internals.
    ///
    /// This is the source-attributed companion to `return_to_deck`; callers
    /// that need production effect semantics should prefer
    /// `EffectContext::return_to_deck`, which also enforces
    /// `can_affect_permanent` gates and uses real source metadata.
    #[doc(hidden)]
    pub fn return_to_deck_from_effect(
        &mut self,
        handle: PermanentHandle,
        effect_player: PlayerId,
    ) -> bool {
        let previous = self.effect_source_player;
        self.effect_source_player = Some(effect_player);
        let moved = self.return_to_deck(handle, crate::enums::StackPosition::Bottom);
        self.effect_source_player = previous;
        moved
    }

    /// Return a permanent's full stack to its owner's deck at `position`.
    /// Preserves bottom-to-top source order in the deck instead of trashing
    /// lower digivolution sources.
    pub fn return_stack_to_deck(
        &mut self,
        handle: PermanentHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        self.return_to_deck_inner(handle, position, true)
    }

    pub(crate) fn return_to_deck_inner(
        &mut self,
        handle: PermanentHandle,
        position: crate::enums::StackPosition,
        include_sources: bool,
    ) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        let player_id = handle.player;
        {
            let player = self.player_mut(player_id);
            if (handle.index as usize) >= player.battle_area.len() {
                return false;
            }
        }

        let cause = self.infer_effect_cause(player_id);
        let subject = ReplacementSubject::Permanent(handle);

        let leave_outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            subject,
            cause,
            Some(Zone::Deck),
        );
        if self.pending_selection.is_some() {
            return false;
        }
        let outcome = match leave_outcome {
            ReplacementOutcome::None => self.try_replace(
                EffectTiming::WhenWouldBeReturnedToDeck,
                subject,
                cause,
                Some(Zone::Deck),
            ),
            other => other,
        };
        if self.pending_selection.is_some() {
            return false;
        }

        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return false;
            }
            ReplacementOutcome::Redirected(Zone::Hand) => {
                return self.return_to_hand(handle).is_some();
            }
            ReplacementOutcome::Redirected(Zone::Trash) => {
                self.delete_permanent_with_cause(handle, cause);
                return false;
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected redirect destination for WhenWouldBeReturnedToDeck: {:?}",
                    other
                );
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)) => {
                return self.return_to_deck_inner(other, position, include_sources);
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "non-Permanent substitute subject for WhenWouldBeReturnedToDeck"
                );
            }
        }

        // Snapshot the leaving permanent's identity BEFORE removal so the
        // OnLeaveField observer's `event_target_*` predicates resolve.
        let leave_snapshot = self
            .player(player_id)
            .battle_area
            .get(handle.index as usize)
            .and_then(|p| {
                let top_handle = p.top_card().handle();
                let data = self.card_data_for_handle(top_handle)?;
                let mut digisources: Vec<crate::card_source::CardHandle> = Vec::new();
                for src in p.card_sources.iter() {
                    let h = src.handle();
                    if h != top_handle {
                        digisources.push(h);
                    }
                }
                let source_count = digisources.len();
                let dp_now = self.effective_dp(handle);
                let is_token = p.top_card().is_token;
                let identity = p.synth_identity(&self.card_data, &self.modifiers, handle);
                Some(crate::trigger_context::DeletedObjectSnapshot {
                    former_controller: player_id,
                    top_card: top_handle,
                    card_kind: identity.kind,
                    retained_kind: identity.retained_kind,
                    traits: identity.traits.clone(),
                    level: identity.level,
                    dp: dp_now,
                    cause: match position {
                        crate::enums::StackPosition::Bottom => {
                            crate::trigger_context::EventCause::DeckBottom
                        }
                        _ => crate::trigger_context::EventCause::Return,
                    },
                    dp_just_before: dp_now,
                    level_just_before: identity.level,
                    cost_just_before: Some(data.play_cost),
                    names_just_before: identity.card_names.clone(),
                    traits_just_before: identity.traits.clone(),
                    source_count_just_before: source_count,
                    digisources_just_before: digisources,
                    is_token,
                })
            });

        let mut perm = self
            .player_mut(player_id)
            .battle_area
            .remove(handle.index as usize);
        // `battle_area` is a Vec, so the remove above COMPACTS it and every
        // queued effect whose carrier sat at a LATER slot now addresses the
        // wrong permanent. Re-key them, as the deletion and link paths already
        // do. Without this a SURVIVING carrier reads as dead in
        // `queued_effect_source_is_live` and its MANDATORY trigger is silently
        // dropped -- measured on EX12-036 Ryugumon, where the standing carrier
        // reported index 1 while the battle area held one permanent at index 0.
        // Clear the departing identity and compact every surviving handle
        // BEFORE overflow/source-trash callbacks can rebuild declarative state.
        // A late clear at the former slot would erase the shifted survivor.
        self.clear_permanent_full(handle);
        self.modifiers.expire_player_on_permanent_leave(handle);
        self.modifiers
            .shift_after_battle_area_remove(handle.player, handle.index);
        self.shift_pending_attack_after_battle_area_remove(handle.player, handle.index);
        self.shift_effect_queue_after_battle_area_remove(player_id, handle.index);

        let Some(top) = perm.card_sources.pop() else {
            return false;
        };
        let mut leaving_sources = perm.card_sources.clone();
        leaving_sources.push(top.clone());
        self.apply_ace_overflow_for_sources(&leaving_sources);

        if include_sources {
            perm.card_sources.push(top);
            self.insert_stack_into_owners_decks(perm.card_sources, position);
        } else {
            // Owner-routed: top card returns to its OWN owner's deck.
            // Track E correctness rule. Identical to controller-routed
            // when owner == controller (the common case today); diverges
            // if a future control-transfer effect sets owner != controller.
            let top_owner = top.owner;
            let host_card = top.handle();
            self.insert_card_into_deck(top_owner, top, position);

            // Sources below the top → each source's OWN owner's trash, all
            // at the same time: ONE event (§15-5-2), enqueued without
            // draining (§15-8-3-2; the leave-field observers below belong to
            // the same return).
            let mut trashed = Vec::new();
            for card in perm.card_sources {
                let source_card = card.handle();
                let owner = card.owner;
                self.player_mut(owner).trash.push(card);
                trashed.push(super::TrashedSource {
                    player: handle.player,
                    host: handle,
                    host_card,
                    card: source_card,
                });
            }
            self.enqueue_digivolution_cards_trashed(
                trashed,
                crate::trigger_context::EventCause::Rule,
            );
        }

        let had_linked = !perm.linked_cards.is_empty();
        for card in perm.linked_cards {
            // Owner-routed: linked cards return to their own owner's trash.
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
        }
        // Phase 8 Task 4: fire OnLinkedCardTrashed if the returning host was
        // carrying any linked cards.
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnLinkedCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
        }

        // OnLeaveField: the permanent left the battle area by return-to-deck.
        if let Some(snapshot) = leave_snapshot {
            self.fire_on_leave_field(handle, snapshot);
        } else {
            self.maybe_drain_effect_queue();
        }
        true
    }
}
