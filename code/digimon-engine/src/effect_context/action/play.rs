//! Play / put-into-play mutations on `EffectContext` — extracted by mechanic.

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
    /// Materialize a token on `controller`'s battle area.
    ///
    /// Looks up `token_name` in `game.token_registry`, synthesizes a
    /// `CardSource` with `is_token = true`, wraps it in a `Permanent`, and
    /// pushes onto `controller.battle_area`. No play cost and no token
    /// OnPlay drain, but entered-field observers fire with `effect_initiated`
    /// so cards that watch effects playing Tokens can see the new permanent.
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

        // `CannotPlayDigimonByEffect` (e.g. BT9-033 Pillomon "Players can't play
        // Digimon by effects") gates token plays too: a Token is a Digimon (every
        // registered token is a Digimon token — see `token_registry`), and DCGO
        // routes `PlayToken` through `CanPlayAsNewPermanent` →
        // `CanNotPutFieldClass(IsDigimon || IsDigiEgg)`, which blocks Digimon
        // tokens under this lock. Mirror the hand/trash play-gate
        // (`Game::play_from_hand_with_cost`) so effect-played tokens are blocked
        // when the controller carries the modifier. (G-PLAY-TOKEN-FLOODGATE.)
        if self
            .game
            .modifiers
            .player_has(controller, crate::enums::ModifierType::CannotPlayDigimonByEffect)
        {
            return None;
        }

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
        let entered = PermanentHandle {
            player: controller,
            index: idx as u8,
        };
        let entered_card = self.game.players[controller as usize].battle_area[idx]
            .top_card()
            .handle();
        let top_card = self.game.players[controller as usize].battle_area[idx].top_card();
        let emitted_card_id = top_card.card_id(&self.game.card_data).to_string();
        let cost_printed = self.game.card_data[top_card.data_index].play_cost as i16;
        let seq = self.game.next_event_seq();
        self.game.events.push(crate::events::GameEvent::Play {
            seq,
            player: controller,
            card_id: emitted_card_id,
            field_index: idx as u8,
            // Token spawn — no memory paid; tokens have play_cost=0
            // in CardData typically but read it explicitly to handle
            // any future token whose printed cost differs.
            cost_paid: 0,
            cost_printed,
            via_alt_path: None,
        });
        self.game.enqueue_triggered(
            crate::enums::EffectTiming::OnEnterFieldAnyone,
            crate::selection::TriggerSource::EnteredField {
                player: controller,
                permanent: entered,
                card: entered_card,
                effect_initiated: true,
            },
        );
        self.game.enqueue_triggered(
            crate::enums::EffectTiming::OnAllyPlayed,
            crate::selection::TriggerSource::EnteredField {
                player: controller,
                permanent: entered,
                card: entered_card,
                effect_initiated: true,
            },
        );
        self.game.drain_effect_queue();
        self.game.mark_until_condition_dirty();
        self.game.reevaluate_until_condition_modifiers_if_dirty();
        Some(entered)
    }

    /// Play a specific revealed card without paying its cost.
    ///
    /// Uses the same hand-transit play machinery as security/material plays so
    /// field capacity, effect-play floodgates, would-play replacement prompts,
    /// On Play dispatch, and entered-field broadcasts remain centralized.
    /// If the play fails synchronously, the card is restored to the reveal pool
    /// at its original position.
    pub fn play_from_reveal_free(
        &mut self,
        player: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> Option<PermanentHandle> {
        let reveal_index = self
            .game
            .revealed_cards
            .iter()
            .position(|revealed| revealed.handle() == card)?;
        let mut taken = self.game.revealed_cards.remove(reveal_index);
        taken.clear_reveal_overlay();
        self.game.player_mut(player).hand.push(taken);
        let hand_index = self.game.player(player).hand.len() - 1;

        match self.game.play_from_hand_with_cost_result_from_origin(
            player,
            hand_index,
            crate::enums::CostDelta::Free,
            PlaySource::ByEffect,
            false,
            PendingWouldPlayOrigin::Reveal {
                index: reveal_index,
            },
        ) {
            PlayFromHandCostResult::Played(field_index) => Some(PermanentHandle {
                player,
                index: field_index as u8,
            }),
            PlayFromHandCostResult::Pending => None,
            PlayFromHandCostResult::Failed => {
                let card = self
                    .game
                    .player_mut(player)
                    .hand
                    .pop()
                    .expect("invariant: revealed card was just pushed to hand");
                let insert_at = reveal_index.min(self.game.revealed_cards.len());
                self.game.revealed_cards.insert(insert_at, card);
                None
            }
        }
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
        let field_index = self.game.play_from_hand_with_cost(
            player,
            hand_index,
            cost_delta,
            PlaySource::ByEffect,
        )?;
        Some(PermanentHandle {
            player,
            index: field_index as u8,
        })
    }

    /// Play a card from `player`'s hand at `hand_index` **without subtracting
    /// memory**. Used by effects that say "play this without paying its memory
    /// cost" (e.g. DSL `PlayFromHandFree` step lowerings).
    ///
    /// Thin alias over `play_from_hand_with_cost(_, _, CostDelta::Free)`:
    /// `CostDelta::Free.resolve(_) == 0` → `effective_cost = 0` →
    /// `pay_memory(0)` is a no-op, so memory is unchanged. OnPlay +
    /// OnEnterFieldAnyone triggers fire as normal.
    ///
    /// Returns the `PermanentHandle` of the new field permanent, or `None` if
    /// the hand index is invalid, the battle area is full, or the play was
    /// gated by a flood-gate (`CannotPlayDigimonByEffect`).
    pub fn play_from_hand_free(
        &mut self,
        player: PlayerId,
        hand_index: usize,
    ) -> Option<PermanentHandle> {
        self.play_from_hand_with_cost(player, hand_index, crate::enums::CostDelta::Free)
    }

    pub fn play_from_hand_free_suppress_on_play(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        match self
            .game
            .play_from_hand_with_cost_result_from_origin_suppress(
                player,
                hand_index,
                crate::enums::CostDelta::Free,
                PlaySource::ByEffect,
                false,
                PendingWouldPlayOrigin::Hand,
                suppress_on_play,
            ) {
            PlayFromHandCostResult::Played(field_index) => Some(PermanentHandle {
                player,
                index: field_index as u8,
            }),
            PlayFromHandCostResult::Pending | PlayFromHandCostResult::Failed => None,
        }
    }

    pub fn play_from_trash_with_cost_suppress_on_play(
        &mut self,
        player: PlayerId,
        trash_index: usize,
        cost_delta: crate::enums::CostDelta,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        let field_index = self.game.play_from_trash_with_cost_suppress(
            player,
            trash_index,
            cost_delta,
            PlaySource::ByEffect,
            suppress_on_play,
        )?;
        Some(PermanentHandle {
            player,
            index: field_index as u8,
        })
    }

    pub fn play_from_hand_free_with_provenance(
        &mut self,
        player: PlayerId,
        hand_index: usize,
    ) -> Option<(PermanentHandle, crate::trigger_context::ProvenanceToken)> {
        let card = self.game.player(player).hand.get(hand_index)?.handle();
        let token = self.game.provenance_token_for_card(card);
        let permanent = self.play_from_hand_free(player, hand_index)?;
        Some((permanent, token))
    }

    /// USE an Option card from `player`'s hand at `hand_index`, deducting
    /// memory according to `cost_delta`. The full Option lifecycle runs
    /// (OnUseOption, OptionMain / mode selection, subtype disposal). The
    /// Option-USE analogue of [`Self::play_from_hand_with_cost`].
    ///
    /// The Main-phase gate is LIFTED for the duration of this call — an
    /// effect that says "play or use 1 … card" may fire from any timing (e.g.
    /// BT25-041's `[When Attacking]`), so the use must be legal outside the
    /// Main phase. `G-PLAY-OR-USE-FROM-HAND`.
    ///
    /// Returns the `OptionPlayResult`. `Pending` means the Option's body (or
    /// a dual-mode mode-select) installed a `PendingSelection` the caller must
    /// drive — the effect-driven phase lift persists across that re-entry.
    pub fn use_option_from_hand_with_cost(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
    ) -> crate::selection::OptionPlayResult {
        self.game
            .use_option_from_hand_with_cost(player, hand_index, cost_delta)
    }

    /// Unified "**play or use** 1 card from hand with `cost_delta` applied"
    /// entry point (Aces / BEATBREAK printed wording). Inspects the hand
    /// card's `CardKind` and routes:
    ///
    /// - `Digimon` / `Tamer` / `DigiEgg` → [`Self::play_from_hand_with_cost`]
    ///   (the card is PLAYED to the battle area at the reduced cost).
    /// - `Option` → [`Self::use_option_from_hand_with_cost`] (the card is USED
    ///   at the reduced use cost, full Option lifecycle).
    /// - `Dual` → because a DUAL card can enter play as a Digimon OR be used as
    ///   an Option, the face is a player CHOICE that is part of the use (no
    ///   auto-selection — §17). A `SelectionKind::EffectChoice` with labels
    ///   `["Play as Digimon", "Use as Option"]` is surfaced; the callback then
    ///   routes to the play or use path. This call returns immediately after
    ///   installing the prompt; the caller must drive the resulting selection.
    ///
    /// `G-PLAY-OR-USE-FROM-HAND`. Used by ST23-04 / ST23-08 / BT25-041
    /// ("you may play or use 1 [Glowing Dawn] trait card from your hand with
    /// the cost reduced by 3").
    pub fn play_or_use_from_hand_with_cost(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
    ) {
        let Some(card) = self.game.player(player).hand.get(hand_index) else {
            return;
        };
        match card.card_kind(&self.game.card_data) {
            crate::enums::CardKind::Digimon
            | crate::enums::CardKind::Tamer
            | crate::enums::CardKind::DigiEgg => {
                let _ = self.play_from_hand_with_cost(player, hand_index, cost_delta);
            }
            crate::enums::CardKind::Option => {
                let _ = self.use_option_from_hand_with_cost(player, hand_index, cost_delta);
            }
            crate::enums::CardKind::Token => {
                // A Token never lives in hand; nothing to play or use.
            }
            crate::enums::CardKind::Dual => {
                self.select_effect_choice(
                    "Play as Digimon or use as Option",
                    vec![
                        "Play as Digimon".to_string(),
                        "Use as Option".to_string(),
                    ],
                    move |cb_ctx, choice| {
                        // Re-resolve the hand index defensively (the hand has
                        // not mutated between installing the prompt and its
                        // resolution within this effect, so `hand_index` is
                        // stable, but guard against an empty/OOB hand).
                        if cb_ctx.game.player(player).hand.get(hand_index).is_none() {
                            return;
                        }
                        match choice {
                            // 0 → Play as Digimon.
                            0 => {
                                let _ = cb_ctx
                                    .play_from_hand_with_cost(player, hand_index, cost_delta);
                            }
                            // 1 → Use as Option.
                            _ => {
                                let _ = cb_ctx
                                    .use_option_from_hand_with_cost(player, hand_index, cost_delta);
                            }
                        }
                    },
                );
            }
        }
    }

    /// Play the top card of `player`'s security stack **without paying
    /// memory**. Used by effects that say "play the top card of your
    /// security stack" (e.g. BT12-091; Phase 2f1 DSL `PlayFromSecurity`
    /// step lowerings).
    ///
    /// Distinct from [`Self::play_pending_security`] (the security-skill
    /// replay path that consumes the transient `Game.pending_security`
    /// during the attack-time security check). This method operates on the
    /// player's persistent `security` zone.
    ///
    /// ## Implementation strategy: hand-transit
    ///
    /// `Game::play_from_hand_with_cost(player, hand_index, CostDelta::Free)`
    /// already encapsulates the full placement path — battle-area capacity
    /// check, `CannotPlayDigimonByEffect` gate, `Permanent::new`, OnPlay
    /// trigger drain, OnEnterFieldAnyone broadcast, `Play` event emission.
    /// Re-introducing the placement body here would duplicate that logic
    /// and risk drift. Instead: pop the top of `player.security`, push it
    /// to the end of `player.hand`, and route through `play_from_hand_free`
    /// at that index. The card spends one tick in hand but never as a
    /// player-visible state — the hand is mutated and consumed inside this
    /// single method call before any selection prompt or event handler can
    /// observe it. The behavior is identical to the spec's suggested
    /// `place_card_in_battle_area` helper without forcing an engine-wide
    /// refactor of `play_from_hand_with_cost` to extract one.
    ///
    /// On rollback (battle area full, flood-gate, etc.) the card is
    /// restored to the top of `security` so this method is a clean no-op
    /// on failure — matching the precedent set by `play_from_hand_free`,
    /// which does not corrupt state on flood-gate-rejected plays.
    ///
    /// Also clears the popped card's entry from `face_up_security` —
    /// `face_up_security` is keyed by `card_index`, and a played card no
    /// longer lives in the security zone, so leaving the bit set would
    /// pollute the tensor's face-up bookkeeping.
    ///
    /// Returns the `PermanentHandle` of the new field permanent, or `None`
    /// if security is empty, the battle area is full, or the play was
    /// gated by a flood-gate.
    pub fn play_from_security(&mut self, player: PlayerId) -> Option<PermanentHandle> {
        let security_index = self
            .game
            .player(player)
            .security
            .iter()
            .position(|card| card.handle() == self.source_card)
            .or_else(|| self.game.player(player).security.len().checked_sub(1))?;
        self.play_from_security_index(player, security_index)
    }

    /// Play a SPECIFIC card from `player`'s security stack — identified by
    /// its `CardHandle` — without paying its cost. Used by DSL clauses that
    /// `select_security` a card and then play exactly that bound card
    /// (e.g. BT13-012 "search your security stack, and you may play 1 red
    /// or yellow Tamer card among it without paying its cost").
    ///
    /// Unlike `play_from_security` (which plays the trigger-context card or
    /// the security top), this resolves the security index of the bound
    /// handle and routes through the same `play_from_security_index`
    /// hand-transit path. Returns `None` if the handle is not in `player`'s
    /// security zone or the play is gated / fails.
    pub fn play_from_security_card(
        &mut self,
        player: PlayerId,
        card: CardHandle,
    ) -> Option<PermanentHandle> {
        let security_index = self
            .game
            .player(player)
            .security
            .iter()
            .position(|c| c.handle() == card)?;
        self.play_from_security_index(player, security_index)
    }

    /// Play a specific card from the transient reveal pool without paying its
    /// memory cost. Used by effects like EX8-050 that reveal cards, allow one
    /// revealed card to be played, then move the remainder elsewhere.
    ///
    /// This mirrors the established security/material hand-transit strategy:
    /// remove the card from `revealed_cards`, park it at the end of `player`'s
    /// hand, and route through the normal play pipeline so floodgates,
    /// OnPlay, OnEnterField, and would-play replacement hooks stay aligned
    /// with every other effect-initiated play. The card is restored to the
    /// reveal pool if the play is immediately rejected or later cancelled.
    pub fn play_from_revealed_free(
        &mut self,
        player: PlayerId,
        card: CardHandle,
    ) -> Option<PermanentHandle> {
        self.play_from_revealed_with_cost(player, card, crate::enums::CostDelta::Free)
    }

    /// Reduced-cost sibling of [`play_from_revealed_free`]: play a specific card
    /// from the reveal pool paying `cost_delta` (e.g. `CostDelta::Reduce(5)` for
    /// "with the cost reduced by 5"), routing through the same hand-transit play
    /// pipeline. The underlying `play_from_hand_with_cost_result_from_origin`
    /// already accepts any `CostDelta`; the free variant just pins it to `Free`.
    /// G-DSL-PLAY-FROM-REVEALED-COST-REDUCED (BT25-074 shape).
    pub fn play_from_revealed_with_cost(
        &mut self,
        player: PlayerId,
        card: CardHandle,
        cost_delta: crate::enums::CostDelta,
    ) -> Option<PermanentHandle> {
        let reveal_index = self
            .game
            .revealed_cards
            .iter()
            .position(|revealed| revealed.handle() == card)?;
        let mut revealed = self.game.revealed_cards.remove(reveal_index);
        revealed.clear_reveal_overlay();

        self.game.player_mut(player).hand.push(revealed);
        let hand_index = self.game.player(player).hand.len() - 1;

        match self.game.play_from_hand_with_cost_result_from_origin(
            player,
            hand_index,
            cost_delta,
            PlaySource::ByEffect,
            false,
            PendingWouldPlayOrigin::Reveal {
                index: reveal_index,
            },
        ) {
            PlayFromHandCostResult::Played(field_index) => Some(PermanentHandle {
                player,
                index: field_index as u8,
            }),
            PlayFromHandCostResult::Pending => None,
            PlayFromHandCostResult::Failed => {
                let card = self
                    .game
                    .player_mut(player)
                    .hand
                    .pop()
                    .expect("invariant: revealed card was just pushed to hand");
                let insert_at = reveal_index.min(self.game.revealed_cards.len());
                self.game.revealed_cards.insert(insert_at, card);
                None
            }
        }
    }

    pub(crate) fn play_from_security_index(
        &mut self,
        player: PlayerId,
        security_index: usize,
    ) -> Option<PermanentHandle> {
        // Opaque-aware: materialize before play — a played card must
        // have a real data_index for cost/effect resolution to work.
        if security_index >= self.game.player(player).security.len() {
            return None;
        }
        self.game
            .ensure_security_materialized(player, security_index);
        let card = {
            let player_state = self.game.player_mut(player);
            if security_index >= player_state.security.len() {
                return None;
            }
            player_state.security.remove(security_index)
        };

        // `face_up_security` is keyed by card_index — clear it whether or
        // not the card was face-up; remove() is a no-op when absent.
        let card_index = card.card_index;
        let was_face_up = self
            .game
            .player_mut(player)
            .face_up_security
            .remove(&card_index);

        // Park at end of hand and play through the established hand-free
        // path. The hand index is the new last position.
        self.game.player_mut(player).hand.push(card);
        let hand_index = self.game.player(player).hand.len() - 1;

        match self.game.play_from_hand_with_cost_result_from_origin(
            player,
            hand_index,
            crate::enums::CostDelta::Free,
            PlaySource::ByEffect,
            false,
            PendingWouldPlayOrigin::SecurityTop { was_face_up },
        ) {
            PlayFromHandCostResult::Played(field_index) => Some(PermanentHandle {
                player,
                index: field_index as u8,
            }),
            PlayFromHandCostResult::Pending => None,
            PlayFromHandCostResult::Failed => {
                // Rollback: pop the card back out of hand and restore it to
                // the top of security so the failure is observable as a
                // no-op. Restore face_up_security entry too in case the
                // caller depended on it.
                let card = self
                    .game
                    .player_mut(player)
                    .hand
                    .pop()
                    .expect("invariant: card was just pushed to hand");
                // Note: we deliberately do NOT re-insert into
                // face_up_security on rollback — the card is back in the
                // security zone but its visibility-state was already
                // consumed by the abortive play attempt. Matches the
                // tradeoff `play_from_hand_with_cost` makes elsewhere on
                // gated rollbacks.
                let player_state = self.game.player_mut(player);
                let restore_at = security_index.min(player_state.security.len());
                player_state.security.insert(restore_at, card);
                None
            }
        }
    }

    /// Remove the source at `source_index` from `target`'s digivolution
    /// stack and play the underlying card into `target.player`'s battle
    /// area, deducting memory according to `cost_delta`. OnPlay effects
    /// fire as if the card had been played from hand.
    ///
    /// Card-text precedent: BT15-080 — "place this card's bottom material
    /// into battle area as a Digimon" (Phase 2f1 DSL `PlayFromMaterials`
    /// step lowering).
    ///
    /// ## Implementation strategy: hand-transit (mirrors `play_from_security`)
    ///
    /// `Game::play_from_hand_with_cost(player, hand_index, cost_delta)`
    /// already encapsulates the full placement path — battle-area capacity
    /// check, `CannotPlayDigimonByEffect` gate, `Permanent::new`, OnPlay
    /// trigger drain, OnEnterFieldAnyone broadcast, `Play` event emission.
    /// Re-introducing the placement body here would duplicate that logic
    /// and risk drift. Instead: pop the chosen `CardSource` out of
    /// `target`'s `card_sources`, push it to the end of the controller's
    /// `hand`, and route through `play_from_hand_with_cost` at that index.
    /// The card spends one tick in hand but never as a player-visible
    /// state — the hand is mutated and consumed inside this single method
    /// call before any selection prompt or event handler can observe it.
    /// Identical pattern to `play_from_security` (Task 3a).
    ///
    /// On rollback (battle area full, flood-gate, etc.) the source is
    /// restored to its **original index** in `target.card_sources` so the
    /// failure is observable as a no-op.
    ///
    /// Returns the `PermanentHandle` of the new field permanent, or `None`
    /// if `target` is invalid, `source_index` is out of bounds, the battle
    /// area is full, memory is insufficient, or the play was gated by a
    /// flood-gate.
    pub fn play_from_materials(
        &mut self,
        target: PermanentHandle,
        source_index: usize,
        cost_delta: crate::enums::CostDelta,
    ) -> Option<PermanentHandle> {
        self.play_from_materials_suppress_on_play(target, source_index, cost_delta, false)
    }

    pub fn play_under_tamer_source_without_cost(
        &mut self,
        source_ref: SourceSelectionRef,
    ) -> Option<PermanentHandle> {
        self.play_under_tamer_source_with_cost(source_ref, crate::enums::CostDelta::Free)
    }

    pub fn play_under_tamer_source_with_cost_reduction(
        &mut self,
        source_ref: SourceSelectionRef,
        reduction: i16,
    ) -> Option<PermanentHandle> {
        self.play_under_tamer_source_with_cost(
            source_ref,
            crate::enums::CostDelta::Reduce(reduction),
        )
    }

    pub fn play_under_tamer_source_with_cost(
        &mut self,
        source_ref: SourceSelectionRef,
        cost_delta: crate::enums::CostDelta,
    ) -> Option<PermanentHandle> {
        if !self.own_tamer_target(source_ref.permanent) {
            return None;
        }
        let source_index = source_ref.source_index as usize;
        {
            let permanent = self
                .game
                .player(source_ref.permanent.player)
                .battle_area
                .get(source_ref.permanent.index as usize)?;
            if source_index + 1 >= permanent.card_sources.len() {
                return None;
            }
            if permanent.card_sources[source_index].handle() != source_ref.card {
                return None;
            }
        }
        self.play_from_materials(source_ref.permanent, source_index, cost_delta)
    }

    pub fn play_from_materials_suppress_on_play(
        &mut self,
        target: PermanentHandle,
        source_index: usize,
        cost_delta: crate::enums::CostDelta,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        if target.index == crate::action::space::BREEDING_TARGET as u8 {
            return self.play_from_breeding_materials_suppress_on_play(
                target,
                source_index,
                cost_delta,
                suppress_on_play,
            );
        }

        // Validate target permanent + source_index up-front using immutable
        // borrows.
        let player = target.player;
        {
            let p = self.game.player(player);
            let perm = p.battle_area.get(target.index as usize)?;
            if source_index >= perm.card_sources.len() {
                return None;
            }
        }

        // Extract the source. `Vec::remove` shifts subsequent sources down
        // one index — that's the desired behavior for material extraction
        // (the stack closes the gap left by the removed source).
        let source = self.game.remove_source_from_permanent(target, source_index);

        // Park at the end of `player`'s hand and route through the standard
        // play-from-hand path. The hand index is the new last position.
        self.game.player_mut(player).hand.push(source);
        let hand_index = self.game.player(player).hand.len() - 1;

        match self
            .game
            .play_from_hand_with_cost_result_from_origin_suppress(
                player,
                hand_index,
                cost_delta,
                PlaySource::ByEffect,
                false,
                PendingWouldPlayOrigin::Source {
                    permanent: target,
                    source_index,
                },
                suppress_on_play,
            ) {
            PlayFromHandCostResult::Played(field_index) => {
                let played = PermanentHandle {
                    player,
                    index: field_index as u8,
                };
                // Soft-remove the carrier slot if `play_from_materials` just
                // consumed its only source. Sibling of the digivolve-from-
                // material fix landed in PR #533. The carrier permanent has
                // empty `card_sources` post-extraction and would panic any
                // downstream `top_card()` reader; the helper drops the slot
                // and routes linked cards to trash per the same contract as
                // `Game::soft_remove_if_emptied`. See
                // `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
                // `qa/archetype-qa/engine-gaps.md` (and the change
                // `fix-zombie-permanent-siblings`).
                //
                // The `played` handle is NOT affected by the soft-remove:
                // play_from_hand_with_cost_result_from_origin_suppress
                // pushes the new permanent at `battle_area.len()` AFTER the
                // source extraction, so any soft-remove of an earlier slot
                // has already shifted the played handle's index downward
                // and the returned `field_index` reflects the post-shift
                // position. But the soft-remove here happens AFTER the
                // play, removing the now-empty carrier, which may shift
                // `played.index` down by 1 if the carrier sat at a lower
                // index than the played permanent.
                let played = Self::shift_handle_after_soft_remove_check(self.game, target, played);
                Some(played)
            }
            PlayFromHandCostResult::Pending => {
                // Decision 2 in `design.md`: do NOT soft-remove on the
                // Pending branch. A parked selection may resume and either
                // commit the play (cleanup happens then via a separate
                // post-resume path) or fail (rollback restores the source
                // into the carrier). Soft-removing now would leave the
                // rollback path with no slot to restore into. The Layer 2
                // guards on `enqueue_from_permanent`,
                // `queued_effect_source_is_live`, and (via this change)
                // `find_event_gated_delay_permanent` /
                // `event_gated_delay_source` tolerate a transient zombie
                // carrier for the duration of the parked selection.
                None
            }
            PlayFromHandCostResult::Failed => {
                // Rollback: pop the card out of hand and reinsert it at
                // its original index in `target.card_sources` so the
                // failure is a clean no-op for callers. Soft-remove MUST
                // NOT have run before this — Decision 2 in `design.md`.
                let card = self
                    .game
                    .player_mut(player)
                    .hand
                    .pop()
                    .expect("invariant: card was just pushed to hand");
                // The target permanent index is still valid here — only
                // hand was mutated by the failed play attempt; the
                // battle-area entry was left untouched.
                self.game
                    .insert_source_into_permanent(target, source_index, card);
                None
            }
        }
    }

    pub(crate) fn play_from_breeding_materials_suppress_on_play(
        &mut self,
        target: PermanentHandle,
        source_index: usize,
        cost_delta: crate::enums::CostDelta,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        let player = target.player;
        {
            let breeding = self.game.player(player).breeding_area.as_ref()?;
            if source_index >= breeding.card_sources.len()
                || source_index + 1 >= breeding.card_sources.len()
            {
                return None;
            }
        }

        let source = self
            .game
            .player_mut(player)
            .breeding_area
            .as_mut()?
            .card_sources
            .remove(source_index);

        self.game.player_mut(player).hand.push(source);
        let hand_index = self.game.player(player).hand.len() - 1;

        match self
            .game
            .play_from_hand_with_cost_result_from_origin_suppress(
                player,
                hand_index,
                cost_delta,
                PlaySource::ByEffect,
                false,
                PendingWouldPlayOrigin::Source {
                    permanent: target,
                    source_index,
                },
                suppress_on_play,
            ) {
            PlayFromHandCostResult::Played(field_index) => Some(PermanentHandle {
                player,
                index: field_index as u8,
            }),
            PlayFromHandCostResult::Pending => None,
            PlayFromHandCostResult::Failed => {
                let card = self
                    .game
                    .player_mut(player)
                    .hand
                    .pop()
                    .expect("invariant: card was just pushed to hand");
                self.game
                    .player_mut(player)
                    .breeding_area
                    .as_mut()?
                    .card_sources
                    .insert(source_index, card);
                None
            }
        }
    }

    /// Play a card from `player`'s trash at `trash_index`, deducting memory
    /// according to `cost_delta`. OnPlay effects fire.
    pub fn play_from_trash_with_cost(
        &mut self,
        player: PlayerId,
        trash_index: usize,
        cost_delta: crate::enums::CostDelta,
    ) -> Option<PermanentHandle> {
        let field_index = self.game.play_from_trash_with_cost(
            player,
            trash_index,
            cost_delta,
            PlaySource::ByEffect,
        )?;
        Some(PermanentHandle {
            player,
            index: field_index as u8,
        })
    }

    /// Play `card` from its controller's trash into the battle area, **without
    /// paying its memory cost** and **without suspending** the resulting
    /// permanent. ETB triggers (`OnPlay` + `OnEnterFieldAnyone`) fire as normal.
    ///
    /// ## Why a thin alias is sufficient (audit finding — Phase D Task 3)
    ///
    /// `Game::play_from_trash_with_cost(player, index, CostDelta::Free)` already
    /// covers all three requirements:
    ///   - **Free**: `CostDelta::Free` resolves to 0 → `pay_memory(0)` is a
    ///     no-op; memory is unchanged.
    ///   - **Unsuspended**: `Permanent::new()` sets `is_suspended = false` by
    ///     default; no extra flag needed.
    ///   - **ETB active**: `fire_on_play` + `OnEnterFieldAnyone` run at the end
    ///     of `play_from_trash_with_cost`, exactly as for hand plays.
    ///
    /// The only gap bridged here is the call-site convenience: callers hold a
    /// `CardHandle` (stable across zone moves), not a positional `trash_index`.
    /// This method locates the card in the controller's trash by handle.
    ///
    /// Returns `None` if the card is not in the controller's trash at call
    /// time (e.g., if another effect moved it elsewhere). This is the
    /// defensive behavior; callers like the deferred-replay drain in
    /// `combat::finalize_permanent_deletion` absorb `None` silently. The
    /// concrete failure mode this guards against: a `<Save>` + `<Fortitude>`
    /// interaction where Save relocates the card under a Tamer between
    /// Fortitude's queueing of the replay and the drain hook firing — at
    /// which point the card is no longer in trash and replaying it would
    /// panic.
    ///
    /// DCGO parity: `Fortitude.cs:54-63`
    ///   `PlayPermanentCards(payCost: false, isTapped: false,
    ///    root: SelectCardEffect.Root.Trash, activateETB: true)`
    ///
    /// Used by: `<Fortitude>` keyword auto-install (Phase D Task 8).
    pub fn play_from_trash_free_unsuspended(
        &mut self,
        card: CardHandle,
    ) -> Option<PermanentHandle> {
        self.play_from_trash_free_unsuspended_inner(card, false)
    }

    /// As [`Self::play_from_trash_free_unsuspended`], but suppresses the
    /// played Digimon's own `[On Play]` effects for this play event only
    /// (PUPPETS-G030). Used by BT5-106's [Security] clause — "Any [On Play]
    /// effects on Digimon played with this effect don't activate." The
    /// suppression is scoped strictly to the just-played permanent and this
    /// single play; other permanents' On Play and every other timing
    /// (`OnEnterFieldAnyone` / `OnAllyPlayed`) fire normally.
    pub fn play_from_trash_free_unsuspended_suppress_on_play(
        &mut self,
        card: CardHandle,
    ) -> Option<PermanentHandle> {
        self.play_from_trash_free_unsuspended_inner(card, true)
    }

    /// As the `_inner` form, but the playing CONTROLLER is explicit —
    /// "YOUR OPPONENT plays 1 ... Digimon card from THEIR trash"
    /// (EX5-060 Dragomon, judge-quiz Q28 /
    /// G-OPPONENT-PLAY-FROM-OWN-TRASH-SUSPENDED): the card is located in
    /// `controller`'s trash and enters `controller`'s battle area.
    pub fn play_from_trash_free_unsuspended_for(
        &mut self,
        controller: crate::enums::PlayerId,
        card: CardHandle,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        self.play_from_trash_free_unsuspended_inner_for(controller, card, suppress_on_play)
    }

    pub(crate) fn play_from_trash_free_unsuspended_inner(
        &mut self,
        card: CardHandle,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        let controller = self.player;
        self.play_from_trash_free_unsuspended_inner_for(controller, card, suppress_on_play)
    }

    pub(crate) fn play_from_trash_free_unsuspended_inner_for(
        &mut self,
        controller: crate::enums::PlayerId,
        card: CardHandle,
        suppress_on_play: bool,
    ) -> Option<PermanentHandle> {
        let trash_index = self
            .game
            .player(controller)
            .trash
            .iter()
            .position(|c| c.handle() == card);
        let trash_index = match trash_index {
            Some(i) => i,
            None => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[debug] play_from_trash_free_unsuspended: card {:?} not in \
                     player {}'s trash; another effect likely relocated it. \
                     Skipping replay.",
                    card, controller
                );
                return None;
            }
        };
        let field_index = self.game.play_from_trash_with_cost_suppress(
            controller,
            trash_index,
            crate::enums::CostDelta::Free,
            PlaySource::ByEffect,
            suppress_on_play,
        )?;
        Some(PermanentHandle {
            player: controller,
            index: field_index as u8,
        })
    }

    /// Play/place `player`'s hand card into their real breeding area by effect.
    pub fn play_to_breeding_from_hand(&mut self, player: PlayerId, hand_index: usize) -> bool {
        self.game.play_to_breeding_from_hand(player, hand_index)
    }

    pub fn play_selected_sources_without_cost(
        &mut self,
        selected: Vec<SourceSelectionRef>,
    ) -> bool {
        self.game
            .play_source_refs_from_effect_without_cost(selected)
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
