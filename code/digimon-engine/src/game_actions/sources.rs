//! Digivolution-source placement operations (Tier 2) — `impl Game`.

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
    pub(crate) fn commit_digixros_material_sources(
        &mut self,
        mut target: PermanentHandle,
    ) -> PermanentHandle {
        let Some(origins) = self
            .pending_digixros_transaction
            .as_ref()
            .map(|transaction| {
                let mut origins = transaction
                    .pre_attached_materials
                    .iter()
                    .chain(transaction.selected_materials.iter())
                    .map(|material| material.origin)
                    .collect::<Vec<_>>();
                origins.sort_by_key(|origin| match origin {
                    DigiXrosMaterialOrigin::BattleArea { permanent, .. } => {
                        (0u8, std::cmp::Reverse(permanent.index))
                    }
                    _ => (1u8, std::cmp::Reverse(0)),
                });
                origins
            })
        else {
            return target;
        };

        for origin in origins {
            let shifts_target_left = matches!(
                origin,
                DigiXrosMaterialOrigin::BattleArea { permanent, .. }
                    if permanent.player == target.player && permanent.index < target.index
            );
            let Some(card) = self.take_digixros_material_origin(origin) else {
                continue;
            };
            if shifts_target_left {
                target.index = target.index.saturating_sub(1);
            }
            if let Some(permanent) = self
                .player_mut(target.player)
                .battle_area
                .get_mut(target.index as usize)
            {
                permanent.push_under(card);
            }
        }

        target
    }

    pub(crate) fn take_card_source_ref(
        &mut self,
        source: crate::enums::CardSourceRef,
    ) -> Option<TakenCardSource> {
        use crate::enums::CardSourceRef;
        let mut face_up_security = None;
        let mut pending_security_defender = None;
        let card = match source {
            CardSourceRef::Hand(p, i) => {
                let player = self.player_mut(p);
                if i >= player.hand.len() {
                    return None;
                }
                player.hand.remove(i)
            }
            CardSourceRef::Trash(p, i) => {
                let player = self.player_mut(p);
                if i >= player.trash.len() {
                    return None;
                }
                player.trash.remove(i)
            }
            CardSourceRef::DeckTop(p) => {
                // Opaque-aware: routes through take_from_deck_top_for_player
                // so opaque opponents materialize from RevealSource. The
                // `?` short-circuits on either deck-out (Ok(None)) or
                // reveal-source error (Err) — both cause the caller to
                // return None (signaling "couldn't take a card").
                match self.take_from_deck_top_for_player(p, crate::opaque_deck::RevealKind::Effect)
                {
                    Ok(Some(c)) => c,
                    Ok(None) => return None,
                    Err(e) => {
                        eprintln!(
                            "[opaque-deck] CardSourceRef::DeckTop take error \
                             for player {}: {}",
                            p, e
                        );
                        return None;
                    }
                }
            }
            CardSourceRef::Security(p, i) => {
                // Opaque-aware: materialize before removing — this is a
                // generic "take from this zone" path used by many
                // effects; downstream routing reads the card's identity.
                if i >= self.player(p).security.len() {
                    return None;
                }
                self.ensure_security_materialized(p, i);
                let player = self.player_mut(p);
                let card = player.security.remove(i);
                if player.face_up_security.remove(&card.card_index) {
                    face_up_security = Some(p);
                }
                card
            }
            CardSourceRef::PendingSecurity => {
                let pending = self.pending_security.take()?;
                if pending.played {
                    self.pending_security = Some(pending);
                    return None;
                }
                pending_security_defender = Some(pending.defender);
                pending.card
            }
            CardSourceRef::Material(h, i) => {
                let perm = self
                    .player_mut(h.player)
                    .battle_area
                    .get_mut(h.index as usize)?;
                if i >= perm.card_sources.len() {
                    return None;
                }
                perm.card_sources.remove(i)
            }
            CardSourceRef::Reveal(h) => {
                let idx = self.revealed_cards.iter().position(|c| c.handle() == h)?;
                let mut taken = self.revealed_cards.remove(idx);
                taken.clear_reveal_overlay();
                taken
            }
        };
        Some(TakenCardSource {
            card,
            restore_face_up_security_for: face_up_security,
            restore_pending_security_for: pending_security_defender,
        })
    }

    pub(crate) fn restore_card_source_ref(
        &mut self,
        source: crate::enums::CardSourceRef,
        taken: TakenCardSource,
    ) -> bool {
        use crate::enums::CardSourceRef;
        let card_index = taken.card.card_index;
        let restored = match source {
            CardSourceRef::Hand(p, i) => {
                let player = self.player_mut(p);
                let idx = i.min(player.hand.len());
                player.hand.insert(idx, taken.card);
                true
            }
            CardSourceRef::Trash(p, i) => {
                let player = self.player_mut(p);
                let idx = i.min(player.trash.len());
                player.trash.insert(idx, taken.card);
                true
            }
            CardSourceRef::DeckTop(p) => {
                self.player_mut(p).deck.push(taken.card);
                true
            }
            CardSourceRef::Security(p, i) => {
                let player = self.player_mut(p);
                let idx = i.min(player.security.len());
                player.security.insert(idx, taken.card);
                true
            }
            CardSourceRef::PendingSecurity => {
                let Some(defender) = taken.restore_pending_security_for else {
                    return false;
                };
                self.pending_security = Some(PendingSecurity {
                    defender,
                    card: taken.card,
                    played: false,
                });
                true
            }
            CardSourceRef::Material(h, i) => {
                let Some(perm) = self
                    .player_mut(h.player)
                    .battle_area
                    .get_mut(h.index as usize)
                else {
                    return false;
                };
                let idx = i.min(perm.card_sources.len());
                perm.card_sources.insert(idx, taken.card);
                true
            }
            CardSourceRef::Reveal(_) => {
                self.revealed_cards.push(taken.card);
                true
            }
        };
        if restored {
            if let Some(player) = taken.restore_face_up_security_for {
                self.player_mut(player).face_up_security.insert(card_index);
            }
        }
        restored
    }

    /// Insert a card at the bottom of `target`'s digivolution stack. The
    /// source card is taken from the zone specified by `source` (hand slot,
    /// trash slot, deck top, security slot, material stack slot, or reveal
    /// pool). Returns false if the source or target is invalid.
    ///
    /// `face_down` sets the inserted `CardSource.face_down` flag (the DCGO
    /// `IsFlipped` analog for digivolution-stack sources). Pass `true` to
    /// stash a face-down source (e.g. a Tamer face-down stash); `false`
    /// preserves the ordinary face-up placement.
    ///
    /// NOTE: `face_down` is honored only for hand / trash / deck-top /
    /// material / reveal sources placed into the breeding or battle area; it
    /// is **not** honored for `CardSourceRef::Security` sources, which are
    /// always placed face-up (DCGO parity).
    pub fn place_as_bottom_source(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        face_down: bool,
    ) -> bool {
        self.place_as_bottom_source_observed(source, target, target.player, face_down)
    }

    /// Insert a card as `target`'s TOP digivolution source — directly
    /// beneath the active top card (`Permanent::push_as_top_source`, DCGO
    /// `AddDigivolutionCardsTop`). The top-position sibling of
    /// `place_as_bottom_source`; identical source zones, `face_down`
    /// semantics (and the Security face-up caveat), and lifecycle.
    /// G-DSL-PLACE-AS-TOP-SOURCE (resolved 2026-07-05).
    pub fn place_as_top_source(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        face_down: bool,
    ) -> bool {
        self.place_as_top_source_observed(source, target, target.player, face_down)
    }

    pub fn place_permanent_as_bottom_sources(
        &mut self,
        source: PermanentHandle,
        target: PermanentHandle,
    ) -> bool {
        if source.index == crate::action::space::BREEDING_TARGET as u8 {
            return false;
        }
        if source == target {
            return false;
        }
        if self
            .player(source.player)
            .battle_area
            .get(source.index as usize)
            .is_none()
        {
            return false;
        }

        let mut adjusted_target = target;
        if target.index == crate::action::space::BREEDING_TARGET as u8 {
            if self.player(target.player).breeding_area.is_none() {
                return false;
            }
        } else {
            if self
                .player(target.player)
                .battle_area
                .get(target.index as usize)
                .is_none()
            {
                return false;
            }
            if source.player == target.player && source.index < target.index {
                adjusted_target.index = adjusted_target.index.saturating_sub(1);
            }
        }

        let removed = self
            .player_mut(source.player)
            .battle_area
            .remove(source.index as usize);
        let cards = removed.card_sources;

        if adjusted_target.index == crate::action::space::BREEDING_TARGET as u8 {
            let Some(breeding) = self
                .player_mut(adjusted_target.player)
                .breeding_area
                .as_mut()
            else {
                return false;
            };
            breeding.card_sources.splice(0..0, cards);
            return true;
        }

        let Some(target_perm) = self
            .player_mut(adjusted_target.player)
            .battle_area
            .get_mut(adjusted_target.index as usize)
        else {
            return false;
        };
        target_perm.card_sources.splice(0..0, cards);
        true
    }

    /// Return one digivolution source from `perm`'s stack to its owner's deck
    /// (bottom when `to_bottom`, else top; Digi-Eggs route to the digitama
    /// deck). This is a *return*, NOT a trash — it does not fire
    /// `OnDigivolutionCardTrashed`. Bottom-route returns fire
    /// `OnDigivolutionCardReturnedToDeckBottom` (BT21-058 / BT18-065's
    /// bottom-scoped observer); top-of-deck returns fire nothing. If removing
    /// the source exposes a Digi-Egg top card, the permanent is deleted (same
    /// exposed-egg cleanup as the trash path).
    ///
    /// Returns `true` when the source handle was found and moved.
    pub(crate) fn return_card_source_to_deck(
        &mut self,
        perm: PermanentHandle,
        card: CardHandle,
        to_bottom: bool,
    ) -> bool {
        let removed = {
            let permanent = match self
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
        let returned_card = removed.handle();

        // Deck routing (deck bottom = index 0; Digi-Eggs -> digitama deck).
        let owner = removed.owner;
        let is_egg = removed.card_kind(&self.card_data) == CardKind::DigiEgg;
        {
            let player = self.player_mut(owner);
            let deck = if is_egg {
                &mut player.digitama_deck
            } else {
                &mut player.deck
            };
            if to_bottom {
                deck.insert(0, removed);
            } else {
                deck.push(removed);
            }
        }

        // Exposed-Digi-Egg cleanup (mirrors the facade trash path).
        let exposed = self
            .player(perm.player)
            .battle_area
            .get(perm.index as usize)
            .is_some_and(|permanent| {
                permanent.top_card().card_kind(&self.card_data) == CardKind::DigiEgg
            });
        if exposed {
            self.delete_permanent_with_effects(perm);
        }

        // G-ENGINE-DIGIVOLUTION-CARD-RETURNED-TO-DECK-BOTTOM: bottom-route
        // observer. The host is the permanent's (still-standing) top card; if
        // the return exposed a Digi-Egg and the permanent was deleted, fall
        // back to the returned card so the observer carries a stable handle.
        if to_bottom {
            let host_card = self
                .player(perm.player)
                .battle_area
                .get(perm.index as usize)
                .map(|permanent| permanent.top_card().handle())
                .unwrap_or(returned_card);
            self.fire_digivolution_card_returned_to_deck_bottom(
                perm.player,
                perm,
                host_card,
                returned_card,
                crate::trigger_context::EventCause::DeckBottom,
            );
        }
        true
    }
}
