//! Breeding-area operations (Tier 2) — `impl Game`.

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
    /// Shared eligibility for a real breeding-to-battle move. Both the action
    /// and effect paths use this so a DSL cost prompt cannot offer a move the
    /// eventual commit would reject.
    pub fn can_move_from_breeding_by_effect(&self, player_id: PlayerId) -> bool {
        if self.game_over {
            return false;
        }
        let Some(player) = self.players.get(player_id as usize) else {
            return false;
        };
        if player.battle_area.len() >= self.rules.field_slots as usize
            || player.breeding_area.as_ref()
                .and_then(|perm| perm.top_card().digimon_dp(&self.card_data)).is_none()
        {
            return false;
        }
        !self.modifiers.has(
            PermanentHandle { player: player_id, index: crate::action::space::BREEDING_TARGET as u8 },
            ModifierType::CannotMove,
        )
    }

    /// Move from breeding to battle area for a player.
    ///
    /// Phase 8 Task 5: after the egg is promoted to `battle_area`, every
    /// `OptionState::Training` permanent the player controls fires
    /// `OnTrainingTrash` and is then trashed via
    /// `delete_permanent_with_cause(Cost)`. The sideways-inheritance scan in
    /// `enqueue_from_permanent` pulls in Training cards' `inherited` effects
    /// for the hatched permanent's timings BEFORE these trash sweeps run,
    /// because the caller's triggered-effect dispatch (e.g. `OnHatch`) has
    /// already drained above the hatch hook in callers that fire it.
    ///
    /// Process the Training trash list in reverse index order so earlier
    /// deletes don't shift later indices out from under us.
    pub fn move_from_breeding(&mut self, player_id: PlayerId) -> bool {
        self.move_from_breeding_with_identity(player_id).is_some()
    }

    /// Commit a move and retain the resulting incarnation before observers
    /// can change the field. Some means the move happened, even if that host
    /// subsequently left during an observer; consumers resolve the identity
    /// again rather than substituting a later occupant of its original slot.
    pub(crate) fn move_from_breeding_with_identity(
        &mut self,
        player_id: PlayerId,
    ) -> Option<(PermanentHandle, u64)> {
        if !self.can_move_from_breeding_by_effect(player_id) {
            return None;
        }

        let player = self.player_mut(player_id);
        if let Some(mut perm) = player.breeding_area.take() {
            // §17-1-3-2-3's "placed in the breeding area by an effect" marker
            // describes the breeding area only; it does not follow the
            // permanent into the battle area.
            perm.placed_by_effect = false;
            player.battle_area.push(perm);
            let moved_handle = PermanentHandle {
                player: player_id,
                index: (player.battle_area.len() - 1) as u8,
            };
            let moved_card = player
                .battle_area
                .last()
                .map(|permanent| permanent.top_card().handle());
            let moved_identity = self.capture_digivolve_host_identity(moved_handle);

            if let Some(card) = moved_card {
                let top = self.player(player_id).battle_area[moved_handle.index as usize].top_card();
                let card_id = top.card_id(&self.card_data).to_string();
                let card_name = top.card_name(&self.card_data).to_string();
                let seq = self.next_event_seq();
                self.events.push(crate::events::GameEvent::BreedingMove {
                    seq, player: player_id, card_id, card_name, field_index: moved_handle.index,
                });
                self.enqueue_triggered(
                    EffectTiming::OnMove,
                    TriggerSource::MovedFromBreeding {
                        player: player_id,
                        permanent: moved_handle,
                        card,
                    },
                );
                self.drain_effect_queue();
            }
            self.mark_until_condition_dirty();
            self.reevaluate_until_condition_modifiers_if_dirty();

            // Phase 8 Task 5: trash every Training permanent the owner
            // controls. Collect handles, then process in reverse so each
            // delete doesn't invalidate the indices of later ones.
            let training_handles: Vec<PermanentHandle> = self
                .player(player_id)
                .battle_area
                .iter()
                .enumerate()
                .filter_map(|(i, perm)| {
                    if let crate::permanent::OptionState::Training { owner, .. } = perm.option_state
                    {
                        if owner == player_id {
                            return Some(PermanentHandle {
                                player: player_id,
                                index: i as u8,
                            });
                        }
                    }
                    None
                })
                .collect();

            for handle in training_handles.into_iter().rev() {
                self.enqueue_triggered(
                    EffectTiming::OnTrainingTrash,
                    TriggerSource::Permanent(handle),
                );
                self.drain_effect_queue();
                self.delete_permanent_with_cause(
                    handle,
                    crate::replacement::ReplacementCause::Cost,
                );
            }

            Some((moved_handle, moved_identity))
        } else {
            None
        }
    }

    /// Effect-initiated breeding promotion.
    ///
    /// This deliberately reuses the same real-zone move path as the normal
    /// breeding action so the permanent leaves `breeding_area`, enters the
    /// battle area with its stack intact, and dispatches the same move
    /// observers.
    pub fn move_from_breeding_by_effect(&mut self, player_id: PlayerId) -> bool {
        self.move_from_breeding(player_id)
    }

    /// Shared eligibility for effect play into breeding. The actual card must
    /// still be in its physical owner's hand. Field capacity is irrelevant.
    pub fn can_play_to_breeding_from_hand(
        &self,
        player_id: PlayerId,
        hand_index: usize,
        effect_controller: PlayerId,
    ) -> bool {
        let Some(player) = self.players.get(player_id as usize) else { return false; };
        let Some(card) = player.hand.get(hand_index) else { return false; };
        if player.breeding_area.is_some() || card.owner != player_id || card.is_token {
            return false;
        }
        let kind = self.card_data[card.data_index].card_kind;
        matches!(kind, CardKind::Digimon | CardKind::Dual)
            && !self.modifiers.player_has(effect_controller, ModifierType::CannotPlayDigimonByEffect)
            && !self.card_play_is_prohibited(effect_controller, card)
    }

    /// Play/place a Digimon from hand into the real breeding area. No On Play
    /// or battle-area entry event fires from this destination.
    pub fn play_to_breeding_from_hand(&mut self, player_id: PlayerId, hand_index: usize) -> bool {
        let controller = self.effect_source_player.unwrap_or(player_id);
        if !self.can_play_to_breeding_from_hand(player_id, hand_index, controller) {
            return false;
        }
        let card = self.player_mut(player_id).hand.remove(hand_index);
        let mut permanent = crate::permanent::Permanent::new(card, self.turn_count);
        // §17-1-3-2-3: placed in the breeding area by an effect.
        permanent.placed_by_effect = true;
        self.player_mut(player_id).breeding_area = Some(permanent);
        true
    }

    /// §17-1-3-2-3 / §17-1-3-2-4: is `player`'s breeding-area permanent a
    /// rule-check trash target? A face-down top card is (§17-1-3-2-4: the
    /// breeding area is part of the field, §3-4-6); so is a top card that is
    /// not a Digimon, unless an effect placed the permanent there
    /// (§17-1-3-2-3). A hatched Digi-Egg is a hatched Digimon (§3-4-7-1) and a
    /// DUAL card there is played as its Digimon face, so neither is a target.
    pub(crate) fn breeding_area_is_rule_trash_target(&self, player: PlayerId) -> bool {
        let Some(perm) = self.player(player).breeding_area.as_ref() else {
            return false;
        };
        let Some(top) = perm.card_sources.last() else {
            return false;
        };
        if top.face_down {
            return true;
        }
        !perm.placed_by_effect
            && !matches!(
                top.card_kind(&self.card_data),
                CardKind::Digimon | CardKind::DigiEgg | CardKind::Dual
            )
    }

    /// Trash `player`'s breeding-area permanent by rule (§17-1-3-2-3 /
    /// §17-1-3-2-4): every card of the stack, then its link cards, goes to its
    /// owner's trash face up (§3-6-3), one `Trash` event each. A rule process,
    /// not an effect (it is not "affected by effects") and not a deletion, so
    /// no `[On Deletion]` / leave triggers. Returns the leaving cards for the
    /// caller's single `<Overflow>` pass (§4-19-5).
    pub(crate) fn trash_breeding_permanent_by_rule(
        &mut self,
        player: PlayerId,
    ) -> Vec<CardSource> {
        let movement_snapshot = self.breeding_movement_snapshot(player);
        let Some(perm) = self.player_mut(player).breeding_area.take() else {
            return Vec::new();
        };
        let handle = PermanentHandle {
            player,
            index: crate::action::space::BREEDING_TARGET as u8,
        };
        self.clear_permanent_full(handle);
        self.modifiers.expire_player_on_permanent_leave(handle);
        // A token stack's cards leave no trash entry (as in
        // `trash_permanent_stack`); unlike that helper, which also drops the
        // link cards of a token stack, its link cards are real cards and go
        // to their owners' trash here.
        let stack = if perm.card_sources.first().is_some_and(|c| c.is_token) {
            Vec::new()
        } else {
            perm.card_sources
        };
        let leaving: Vec<CardSource> =
            stack.iter().chain(perm.linked_cards.iter()).cloned().collect();
        for card in stack.into_iter().chain(perm.linked_cards) {
            let owner = card.owner;
            self.trash_card(owner, card);
        }
        self.mark_until_condition_dirty();
        // Recorder parity with the battle-area rule trash
        // (`trash_battle_permanent_by_rule`).
        self.observe_rule_trash_completed(movement_snapshot);
        leaving
    }

    pub fn bind_training_permanent_to_permanent(
        &mut self,
        training: PermanentHandle,
        trained: PermanentHandle,
    ) -> bool {
        let Some(trained_top_card) = self
            .player(trained.player)
            .battle_area
            .get(trained.index as usize)
            .map(|perm| perm.top_card().handle())
        else {
            return false;
        };

        let Some(training_perm) = self
            .player_mut(training.player)
            .battle_area
            .get_mut(training.index as usize)
        else {
            return false;
        };
        if let crate::permanent::OptionState::Training {
            owner,
            trained: trained_slot,
        } = &mut training_perm.option_state
        {
            if *owner == trained.player {
                *trained_slot = Some(crate::permanent::TrainingBinding {
                    handle: trained,
                    top_card: trained_top_card,
                });
                return true;
            }
        }
        false
    }

    /// Phase F Task 6 — breeding-area Training dispatcher.
    ///
    /// Activates the `<Training>` `[Main]` effect on the controller's
    /// breeding-area permanent. Restricted to the `<Training>` effect by
    /// `effect.name == "<Training>"` so this dispatcher cannot leak any
    /// other `MainOnField` effect from breeding (RULES_CONTEXT 16-41 —
    /// only Training activates from breeding).
    ///
    /// Independent gate re-check (matches the mask emitter):
    ///   1. Breeding-area permanent exists.
    ///   2. Top card carries the printed `Keyword::Training`.
    ///   3. Carrier is unsuspended.
    ///
    /// On success, the auto-installed `<Training>` body's `process` runs
    /// with `source_permanent = Some(PermanentHandle { player, index: 14 })`.
    /// The keyword's `process` mutates breeding-area state directly
    /// (suspends the carrier; calls
    /// `EffectContext::training_place_deck_top_under_self_face_down` which
    /// inserts the new source into the breeding permanent when the carrier
    /// is not in `battle_area`).
    pub(crate) fn activate_breeding_main_training(&mut self, player_id: PlayerId) -> bool {
        use crate::action::space::BREEDING_TARGET;
        use crate::effect_context::EffectContext;
        use crate::enums::{EffectTiming, Keyword};
        use crate::permanent::PermanentHandle;

        // Gate 1+2+3: breeding exists, top has Training, not suspended.
        let (top_card_id, top_handle) = {
            let Some(player) = self.players.get(player_id as usize) else {
                return false;
            };
            let Some(breeding) = player.breeding_area.as_ref() else {
                return false;
            };
            if breeding.is_suspended {
                return false;
            }
            let top = breeding.top_card();
            let top_data = &self.card_data[top.data_index];
            if !top_data.keywords.contains(&Keyword::Training) {
                return false;
            }
            (top.card_id(&self.card_data).to_string(), top.handle())
        };

        // Look up effects for the top card; we want only `<Training>`.
        let effects = match self.effects_for_card(&top_card_id, top_handle) {
            Some(e) => e,
            None => return false,
        };

        let perm_handle = PermanentHandle {
            player: player_id,
            index: BREEDING_TARGET as u8,
        };

        for effect in effects.iter() {
            if effect.timing != EffectTiming::MainOnField {
                continue;
            }
            if effect.inherited {
                continue;
            }
            // Restrict to the `<Training>` keyword auto-effect — never leak
            // a hand-rolled or unrelated MainOnField that happens to share
            // the same card.
            if effect.name != "<Training>" {
                continue;
            }
            // Note: we deliberately skip `effect.condition` evaluation here
            // — it would short-circuit on `source_permanent()` returning
            // None for the breeding handle. The `is_suspended` gate above
            // is the only thing the condition checks, and we've already
            // re-verified it.
            if let Some(process) = &effect.process {
                let mut ctx = EffectContext::new(self, top_handle, Some(perm_handle), player_id);
                process(&mut ctx);
            }
            return true;
        }
        false
    }

    /// Digivolve a hand card onto the breeding-area permanent. Python
    /// parity for `action_digivolve_breeding(hand_idx)` — same flow as
    /// `digivolve_from_hand` minus the trigger/observer firing (breeding
    /// digivolve does NOT fire `WhenDigivolving`).
    pub fn digivolve_from_hand_onto_breeding(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        source: PlaySource,
    ) -> bool {
        if self.current_phase != GamePhase::Main {
            self.logger.log(&format!(
                "[Rejected] digivolve_breeding: not in Main phase (phase={:?})",
                self.current_phase
            ));
            return false;
        }
        let player = self.player(player_id);
        if hand_index >= player.hand.len() {
            self.logger.log(&format!(
                "[Rejected] digivolve_breeding: hand index {} out of range (hand size={})",
                hand_index,
                player.hand.len()
            ));
            return false;
        }
        if player.breeding_area.is_none() {
            self.logger
                .log("[Rejected] digivolve_breeding: breeding area is empty");
            return false;
        }

        let card = player.hand[hand_index].clone();
        // Route enumeration mirrors the battle-area digivolve: printed evo-cost
        // circles PLUS the DSL alt-digivolve paths (printed "[Digivolve]
        // [Name]: Cost N" alt evo boxes — e.g. EX11-014 Penguinmon over a
        // breeding Hiyarimon for cost 0). DCGO applies
        // `AddSelfDigivolutionRequirementStaticEffect` costs to ANY target
        // permanent (`CardSource.CostList`), with no battle-area-only gate.
        // App Fusion is excluded: its host gate needs LINKED cards, which
        // breeding permanents cannot have (and its commit path consumes them).
        let breeding_handle = PermanentHandle {
            player: player_id,
            index: crate::action::space::BREEDING_TARGET as u8,
        };
        let routes: Vec<crate::dna_digivolve::DigivolveRouteMatch> = self
            .all_digivolve_routes_for_card(&card, breeding_handle)
            .into_iter()
            .filter(|route| !route.app_fusion)
            .collect();
        let Some(min_route) = routes.iter().min_by_key(|r| r.memory_cost).copied() else {
            self.logger.log(&format!(
                "[Rejected] digivolve_breeding: card {} cannot digivolve onto breeding {} (no applicable route)",
                card.card_id(&self.card_data),
                self.player(player_id)
                    .breeding_area
                    .as_ref()
                    .map(|b| b.top_card().card_id(&self.card_data))
                    .unwrap_or(""),
            ));
            return false;
        };

        // Rule 17 — when the breeding base satisfies MORE THAN ONE
        // distinct-cost route and the player has not yet chosen, prompt for
        // WHICH cost to pay rather than auto-selecting the cheapest (same
        // contract as `digivolve_from_hand_inner`). The choice re-enters this
        // function via the `BREEDING_TARGET` field-index sentinel.
        if self.pending_digivolve_route_choice.is_none() && routes.len() > 1 {
            self.install_digivolve_cost_choice_prompt(
                player_id,
                hand_index,
                crate::action::space::BREEDING_TARGET as usize,
                source,
                routes,
            );
            return false;
        }
        let route = self
            .pending_digivolve_route_choice
            .take()
            .unwrap_or(min_route);
        let printed_cost = route.memory_cost;

        // Pass the breeding-target hand card as the cost-target so
        // target-aware predicates can fire (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET).
        // Note: breeding digivolve does not have a battle-area target
        // permanent — the breeding permanent is the source. Mark the
        // breeding handle (sentinel, built above) as the target permanent so
        // self-scoped predicates work analogously.
        let target = CostTargetContext {
            card: card.handle(),
            from_hand: true,
            is_digivolve: true,
            target_permanents: [Some(breeding_handle), None],
        };
        let total_reduction = self.scan_before_pay_cost_reduction_with_target(
            player_id,
            CostReductionKind::Digivolve,
            Some(target),
        );
        // Observer dispatch — G-BEFORE-PAY-COST-GAIN-MEMORY.
        self.scan_before_pay_cost_observers(player_id, Some(target));
        let effective_cost = (printed_cost as i32 - total_reduction).max(0) as u16;

        if !self.pay_memory(effective_cost) {
            self.logger.log(&format!(
                "[Rejected] digivolve_breeding: cannot pay memory cost {} (current memory={})",
                effective_cost, self.memory
            ));
            return false;
        }

        // Capture event payload (top_card_id from removed; from_stack_top
        // from the breeding's current top) before the move mutates state.
        let removed_card_id = self.player(player_id).hand[hand_index]
            .card_id(&self.card_data)
            .to_string();
        let removed_card_name = self.player(player_id).hand[hand_index]
            .card_name(&self.card_data)
            .to_string();
        let from_stack_top = self
            .player(player_id)
            .breeding_area
            .as_ref()
            .map(|b| b.top_card().card_id(&self.card_data).to_string())
            .unwrap_or_default();

        let turn = self.turn_count;
        let removed = self.player_mut(player_id).hand.remove(hand_index);
        let player_mut = self.player_mut(player_id);
        if let Some(breeding) = player_mut.breeding_area.as_mut() {
            breeding.digivolve(removed, turn);
        }
        player_mut.draw();

        // `GameEvent::Digivolve` for the breeding-area digivolve path.
        // `field_index` uses `BREEDING_TARGET` (the breeding-slot marker
        // from `crate::action::space`) since breeding is not a regular
        // battle_area index. Regular evo-cost path — was_dna/was_blast_dna
        // both false.
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Digivolve {
            seq,
            player: player_id,
            top_card_id: removed_card_id,
            card_name: removed_card_name,
            field_index: crate::action::space::BREEDING_TARGET as u8,
            from_stack_top,
            was_dna: false,
            was_blast_dna: false,
            memory_paid: effective_cost as i16,
        });

        // Reward-shaping counter — see commit_digivolve_from_hand_no_replace
        // for the design rationale. Bumped here because breeding digivolve
        // is its own user-action choke point (no separate commit helper).
        self.n_digivolutions[player_id as usize] += 1;

        // Breeding digivolve does NOT fire WhenDigivolving (Python parity).
        self.check_turn_end();
        true
    }
}

#[cfg(test)]
mod rule_check_breeding_tests {
    //! §17-1-3-2-3 / §17-1-3-2-4 breeding-area edges the conformance tests
    //! don't stage: the "placed by an effect" exception, a hatched Digi-Egg,
    //! and a face-down top in the breeding area (part of the field, §3-4-6).
    use crate::debug_runner::{make_test_card, DebugRunner};
    use crate::enums::CardKind;

    fn runner() -> DebugRunner {
        let mut tamer = make_test_card("RC-TAMER", "Tamer");
        tamer.card_kind = CardKind::Tamer;
        tamer.dp = None;
        tamer.level = None;
        let mut egg = make_test_card("RC-EGG", "Egg");
        egg.card_kind = CardKind::DigiEgg;
        egg.dp = None;
        egg.level = Some(2);
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("RC-BODY", "Body"))
            .add_card(tamer)
            .add_card(egg)
            .deck(0, &["RC-BODY"; 20])
            .deck(1, &["RC-BODY"; 20])
            .memory(3)
            .start();
        r.skip_mulligan();
        r.game.enter_main_phase();
        r
    }

    #[test]
    fn non_digimon_placed_in_breeding_area_by_an_effect_stays() {
        let mut r = runner();
        r.place_in_breeding(0, "RC-TAMER");
        r.game.players[0].breeding_area.as_mut().unwrap().placed_by_effect = true;
        r.game.drain_effect_queue();
        assert!(
            r.game.players[0].breeding_area.is_some(),
            "§17-1-3-2-3 excepts cards placed in the breeding area by an effect"
        );
        assert!(r.game.players[0].trash.is_empty());
    }

    #[test]
    fn hatched_digi_egg_in_breeding_area_stays() {
        let mut r = runner();
        r.place_in_breeding(0, "RC-EGG");
        r.game.drain_effect_queue();
        assert!(r.game.players[0].breeding_area.is_some());
        assert!(r.game.players[0].trash.is_empty());
    }

    #[test]
    fn face_down_top_in_breeding_area_is_trashed_face_up() {
        let mut r = runner();
        r.place_in_breeding(0, "RC-BODY");
        r.game.players[0].breeding_area.as_mut().unwrap().card_sources[0].face_down = true;
        r.game.drain_effect_queue();
        assert!(r.game.players[0].breeding_area.is_none());
        let trash = &r.game.players[0].trash;
        assert_eq!(trash.len(), 1);
        assert!(!trash[0].face_down, "trash is face up (§3-6-3)");
    }

    #[test]
    fn play_to_breeding_from_hand_marks_the_permanent_placed_by_effect() {
        let mut r = runner();
        let index = r.add_to_hand(0, "RC-BODY");
        assert!(r.game.play_to_breeding_from_hand(0, index));
        assert!(r.game.players[0].breeding_area.as_ref().unwrap().placed_by_effect);
        // The marker describes the breeding area only: moving out clears it.
        assert!(r.game.move_from_breeding(0));
        let moved = r.game.players[0].battle_area.last().unwrap();
        assert!(!moved.placed_by_effect);
    }

    #[test]
    fn card_eval_promotion_emits_identity_once_and_failed_move_emits_nothing() {
        let mut r = runner();
        r.place_in_breeding(0, "RC-BODY");
        r.game.drain_events();
        assert!(r.game.move_from_breeding(0));
        let events = r.game.drain_events();
        let moved: Vec<_> = events.iter().filter(|e| e.type_str() == "BreedingMove").collect();
        assert_eq!(moved.len(), 1);
        let json = serde_json::to_value(moved[0]).unwrap();
        assert_eq!(json["card_id"], "RC-BODY");
        assert_eq!(json["player"], 0);
        assert_eq!(json["field_index"], 0);
        assert!(!r.game.move_from_breeding(0));
        assert!(!r.game.drain_events().iter().any(|e| e.type_str() == "BreedingMove"));
    }

    #[test]
    fn card_eval_hatch_and_effect_evolution_are_observational_events() {
        let mut r = runner();
        let egg_index = r.add_to_hand(0, "RC-EGG");
        let egg = r.game.players[0].hand.remove(egg_index);
        r.game.players[0].digitama_deck.push(egg);
        r.game.drain_events();
        assert!(r.game.hatch(0));
        let events = r.game.drain_events();
        assert!(events.iter().any(|e| matches!(e, crate::events::GameEvent::Hatch { card_id, player: 0, .. } if card_id == "RC-EGG")));
        r.place_on_field(0, "RC-BODY", Some(0));
        let hand_index = r.add_to_hand(0, "RC-BODY");
        let card = r.game.players[0].hand.remove(hand_index);
        assert!(r.game.digivolve_onto(0, 0, card));
        let events = r.game.drain_events();
        assert!(events.iter().any(|e| matches!(e, crate::events::GameEvent::EvalDigivolve { top_card_id, memory_paid: 0, .. } if top_card_id == "RC-BODY")));
        assert!(!events.iter().any(|e| matches!(e, crate::events::GameEvent::Digivolve { .. })));
    }
}
