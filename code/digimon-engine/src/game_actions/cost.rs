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
    /// Resumed declarations no longer have the origin caller on the stack to
    /// roll back a failed play. Restore the captured physical card exactly once.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn continue_resumed_play_cost_chain(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: CostTargetContext,
        cost_delta: CostDelta,
        source: PlaySource,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
        accumulated_reduction: crate::game_actions::PlayCostAdjustments,
        processed: Vec<CostReductionKey>,
    ) -> PlayFromHandCostResult {
        let controller = self.effect_source_player.unwrap_or(player_id);
        let result = self.continue_play_from_hand_cost_reduction_chain(
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
        if matches!(result, PlayFromHandCostResult::Failed) {
            self.restore_pending_would_play_origin(crate::game::PendingWouldPlayResume {
                player: player_id,
                card: target.card,
                effective_cost: 0,
                origin,
                effect_initiated: source == PlaySource::ByEffect,
                play_controller: controller,
                suppress_on_play,
            });
        }
        result
    }

    pub(crate) fn with_play_declaration_controller<T>(
        &mut self,
        controller: Option<PlayerId>,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = self.effect_source_player;
        self.effect_source_player = controller;
        let result = f(self);
        self.effect_source_player = previous;
        result
    }

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
        mut accumulated_reduction: crate::game_actions::PlayCostAdjustments,
        mut processed: Vec<CostReductionKey>,
    ) -> PlayFromHandCostResult {
        let effect_controller = self.effect_source_player;
        loop {
            let candidates = self.collect_before_pay_cost_reducers(
                player_id,
                Some(target),
                &processed,
                CostReductionKind::Play,
            );
            let preferred = self
                .preferred_play_cost_effect
                .take()
                .filter(|key| candidates.iter().any(|c| c.key == *key));
            let any_assignment = candidates.iter().any(|c| {
                self.effects_for_card(&c.key.card_id, c.key.source_card)
                    .and_then(|v| {
                        v.get(c.key.effect_slot as usize)
                            .and_then(|e| e.cost_assignment)
                    })
                    .is_some()
            });
            if candidates.len() > 1 && any_assignment && preferred.is_none() {
                let key = candidates[0].key.clone();
                self.install_play_cost_order(PlayFromHandCostReductionPromptState {
                    order: Some(play_cost_assignment::PlayCostOrder {
                        choices: candidates,
                        page: 0,
                    }),
                    effect_controller,
                    player_id,
                    hand_index,
                    target,
                    cost_delta,
                    source,
                    origin,
                    suppress_on_play,
                    accumulated_reduction,
                    processed,
                    key,
                    outer_conts: Vec::new(),
                });
                return PlayFromHandCostResult::Pending;
            }
            let selected = preferred
                .and_then(|k| candidates.iter().position(|c| c.key == k))
                .unwrap_or(0);
            let Some(candidate) = candidates.into_iter().nth(selected) else {
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
            if !candidate.optional && (!candidate.has_pay_cost || candidate.pay_cost_self_gated) {
                let key = candidate.key.clone();
                self.apply_play_cost_candidate(&key, target, &mut accumulated_reduction);
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
                    game.with_play_declaration_controller(effect_controller, |game| {
                        let mut processed = decline_processed;
                        processed.push(decline_key);
                        let _ = game.continue_resumed_play_cost_chain(
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
                    });
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
                prompt: match candidate.assignment {
                    Some(value) => format!("Set play cost to {value} using {}?", candidate.label),
                    None => format!("Use {} to reduce play cost?", candidate.label),
                },
                effect_choices: Some(vec![crate::selection::EffectChoiceEntry {
                    count_choice: None,
                    label: match candidate.assignment {
                        Some(value) => format!("{} (set play cost to {value})", candidate.label),
                        None => format!("{} (-{})", candidate.label, candidate.amount),
                    },
                    action_id: crate::action::space::HAND_EFFECT_START,
                    source_card: Some(key.source_card),
                    source_kind: Some(source_kind),
                    timing: Some(crate::enums::EffectTiming::BeforePayCost),
                    is_optional: candidate.optional,
                    keyword: None,
                    observation_metadata: Default::default(),
                }]),
                source_card: key.source_card,
                source_permanent: key.source_permanent,
                source_kind,
                callback: Box::new(move |game: &mut Game, _action_id: u16| {
                    game.with_play_declaration_controller(effect_controller, |game| {
                        let mut processed = accept_processed;
                        let mut reduction = accumulated_reduction;
                        game.apply_play_cost_candidate(&accept_key, target, &mut reduction);
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
                        let _ = game.continue_resumed_play_cost_chain(
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
                    });
                }),
                on_decline,
            });
            self.pending_selection_resume = Some(crate::resume::ResumeStack {
                frames: vec![crate::resume::ResumeFrame::PlayFromHandCostReductionPrompt(
                    PlayFromHandCostReductionPromptState {
                        order: None,
                        effect_controller,
                        player_id,
                        hand_index,
                        target,
                        cost_delta,
                        source,
                        origin,
                        suppress_on_play,
                        accumulated_reduction,
                        processed,
                        key,
                        outer_conts: Vec::new(),
                    },
                )],
            });
            return PlayFromHandCostResult::Pending;
        }
    }

    pub(crate) fn run_play_from_hand_cost_reduction_prompt_step(
        &mut self,
        mut state: PlayFromHandCostReductionPromptState,
        action_id: u16,
        is_pass: bool,
    ) {
        if let Some(order) = state.order.take() {
            let Some(index) = action_id
                .checked_sub(30)
                .filter(|i| *i < 28)
                .map(|i| order.page * 28 + i as usize)
            else {
                return;
            };
            let Some(choice) = order.choices.get(index) else {
                return;
            };
            let previous = self.preferred_play_cost_effect.replace(choice.key.clone());
            self.with_play_declaration_controller(state.effect_controller, |game| {
                let _ = game.continue_resumed_play_cost_chain(
                    state.player_id,
                    state.hand_index,
                    state.target,
                    state.cost_delta,
                    state.source,
                    state.origin,
                    state.suppress_on_play,
                    state.accumulated_reduction,
                    state.processed,
                );
            });
            self.preferred_play_cost_effect = previous;
            return;
        }
        self.with_play_declaration_controller(state.effect_controller, |game| {
            game.run_play_from_hand_cost_reduction_prompt_step_scoped(state, is_pass)
        });
    }

    fn run_play_from_hand_cost_reduction_prompt_step_scoped(
        &mut self,
        state: PlayFromHandCostReductionPromptState,
        is_pass: bool,
    ) {
        let PlayFromHandCostReductionPromptState {
            order: _,
            effect_controller: _,
            player_id,
            hand_index,
            target,
            cost_delta,
            source,
            origin,
            suppress_on_play,
            accumulated_reduction,
            mut processed,
            key,
            outer_conts: _,
        } = state;

        if is_pass {
            processed.push(key);
            let _ = self.continue_resumed_play_cost_chain(
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
            return;
        }

        let mut reduction = accumulated_reduction;
        self.apply_play_cost_candidate(&key, target, &mut reduction);
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
                reduction,
                processed,
            );
            return;
        }
        let _ = self.continue_resumed_play_cost_chain(
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
    }

    /// Snapshot every reducer triggered by this legal digivolution. Mask and
    /// route-availability queries never call this execution-only path.
    /// Free reductions accumulate; paid reducers expose their resolution order
    /// and suspend choices before the caller pays the final digivolution cost.
    pub(crate) fn try_prompt_player_digivolve_cost_reducer(
        &mut self,
        acting_player: PlayerId,
        target: PermanentHandle,
        hand_index: usize,
        source: PlaySource,
    ) -> Option<bool> {
        let card = self.player(acting_player).hand.get(hand_index)?.handle();
        self.try_prompt_digivolve_reducer_chain(
            acting_player,
            target,
            CostTargetContext {
                card,
                from_hand: true,
                is_digivolve: true,
                target_permanents: [Some(target), None],
            },
            source,
            None,
        )
    }

    pub(crate) fn try_prompt_digivolve_reducer_chain(
        &mut self,
        acting_player: PlayerId,
        target: PermanentHandle,
        cost_target: CostTargetContext,
        source: PlaySource,
        effect: Option<super::digivolve::EffectDigivolveCostChoiceState>,
    ) -> Option<bool> {
        self.try_prompt_digivolve_reducer_chain_mode(
            acting_player,
            target,
            cost_target,
            source,
            effect,
            false,
        )
    }

    pub(crate) fn try_prompt_digivolve_reducer_chain_mode(
        &mut self,
        acting_player: PlayerId,
        target: PermanentHandle,
        cost_target: CostTargetContext,
        source: PlaySource,
        effect: Option<super::digivolve::EffectDigivolveCostChoiceState>,
        affordability_probe: bool,
    ) -> Option<bool> {
        self.try_prompt_digivolve_reducer_operation(acting_player, target, cost_target,
            source, effect, None, affordability_probe)
    }

    pub(crate) fn try_prompt_dna_digivolve_reducer_chain(
        &mut self,
        declaration: super::effect_dna::EffectDnaHandDeclaration,
        target: PermanentHandle,
        cost_target: CostTargetContext,
        affordability_probe: bool,
    ) -> Option<bool> {
        self.try_prompt_digivolve_reducer_operation(declaration.player, target, cost_target,
            PlaySource::ByEffect, None, Some(declaration), affordability_probe)
    }

    #[allow(clippy::too_many_arguments)]
    fn try_prompt_digivolve_reducer_operation(
        &mut self,
        acting_player: PlayerId,
        target: PermanentHandle,
        cost_target: CostTargetContext,
        source: PlaySource,
        effect: Option<super::digivolve::EffectDigivolveCostChoiceState>,
        dna_effect: Option<super::effect_dna::EffectDnaHandDeclaration>,
        affordability_probe: bool,
    ) -> Option<bool> {
        // CR 3-4-7-6: HPD, Digisorption and ordinary would-evolve payment
        // effects do not trigger for a breeding host. The initiating effect's
        // own explicit cost delta is applied by its declaration path.
        if target.index == crate::action::space::BREEDING_TARGET as u8 {
            return None;
        }
        let top_colors = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)?
            .top_card()
            .digimon_colors(&self.card_data)
            .to_vec();
        use super::digivolve::{DigivolveReducerChoice, DigivolveReducerState};
        let mut remaining: Vec<_> = self
            .player_digivolve_cost_reducers
            .iter()
            .enumerate()
            .filter(|(_, reducer)| reducer.qualifies(acting_player, target, &top_colors))
            .map(|(index, reducer)| DigivolveReducerChoice::Player {
                index,
                reducer: reducer.clone(),
            })
            .collect();
        // §16-10-1/-2: `<Digisorption -N>` is an immediate-type effect of the
        // card in the HAND that a digivolution would occur into, offered
        // before the cost is paid. One entry per printed instance (§16-10-4).
        if cost_target.from_hand && cost_target.is_digivolve {
            if let Some(data) = self.card_data_for_handle(cost_target.card) {
                for keyword in crate::game::face_keywords(data) {
                    if let Keyword::Digisorption(n) = keyword {
                        remaining.push(DigivolveReducerChoice::Digisorption {
                            source_card: cost_target.card,
                            amount: i32::from(n),
                        });
                    }
                }
            }
        }
        let field_candidates = self.collect_before_pay_cost_reducers(
            acting_player,
            Some(cost_target),
            &[],
            CostReductionKind::Digivolve,
        );
        for candidate in field_candidates {
            // These are exactly the field classes excluded by the final
            // synchronous scan, so a payment can never be applied twice.
            if candidate.has_pay_cost
                && (candidate.pay_cost_interactive
                    || (candidate.optional
                        && candidate.amount > 0
                        && self.cost_reducer_pay_cost_payable(&candidate.key)))
            {
                remaining.push(DigivolveReducerChoice::Field(candidate));
            }
        }
        if remaining.is_empty() {
            return None;
        }
        let host_identity = self.capture_digivolve_host_identity(target);
        Some(
            self.finish_player_digivolve_reducer_chain(DigivolveReducerState {
                current: None,
                acting_player,
                host_identity,
                source,
                target: cost_target,
                remaining,
                reduction: 0,
                choosing_order: false,
                affordability_probe,
                effect,
                dna_effect,
            }),
        )
    }

    /// Execute one field payment within the shared chain. Keep its original
    /// controller, source handle and cost target, including arbitrary DSL
    /// selections. The chain resumes only after the entire payment settles.
    pub(crate) fn run_digivolve_reducer_chain_payment(
        &mut self,
        state: super::digivolve::DigivolveReducerState,
        candidate: CostReductionCandidate,
    ) -> Option<super::digivolve::DigivolveReducerState> {
        let previous = (self.effect_source_player, self.effect_source_card,
            self.effect_source_permanent, self.current_dna_origin, self.current_trigger_context.clone());
        if let Some(dna) = &state.dna_effect {
            self.effect_source_player = Some(dna.source_controller);
            self.effect_source_card = Some(dna.source_card);
            self.effect_source_permanent = dna.source_permanent;
            self.current_dna_origin = Some(true);
            self.current_trigger_context = dna.trigger_context.clone();
        }
        let result = self.run_digivolve_reducer_chain_payment_inner(state, candidate);
        (self.effect_source_player, self.effect_source_card, self.effect_source_permanent,
            self.current_dna_origin, self.current_trigger_context) = previous;
        result
    }

    fn run_digivolve_reducer_chain_payment_inner(
        &mut self,
        mut state: super::digivolve::DigivolveReducerState,
        candidate: CostReductionCandidate,
    ) -> Option<super::digivolve::DigivolveReducerState> {
        let key = candidate.key;
        let Some(amount) = self.inspect_cost_reduction_candidate(&key, Some(state.target)) else {
            return Some(state);
        };
        let Some(mut effects) = self.effects_for_card_owned(&key.card_id, key.source_card) else {
            return Some(state);
        };
        let Some(effect) = effects.get_mut(key.effect_slot as usize) else {
            return Some(state);
        };
        let max_per_turn = effect.max_per_turn;
        let Some(pay_cost) = effect.pay_cost_fn.take() else {
            return Some(state);
        };
        let pending_before = self.pending_selection.is_some();
        let (paid, unpayable) = {
            let mut ctx = EffectContext::new_with_cost_target(
                self,
                key.source_card,
                key.source_permanent,
                key.controller,
                state.target.card,
                state.target.from_hand,
            );
            ctx.cost_is_digivolve = true;
            ctx.cost_target_permanents = state.target.target_permanents_vec();
            let paid = pay_cost(&mut ctx);
            (paid, ctx.cost_unpayable)
        };
        if !pending_before && self.pending_selection.is_some() {
            if max_per_turn > 0 {
                self.record_cost_reducer_activation(&key);
            }
            self.wrap_digivolve_reducer_chain_payment(state, amount, true);
            return None;
        }
        let variable = self
            .pending_cost_reduction_amount_override
            .take()
            .unwrap_or(0);
        if paid && !unpayable {
            if max_per_turn > 0 {
                self.record_cost_reducer_activation(&key);
            }
            state.reduction += amount + variable;
        }
        Some(state)
    }

    /// Data hook for DSL payments; legacy raw callbacks retain the same
    /// recursive wrapping behavior as the original interactive reducer path.
    fn wrap_digivolve_reducer_chain_payment(
        &mut self,
        state: super::digivolve::DigivolveReducerState,
        amount: i32,
        paid: bool,
    ) {
        let Some(mut pending) = self.pending_selection.take() else {
            return;
        };
        if self.pending_selection_resume.is_some() {
            self.pending_selection = Some(pending);
            self.after_selection_resume_hooks.0.push(
                crate::resume::AfterSelectionHook::DigivolveReducerChain {
                    state,
                    amount,
                    paid,
                },
            );
            return;
        }
        let callback = pending.callback;
        let accepted = state.clone();
        pending.callback = Box::new(move |game, action| {
            callback(game, action);
            game.resume_digivolve_reducer_chain_after_payment(
                accepted.clone(),
                amount,
                paid && !game.dsl_clause_aborted,
            );
        });
        pending.on_decline = pending.on_decline.take().map(|decline| {
            Box::new(move |game: &mut Game| {
                decline(game);
                // A declined self-gated payment contributes no reduction.
                game.resume_digivolve_reducer_chain_after_payment(state.clone(), amount, false);
            }) as crate::selection::DeclineCallback
        });
        self.pending_selection = Some(pending);
    }

    pub(crate) fn resume_digivolve_reducer_chain_after_payment(
        &mut self,
        mut state: super::digivolve::DigivolveReducerState,
        amount: i32,
        paid: bool,
    ) {
        if self.pending_selection.is_some() {
            self.wrap_digivolve_reducer_chain_payment(state, amount, paid);
            return;
        }
        let variable = self
            .pending_cost_reduction_amount_override
            .take()
            .unwrap_or(0);
        if paid {
            state.reduction += amount + variable;
        }
        self.resume_player_digivolve_reducers(state);
    }

    /// G-COST-REDUCTION-INTERACTIVE-PAY-COST (digivolve half) — consult the
    /// field-hosted `BeforePayCost` reducers for the digivolution of `target`
    /// for one whose `pay_cost` is INTERACTIVE (installs a selection — e.g.
    /// ST23-03 Cougarmon's `trash_bottom_face_down_source_under_tamer`). The
    /// synchronous scan (`scan_before_pay_cost_reduction_with_target`) cannot
    /// host a parking pay_cost, so such a reducer is handled here BEFORE the
    /// scan: run its pay_cost (which parks), wrap the parked selection's
    /// resolution to credit the reduction into
    /// `Game::pending_interactive_digivolve_reduction` and re-enter the
    /// digivolve. Returns `true` (caller aborts; the callbacks re-enter) when a
    /// reducer was offered. Returns `false` when no interactive reducer
    /// qualifies — the synchronous scan then handles the remaining
    /// (non-interactive) reducers as before.
    ///
    /// Only the FIRST qualifying interactive reducer is handled per call; a
    /// second would be offered on the re-entry (its pay_cost is a fresh park).
    /// Respects `cost_unpayable` (no eligible cost target → the pay_cost flags
    /// it and credits nothing) and the reducer's `optional` flag (an optional
    /// reducer gets an accept/decline gate first; a mandatory one runs the
    /// pay_cost directly so its own selection is the first prompt).
    pub(super) fn try_prompt_interactive_digivolve_cost_reducer(
        &mut self,
        acting_player: PlayerId,
        target: CostTargetContext,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
    ) -> bool {
        let candidates = self.collect_before_pay_cost_reducers(
            acting_player,
            Some(target),
            &[],
            CostReductionKind::Digivolve,
        );
        // Two reducer classes are routed through this prompt:
        //  (a) an INTERACTIVE pay_cost (parks on a selection) — mandatory or
        //      optional (`G-COST-REDUCTION-INTERACTIVE-PAY-COST`);
        //  (b) an OPTIONAL reducer whose pay_cost is SYNCHRONOUS (P-200 Kanan
        //      Yuki's "by suspending this Tamer" — DCGO isOptional true). The
        //      synchronous scan below skips every `optional` candidate, so
        //      without this the reducer was silently never offered on the
        //      digivolve path. It is offered only when it actually reduces
        //      this digivolution (`amount > 0` — a play-only reducer such as
        //      BT26-088's surfaces here with amount 0) and its cost is
        //      payable (the suspend-self data twin, `activation_cost_kind`).
        //      The accept branch runs the pay_cost synchronously and credits
        //      `pending_interactive_digivolve_reduction`; the re-entry's scan
        //      still skips the optional candidate, so no double-apply.
        //  `G-COST-REDUCTION-OPTIONAL-SYNC-PAY-COST-DIGIVOLVE`.
        let Some(candidate) = candidates.into_iter().find(|c| {
            if !c.has_pay_cost {
                return false;
            }
            if c.pay_cost_interactive {
                return true;
            }
            c.optional && c.amount > 0 && self.cost_reducer_pay_cost_payable(&c.key)
        }) else {
            return false;
        };

        let key = candidate.key.clone();
        let source_kind = self.effect_source_kind_for_handle(key.source_card);

        if candidate.optional {
            // Optional reducer → accept/decline gate first. On accept, run the
            // (parking) pay_cost; on decline, re-enter the digivolve at full
            // cost (the synchronous scan skips this interactive reducer).
            let accept_key = key.clone();
            let previous_phase = self.current_phase;
            self.current_phase = GamePhase::EffectChoice;
            let label = candidate.label.clone();
            let amount = candidate.amount;
            self.pending_selection = Some(PendingSelection {
                zone_owner: None,
                kind: SelectionKind::EffectChoice,
                selecting_player: acting_player,
                previous_phase,
                valid_action_ids: vec![crate::action::space::HAND_EFFECT_START],
                is_optional: true,
                prompt: format!("Use {} to reduce digivolution cost?", label),
                effect_choices: Some(vec![crate::selection::EffectChoiceEntry {
                    count_choice: None,
                    label: format!("{} (-{})", label, amount),
                    action_id: crate::action::space::HAND_EFFECT_START,
                    source_card: Some(key.source_card),
                    source_kind: Some(source_kind),
                    timing: Some(crate::enums::EffectTiming::BeforePayCost),
                    is_optional: true,
                    keyword: None,
                    observation_metadata: Default::default(),
                }]),
                source_card: key.source_card,
                source_permanent: key.source_permanent,
                source_kind,
                callback: Box::new(move |game: &mut Game, _action_id: u16| {
                    // On accept, run the (parking) pay_cost. If it does NOT park
                    // (synchronous / unpayable), re-enter the digivolve here
                    // (we are in a callback — there is no original frame to
                    // return to); the reduction was credited iff it was paid.
                    let parked = game.run_interactive_digivolve_reducer_pay_cost(
                        accept_key,
                        target,
                        acting_player,
                        hand_index,
                        field_index,
                        source,
                    );
                    if !parked {
                        game.digivolve_from_hand_inner(
                            acting_player,
                            hand_index,
                            field_index,
                            source,
                            true,
                        );
                    }
                }),
                on_decline: Some(Box::new(move |game: &mut Game| {
                    game.digivolve_from_hand_inner(
                        acting_player,
                        hand_index,
                        field_index,
                        source,
                        true,
                    );
                })),
            });
            self.pending_selection_resume = Some(crate::resume::ResumeStack {
                frames: vec![
                    crate::resume::ResumeFrame::InteractiveDigivolveCostReductionPrompt(
                        InteractiveDigivolveCostReductionPromptState {
                            acting_player,
                            target,
                            hand_index,
                            field_index,
                            source,
                            key,
                            outer_conts: Vec::new(),
                        },
                    ),
                ],
            });
            return true;
        }

        // Mandatory reducer ("reduce the cost by N", no "you may") — run the
        // pay_cost directly. Its own (parking) selection IS the first prompt.
        // If it PARKS, the continuation re-enters the digivolve → abort here
        // (return true). If it resolves synchronously (paid or unpayable), the
        // reduction was credited inline and the ORIGINAL `digivolve_from_hand_inner`
        // frame continues — return false so it proceeds (the synchronous scan
        // skips this `pay_cost_interactive` reducer, so no double-apply).
        self.run_interactive_digivolve_reducer_pay_cost(
            key,
            target,
            acting_player,
            hand_index,
            field_index,
            source,
        )
    }

    /// Read-only payability probe for a field-hosted reducer's pay_cost, via
    /// its `activation_cost_kind` data twin (set by the DSL lowering for the
    /// suspend-self idiom). A reducer without a declared cost shape is assumed
    /// payable (its pay_cost reports failure at resolution instead).
    /// `G-COST-REDUCTION-OPTIONAL-SYNC-PAY-COST-DIGIVOLVE`.
    pub(super) fn cost_reducer_pay_cost_payable(&self, key: &CostReductionKey) -> bool {
        let Some(effects) = self.effects_for_card(&key.card_id, key.source_card) else {
            return false;
        };
        let Some(effect) = effects.get(key.effect_slot as usize) else {
            return false;
        };
        match effect.activation_cost_kind {
            Some(kind) => kind.is_payable(self, key.source_permanent),
            None => true,
        }
    }

    /// The reducer's optional processing condition (its `pay_cost`'s leading
    /// cost steps, `Effect::optional_condition_guard`) can be executed now
    /// (§15-7-4). A reducer without a recognised cost shape passes.
    /// G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED.
    pub(crate) fn cost_reducer_condition_payable(
        &self,
        key: &CostReductionKey,
        effect: &crate::effect::Effect,
    ) -> bool {
        effect.optional_condition_guard.as_ref().is_none_or(|guard| {
            self.cost_guard_holds_at(guard, key.source_card, key.source_permanent, key.controller)
        })
    }

    pub(crate) fn run_interactive_digivolve_cost_reduction_prompt_step(
        &mut self,
        state: InteractiveDigivolveCostReductionPromptState,
        is_pass: bool,
    ) {
        if is_pass {
            self.digivolve_from_hand_inner(
                state.acting_player,
                state.hand_index,
                state.field_index,
                state.source,
                true,
            );
            return;
        }

        let parked = self.run_interactive_digivolve_reducer_pay_cost(
            state.key,
            state.target,
            state.acting_player,
            state.hand_index,
            state.field_index,
            state.source,
        );
        if !parked {
            self.digivolve_from_hand_inner(
                state.acting_player,
                state.hand_index,
                state.field_index,
                state.source,
                true,
            );
        }
    }

    /// Run a field-hosted interactive digivolve reducer's `pay_cost`. Returns
    /// `true` if it PARKED on a selection (the continuation that credits the
    /// reduction + re-enters the digivolve has been wired behind the park), or
    /// `false` if it resolved synchronously (the reduction was credited into
    /// `pending_interactive_digivolve_reduction` iff the cost was actually paid;
    /// the CALLER must drive the digivolve from here — it does NOT re-enter on
    /// the synchronous path).
    /// `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
    fn run_interactive_digivolve_reducer_pay_cost(
        &mut self,
        key: CostReductionKey,
        target: CostTargetContext,
        acting_player: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
    ) -> bool {
        let amount = self
            .inspect_cost_reduction_candidate(&key, Some(target))
            .unwrap_or(0);
        // `effects_for_card_owned` returns an OWNED Vec rebuilt each call, so move
        // the (non-`Clone`) boxed `pay_cost_fn` out of it rather than borrowing
        // (the memoized `effects_for_card` hands back a shared `Arc`).
        let Some(mut effects) = self.effects_for_card_owned(&key.card_id, key.source_card) else {
            return false;
        };
        let Some(effect) = effects.get_mut(key.effect_slot as usize) else {
            return false;
        };
        let max_per_turn = effect.max_per_turn;
        let Some(pay_cost_fn) = effect.pay_cost_fn.take() else {
            return false;
        };

        let pending_before = self.pending_selection.is_some();
        let (synchronous_ok, cost_unpayable) = {
            let mut ctx = EffectContext::new_with_cost_target(
                self,
                key.source_card,
                key.source_permanent,
                key.controller,
                target.card,
                target.from_hand,
            );
            ctx.cost_is_digivolve = target.is_digivolve;
            let ok = pay_cost_fn(&mut ctx);
            (ok, ctx.cost_unpayable)
        };
        let parked = !pending_before && self.pending_selection.is_some();

        if parked {
            // The pay_cost installed a selection (the interactive Tamer pick).
            // The pick is mandatory once reached (no PASS for
            // `trash_bottom_face_down_source_under_tamer`), so it WILL be paid
            // on resolution — credit the reduction behind the park.
            if max_per_turn > 0 {
                self.record_cost_reducer_activation(&key);
            }
            self.wrap_interactive_digivolve_reducer_continuation(
                amount,
                acting_player,
                hand_index,
                field_index,
                source,
            );
            return true;
        }

        // Synchronous outcome (no park): credit only when actually paid — a
        // `cost_unpayable` abort (no eligible Tamer) credits nothing. The caller
        // drives the digivolve.
        if synchronous_ok && !cost_unpayable {
            if max_per_turn > 0 {
                self.record_cost_reducer_activation(&key);
            }
            self.pending_interactive_digivolve_reduction += amount;
        }
        false
    }

    /// Wrap the parked pay_cost selection so its resolution credits `amount`
    /// into `pending_interactive_digivolve_reduction` and re-enters the
    /// digivolve. `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
    fn wrap_interactive_digivolve_reducer_continuation(
        &mut self,
        amount: i32,
        acting_player: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
    ) {
        let Some(mut pending) = self.pending_selection.take() else {
            return;
        };
        // Resume-aware (make-engine-cloneable Batch 2): defer the continuation
        // onto the resume channel when the parked select is data-driven (its
        // closure is bypassed by `run_resume`).
        if self.pending_selection_resume.is_some() {
            self.pending_selection = Some(pending);
            self.after_selection_resume_hooks.0.push(
                crate::resume::AfterSelectionHook::InteractiveDigivolveReducer {
                    amount,
                    acting_player,
                    hand_index,
                    field_index,
                    source,
                },
            );
            return;
        }
        let original_callback = pending.callback;
        pending.callback = Box::new(move |game: &mut Game, action_id: u16| {
            // The pay_cost's own callback pays the cost (e.g. trashes the
            // face-down source). It may install nested selections; if so, the
            // resume re-wraps until the cost fully resolves.
            original_callback(game, action_id);
            game.resume_interactive_digivolve_reducer_after_pending(
                amount,
                acting_player,
                hand_index,
                field_index,
                source,
            );
        });
        // The pay_cost selection itself is mandatory once entered (the player
        // already opted in via the accept gate, or the reducer is mandatory).
        // Preserve any inner decline path the pay_cost installed.
        let original_decline = pending.on_decline.take();
        pending.on_decline = original_decline.map(|orig| {
            Box::new(move |game: &mut Game| {
                orig(game);
                game.resume_interactive_digivolve_reducer_after_pending(
                    amount,
                    acting_player,
                    hand_index,
                    field_index,
                    source,
                );
            }) as crate::selection::DeclineCallback
        });
        self.pending_selection = Some(pending);
    }

    /// Resume the digivolve after an interactive reducer's parked pay_cost
    /// resolves. If the pay_cost installed a FURTHER selection, re-wrap; else
    /// credit the reduction (only when the cost was actually paid) and re-enter
    /// the digivolve. `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
    pub(crate) fn resume_interactive_digivolve_reducer_after_pending(
        &mut self,
        amount: i32,
        acting_player: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
    ) {
        if self.pending_selection.is_some() {
            self.wrap_interactive_digivolve_reducer_continuation(
                amount,
                acting_player,
                hand_index,
                field_index,
                source,
            );
            return;
        }
        // The parked pay_cost resolved (the mandatory Tamer pick paid the
        // cost). Credit the reduction and re-enter the digivolve. A
        // `delete_for_cost_reduction` in the pay_cost stages a VARIABLE amount in
        // `pending_cost_reduction_amount_override` (drained here so it never
        // leaks into a later cost). `G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST`.
        let variable = self
            .pending_cost_reduction_amount_override
            .take()
            .unwrap_or(0);
        self.pending_interactive_digivolve_reduction += amount + variable;
        self.digivolve_from_hand_inner(acting_player, hand_index, field_index, source, true);
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
        // Synchronous payments can compact or remove the declared host and
        // later reducer sources. Pin these stacks before running any payment.
        let target_identities = cost_target
            .filter(|target| target.is_digivolve)
            .map(|target| {
                target.target_permanents.map(|handle| {
                    handle
                        .filter(|handle| {
                            handle.index != crate::action::space::BREEDING_TARGET as u8
                        })
                        .map(|handle| (handle.player, self.capture_digivolve_host_identity(handle)))
                })
            });
        let candidates: Vec<_> = candidates
            .into_iter()
            .map(|candidate| {
                let source_identity = if target_identities.is_some() {
                    candidate
                        .key
                        .source_permanent
                        .filter(|handle| {
                            handle.index != crate::action::space::BREEDING_TARGET as u8
                        })
                        .map(|handle| (handle.player, self.capture_digivolve_host_identity(handle)))
                } else {
                    None
                };
                (candidate, source_identity)
            })
            .collect();
        let mut total = 0;
        for (mut candidate, source_identity) in candidates {
            let mut cost_target = cost_target;
            if let (Some(identities), Some(target)) = (target_identities, cost_target.as_mut()) {
                if target.from_hand
                    && !self
                        .player(acting_player)
                        .hand
                        .iter()
                        .any(|card| card.handle() == target.card)
                {
                    break;
                }
                for (index, identity) in identities.into_iter().enumerate() {
                    if let Some((player, identity)) = identity {
                        let Some(handle) = self.resolve_digivolve_host_identity(player, identity)
                        else {
                            return total;
                        };
                        target.target_permanents[index] = Some(handle);
                    }
                }
            }
            if let Some((player, identity)) = source_identity {
                let Some(handle) = self.resolve_digivolve_host_identity(player, identity) else {
                    continue;
                };
                if !self.player(player).battle_area[handle.index as usize]
                    .card_sources
                    .iter()
                    .any(|card| card.handle() == candidate.key.source_card)
                {
                    continue;
                }
                candidate.key.source_permanent = Some(handle);
            }
            // An INTERACTIVE pay_cost reducer (its `pay_cost` installs a
            // selection — e.g. `trash_bottom_face_down_source_under_tamer`)
            // cannot be resolved by this SYNCHRONOUS scan: a park would leave a
            // dangling `pending_selection` mid-digivolve/Option-use. Such a
            // reducer is handled by a dedicated pre-scan interactive prompt
            // (`try_prompt_interactive_digivolve_cost_reducer` / the Option-use
            // sibling) that wraps the play continuation behind the park and
            // re-enters with the reduction pre-credited. Skip it here so it is
            // never double-applied on re-entry.
            // `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
            if candidate.pay_cost_interactive {
                continue;
            }
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
                self.apply_cost_reduction_candidate(&candidate.key, resolved_target, false)
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
        let reductions_blocked = self
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
                ));

        let mut candidates = Vec::new();
        // `when_playing_this` reducers are scoped to "when THIS card would be
        // PLAYED" (or, for Options, used) — a DIGIVOLVE cost must not host the
        // target hand card's own play-cost reducers. DCGO gates these with
        // `CanTriggerWhenPermanentWouldPlay`, which a digivolve never fires
        // (e.g. BT18-073 "When this card would be played, by deleting 1 of
        // your [Composite] Digimon, reduce the play cost by 4" must not be
        // offered when digivolving BT18-073 over a Lv.5 [Composite] base).
        // Dropping the target here removes ONLY the target-hosted
        // `when_playing_this` block from `before_pay_cost_source_infos`;
        // field/breeding-hosted reducers still collect, and the full
        // `cost_target` still threads into candidate inspection below so
        // target-aware predicates keep firing. Caught by
        // `tests/alt_path_reachability.rs` (BT18-073).
        let source_scan_target = cost_target.filter(|t| !t.is_digivolve);
        for info in
            self.before_pay_cost_source_infos(acting_player, source_scan_target.map(|t| t.card))
        {
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
            // §15-7-4 (Ver. 4.3): an optional reducer whose "by X" condition
            // can't be executed can't be chosen -- it is not a candidate at
            // all (no "Cost reduction" choice, no accept prompt).
            // G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED.
            if effect.optional && !self.cost_reducer_condition_payable(&key, effect) {
                continue;
            }
            if effect.cost_assignment.is_some() {
                if cost_kind != CostReductionKind::Play {
                    continue;
                }
            } else if reductions_blocked {
                continue;
            }
            if amount <= 0 && effect.pay_cost_fn.is_none() && effect.cost_assignment.is_none() {
                continue;
            }
            candidates.push(CostReductionCandidate {
                assignment: effect.cost_assignment,
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
                pay_cost_interactive: effect.pay_cost_interactive,
            });
        }
        candidates
    }

    pub(crate) fn inspect_cost_reduction_candidate(
        &self,
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

    /// Run a cost reducer's `pay_cost` (if any) and, on success, return the
    /// reduction `amount` to credit.
    ///
    /// `allow_interactive_pay_cost` controls how a `pay_cost` that PARKS on a
    /// `PendingSelection` (e.g. `trash_bottom_face_down_source_under_tamer`'s
    /// mandatory Tamer pick — a 1-option selection the no-approximations
    /// contract never auto-resolves) is treated:
    ///
    /// - `true` (the **play-from-hand chain** call sites): a parked pay_cost
    ///   WILL be paid when its selection resolves, so credit the `amount` now
    ///   and let the caller wrap the play continuation behind the park
    ///   (`continue_play_from_hand_cost_reduction_chain` already wraps when
    ///   `pending_selection.is_some()` after this returns). Closes
    ///   `G-COST-REDUCTION-INTERACTIVE-PAY-COST` for the play path.
    /// - `false` (the **synchronous scan** path, which cannot wrap a
    ///   continuation): a parked pay_cost is treated as a non-credit — preserves
    ///   the pre-existing behavior (the Option-use / digivolve scan paths
    ///   already pre-filter optional/paid reducers, so no card reaches here and
    ///   parks today; the guard keeps a future one from crediting a reduction it
    ///   cannot honour).
    ///
    /// A genuine synchronous failure (the pay_cost returns `false` WITHOUT
    /// installing a selection — e.g. an unpayable cost) never credits, in
    /// either mode.
    pub(crate) fn apply_cost_reduction_candidate(
        &mut self,
        key: &CostReductionKey,
        cost_target: CostTargetContext,
        allow_interactive_pay_cost: bool,
    ) -> Option<i32> {
        let amount = self.inspect_cost_reduction_candidate(key, Some(cost_target))?;
        let effects = self.effects_for_card(&key.card_id, key.source_card)?;
        let effect = effects.get(key.effect_slot as usize)?;
        if effect.cost_assignment.is_some() {
            return None;
        }
        if let Some(pay_cost_fn) = &effect.pay_cost_fn {
            let pending_before = self.pending_selection.is_some();
            let mut ctx = EffectContext::new_with_cost_target(
                self,
                key.source_card,
                key.source_permanent,
                key.controller,
                cost_target.card,
                cost_target.from_hand,
            );
            ctx.cost_is_digivolve = cost_target.is_digivolve;
            ctx.cost_target_permanents = cost_target.target_permanents_vec();
            if !pay_cost_fn(&mut ctx) {
                // The pay_cost did not complete synchronously. Distinguish a
                // PARK (it installed a fresh PendingSelection — the cost will be
                // paid on resolution) from a genuine synchronous failure.
                let parked = allow_interactive_pay_cost
                    && !pending_before
                    && self.pending_selection.is_some();
                if !parked {
                    return None;
                }
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
        // Route through the shared OPT slot so the two `scope: both` copies
        // record onto ONE per-turn counter. G-ENGINE-SHARED-OPT-SCOPE-BOTH-REDUCER.
        let opt_slot = self.cost_reducer_opt_slot(key);
        if source.index == crate::action::space::BREEDING_TARGET as u8 {
            if let Some(perm) = self.player_mut(source.player).breeding_area.as_mut() {
                perm.record_activation(key.source_card, opt_slot);
            }
            return;
        }
        if let Some(perm) = self
            .player_mut(source.player)
            .battle_area
            .get_mut(source.index as usize)
        {
            perm.record_activation(key.source_card, opt_slot);
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
        // Same play-only scoping as the reducer scan: a target-hosted
        // `when_playing_this` observer ("when this card would be played, ...")
        // must not fire for a DIGIVOLVE cost. Field-hosted observers (e.g.
        // BT12-022's "when this Digimon would DNA digivolve into ..." — gated
        // by `source_is_cost_target_permanent`, not `when_playing_this`) are
        // unaffected; the full `cost_target` still threads into condition/body
        // contexts below. No current observer sets `when_playing_this`, so
        // this is future-proofing kept in lockstep with
        // `collect_before_pay_cost_reducers`.
        let observer_scan_target = cost_target.filter(|t| !t.is_digivolve);
        let infos = self
            .before_pay_cost_observer_infos(acting_player, observer_scan_target.map(|t| t.card));
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
                let mut ctx = if let Some(target) = cost_target {
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
                if let Some(target) = cost_target {
                    ctx.cost_is_digivolve = target.is_digivolve;
                }
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
                if let Some(target) = cost_target {
                    ctx.cost_is_digivolve = target.is_digivolve;
                    ctx.cost_target_permanents = target.target_permanents_vec();
                }
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
        let opt_slot = self.observer_opt_slot(info);
        if let Some(perm) = self
            .player_mut(source.player)
            .battle_area
            .get_mut(source.index as usize)
        {
            perm.record_activation(info.source_card, opt_slot);
        }
    }
}
