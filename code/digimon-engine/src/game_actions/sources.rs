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
    /// Remove the digivolution source at `source_index` from `target`'s stack
    /// and trash it to its own owner's trash, firing
    /// `OnDigivolutionCardTrashed` once for it with the standard effect cause.
    ///
    /// The single Tier-2 "remove one below-top source and trash+fire" operation
    /// for the facade's `trash_bottom_face_down_source`. The host card is read
    /// AFTER the removal (the permanent keeps its top, so it is unchanged by the
    /// removal). The caller validated `target` / `source_index` and the trash
    /// protections; returns `false` (no mutation) if either is stale.
    pub(crate) fn trash_source_at_index_and_fire(
        &mut self,
        target: PermanentHandle,
        source_index: usize,
    ) -> bool {
        let Some(permanent) = self
            .player_mut(target.player)
            .battle_area
            .get_mut(target.index as usize)
        else {
            return false;
        };
        // Never the top card: the permanent must keep its own top (a source is
        // always below it).
        if source_index + 1 >= permanent.card_sources.len() {
            return false;
        }
        let removed = permanent.card_sources.remove(source_index);
        let host_card = permanent.top_card().handle();
        self.trash_source_and_fire(removed.owner, target, removed, host_card);
        true
    }

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

        let host = self.player(target.player).battle_area[target.index as usize]
            .top_card().handle();
        for origin in origins {
            // Leave replacements can relocate selected materials. Consumption
            // already resolves their physical card identity; relocate the host
            // by identity too, rather than comparing obsolete field indices.
            // The entering host itself is never one of its own materials.
            if origin.card() == host {
                continue;
            }
            let Some(card) = self.take_digixros_material_origin(origin) else {
                continue;
            };
            target.index = self.player(target.player)
                .battle_area
                .iter().position(|p| p.top_card().handle() == host)
                .expect("consuming another DigiXros material preserves the host") as u8;
            self.player_mut(target.player).battle_area[target.index as usize].push_under(card);
        }

        target
    }

    /// Locate `h` in any zone an effect can lift a card from: hand, trash,
    /// security, a battle-area stack (top card included), a link card, the
    /// reveal pool or the pending security card.
    pub(crate) fn locate_card_source_ref(
        &self,
        h: CardHandle,
    ) -> Option<crate::enums::CardSourceRef> {
        use crate::enums::CardSourceRef;
        for pid in 0..self.players.len() {
            let player_id = pid as PlayerId;
            let player = self.player(player_id);
            if let Some(idx) = player.hand.iter().position(|c| c.handle() == h) {
                return Some(CardSourceRef::Hand(player_id, idx));
            }
            if let Some(idx) = player.trash.iter().position(|c| c.handle() == h) {
                return Some(CardSourceRef::Trash(player_id, idx));
            }
            if let Some(idx) = player.security.iter().position(|c| c.handle() == h) {
                return Some(CardSourceRef::Security(player_id, idx));
            }
            for (perm_idx, perm) in player.battle_area.iter().enumerate() {
                let host = PermanentHandle {
                    player: player_id,
                    index: perm_idx as u8,
                };
                if let Some(src_idx) = perm.card_sources.iter().position(|c| c.handle() == h) {
                    return Some(CardSourceRef::Material(host, src_idx));
                }
                if let Some(link_idx) = perm.linked_cards.iter().position(|c| c.handle() == h) {
                    return Some(CardSourceRef::Link(host, link_idx));
                }
            }
        }
        if self.revealed_cards.iter().any(|c| c.handle() == h) {
            return Some(CardSourceRef::Reveal(h));
        }
        if self
            .pending_security
            .as_ref()
            .filter(|pending| !pending.played)
            .is_some_and(|pending| pending.card.handle() == h)
        {
            return Some(CardSourceRef::PendingSecurity);
        }
        None
    }

    /// Lift the card `h` out of whatever zone it is in (see
    /// [`Game::locate_card_source_ref`]); `None` if it can't be located.
    pub(crate) fn take_card_by_handle(&mut self, h: CardHandle) -> Option<CardSource> {
        let src = self.locate_card_source_ref(h)?;
        self.take_card_source_ref(src).map(|t| t.card)
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
            CardSourceRef::Link(h, i) => {
                let perm = self
                    .player_mut(h.player)
                    .battle_area
                    .get_mut(h.index as usize)?;
                if i >= perm.linked_cards.len() {
                    return None;
                }
                perm.linked_cards.remove(i)
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
                player.insert_in_hand(idx, taken.card);
                true
            }
            CardSourceRef::Trash(p, i) => {
                let player = self.player_mut(p);
                let idx = i.min(player.trash.len());
                player.insert_in_trash(idx, taken.card);
                true
            }
            CardSourceRef::DeckTop(p) => {
                self.player_mut(p).put_on_deck_top(taken.card);
                true
            }
            CardSourceRef::Security(p, i) => {
                let player = self.player_mut(p);
                let idx = i.min(player.security.len());
                player.restore_in_security(idx, taken.card);
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
            CardSourceRef::Link(h, i) => {
                let Some(perm) = self
                    .player_mut(h.player)
                    .battle_area
                    .get_mut(h.index as usize)
                else {
                    return false;
                };
                let idx = i.min(perm.linked_cards.len());
                perm.linked_cards.insert(idx, taken.card);
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

    /// Place the permanent `source` under `target` as its bottom
    /// digivolution card. The source permanent leaves the battle area
    /// (CR 4.3 §4-7-7): only its top card moves; the cards under it and any
    /// linked cards are trashed at the same time (§4-7-8), as one event
    /// (§15-5-2).
    ///
    /// Routed through the field-top source move
    /// (`begin_field_top_source_move`, `field_top_source_move.rs`), so the
    /// donor's `WhenWouldLeaveBattleArea` replacement window opens before it
    /// is removed (e.g. BT24-101 Jupitermon's "[TS] Digimon or Tamers ...
    /// don't leave"; an optional replacement parks a prompt and the move
    /// resumes when it resolves), and the [On Leave] observers see the
    /// departure with a pre-removal snapshot (`fire_on_leave_field`). DCGO
    /// `Permanent.AddDigivolutionCardsBottom` → `IPlacePermanentToDigivolutionCards`
    /// (`willBeRemoveField`, `WhenRemoveField`, `OnLeaveFieldAnyone`,
    /// `DiscardEvoRoots`). Returns `true` only when the placement completed
    /// synchronously. G-ENGINE-PLACE-PERMANENT-AS-SOURCE-NO-LEAVE-WINDOW.
    pub fn place_permanent_as_bottom_sources(
        &mut self,
        source: PermanentHandle,
        target: PermanentHandle,
    ) -> bool {
        let Some(source_ref) = self.field_top_material_ref(source) else {
            return false;
        };
        self.place_as_bottom_source(source_ref, target, false)
    }

    /// The `CardSourceRef` addressing the TOP card of the battle-area
    /// permanent `source` — the form every "place this permanent as a
    /// digivolution card" placement takes, so it routes into the field-top
    /// source move (`place_as_source_observed`). `None` for the breeding
    /// area or an empty / missing slot.
    pub(crate) fn field_top_material_ref(
        &self,
        source: PermanentHandle,
    ) -> Option<crate::enums::CardSourceRef> {
        if source.index == crate::action::space::BREEDING_TARGET as u8 {
            return None;
        }
        let top = self
            .players
            .get(source.player as usize)?
            .battle_area
            .get(source.index as usize)?
            .card_sources
            .len()
            .checked_sub(1)?;
        Some(crate::enums::CardSourceRef::Material(source, top))
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
        self.player_mut(owner)
            .put_in_deck_or_digi_egg_deck(removed, is_egg, to_bottom);
        if !is_egg {
            self.note_effect_deck_add_if_resolving();
        }

        // The source has left the stack — refresh materialized declaratives so
        // continuous grants sourced from the departed card stop applying
        // immediately (same contract as `fire_digivolution_card_trashed`).
        // The top-of-deck route fires no observer, so this is its only
        // recomputation before the next decode-boundary tick.
        self.tick_declarative_effects();

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

#[cfg(test)]
mod digixros_relocated_material_tests {
    use super::*;
    use crate::debug_runner::{make_test_card, DebugRunner};

    #[test]
    fn commit_tracks_host_after_material_original_slot_has_moved() {
        let mut r=DebugRunner::builder().add_card(make_test_card("MATERIAL","Material"))
            .add_card(make_test_card("OTHER","Other"))
            .add_card(make_test_card("HOST","Host")).build();
        r.place_stack(0,&["MATERIAL"]);r.place_stack(0,&["OTHER"]);r.place_stack(0,&["HOST"]);
        let material=r.game.players[0].battle_area[0].top_card().handle();
        let host=r.game.players[0].battle_area[2].top_card().handle();
        let mut transaction=DigiXrosTransaction::new(host,0,0,vec![]);
        transaction.pre_attach_extra_material(DigiXrosMaterialOrigin::BattleArea{
            permanent:PermanentHandle{player:0,index:2},card:material},0);
        r.game.pending_digixros_transaction=Some(transaction);
        let mut cloned=r.game.clone();
        for game in [&mut r.game,&mut cloned] {
            let entered=game.commit_digixros_material_sources(PermanentHandle{player:0,index:2});
            assert_eq!(entered.index,1);
            assert_eq!(game.players[0].battle_area.len(),2);
            let stack=&game.players[0].battle_area[entered.index as usize].card_sources;
            assert_eq!(stack.len(),2);assert_eq!(stack[0].handle(),material);assert_eq!(stack[1].handle(),host);
            assert_eq!(game.players[0].battle_area[0].top_card().card_id(&game.card_data),"OTHER");
        }
    }
}

#[cfg(test)]
mod place_permanent_as_source_leave_window_tests {
    //! G-ENGINE-PLACE-PERMANENT-AS-SOURCE-NO-LEAVE-WINDOW: placing a whole
    //! permanent as a digivolution card is a leave-the-battle-area event
    //! (CR 4.3 §4-7-7), so the donor's would-leave replacements get a window.
    use crate::action::space::{PASS, REPLACEMENT_ACCEPT};
    use crate::card_source::CardHandle;
    use crate::debug_runner::{make_test_card, DebugRunner};
    use crate::effect::{CardEffect, Effect};
    use crate::permanent::PermanentHandle;

    struct Protect;
    impl CardEffect for Protect {
        fn effects(&self, c: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_leave_battle_area(c)
                .optional()
                .replacement_process(|ctx| ctx.cancel())
                .build()]
        }
    }

    fn parked() -> DebugRunner {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("HOST", "Host"))
            .add_card(make_test_card("DONOR", "Donor"))
            .deck(0, &["HOST"; 8])
            .deck(1, &["HOST"; 8])
            .start();
        r.register_effect("DONOR", std::sync::Arc::new(Protect));
        r.skip_mulligan();
        r.place_on_field(0, "HOST", Some(0));
        r.place_on_field(0, "DONOR", Some(0));
        let placed = r.game.place_permanent_as_bottom_sources(
            PermanentHandle { player: 0, index: 1 },
            PermanentHandle { player: 0, index: 0 },
        );
        assert!(!placed, "the placement parks on the replacement prompt");
        assert!(r.pending_kind().is_some(), "the would-leave window opened");
        assert_eq!(r.game.pending_field_top_source_moves.len(), 1);
        r
    }

    fn host_ids(r: &DebugRunner) -> Vec<String> {
        r.game.players[0].battle_area[0]
            .card_sources
            .iter()
            .map(|c| c.card_id(&r.game.card_data).to_string())
            .collect()
    }

    #[test]
    fn accepting_the_replacement_keeps_the_permanent_on_the_field() {
        let mut r = parked();
        r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
        assert_eq!(r.game.players[0].battle_area.len(), 2, "the donor stays");
        assert_eq!(host_ids(&r), vec!["HOST"]);
        assert!(r.game.pending_field_top_source_moves.is_empty());
    }

    #[test]
    fn declining_the_replacement_completes_the_placement() {
        let mut r = parked();
        r.execute_action(0, PASS).unwrap();
        assert_eq!(r.game.players[0].battle_area.len(), 1, "the donor left");
        assert_eq!(host_ids(&r), vec!["DONOR", "HOST"]);
        assert!(r.game.pending_field_top_source_moves.is_empty());
    }
}
