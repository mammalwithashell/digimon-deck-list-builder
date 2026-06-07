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
        let field_slots = self.rules.field_slots;
        {
            let player = self.player(player_id);
            if player.battle_area.len() >= field_slots as usize {
                return false;
            }
            let can_move = player
                .breeding_area
                .as_ref()
                .and_then(|perm| perm.level(&self.card_data))
                .unwrap_or(0)
                >= 3;
            if !can_move {
                return false;
            }
        }
        // Track C / D consult site: `CannotMove` on the breeding-area permanent
        // blocks the breeding → battle move (player-action AND effect-driven —
        // `move_from_breeding_by_effect` delegates here). Distinct from
        // `CannotSuspend` which only blocks orientation flips. The canonical
        // breeding handle is `{ player, index: BREEDING_TARGET }`; modifier
        // installers should target that handle to gate the move.
        let breeding_handle = crate::permanent::PermanentHandle {
            player: player_id,
            index: crate::action::space::BREEDING_TARGET as u8,
        };
        if self
            .modifiers
            .has(breeding_handle, crate::enums::ModifierType::CannotMove)
        {
            return false;
        }

        let player = self.player_mut(player_id);
        if let Some(perm) = player.breeding_area.take() {
            player.battle_area.push(perm);
            let moved_handle = PermanentHandle {
                player: player_id,
                index: (player.battle_area.len() - 1) as u8,
            };
            let moved_card = player
                .battle_area
                .last()
                .map(|permanent| permanent.top_card().handle());

            if let Some(card) = moved_card {
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

            true
        } else {
            false
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

    /// Play/place a Digimon from hand into the real breeding area.
    ///
    /// Returns false if the hand index is invalid, the card is not a Digimon
    /// card, or the breeding area is already occupied.
    pub fn play_to_breeding_from_hand(&mut self, player_id: PlayerId, hand_index: usize) -> bool {
        {
            let player = self.player(player_id);
            if player.breeding_area.is_some() || hand_index >= player.hand.len() {
                return false;
            }
            let card = &player.hand[hand_index];
            let kind = self.card_data[card.data_index].card_kind;
            if !matches!(kind, CardKind::Digimon | CardKind::Dual) {
                return false;
            }
        }

        let card = self.player_mut(player_id).hand.remove(hand_index);
        let permanent = crate::permanent::Permanent::new(card, self.turn_count);
        self.player_mut(player_id).breeding_area = Some(permanent);
        true
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
    /// other `MainOnField` effect from breeding (RULES_CONTEXT 16-40 —
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

        for effect in &effects {
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
        _source: PlaySource,
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
        let Some(breeding) = player.breeding_area.as_ref() else {
            self.logger
                .log("[Rejected] digivolve_breeding: breeding area is empty");
            return false;
        };

        let card = player.hand[hand_index].clone();
        if !self.can_digivolve(&card, breeding) {
            self.logger.log(&format!(
                "[Rejected] digivolve_breeding: card {} cannot digivolve onto breeding {} (evo-cost mismatch)",
                card.card_id(&self.card_data),
                breeding.top_card().card_id(&self.card_data),
            ));
            return false;
        }

        let base_top = breeding.top_card();
        let base_level = base_top.digimon_level(&self.card_data).unwrap();
        let base_colors = base_top.digimon_colors(&self.card_data);
        let printed_cost = card
            .digivolution_costs(&self.card_data)
            .iter()
            .filter(|ec| {
                ec.level == base_level
                    && crate::action::mask::evo_color(ec.card_color)
                        .map(|c| base_colors.contains(&c))
                        .unwrap_or(false)
            })
            .map(|ec| ec.memory_cost)
            .min()
            .expect("can_digivolve guarantees at least one matching evo_cost");

        // Pass the breeding-target hand card as the cost-target so
        // target-aware predicates can fire (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET).
        // Note: breeding digivolve does not have a battle-area target
        // permanent — the breeding permanent is the source. Mark the
        // breeding handle as the target permanent so self-scoped
        // predicates work analogously.
        let breeding_handle = PermanentHandle {
            player: player_id,
            index: crate::action::space::BREEDING_TARGET as u8,
        };
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
