//! Cost-reduction / before-pay-cost machinery (Tier 2) — `impl Game`.

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
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn continue_play_from_hand_cost_reduction_chain(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: CostTargetContext,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
        mut accumulated_reduction: i32,
        mut processed: Vec<CostReductionKey>,
    ) -> PlayFromHandCostResult {
        loop {
            let candidates = self.collect_before_pay_cost_reducers(
                player_id,
                Some(target),
                &processed,
                CostReductionKind::Play,
            );
            let Some(candidate) = candidates.into_iter().next() else {
                // All generic cost reducers resolved. Before committing the
                // play, offer the Assembly alt-path (G-ASSEMBLY-PLAY-EXECUTION,
                // change `fix-ad1-025-assembly-data`) when the played card has
                // an `assembly` path whose materials are present in the
                // controller's trash. Falls through to the normal play finish
                // when the card is not assembly-capable or its materials are
                // unavailable.
                return self.assembly_or_finish_play_from_hand(
                    player_id,
                    hand_index,
                    target.card,
                    cost_delta,
                    source,
                    origin,
                    suppress_on_play,
                    accumulated_reduction,
                );
            };

            // Auto-apply reducers with NO interactive cost: a reducer bearing
            // a `pay_cost_fn` (e.g. "trash 2 cards" / "by suspending this
            // Tamer") imposes a real cost the player chooses to pay, so it
            // must park behind an explicit acceptance prompt below rather than
            // fire silently here (Working Rule §17 — no auto-selections; every
            // choice surfaces through `pending_selection`).
            //
            // EXCEPTION — a `pay_cost` that *begins with its own declinable
            // selection* (`pay_cost_self_gated`, e.g. BT12-112's optional
            // "place 1 [Shoutmon]") already surfaces the player's opt-in/opt-out
            // as its first prompt. Wrapping it in the confirmation gate would
            // be redundant AND wrongly mandatory (the gate's `is_optional`
            // tracks the effect-level `optional`, which such cards leave
            // `false`). Auto-apply runs the `pay_cost` directly so its inner
            // optional select IS the acceptance prompt — matching DCGO, which
            // surfaces the Shoutmon preattach as an optional "you may".
            if !candidate.optional
                && (!candidate.has_pay_cost || candidate.pay_cost_self_gated)
            {
                let key = candidate.key.clone();
                if let Some(amount) = self.apply_cost_reduction_candidate(&key, target) {
                    accumulated_reduction += amount;
                }
                processed.push(key);
                if self.pending_selection.is_some() {
                    self.wrap_pending_play_cost_continuation(
                        player_id,
                        hand_index,
                        target,
                        cost_delta,
                        source,
                        origin,
                        suppress_on_play,
                        accumulated_reduction,
                        processed,
                    );
                    return PlayFromHandCostResult::Pending;
                }
                continue;
            }

            let key = candidate.key.clone();
            let source_kind = self.effect_source_kind_for_handle(key.source_card);
            let accept_key = key.clone();
            let decline_key = key.clone();
            let accept_processed = processed.clone();
            let decline_processed = processed.clone();
            let on_decline = candidate.optional.then(|| {
                Box::new(move |game: &mut Game| {
                    let mut processed = decline_processed;
                    processed.push(decline_key);
                    let _ = game.continue_play_from_hand_cost_reduction_chain(
                        player_id,
                        hand_index,
                        target,
                        cost_delta,
                        source,
                        origin,
                        suppress_on_play,
                        accumulated_reduction,
                        processed,
                    );
                }) as crate::selection::DeclineCallback
            });
            let previous_phase = self.current_phase;
            self.current_phase = GamePhase::EffectChoice;
            self.pending_selection = Some(PendingSelection {
                zone_owner: None,
                kind: SelectionKind::EffectChoice,
                selecting_player: player_id,
                previous_phase,
                valid_action_ids: vec![crate::action::space::HAND_EFFECT_START],
                is_optional: candidate.optional,
                prompt: format!("Use {} to reduce play cost?", candidate.label),
                effect_choices: Some(vec![crate::selection::EffectChoiceEntry {
                    label: format!("{} (-{})", candidate.label, candidate.amount),
                    action_id: crate::action::space::HAND_EFFECT_START,
                    source_card: Some(key.source_card),
                    source_kind: Some(source_kind),
                    timing: Some(crate::enums::EffectTiming::BeforePayCost),
                    is_optional: candidate.optional,
                    observation_metadata: Default::default(),
                }]),
                source_card: key.source_card,
                source_permanent: key.source_permanent,
                source_kind,
                callback: Box::new(move |game: &mut Game, _action_id: u16| {
                    let mut processed = accept_processed;
                    let mut reduction = accumulated_reduction;
                    if let Some(amount) = game.apply_cost_reduction_candidate(&accept_key, target) {
                        reduction += amount;
                    }
                    processed.push(accept_key);
                    if game.pending_selection.is_some() {
                        game.wrap_pending_play_cost_continuation(
                            player_id,
                            hand_index,
                            target,
                            cost_delta,
                            source,
                            origin,
                            suppress_on_play,
                            reduction,
                            processed,
                        );
                        return;
                    }
                    let _ = game.continue_play_from_hand_cost_reduction_chain(
                        player_id,
                        hand_index,
                        target,
                        cost_delta,
                        source,
                        origin,
                        suppress_on_play,
                        reduction,
                        processed,
                    );
                }),
                on_decline,
            });
            return PlayFromHandCostResult::Pending;
        }
    }

    /// G-COST-REDUCE-ALLY-DIGIVOLVE — consult `Game::player_digivolve_cost_reducers`
    /// for a reducer that qualifies for the digivolution of `target` by
    /// `acting_player`. When one qualifies AND its (suspend) cost is payable,
    /// install an interactive accept/decline `PendingSelection` and return
    /// `true` (the caller must abort and let the callbacks re-enter the
    /// digivolve). Returns `false` if no reducer qualifies, or the reducer's
    /// suspend cost is unpayable — in which case the reducer stays armed and
    /// the digivolve proceeds at the unreduced cost.
    ///
    /// Only the FIRST qualifying reducer is offered per digivolution (a
    /// second qualifying reducer would be offered on a subsequent
    /// digivolution after this one resolves; BT3-103's `single_fire` means
    /// a player rarely has more than one armed at once).
    pub(crate) fn try_prompt_player_digivolve_cost_reducer(
        &mut self,
        acting_player: PlayerId,
        target: PermanentHandle,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
    ) -> bool {
        if self.player_digivolve_cost_reducers.is_empty() {
            return false;
        }
        // The flood-gates that suppress field-hosted digivolve reducers must
        // also suppress the player-scoped reducer (see
        // `collect_before_pay_cost_reducers`).
        if self
            .modifiers
            .player_has(acting_player, ModifierType::CannotReduceCost)
            || self
                .modifiers
                .player_has(acting_player, ModifierType::CannotReduceDigivolveCost)
            || self.modifiers.any_other_player_has(
                acting_player,
                ModifierType::OpponentCannotReduceDigivolveCost,
            )
        {
            return false;
        }
        // Top-card colors of the digivolving permanent (the permanent is the
        // SOURCE of the digivolution — BT3-103 keys on "your green Digimon").
        let top_colors: Vec<crate::enums::CardColor> = match self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        {
            Some(perm) => perm.top_card().digimon_colors(&self.card_data).to_vec(),
            None => return false,
        };
        let Some(reducer_idx) = self
            .player_digivolve_cost_reducers
            .iter()
            .position(|r| r.qualifies(acting_player, target, &top_colors))
        else {
            return false;
        };
        let reducer = self.player_digivolve_cost_reducers[reducer_idx].clone();

        // Verify the suspend cost is payable: the player must have at least
        // one unsuspended Digimon to suspend. If not, the reducer cannot
        // fire — leave it armed (per the gap's single-fire rule: a
        // cost-impossible attempt does NOT consume the reducer) and let the
        // digivolution proceed at the unreduced cost.
        if reducer.suspend_cost && self.suspendable_own_digimon(acting_player).is_empty() {
            return false;
        }

        self.install_player_digivolve_reducer_prompt(
            reducer_idx,
            reducer,
            acting_player,
            hand_index,
            field_index,
            source,
        );
        true
    }

    /// Scan all battle-area permanents of both players for
    /// `EffectTiming::BeforePayCost` effects whose condition passes, and
    /// accumulate the total cost reduction.
    ///
    /// **Critical invariant (Python Issue 24 avoidance):** only effects from
    /// permanents currently in `battle_area` with timing exactly
    /// `BeforePayCost` are included. Effects in trash, hand, or from any
    /// other timing (OnPlay, etc.) are never scanned here.
    ///
    /// For each qualifying effect:
    /// 1. Check condition (immutable read context — dropped immediately).
    /// 2. Check inherited/top-card filter.
    /// 3. Compute reduction amount from `cost_reduction_fn` or static
    ///    `cost_reduction` (immutable read context — dropped immediately).
    /// 4. **Phase 5 Task 4 — pay_cost_fn gate:** if `effect.pay_cost_fn` is
    ///    Some, invoke the closure with a mutable context. Returning `false`
    ///    skips this effect's reduction contribution (the play proceeds at
    ///    higher cost but does NOT fail). Returning `true` means the cost was
    ///    paid and the reduction applies.
    /// 5. Accumulate the reduction into the running total.
    ///
    /// Returns the total as `i32`. The caller is responsible for the final
    /// `effective_cost = max(0, base_cost - total_reduction)` computation.
    ///
    /// **Signature change (Phase 5 Task 4):** takes `&mut self` so that
    /// `pay_cost_fn` closures can mutate game state (e.g., trash cards).
    /// All callers already hold `&mut self`, so this is a pure signature
    /// refinement with no behavioral impact on the call sites.
    ///
    /// **Signature change (Phase 6 Task 4):** takes `acting_player` so that
    /// the `CannotReducePlayCost` flood-gate can suppress all reductions for
    /// the acting player. Callers pass their `player_id` argument.
    /// Walks `EffectTiming::BeforePayCost` effects whose condition passes
    /// and accumulates the total cost reduction. Threads an optional
    /// cost-target card through candidate collection and reducer
    /// application so target-aware predicates (e.g.
    /// `cost_target: { trait_has: Free }`) can fire on the digivolve
    /// path. The play-from-hand path goes through
    /// `continue_play_from_hand_cost_reduction_chain`, which has its own
    /// chain-style target threading; the digivolve path calls this
    /// function with the hand-card handle being digivolved into.
    ///
    /// G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (Phase 2 Track H closure).
    pub(crate) fn scan_before_pay_cost_reduction_with_target(
        &mut self,
        acting_player: crate::enums::PlayerId,
        cost_kind: CostReductionKind,
        cost_target: Option<CostTargetContext>,
    ) -> i32 {
        let candidates =
            self.collect_before_pay_cost_reducers(acting_player, cost_target, &[], cost_kind);
        let mut total = 0;
        for candidate in candidates {
            // Optional reducers still need an explicit play-cost choice flow.
            // A `pay_cost`-bearing reducer (e.g. BT5-092's "by suspending this
            // Tamer") IS resolvable here when there is a real cost target —
            // `apply_cost_reduction_candidate` runs the synchronous pay_cost
            // and only counts the reduction if it succeeds
            // (G-COST-REDUCTION-DIGIVOLVE-INTO). Without a real cost target
            // (the sentinel fallback below) a paid reducer is still skipped.
            if candidate.optional || (candidate.has_pay_cost && cost_target.is_none()) {
                self.logger.log(
                    "[Skipped] optional/paid BeforePayCost reducer requires explicit pending play-cost context",
                );
                continue;
            }
            // Without a real cost target, fall back to the source card as a
            // sentinel target (matches the previous behavior so existing
            // cost-reduction tests are unaffected). Target-aware predicates
            // (`cost_target: { ... }`) cannot pass in that mode because
            // `cost_target_card` is the source itself, not a digivolve
            // candidate — which is correct, since no real digivolve target
            // exists in that dispatch.
            let resolved_target = cost_target.unwrap_or(CostTargetContext {
                card: candidate.key.source_card,
                from_hand: false,
                is_digivolve: false,
                target_permanents: [None, None],
            });
            if let Some(amount) =
                self.apply_cost_reduction_candidate(&candidate.key, resolved_target)
            {
                total += amount;
            }
        }
        total
    }

    pub(crate) fn collect_before_pay_cost_reducers(
        &mut self,
        acting_player: PlayerId,
        cost_target: Option<CostTargetContext>,
        processed: &[CostReductionKey],
        cost_kind: CostReductionKind,
    ) -> Vec<CostReductionCandidate> {
        if self
            .modifiers
            .player_has(acting_player, ModifierType::CannotReduceCost)
            || (cost_kind == CostReductionKind::Play
                && self
                    .modifiers
                    .any_player_has(ModifierType::CannotReducePlayCost))
            || (cost_kind == CostReductionKind::Digivolve
                && self
                    .modifiers
                    .player_has(acting_player, ModifierType::CannotReduceDigivolveCost))
            || (cost_kind == CostReductionKind::Digivolve
                && self.modifiers.any_other_player_has(
                    acting_player,
                    ModifierType::OpponentCannotReduceDigivolveCost,
                ))
        {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        for info in self.before_pay_cost_source_infos(acting_player, cost_target.map(|t| t.card)) {
            let key = CostReductionKey {
                source_card: info.source_card,
                source_permanent: info.source_permanent,
                controller: info.controller,
                card_id: info.card_id,
                effect_slot: info.effect_slot,
                is_under: info.is_under,
            };
            if processed.contains(&key) {
                continue;
            }
            let Some(amount) = self.inspect_cost_reduction_candidate(&key, cost_target) else {
                continue;
            };
            let Some(effects) = self.effects_for_card(&key.card_id, key.source_card) else {
                continue;
            };
            let Some(effect) = effects.get(key.effect_slot as usize) else {
                continue;
            };
            if amount <= 0 && effect.pay_cost_fn.is_none() {
                continue;
            }
            candidates.push(CostReductionCandidate {
                key,
                label: if effect.name.is_empty() {
                    "cost reducer".to_string()
                } else {
                    effect.name.clone()
                },
                amount,
                optional: effect.optional,
                has_pay_cost: effect.pay_cost_fn.is_some(),
                pay_cost_self_gated: effect.pay_cost_self_gated,
            });
        }
        candidates
    }

    pub(crate) fn inspect_cost_reduction_candidate(
        &mut self,
        key: &CostReductionKey,
        cost_target: Option<CostTargetContext>,
    ) -> Option<i32> {
        let effects = self.effects_for_card(&key.card_id, key.source_card)?;
        let effect = effects.get(key.effect_slot as usize)?;
        if effect.timing != EffectTiming::BeforePayCost {
            return None;
        }
        if key.is_under != effect.inherited {
            return None;
        }
        if effect.max_per_turn > 0 && self.cost_reducer_activation_count(key) >= effect.max_per_turn
        {
            return None;
        }
        let cond_ok = if let Some(cond) = &effect.condition {
            let ctx = if let Some(target) = cost_target {
                EffectReadContext::new_with_cost_target(
                    self,
                    key.source_card,
                    key.source_permanent,
                    key.controller,
                    target.card,
                    target.from_hand,
                )
                .with_cost_is_digivolve(target.is_digivolve)
                .with_cost_target_permanents(target.target_permanents_vec())
            } else {
                EffectReadContext::new(self, key.source_card, key.source_permanent, key.controller)
            };
            cond(&ctx)
        } else {
            true
        };
        if !cond_ok {
            return None;
        }
        let amount = if let Some(reduction_fn) = &effect.cost_reduction_fn {
            let ctx = if let Some(target) = cost_target {
                EffectReadContext::new_with_cost_target(
                    self,
                    key.source_card,
                    key.source_permanent,
                    key.controller,
                    target.card,
                    target.from_hand,
                )
                .with_cost_is_digivolve(target.is_digivolve)
                .with_cost_target_permanents(target.target_permanents_vec())
            } else {
                EffectReadContext::new(self, key.source_card, key.source_permanent, key.controller)
            };
            reduction_fn(&ctx).max(0)
        } else {
            effect.cost_reduction.max(0)
        };
        Some(amount)
    }

    pub(crate) fn apply_cost_reduction_candidate(
        &mut self,
        key: &CostReductionKey,
        cost_target: CostTargetContext,
    ) -> Option<i32> {
        let amount = self.inspect_cost_reduction_candidate(key, Some(cost_target))?;
        let effects = self.effects_for_card(&key.card_id, key.source_card)?;
        let effect = effects.get(key.effect_slot as usize)?;
        if let Some(pay_cost_fn) = &effect.pay_cost_fn {
            let mut ctx = EffectContext::new_with_cost_target(
                self,
                key.source_card,
                key.source_permanent,
                key.controller,
                cost_target.card,
                cost_target.from_hand,
            );
            ctx.cost_is_digivolve = cost_target.is_digivolve;
            if !pay_cost_fn(&mut ctx) {
                return None;
            }
        }
        if effect.max_per_turn > 0 {
            self.record_cost_reducer_activation(key);
        }
        Some(amount)
    }

    pub(crate) fn record_cost_reducer_activation(&mut self, key: &CostReductionKey) {
        let Some(source) = key.source_permanent else {
            return;
        };
        if source.index == crate::action::space::BREEDING_TARGET as u8 {
            if let Some(perm) = self.player_mut(source.player).breeding_area.as_mut() {
                perm.record_activation(key.source_card, key.effect_slot);
            }
            return;
        }
        if let Some(perm) = self
            .player_mut(source.player)
            .battle_area
            .get_mut(source.index as usize)
        {
            perm.record_activation(key.source_card, key.effect_slot);
        }
    }

    /// Fire all `BeforePayCostObserve` effects on the field and (if the
    /// target hand card has `when_playing_this`) on the target itself.
    /// Runs at the same dispatch point as the cost-reduction scan;
    /// observer bodies mutate state (gain memory, etc.) BEFORE the final
    /// `pay_memory` for the play/digivolve action.
    ///
    /// Activity gating: observers honor `max_per_turn` via the same
    /// per-permanent activation count as cost reducers. Observers on a
    /// permanent without a source_permanent (i.e. on the target hand card
    /// via `when_playing_this`) skip the activation record.
    ///
    /// No-approximations §17: observer bodies that install a pending
    /// selection are a v2 extension and are not yet supported — a
    /// debug-only log fires if one is detected. BG Imperial's six initial
    /// refs (BT12-022, BT12-050, et al.) all have scalar bodies, so this
    /// limitation does not block the closure.
    pub(crate) fn scan_before_pay_cost_observers(
        &mut self,
        acting_player: PlayerId,
        cost_target: Option<CostTargetContext>,
    ) {
        if self
            .modifiers
            .player_has(acting_player, ModifierType::CannotReduceCost)
        {
            // Be conservative: if the acting player can't reduce cost,
            // assume per-player observer suppression as well. (No card
            // currently relies on observer-during-suppressed-cost
            // semantics; the Track H gap closure does not need it.)
            return;
        }
        let infos = self.before_pay_cost_observer_infos(acting_player, cost_target.map(|t| t.card));
        for info in infos {
            let Some(effects) = self.effects_for_card(&info.card_id, info.source_card) else {
                continue;
            };
            let Some(effect) = effects.get(info.effect_slot as usize) else {
                continue;
            };
            if effect.timing != EffectTiming::BeforePayCostObserve {
                continue;
            }
            if info.is_under != effect.inherited {
                continue;
            }
            if effect.max_per_turn > 0
                && self.observer_activation_count(&info) >= effect.max_per_turn
            {
                continue;
            }
            let cond_ok = if let Some(cond) = &effect.condition {
                let ctx = if let Some(target) = cost_target {
                    EffectReadContext::new_with_cost_target(
                        self,
                        info.source_card,
                        info.source_permanent,
                        info.controller,
                        target.card,
                        target.from_hand,
                    )
                    .with_cost_target_permanents(target.target_permanents_vec())
                } else {
                    EffectReadContext::new(
                        self,
                        info.source_card,
                        info.source_permanent,
                        info.controller,
                    )
                };
                cond(&ctx)
            } else {
                true
            };
            if !cond_ok {
                continue;
            }
            if let Some(process) = &effect.process {
                let mut ctx = if let Some(target) = cost_target {
                    EffectContext::new_with_cost_target(
                        self,
                        info.source_card,
                        info.source_permanent,
                        info.controller,
                        target.card,
                        target.from_hand,
                    )
                } else {
                    EffectContext::new(
                        self,
                        info.source_card,
                        info.source_permanent,
                        info.controller,
                    )
                };
                process(&mut ctx);
            }
            if effect.max_per_turn > 0 {
                self.record_observer_activation(&info);
            }
        }
    }

    pub(crate) fn record_observer_activation(&mut self, info: &BeforePayCostSourceInfo) {
        let Some(source) = info.source_permanent else {
            return;
        };
        if source.index == crate::action::space::BREEDING_TARGET as u8 {
            if let Some(perm) = self.player_mut(source.player).breeding_area.as_mut() {
                perm.record_activation(info.source_card, info.effect_slot);
            }
            return;
        }
        if let Some(perm) = self
            .player_mut(source.player)
            .battle_area
            .get_mut(source.index as usize)
        {
            perm.record_activation(info.source_card, info.effect_slot);
        }
    }
}
