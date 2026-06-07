//! Card/handle/provenance-token resolution (Tier 1).

#![allow(unused_imports)]
use super::*;
use crate::aura::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::effect::*;
use crate::enums::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::selection::*;
use crate::trigger_context::*;

impl Game {
    /// Resolve a `CardHandle` to its `&CardData` by scanning all zones —
    /// mirrors `card_kind_for_handle` but returns the full data record so
    /// callers can read name, traits, colors, etc. Used by the DSL predicate
    /// evaluator (`dsl_cards::predicate`).
    ///
    /// Returns `None` if no `CardSource` with the given `card_index` is found.
    pub fn card_data_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> Option<&crate::card_data::CardData> {
        let target_index = handle.0;
        for player in &self.players {
            if let Some(cs) = player.hand.iter().find(|c| c.card_index == target_index) {
                return Some(&self.card_data[cs.data_index]);
            }
            if let Some(cs) = player.trash.iter().find(|c| c.card_index == target_index) {
                return Some(&self.card_data[cs.data_index]);
            }
            for perm in &player.battle_area {
                if let Some(cs) = perm
                    .card_sources
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(&self.card_data[cs.data_index]);
                }
                if let Some(cs) = perm
                    .linked_cards
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(&self.card_data[cs.data_index]);
                }
            }
            if let Some(breeding) = &player.breeding_area {
                if let Some(cs) = breeding
                    .card_sources
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(&self.card_data[cs.data_index]);
                }
            }
            if let Some(cs) = player
                .security
                .iter()
                .find(|c| c.card_index == target_index)
            {
                return Some(&self.card_data[cs.data_index]);
            }
            if let Some(cs) = player.deck.iter().find(|c| c.card_index == target_index) {
                return Some(&self.card_data[cs.data_index]);
            }
        }
        if let Some(cs) = self
            .revealed_cards
            .iter()
            .find(|c| c.card_index == target_index)
        {
            return Some(&self.card_data[cs.data_index]);
        }
        None
    }

    /// Resolve a `CardHandle` to its live `CardSource` instance.
    pub fn card_source_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> Option<&crate::card_source::CardSource> {
        let target_index = handle.0;
        for player in &self.players {
            if let Some(cs) = player.hand.iter().find(|c| c.card_index == target_index) {
                return Some(cs);
            }
            if let Some(cs) = player.trash.iter().find(|c| c.card_index == target_index) {
                return Some(cs);
            }
            for perm in &player.battle_area {
                if let Some(cs) = perm
                    .card_sources
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(cs);
                }
                if let Some(cs) = perm
                    .linked_cards
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(cs);
                }
            }
            if let Some(breeding) = &player.breeding_area {
                if let Some(cs) = breeding
                    .card_sources
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(cs);
                }
            }
            if let Some(cs) = player
                .security
                .iter()
                .find(|c| c.card_index == target_index)
            {
                return Some(cs);
            }
            if let Some(cs) = player.deck.iter().find(|c| c.card_index == target_index) {
                return Some(cs);
            }
        }
        self.revealed_cards
            .iter()
            .find(|c| c.card_index == target_index)
    }

    pub fn provenance_token_for_card(
        &self,
        card: crate::card_source::CardHandle,
    ) -> crate::trigger_context::ProvenanceToken {
        crate::trigger_context::ProvenanceToken::from(card)
    }

    pub fn resolve_provenance_token(
        &self,
        token: crate::trigger_context::ProvenanceToken,
    ) -> Option<crate::trigger_context::EventSubject> {
        if token.0 > u16::MAX as u64 {
            return None;
        }
        let card = crate::card_source::CardHandle(token.0 as u16);
        let target_index = card.0;

        for (player_index, player) in self.players.iter().enumerate() {
            let player_id = player_index as crate::enums::PlayerId;
            for (index, permanent) in player.battle_area.iter().enumerate() {
                if permanent
                    .card_sources
                    .iter()
                    .any(|source| source.card_index == target_index)
                {
                    return Some(crate::trigger_context::EventSubject::Permanent(
                        PermanentHandle {
                            player: player_id,
                            index: index as u8,
                        },
                    ));
                }
                if permanent
                    .linked_cards
                    .iter()
                    .any(|source| source.card_index == target_index)
                {
                    return Some(crate::trigger_context::EventSubject::Card {
                        card,
                        zone: crate::enums::Zone::BattleArea,
                    });
                }
            }
            if let Some(breeding) = &player.breeding_area {
                if breeding
                    .card_sources
                    .iter()
                    .any(|source| source.card_index == target_index)
                {
                    return Some(crate::trigger_context::EventSubject::Permanent(
                        PermanentHandle {
                            player: player_id,
                            index: crate::action::space::BREEDING_TARGET as u8,
                        },
                    ));
                }
            }
            if player
                .hand
                .iter()
                .any(|source| source.card_index == target_index)
            {
                return Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Hand,
                });
            }
            if player
                .trash
                .iter()
                .any(|source| source.card_index == target_index)
            {
                return Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Trash,
                });
            }
            if player
                .security
                .iter()
                .any(|source| source.card_index == target_index)
            {
                return Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Security,
                });
            }
            if player
                .deck
                .iter()
                .any(|source| source.card_index == target_index)
            {
                return Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Deck,
                });
            }
            if player
                .digitama_deck
                .iter()
                .any(|source| source.card_index == target_index)
            {
                return Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::DigitamaDeck,
                });
            }
        }
        if self
            .revealed_cards
            .iter()
            .any(|source| source.card_index == target_index)
        {
            return Some(crate::trigger_context::EventSubject::Card {
                card,
                zone: crate::enums::Zone::Reveal,
            });
        }
        None
    }

    /// Strict variant of [`resolve_provenance_token`] for "is this played card
    /// still a Digimon on the battle area?" identity checks.
    ///
    /// Returns `Some(handle)` only when the card identified by `token` is
    /// currently the **top card** of a battle-area permanent. Yields `None` if
    /// the card is a digivolution card under a different top, has been removed
    /// from play, or is in any other zone (hand, trash, security, deck,
    /// linked_cards, reveal).
    ///
    /// This is the resolution semantic required by play-verb `bind_as`
    /// bindings ([`crate::dsl_cards::bindings::BindingValue::PlayedPermanent`])
    /// consumed by `return_to_hand` and friends after a `schedule_delayed`
    /// boundary. The permissive [`resolve_provenance_token`] — which returns
    /// `Permanent(handle)` for *any* card in *any* permanent's `card_sources` —
    /// matches DCGO's `IsPermanentExistsOnBattleArea(selectedPermanent)` only
    /// for the specific case where the played card is still the carrier's top;
    /// once the played card became a digivolution card the original
    /// `Permanent` object would have been replaced and the check would fail.
    ///
    /// See change `fix-played-binding-uses-provenance` for the cross-engine
    /// rationale and the BT16-085 + Paildramon scenario this exists to handle.
    pub fn resolve_token_as_battle_area_top(
        &self,
        token: crate::trigger_context::ProvenanceToken,
    ) -> Option<PermanentHandle> {
        if token.0 > u16::MAX as u64 {
            return None;
        }
        let target_index = token.0 as u16;
        for (player_index, player) in self.players.iter().enumerate() {
            for (index, permanent) in player.battle_area.iter().enumerate() {
                if permanent.top_card().card_index == target_index {
                    return Some(PermanentHandle {
                        player: player_index as crate::enums::PlayerId,
                        index: index as u8,
                    });
                }
            }
        }
        None
    }
}
