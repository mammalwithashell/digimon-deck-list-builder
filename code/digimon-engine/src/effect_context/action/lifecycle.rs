//! Permanent lifecycle / misc mutations on `EffectContext` — extracted by mechanic.

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
    pub(crate) fn schedule_provenance_deletion(&mut self, permanent: PermanentHandle, opponents_turn: bool) {
        let Some(top) = self
            .game
            .player(permanent.player)
            .battle_area
            .get(permanent.index as usize)
            .map(|perm| perm.top_card().handle())
        else {
            return;
        };
        let token = self.game.provenance_token_for_card(top);
        let entry = crate::scheduled_effects::ScheduledProvenanceDeletion {
            token,
            controller: self.player,
        };
        if opponents_turn {
            self.game.scheduled_provenance_deletions_opp.push(entry);
        } else {
            self.game.scheduled_provenance_deletions.push(entry);
        }
    }

    /// Decline the pay cost for a queued triggered effect that parked during
    /// `pay_cost_fn`. The effect queue will discard the parked process tail
    /// after the current selection callback unwinds.
    pub fn decline_pending_pay_cost(&mut self) {
        self.game.decline_pending_pay_cost();
    }

    pub fn delete_permanent(&mut self, target: PermanentHandle) {
        if !self.can_affect_permanent(target) {
            return;
        }
        // Route through the Game-level fire-site so OnDeletion observers and
        // WhenWouldBeDeleted replacements run. `delete_permanent_with_effects`
        // infers cause from `effect_source_player` / `pending_attack` /
        // `security_resolution`.
        self.game.delete_permanent_with_effects(target);
    }

    /// Pay the source permanent's return-self-to-deck-bottom activation
    /// cost.
    ///
    /// Used as the closure body for
    /// [`crate::effect::EffectBuilder::activation_cost`] on Tamer
    /// triggered abilities like "By returning this Tamer to the bottom
    /// of the deck..." (BT22-088 / BT22-094 / BT17-093 / EX11-071
    /// family). Moves the top card of the source permanent to the
    /// controller's deck bottom, trashes the rest of the digivolution
    /// stack per standard return-to-deck rules, and fires
    /// `OnLeaveField`. Returns `false` if the source permanent is gone
    /// (extremely unlikely mid-trigger but possible if a prior chain
    /// destroyed it).
    pub fn return_self_to_deck_bottom_as_cost(&mut self) -> bool {
        let Some(handle) = self.source_permanent else {
            return false;
        };
        if self.source_permanent().is_none() {
            return false;
        }
        // Use the top-card-only return path: the source's top card moves
        // to its owner's deck bottom; any remaining digivolution sources
        // are trashed by `Game::return_to_deck`. Mirrors the
        // `return_to_deck { include_sources: false, position: bottom }`
        // DSL step shape applied to `source`.
        self.game
            .return_to_deck(handle, crate::enums::StackPosition::Bottom)
    }

    /// Drain `player`'s trash and append each card to its **owner's** deck
    /// bottom. Returns the handles of moved cards in their original trash
    /// order. Track E Tier 2 Task 7 — bulk move primitive used by printed
    /// text like BT17-077 Imperialdramon: Paladin Mode "return all cards
    /// in your trash to the bottom of the deck."
    ///
    /// Owner-routed: each card consults its `CardSource.owner` field, not
    /// the `player` parameter. In the common case where every card in a
    /// player's trash was originally owned by that player, this is a pure
    /// drain into the same player's deck. In the cross-player case (a card
    /// was effect-moved into the opposing trash by a prior effect), each
    /// card returns to its original owner's deck — matching the rules-default
    /// behavior for cards moving between owners' zones.
    ///
    /// Does NOT fire `OnReturn` per card — the existing engine doesn't have
    /// a `Game::return_to_deck` per-card observer dispatch and the printed
    /// cards consuming this primitive bind the moved set as an ordered set
    /// for downstream predicates rather than per-card observation. Treated
    /// as a bulk move; per-card observer fan-out can land as a follow-up.
    pub fn return_all_trash_to_deck_bottom(&mut self, player: PlayerId) -> Vec<CardHandle> {
        // Drain trash in order. Each card is appended to the start of its
        // owner's deck (deck bottom = index 0 by convention; deck top =
        // Vec end, the position drawn from first).
        let drained: Vec<crate::card_source::CardSource> =
            std::mem::take(&mut self.game.player_mut(player).trash);
        let mut handles = Vec::with_capacity(drained.len());
        for card in drained {
            handles.push(card.handle());
            let owner = card.owner;
            self.game.player_mut(owner).deck.insert(0, card);
        }
        handles
    }
}
