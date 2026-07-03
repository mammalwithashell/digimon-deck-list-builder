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
    /// Schedule a one-shot delayed effect to fire at a future timing
    /// boundary. Used by `CompiledStep::ScheduleDelayed` lowering (Phase 2f4
    /// Task 3) for card text like "at the end of your next turn, do X".
    ///
    /// The effect's `body`, `captured_bindings`, source card, and source
    /// permanent are stored on `Game::scheduled_effects`. When
    /// `scheduled_effects::fire_scheduled_for_timing(game, when)` drains the
    /// queue, a fresh `EffectContext` is constructed against the captured
    /// `(controller, source_card, source_permanent)` and the body runs
    /// through `run_steps` with the captured bindings replayed.
    pub fn schedule_delayed(
        &mut self,
        when: EffectTiming,
        body: Vec<CompiledStep>,
        captured_bindings: Bindings,
    ) {
        self.schedule_delayed_with_runtime(when, body, captured_bindings, StepRuntime::default());
    }

    pub fn schedule_delayed_with_runtime(
        &mut self,
        when: EffectTiming,
        body: Vec<CompiledStep>,
        captured_bindings: Bindings,
        runtime: StepRuntime,
    ) {
        let entry = ScheduledEffect {
            when,
            body,
            source_card: self.source_card,
            source_permanent: self.source_permanent,
            source_kind: self.source_kind,
            controller: self.player,
            captured_bindings,
            scheduled_at_turn: self.game.turn_count,
            runtime,
        };
        self.game.scheduled_effects.push(entry);
    }

    /// PUPPETS-G003 — schedule a deletion of `permanent` at the end of the
    /// current turn, keyed to the permanent's *stable identity* rather than
    /// its (shifting) battle-area index.
    ///
    /// Used by effects whose text says "At turn end, delete the Digimon this
    /// effect played" (EX11-022 Karakurumon, EX11-061 Mirai Kinosaki). Pass
    /// the `PermanentHandle` returned by a free-play step (`play_from_hand_free`
    /// / `play_union_bound_free` / etc.); this captures the top card's
    /// `ProvenanceToken` *now*, while the handle is still valid.
    ///
    /// At turn end `fire_scheduled_provenance_deletions` resolves the token:
    /// if the played permanent is still on the battle area it is deleted (as
    /// the controller's own effect); if it already left, the entry is a
    /// silent no-op. A handle that does not currently point at a live
    /// permanent is ignored (nothing to schedule).
    pub fn schedule_delete_at_end_of_turn(&mut self, permanent: PermanentHandle) {
        self.schedule_provenance_deletion(permanent, false);
    }

    /// PUPPETS-G016 — schedule a deletion of `permanent` at the end of the
    /// **opponent's** turn, keyed to the permanent's *stable identity*.
    ///
    /// Used by P-165 ShoeShoemon ("At the end of your opponent's turn, delete
    /// that token"). Analogous to `schedule_delete_at_end_of_turn` but the
    /// deletion fires from `rotate_turn_player` rather than
    /// `fire_end_of_your_turn`.
    pub fn schedule_delete_at_end_of_opponents_turn(&mut self, permanent: PermanentHandle) {
        self.schedule_provenance_deletion(permanent, true);
    }

    pub(crate) fn schedule_provenance_deletion(
        &mut self,
        permanent: PermanentHandle,
        opponents_turn: bool,
    ) {
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

    pub fn place_self_as_delay_option_permanent(&mut self) {
        let source_card = if let Some(source_perm) = self.source_permanent {
            if !matches!(
                self.game.card_kind_for_handle(self.source_card),
                Some(CardKind::Option)
            ) {
                return;
            }
            let Some(source_card) =
                self.remove_source_card_from_permanent(source_perm, self.source_card)
            else {
                return;
            };
            source_card
        } else {
            if let Some(pending) = self.game.pending_security.take() {
                if pending.played
                    || pending.card.handle() != self.source_card
                    || pending.card.card_kind(&self.game.card_data) != CardKind::Option
                {
                    self.game.pending_security = Some(pending);
                    return;
                }
                pending.card
            } else if let Some(pending) = self.game.pending_option.take() {
                // Real Option-play lifecycle: play_option_core moved the Option
                // into the single-occupancy `pending_option` slot BEFORE running
                // the [Main] body, so it is no longer in hand/trash. Claim it for
                // self-placement when it is OUR source Option. Taking it here
                // leaves `pending_option` empty, so the subsequent
                // `dispose_option` (play_option_core step 8) finds nothing and
                // skips the Standard trash — exactly the desired end state.
                // Fixes G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH.
                if pending.card.handle() != self.source_card
                    || pending.card.card_kind(&self.game.card_data) != CardKind::Option
                {
                    self.game.pending_option = Some(pending);
                    return;
                }
                pending.card
            } else {
                let Some(source_card) = self.remove_source_option_from_controller_zones() else {
                    return;
                };
                source_card
            }
        };

        // The physical Option moves to its card owner/controller's battle area,
        // matching normal Delay placement from hand/trash.
        let owner = source_card.owner;
        let placed_card = source_card.handle();
        let card_id = source_card.card_id(&self.game.card_data).to_string();
        let trigger = self
            .game
            .effects_for_card(&card_id, placed_card)
            .unwrap_or_default()
            .iter()
            .find_map(|effect| effect.delay_trigger)
            .unwrap_or(DelayTrigger::EndOfYourNextTurn);
        let mut permanent = Permanent::new(source_card, self.game.turn_count);
        permanent.option_state = crate::permanent::OptionState::Delayed {
            owner,
            trash_on_turn: self.game.compute_delay_trash_turn(owner, trigger),
            trigger,
            placed_on_turn: self.game.turn_count,
        };
        self.game.player_mut(owner).battle_area.push(permanent);

        let handle = PermanentHandle {
            player: owner,
            index: (self.game.player(owner).battle_area.len() - 1) as u8,
        };
        self.game.enqueue_triggered(
            EffectTiming::OnOptionPlaced,
            crate::selection::TriggerSource::OptionPlaced {
                player: owner,
                permanent: Some(handle),
                linked_host: None,
                card: placed_card,
            },
        );
        self.game.drain_effect_queue();
    }

    /// Relocate THIS effect's source Option (an in-battle-area field Option)
    /// face-down under `target`, a chosen own permanent. The Option leaves the
    /// battle area and its card becomes the bottom-most digivolution source of
    /// `target`. Distinct from a trash: fires neither `OnOptionTrashed` nor
    /// `OnDigivolutionCardTrashed` (the card MOVES, it is not trashed).
    ///
    /// Used by ST23-15 e-Pulse / ST24-15 DNA Charge's
    /// `[Start of Your Main Phase] By placing this card from the battle area
    /// face down under any of your [BEATBREAK]/[DATA SQUAD] trait Tamers, …`.
    /// The caller runs the `<Draw 1>` + memory gain after a `true` return.
    /// G-MOVE-SELF-OPTION-UNDER-PERMANENT.
    ///
    /// Returns `false` (no mutation) when there is no source permanent (the
    /// Option is not on the field) or `target` is not a legal own permanent.
    pub fn move_self_option_under_permanent(
        &mut self,
        target: PermanentHandle,
        face_down: bool,
    ) -> bool {
        let Some(option_handle) = self.source_permanent else {
            return false;
        };
        // The target must be a battle-area permanent the controller owns.
        if target.player != self.player {
            return false;
        }
        if self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .is_none()
        {
            return false;
        }
        self.game
            .move_field_option_under_permanent(option_handle, target, face_down)
    }

    /// Place THIS effect's source Option as the bottom digivolution card of
    /// `target`, a chosen own permanent — the "Then, place this card as the
    /// bottom digivolution card of 1 of your … Digimon" [Main] tail on P-180 /
    /// EX7-070 / EX7-071 (Gap 1, G-OPTION-PLACE-SELF-UNDER-PERMANENT-ON-PLAY-PATH).
    ///
    /// This composes with the standard Option [Main]-play path
    /// (`play_option_core` / `play_option_from_hand`), where the source Option
    /// is the in-flight `pending_option` (already removed from hand/trash, not
    /// yet a field permanent) — so `move_self_option_under_permanent`'s
    /// field-permanent path does NOT apply. The source-of-truth for how to
    /// claim the in-flight Option is the RESOLVED sibling
    /// `place_self_as_delay_option_permanent`: prefer the in-flight
    /// `pending_option` slot, then a [Security]-resolution `pending_security`,
    /// then a raw hand/trash source (for direct callers that never went through
    /// `play_option_core`).
    ///
    /// Claiming `pending_option` leaves it empty, so `play_option_core`'s step-8
    /// `dispose_option` finds nothing and skips the Standard trash — exactly the
    /// desired end state (the Option MOVES under `target` rather than trashing).
    /// Fires no `OnOptionTrashed` (the card is not trashed).
    ///
    /// The card is seated FACE-UP by default (`face_down: false`); P-180's tail
    /// places it face up. The `face_down` axis is settable for parity with
    /// `move_self_option_under_permanent`.
    ///
    /// Returns `false` (no state change) when: the source card is not an Option,
    /// the in-flight Option is not OUR source card, `target` is not an own live
    /// permanent, or no claimable Option source exists. The caller (DSL step /
    /// clause tail) gates any follow-up on the returned `bool`.
    pub fn place_self_under_permanent(
        &mut self,
        target: PermanentHandle,
        face_down: bool,
    ) -> bool {
        // The target must be a battle-area permanent the controller owns.
        if target.player != self.player {
            return false;
        }
        if self
            .game
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .is_none()
        {
            return false;
        }
        // A live field-Option source permanent (the card was already placed in
        // the battle area, e.g. the RESOLVED-sibling relocate path) routes to
        // `move_self_option_under_permanent` — that path pops the field
        // permanent and fires no trash observers.
        if let Some(source_perm) = self.source_permanent {
            if self
                .game
                .option_field_state(source_perm)
                .is_some()
            {
                return self
                    .game
                    .move_field_option_under_permanent(source_perm, target, face_down);
            }
            // A non-Option field source (a Digimon/Tamer running this as an
            // effect) cannot place "this card" under a permanent.
            return false;
        }

        // Play-path: claim the in-flight Option. Mirrors
        // `place_self_as_delay_option_permanent`'s claim precedence.
        let source_card = if let Some(pending) = self.game.pending_security.take() {
            if pending.played
                || pending.card.handle() != self.source_card
                || pending.card.card_kind(&self.game.card_data) != CardKind::Option
            {
                self.game.pending_security = Some(pending);
                return false;
            }
            pending.card
        } else if let Some(pending) = self.game.pending_option.take() {
            // Real Option-play lifecycle: `play_option_core` moved the Option
            // into the single-occupancy `pending_option` slot BEFORE running
            // the [Main] body, so it is no longer in hand/trash. Claim it for
            // self-placement when it is OUR source Option. Taking it here leaves
            // `pending_option` empty, so the subsequent `dispose_option`
            // (play_option_core step 8) finds nothing and skips the Standard
            // trash — exactly the desired end state.
            if pending.card.handle() != self.source_card
                || pending.card.card_kind(&self.game.card_data) != CardKind::Option
            {
                self.game.pending_option = Some(pending);
                return false;
            }
            pending.card
        } else {
            // Direct caller (never went through `play_option_core`): pull the
            // source Option out of the controller's hand/trash.
            let Some(source_card) = self.remove_source_option_from_controller_zones() else {
                return false;
            };
            source_card
        };

        self.game
            .seat_card_source_under_permanent(source_card, target, face_down)
    }

    /// Decline the pay cost for a queued triggered effect that parked during
    /// `pay_cost_fn`. The effect queue will discard the parked process tail
    /// after the current selection callback unwinds.
    pub fn decline_pending_pay_cost(&mut self) {
        self.game.decline_pending_pay_cost();
    }

    pub fn gain_memory(&mut self, amount: i16) {
        let target = self.player;
        // Phase 6: CannotGainMemoryByEffect — suppress all memory gains by effect.
        if self
            .game
            .modifiers
            .player_has(target, ModifierType::CannotGainMemoryByEffect)
        {
            return;
        }
        // Phase 6: CannotGainMemoryExceptFromTamers — only Tamer-sourced gains are
        // allowed; block Digimon/Option-sourced gains.
        if self
            .game
            .modifiers
            .player_has(target, ModifierType::CannotGainMemoryExceptFromTamers)
            && !self.source_is_tamer()
        {
            return;
        }
        // Track C / D consult site (2026-05-08): permanent-scoped
        // `CannotAddMemory` — while any permanent in the acting player's
        // battle area carries this modifier, the controller's effects
        // can't add memory. Sibling of player-scoped
        // `CannotGainMemoryByEffect` for printed text anchored to a
        // specific Digimon.
        let battle_area_len = self.game.player(target).battle_area.len();
        for i in 0..battle_area_len {
            let h = PermanentHandle {
                player: target,
                index: i as u8,
            };
            if self.game.modifiers.has(h, ModifierType::CannotAddMemory) {
                return;
            }
        }
        let source = self.effect_memory_source();
        self.game
            .gain_memory_for_player_sourced(target, amount, source);
    }

    /// Resolve the current effect's source card identity `(card_id, name)`
    /// for memory-change attribution. Reads the installing `source_card`
    /// (covers tamers, options, and Digimon effects); `None` if it can't be
    /// resolved (the resulting `MemoryChange` is then unattributed).
    fn effect_memory_source(&self) -> Option<(String, String)> {
        self.game
            .card_data_for_handle(self.source_card)
            .map(|cd| (cd.card_id.clone(), cd.card_name.clone()))
    }

    pub fn lose_memory(&mut self, amount: i16) {
        // Controller-relative, mirroring `gain_memory`: the effect's
        // controller loses the memory (general_rule.pdf p.7 — "Lose X
        // memory" moves the marker toward YOUR opponent's side; DCGO
        // scripts run `card.Owner.AddMemory(-n)`). A raw seesaw
        // subtraction would charge the TURN player when a non-turn
        // player's effect (e.g. a defender's [On Deletion]) loses memory.
        // Memory-GAIN restrictions (CannotGainMemoryByEffect etc.) do not
        // apply to losses — DCGO only consults CanAddMemory for gains.
        let target = self.player;
        let source = self.effect_memory_source();
        // Loss is a controller-relative negative gain (mirrors
        // `lose_memory_for_player`), routed through the sourced path so the
        // emitted `MemoryChange` carries the effect's source identity.
        self.game
            .gain_memory_for_player_sourced(target, -amount, source);
    }

    pub fn set_memory(&mut self, value: i16) {
        self.game.set_memory(value);
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
    /// Route a card returned "to the deck" to the rules-correct deck. A
    /// Digi-Egg can NEVER enter the main deck (Comprehensive Rules), so it is
    /// sent to the Digi-Egg (digitama) deck instead — which still satisfies a
    /// "send N to the bottom of the deck" cost (judge-quiz Q22). The card
    /// returns to its OWNER's deck. `to_bottom` selects the bottom (index 0)
    /// vs the top (`Vec` end, drawn first). G-RETURN-TRASH-DIGI-EGG-ROUTING.
    ///
    /// NOTE (audit pending): the permanent-stack returns
    /// (`Game::return_to_deck` / `return_stack_to_deck`) and reveal-zone returns
    /// share the same Digi-Egg rule for any egg in a returned digivolution
    /// stack; those live on a different (game.rs) path and are out of scope for
    /// this trash→deck fix.
    pub(crate) fn move_card_to_deck(
        &mut self,
        card: crate::card_source::CardSource,
        to_bottom: bool,
    ) {
        let owner = card.owner;
        let is_egg = card.card_kind(&self.game.card_data) == CardKind::DigiEgg;
        let player = self.game.player_mut(owner);
        let deck = if is_egg {
            &mut player.digitama_deck
        } else {
            &mut player.deck
        };
        if to_bottom {
            deck.insert(0, card);
        } else {
            deck.push(card);
        }
    }

    pub fn return_all_trash_to_deck_bottom(&mut self, player: PlayerId) -> Vec<CardHandle> {
        // Drain trash in order. Each card is appended to the start of its
        // owner's deck (deck bottom = index 0 by convention; deck top =
        // Vec end, the position drawn from first). Digi-Eggs route to the
        // digitama deck (G-RETURN-TRASH-DIGI-EGG-ROUTING).
        let drained: Vec<crate::card_source::CardSource> =
            std::mem::take(&mut self.game.player_mut(player).trash);
        let mut handles = Vec::with_capacity(drained.len());
        for card in drained {
            handles.push(card.handle());
            self.move_card_to_deck(card, true);
        }
        handles
    }
}
