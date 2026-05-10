//! Triggered-effect queue + drainer.
//!
//! When one or more effects fire at the same timing window (OnPlay, OnAttack,
//! OnDeletion, EndOfYourTurn, ...), `enqueue_triggered` collects them into
//! `Game.effect_queue`, then `drain_effect_queue` resolves them one at a
//! time.
//!
//! Ordering rules (per Digimon TCG, confirmed against DCGO):
//! - The **turn player** resolves all of their queued triggers before the
//!   non-turn-player resolves any of theirs.
//! - Within a single controller's bundle, **the controller picks the
//!   order**. If only one effect is queued for them, it auto-fires; if
//!   multiple are queued, a `TriggerOrder` selection prompts for order.
//! - Optional triggers may be declined individually. When the remaining
//!   triggers for the current chooser are all optional, the prompt carries
//!   a PASS bit that declines **all of them at once**.
//!
//! The drainer has a hard cap of `MAX_CHAIN_DEPTH` iterations as a safety
//! rail against self-triggering loops. Matches Python's
//! `_resolve_effect_stack` max=50 bound.

use crate::action::space::{
    BREEDING_TARGET, HAND_EFFECT_END, HAND_EFFECT_START, HAND_MAIN_LIMIT, PASS,
};
use crate::card_source::CardHandle;
use crate::effect::ReFireableEffect;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{
    CardKind, DelayTrigger, EffectSourceKind, EffectTiming, GamePhase, ModifierType, PlayerId,
};
use crate::game::{DelayedOptionLifecycleResume, DelayedOptionLifecycleResumeKind, Game};
use crate::permanent::{OptionState, PermanentHandle};
use crate::replacement::ReplacementCause;
use crate::selection::{
    DeclineCallback, EffectChoiceEntry, PendingEffectSecurityRemoval, PendingPayCostEffect,
    PendingSecurity, PendingSelection, QueuedEffect, SecurityRemovalDestination, SelectionKind,
    TriggerSource,
};
use crate::trigger_context::TriggerContext;

/// Max iterations the drainer will take before aborting a suspected infinite
/// chain. Matches Python's `_resolve_effect_stack` limit.
pub const MAX_CHAIN_DEPTH: u16 = 50;

fn source_kind_for_card_kind(kind: CardKind) -> EffectSourceKind {
    match kind {
        CardKind::Digimon | CardKind::DigiEgg | CardKind::Dual => EffectSourceKind::Digimon,
        CardKind::Tamer => EffectSourceKind::Tamer,
        CardKind::Option => EffectSourceKind::Option,
        CardKind::Token => EffectSourceKind::Rule,
    }
}

fn security_activation_blocked_for_timing(
    game: &Game,
    player: PlayerId,
    timing: EffectTiming,
) -> bool {
    matches!(timing, EffectTiming::SecuritySkill)
        && game
            .modifiers
            .player_has(player, ModifierType::CannotActivateSecurityEffects)
}

fn permanent_activation_blocked_for_timing(
    game: &Game,
    handle: PermanentHandle,
    timing: EffectTiming,
) -> bool {
    // ── Per-timing player-scoped/permanent-scoped category gates ──
    let category_block = match timing {
        EffectTiming::WhenDigivolving => game
            .modifiers
            .has(handle, ModifierType::CannotActivateWhenDigivolvingEffects),
        EffectTiming::WhenAttacking => game
            .modifiers
            .has(handle, ModifierType::CannotActivateWhenAttackingEffects),
        _ => false,
    };
    if category_block {
        return true;
    }

    // ── DisableEffect timing-suppression ──
    // Track C taxonomy (2026-05-06): a permanent-scoped
    // `ModifierType::DisableEffect` entry whose `disable_effect_timing`
    // matches the firing timing suppresses that timing only on this
    // permanent. Other timings on the same permanent fire normally.
    // Mirrors DCGO `DisableEffectClass.cs`. Used by the TS Olympos
    // timing-suppression slice (see RUST_ENGINE_API.md "Modifier
    // consult-site checklist").
    if game.modifiers.is_timing_disabled(handle, timing) {
        return true;
    }

    false
}

impl Game {
    // ─── Public API ─────────────────────────────────────────────────

    fn queued_refired_effect(&self, effect: ReFireableEffect) -> QueuedEffect {
        let is_turn_player = effect.controller == self.turn_player();
        QueuedEffect {
            source_card: effect.source_card,
            source_permanent: Some(effect.source),
            source_kind: effect.source_kind,
            attribution_source_card: effect.attribution_source_card,
            attribution_source_kind: effect.attribution_source_kind,
            bypass_once_per_turn: effect.bypass_once_per_turn,
            controller: effect.controller,
            timing: effect.timing,
            trigger_context: None,
            effect_slot: effect.effect_id,
            is_optional: false,
            is_turn_player,
            card_id: effect.card_id,
            allow_below_top_liveness: false,
            dna_origin_context: None,
            granted_effect_id: None,
        }
    }

    pub(crate) fn run_refired_effect(&mut self, effect: ReFireableEffect) {
        let qe = self.queued_refired_effect(effect);
        self.run_queued_effect(qe);
    }

    pub(crate) fn install_refire_effect_selection(
        &mut self,
        chooser: PlayerId,
        effects: Vec<ReFireableEffect>,
        optional: bool,
    ) {
        if effects.is_empty() {
            return;
        }

        let capped = effects.len().min(HAND_MAIN_LIMIT);
        let mut valid_action_ids: Vec<u16> = Vec::with_capacity(capped);
        let mut choices: Vec<EffectChoiceEntry> = Vec::with_capacity(capped);
        for (pos, effect) in effects.iter().take(capped).enumerate() {
            let action_id = HAND_EFFECT_START + pos as u16;
            let observation_metadata = self
                .effects_for_card(&effect.card_id, effect.source_card)
                .and_then(|effects| {
                    effects
                        .get(effect.effect_id as usize)
                        .map(|effect| effect.observation_metadata)
                })
                .unwrap_or_default();
            valid_action_ids.push(action_id);
            choices.push(EffectChoiceEntry {
                label: format!("{} {}", effect.card_id, effect.timing_key),
                action_id,
                source_card: Some(effect.source_card),
                source_kind: Some(effect.source_kind),
                timing: Some(effect.timing),
                is_optional: optional,
                observation_metadata,
            });
        }

        let head = effects[0].clone();
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;
        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::EffectChoice,
            selecting_player: chooser,
            previous_phase,
            valid_action_ids,
            is_optional: optional,
            prompt: "Choose an effect to activate".to_string(),
            effect_choices: Some(choices),
            source_card: head.source_card,
            source_permanent: Some(head.source),
            source_kind: head.source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let pos = action_id.saturating_sub(HAND_EFFECT_START) as usize;
                if let Some(effect) = effects.get(pos).cloned() {
                    game.run_refired_effect(effect);
                }
            }),
            on_decline: optional.then(|| Box::new(|_game: &mut Game| {}) as DeclineCallback),
        });
    }

    /// Collect every effect on `source` whose timing matches `timing` and
    /// whose `is_*` flag matches the timing, append them to `effect_queue`.
    ///
    /// **Does not drive execution.** Call `drain_effect_queue()` afterward
    /// to resolve the collected effects, or call `enqueue_triggered` for
    /// multiple sources first and drain once at the end.
    pub fn enqueue_triggered(&mut self, timing: EffectTiming, source: TriggerSource) {
        match source {
            TriggerSource::Permanent(handle) => {
                let trigger_context = self.trigger_context_for_source(&source, Some(handle));
                self.enqueue_from_permanent(timing, handle, Some(trigger_context));
            }
            TriggerSource::PlayerBattleArea(player) => {
                // Snapshot indices up-front. Firing an effect via the drainer
                // can mutate the battle_area, but enqueueing itself is pure.
                let count = self.player(player).battle_area.len();
                for i in 0..count {
                    let handle = PermanentHandle {
                        player,
                        index: i as u8,
                    };
                    let trigger_context = self.trigger_context_for_source(&source, Some(handle));
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
            }
            TriggerSource::PlayerBreedingArea(player) => {
                let handle = PermanentHandle {
                    player,
                    index: BREEDING_TARGET as u8,
                };
                let trigger_context = self.trigger_context_for_source(&source, Some(handle));
                self.enqueue_from_breeding_permanent(timing, handle, Some(trigger_context));
            }
            TriggerSource::SecurityRevealed { defender, card } => {
                let trigger_context = self.trigger_context_for_source(&source, None);
                self.enqueue_from_security_card(
                    timing,
                    defender,
                    card,
                    Some(trigger_context.clone()),
                );
                if timing == EffectTiming::OnLoseSecurity {
                    let count = self.player(defender).battle_area.len();
                    for i in 0..count {
                        let handle = PermanentHandle {
                            player: defender,
                            index: i as u8,
                        };
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context.clone()));
                    }
                }
            }
            TriggerSource::SecurityStackCard { player, card } => {
                let trigger_context = self.trigger_context_for_source(&source, None);
                self.enqueue_from_security_stack_card(timing, player, card, Some(trigger_context));
            }
            TriggerSource::OnSecurityCheck { defender, .. } => {
                // Observer timing: scan every permanent in the defender's
                // battle area for `OnSecurityCheck`-timed effects. Attacker
                // + revealed card metadata are carried through
                // `game.security_resolution` for the drained effects to
                // read via `EffectContext::attacker` / `security_digimon`
                // / the defender's `last_security_reveal` snapshot.
                let count = self.player(defender).battle_area.len();
                for i in 0..count {
                    let handle = PermanentHandle {
                        player: defender,
                        index: i as u8,
                    };
                    let trigger_context = self.trigger_context_for_source(&source, Some(handle));
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
            }
            TriggerSource::MovedFromBreeding { player, .. } => {
                let count = self.player(player).battle_area.len();
                for i in 0..count {
                    let handle = PermanentHandle {
                        player,
                        index: i as u8,
                    };
                    let trigger_context = self.trigger_context_for_source(&source, Some(handle));
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
            }
            TriggerSource::Digivolved { .. } => {
                for player in 0..self.players.len() {
                    let player = player as PlayerId;
                    let count = self.player(player).battle_area.len();
                    for i in 0..count {
                        let handle = PermanentHandle {
                            player,
                            index: i as u8,
                        };
                        let trigger_context =
                            self.trigger_context_for_source(&source, Some(handle));
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                }
            }
            TriggerSource::EnteredField { player, .. } => {
                let scan_all_players = timing != EffectTiming::OnAllyPlayed;
                let start = if scan_all_players { 0 } else { player as usize };
                let end = if scan_all_players {
                    self.players.len()
                } else {
                    (player as usize).saturating_add(1).min(self.players.len())
                };
                for scan_player in start..end {
                    let player = scan_player as PlayerId;
                    let count = self.player(player).battle_area.len();
                    for i in 0..count {
                        let handle = PermanentHandle {
                            player,
                            index: i as u8,
                        };
                        let trigger_context =
                            self.trigger_context_for_source(&source, Some(handle));
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                }
                if timing == EffectTiming::OnAllyPlayed {
                    let trigger_context = self.trigger_context_for_source(&source, None);
                    self.enqueue_from_player_trash(timing, player, Some(trigger_context));
                }
            }
            TriggerSource::OptionPlaced { .. } => {
                for player in 0..self.players.len() {
                    let player = player as PlayerId;
                    let count = self.player(player).battle_area.len();
                    for i in 0..count {
                        let handle = PermanentHandle {
                            player,
                            index: i as u8,
                        };
                        let trigger_context =
                            self.trigger_context_for_source(&source, Some(handle));
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                    let breeding_handle = PermanentHandle {
                        player,
                        index: BREEDING_TARGET as u8,
                    };
                    let trigger_context =
                        self.trigger_context_for_source(&source, Some(breeding_handle));
                    self.enqueue_from_breeding_permanent(
                        timing,
                        breeding_handle,
                        Some(trigger_context),
                    );
                }
            }
            TriggerSource::OptionTrashed { .. } => {
                for player in 0..self.players.len() {
                    let player = player as PlayerId;
                    let count = self.player(player).battle_area.len();
                    for i in 0..count {
                        let handle = PermanentHandle {
                            player,
                            index: i as u8,
                        };
                        let trigger_context =
                            self.trigger_context_for_source(&source, Some(handle));
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                    let breeding_handle = PermanentHandle {
                        player,
                        index: BREEDING_TARGET as u8,
                    };
                    let trigger_context =
                        self.trigger_context_for_source(&source, Some(breeding_handle));
                    self.enqueue_from_breeding_permanent(
                        timing,
                        breeding_handle,
                        Some(trigger_context),
                    );
                }
            }
            TriggerSource::EventObserved { .. } | TriggerSource::AttackTargetChanged { .. } => {
                for player in 0..self.players.len() {
                    let player = player as PlayerId;
                    let count = self.player(player).battle_area.len();
                    for i in 0..count {
                        let handle = PermanentHandle {
                            player,
                            index: i as u8,
                        };
                        let trigger_context =
                            self.trigger_context_for_source(&source, Some(handle));
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                }
            }
            TriggerSource::SourceTrashedFromStack { player, card, .. } => {
                let trigger_context = self.trigger_context_for_source(&source, None);
                self.enqueue_from_trashed_source(timing, player, card, Some(trigger_context));
                for player in 0..self.players.len() {
                    let player = player as PlayerId;
                    let count = self.player(player).battle_area.len();
                    for i in 0..count {
                        let handle = PermanentHandle {
                            player,
                            index: i as u8,
                        };
                        let trigger_context =
                            self.trigger_context_for_source(&source, Some(handle));
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                }
            }
            TriggerSource::SecurityRemoved {
                observer_player, ..
            } => {
                let count = self.player(observer_player).battle_area.len();
                for i in 0..count {
                    let handle = PermanentHandle {
                        player: observer_player,
                        index: i as u8,
                    };
                    let trigger_context = self.trigger_context_for_source(&source, Some(handle));
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
                let breeding_handle = PermanentHandle {
                    player: observer_player,
                    index: BREEDING_TARGET as u8,
                };
                let trigger_context =
                    self.trigger_context_for_source(&source, Some(breeding_handle));
                self.enqueue_from_breeding_permanent(
                    timing,
                    breeding_handle,
                    Some(trigger_context),
                );
            }
            TriggerSource::SecurityPlaced {
                affected_player, ..
            } => {
                let count = self.player(affected_player).battle_area.len();
                for i in 0..count {
                    let handle = PermanentHandle {
                        player: affected_player,
                        index: i as u8,
                    };
                    let trigger_context = self.trigger_context_for_source(&source, Some(handle));
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
                let breeding_handle = PermanentHandle {
                    player: affected_player,
                    index: BREEDING_TARGET as u8,
                };
                let trigger_context =
                    self.trigger_context_for_source(&source, Some(breeding_handle));
                self.enqueue_from_breeding_permanent(
                    timing,
                    breeding_handle,
                    Some(trigger_context),
                );
            }
            TriggerSource::SecurityDiscarded {
                affected_player,
                card,
                ..
            } => {
                let trigger_context = self.trigger_context_for_source(&source, None);
                self.enqueue_from_security_card(
                    timing,
                    affected_player,
                    card,
                    Some(trigger_context),
                );
            }
        }
        if matches!(
            source,
            TriggerSource::EventObserved { .. } | TriggerSource::AttackTargetChanged { .. }
        ) {
            let trigger_context = self.trigger_context_for_source(&source, None);
            self.enqueue_event_gated_delayed_options(timing, trigger_context);
        }
    }

    /// Collect inherited effects from the exact source card that was just
    /// trashed out of a digivolution stack. The card is no longer live under
    /// its host by this point, so the queued effect intentionally carries no
    /// source permanent and reads host/trashed-source details from trigger
    /// context predicates such as `host_permanent_trait_has`.
    fn enqueue_from_trashed_source(
        &mut self,
        timing: EffectTiming,
        controller: PlayerId,
        source_card: CardHandle,
        trigger_context: Option<TriggerContext>,
    ) {
        let Some(card_data) = self.card_data_for_handle(source_card) else {
            return;
        };
        let card_id = card_data.card_id.clone();
        let source_kind = source_kind_for_card_kind(card_data.card_kind);
        let Some(effects) = self.effects_for_card(&card_id, source_card) else {
            return;
        };
        let is_turn_player = controller == self.turn_player();
        for (slot, effect) in effects.iter().enumerate() {
            if !effect.inherited {
                continue;
            }
            if !timing_flag_matches(effect, timing) {
                continue;
            }
            self.effect_queue.push_back(QueuedEffect {
                source_card,
                source_permanent: None,
                source_kind,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                controller,
                timing,
                trigger_context: trigger_context.clone(),
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.clone(),
                allow_below_top_liveness: false,
                dna_origin_context: self.current_dna_origin,
                granted_effect_id: None,
            });
        }
    }

    /// Collect top-level observer effects from a player's trash. These
    /// sources are not attached to a live permanent; the trigger payload
    /// carries the played/deleted/moved subject instead.
    fn enqueue_from_player_trash(
        &mut self,
        timing: EffectTiming,
        controller: PlayerId,
        trigger_context: Option<TriggerContext>,
    ) {
        let trash_sources: Vec<(String, CardHandle, EffectSourceKind)> = self
            .players
            .get(controller as usize)
            .map(|player| {
                player
                    .trash
                    .iter()
                    .map(|card| {
                        (
                            card.card_id(&self.card_data).to_string(),
                            card.handle(),
                            source_kind_for_card_kind(card.card_kind(&self.card_data)),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        let is_turn_player = controller == self.turn_player();
        for (card_id, source_card, source_kind) in trash_sources {
            let Some(effects) = self.effects_for_card(&card_id, source_card) else {
                continue;
            };
            for (slot, effect) in effects.iter().enumerate() {
                if effect.inherited || effect.linked {
                    continue;
                }
                if !timing_flag_matches(effect, timing) {
                    continue;
                }
                self.effect_queue.push_back(QueuedEffect {
                    source_card,
                    source_permanent: None,
                    source_kind,
                    attribution_source_card: None,
                    attribution_source_kind: None,
                    bypass_once_per_turn: false,
                    controller,
                    timing,
                    trigger_context: trigger_context.clone(),
                    effect_slot: slot as u8,
                    is_optional: effect.optional,
                    is_turn_player,
                    card_id: card_id.clone(),
                    allow_below_top_liveness: false,
                    dna_origin_context: self.current_dna_origin,
                    granted_effect_id: None,
                });
            }
        }
    }

    /// Drain the effect queue. Fires each queued effect in order, pausing
    /// when an effect installs a `pending_selection` or when the queue
    /// contains multiple triggers for a single chooser (installs a
    /// `TriggerOrder` selection and returns).
    ///
    /// Idempotent — safe to call when the queue is already empty. Callers
    /// should invoke this after every `enqueue_triggered` call, and again
    /// after `resolve_selection` unless that call installed a new selection.
    pub fn drain_effect_queue(&mut self) {
        loop {
            if self.pending_selection.is_some() {
                return;
            }
            if self.effect_queue.is_empty() {
                // Track H §3 Phase 4i superseded the prior inline-fire
                // flush: granted-triggered-effect entries now ride the
                // standard `QueuedEffect` queue via `granted_effect_id`
                // so they compose with selections, max-per-turn, etc.
                // The `pending_granted_fires` Vec is no longer driven
                // by enqueue_from_permanent; left in place as a no-op
                // (and remains usable by raw_rust callers of
                // `Game::fire_granted_triggered_effects`).
                self.effect_chain_depth = 0;
                self.reevaluate_until_condition_modifiers_if_dirty();
                return;
            }

            self.effect_chain_depth = self.effect_chain_depth.saturating_add(1);
            if self.effect_chain_depth > MAX_CHAIN_DEPTH {
                // Suspected self-triggering loop — drop the remaining queue.
                // Matches Python's defensive behavior.
                self.effect_queue.clear();
                self.effect_chain_depth = 0;
                return;
            }

            let Some(chooser) = self.next_chooser() else {
                self.effect_chain_depth = 0;
                return;
            };

            let bundle: Vec<usize> = self
                .effect_queue
                .iter()
                .enumerate()
                .filter_map(|(i, qe)| (qe.controller == chooser).then_some(i))
                .collect();

            debug_assert!(
                !bundle.is_empty(),
                "next_chooser returned a player with no queued effects"
            );

            if bundle.len() == 1 {
                let idx = bundle[0];
                // Single trigger — auto-fire into its body. Optionality is
                // exposed by the body's first actionable pending selection;
                // this avoids prompting for optional effects whose filters
                // produce no legal follow-up.
                let qe = self
                    .effect_queue
                    .remove(idx)
                    .expect("bundle index in-bounds by construction");
                self.run_queued_effect(qe);
                continue;
            }

            // §2.5i: a security-reveal bundle coming from a single
            // revealed card must auto-fire in collection order with no
            // prompt. Python's `_execute_security_checks` iterates the
            // effect list in order and fires each; Rust previously
            // installed a `TriggerOrder` selection for `≥2` effects.
            let is_single_source_security = bundle.iter().all(|&i| {
                let qe = &self.effect_queue[i];
                qe.timing == EffectTiming::SecuritySkill
            }) && bundle
                .iter()
                .map(|&i| self.effect_queue[i].source_card)
                .all(|sc| sc == self.effect_queue[bundle[0]].source_card);

            if is_single_source_security {
                // Drain in the same order the queue holds them.
                let indices = bundle.clone();
                for (k, &i) in indices.iter().enumerate() {
                    // Each remove shifts later indices down by one
                    // relative to `i`. Subtract `k` to compensate.
                    let qe = self
                        .effect_queue
                        .remove(i - k)
                        .expect("bundle index in-bounds by construction");
                    self.run_queued_effect(qe);
                }
                continue;
            }

            // Multi-trigger bundle — install a TriggerOrder selection.
            // Cap at HAND_MAIN_LIMIT (30) to fit the reused 30-59 action
            // range. Overflow auto-fires in collection order after the prompt
            // completes (rare; see the cap handling inside install_*).
            let any_mandatory = bundle.iter().any(|&i| !self.effect_queue[i].is_optional);
            self.install_trigger_order_selection(chooser, &bundle, !any_mandatory);
            return;
        }
    }

    /// Drain triggered effects while exposing the DNA-origin bit to condition
    /// and process contexts. The previous value is restored after the drain.
    pub(crate) fn drain_effect_queue_with_dna_origin(&mut self, dna_origin: bool) {
        let prev = self.current_dna_origin;
        self.current_dna_origin = Some(dna_origin);
        self.drain_effect_queue();
        self.current_dna_origin = prev;
    }

    // ─── Internal helpers ───────────────────────────────────────────

    fn trigger_context_for_source(
        &self,
        source: &TriggerSource,
        source_permanent: Option<PermanentHandle>,
    ) -> TriggerContext {
        let mut context = match *source {
            TriggerSource::Permanent(handle) => TriggerContext {
                target_permanent: Some(handle),
                target_card: self.top_card_handle(handle),
                source_player: Some(handle.player),
                ..TriggerContext::default()
            },
            TriggerSource::PlayerBattleArea(player) => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                source_player: Some(player),
                ..TriggerContext::default()
            },
            TriggerSource::PlayerBreedingArea(player) => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                source_player: Some(player),
                ..TriggerContext::default()
            },
            TriggerSource::SecurityRevealed { defender, card } => TriggerContext {
                target_card: Some(card),
                event_card: Some(card),
                source_player: Some(defender),
                was_security_skill: true,
                ..TriggerContext::default()
            },
            TriggerSource::SecurityStackCard { player, card } => TriggerContext {
                target_card: Some(card),
                event_card: Some(card),
                source_player: Some(player),
                was_security_skill: false,
                ..TriggerContext::default()
            },
            TriggerSource::OnSecurityCheck {
                attacker,
                defender,
                revealed_card,
                ..
            } => TriggerContext {
                target_permanent: Some(attacker),
                target_card: Some(revealed_card),
                event_card: Some(revealed_card),
                source_player: Some(defender),
                was_security_skill: false,
                ..TriggerContext::default()
            },
            TriggerSource::MovedFromBreeding {
                player,
                permanent,
                card,
            } => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_permanent: Some(permanent),
                event_card: Some(card),
                source_player: Some(player),
                ..TriggerContext::default()
            },
            TriggerSource::Digivolved {
                player,
                permanent,
                card,
                effect_initiated,
                dna_origin,
            } => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_permanent: Some(permanent),
                event_card: Some(card),
                source_player: Some(player),
                effect_initiated,
                dna_origin,
                ..TriggerContext::default()
            },
            TriggerSource::EnteredField {
                player,
                permanent,
                card,
                effect_initiated,
            } => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_permanent: Some(permanent),
                event_card: Some(card),
                source_player: Some(player),
                effect_initiated,
                ..TriggerContext::default()
            },
            TriggerSource::OptionPlaced {
                player,
                permanent,
                linked_host,
                card,
            } => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_permanent: permanent,
                event_card: Some(card),
                event_source_card: Some(card),
                event_host_card: linked_host.and_then(|h| self.top_card_handle(h)),
                event_host_permanent: linked_host,
                source_player: Some(player),
                ..TriggerContext::default()
            },
            TriggerSource::OptionTrashed {
                player,
                card,
                cause,
                last_state,
            } => TriggerContext {
                subject: Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::BattleArea,
                }),
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_card: Some(card),
                affected_player: Some(player),
                source_player: Some(player),
                cause: Some(crate::trigger_context::EventCause::from(cause)),
                option_last_field_state: Some(last_state),
                moved_card_sets: vec![crate::trigger_context::MovedCardSet {
                    cards: vec![card],
                    from: Some(crate::enums::Zone::BattleArea),
                    to: Some(crate::enums::Zone::Trash),
                }],
                ..TriggerContext::default()
            },
            TriggerSource::EventObserved {
                player,
                permanent,
                card,
            } => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_permanent: Some(permanent),
                event_card: Some(card),
                source_player: Some(player),
                ..TriggerContext::default()
            },
            TriggerSource::AttackTargetChanged {
                player,
                attacker,
                card,
                old_target,
                new_target,
                reason,
            } => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_permanent: Some(attacker),
                event_card: Some(card),
                attack_target_change: Some(crate::trigger_context::AttackTargetChange {
                    attacker,
                    old_target,
                    new_target,
                    reason,
                    controller: player,
                }),
                source_player: Some(player),
                ..TriggerContext::default()
            },
            TriggerSource::SourceTrashedFromStack {
                player,
                host,
                host_card,
                card,
                cause,
            } => TriggerContext {
                subject: Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::BattleArea,
                }),
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_card: Some(card),
                event_source_card: Some(card),
                event_host_card: Some(host_card),
                event_host_permanent: Some(host),
                affected_player: Some(player),
                source_player: Some(player),
                cause: Some(cause),
                moved_card_sets: vec![crate::trigger_context::MovedCardSet {
                    cards: vec![card],
                    from: Some(crate::enums::Zone::BattleArea),
                    to: Some(crate::enums::Zone::Trash),
                }],
                ..TriggerContext::default()
            },
            TriggerSource::SecurityRemoved {
                affected_player,
                source_player,
                card,
                cause,
                ..
            } => TriggerContext {
                subject: Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Security,
                }),
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_card: Some(card),
                affected_player: Some(affected_player),
                source_player: Some(source_player),
                cause: Some(cause),
                moved_card_sets: vec![crate::trigger_context::MovedCardSet {
                    cards: vec![card],
                    from: Some(crate::enums::Zone::Security),
                    to: None,
                }],
                ..TriggerContext::default()
            },
            TriggerSource::SecurityPlaced {
                affected_player,
                source_player,
                card,
                cause,
            } => TriggerContext {
                subject: Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Security,
                }),
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_card: Some(card),
                affected_player: Some(affected_player),
                source_player: Some(source_player),
                cause: Some(cause),
                moved_card_sets: vec![crate::trigger_context::MovedCardSet {
                    cards: vec![card],
                    from: None,
                    to: Some(crate::enums::Zone::Security),
                }],
                ..TriggerContext::default()
            },
            TriggerSource::SecurityDiscarded {
                affected_player,
                source_player,
                card,
                cause,
            } => TriggerContext {
                subject: Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Security,
                }),
                event_card: Some(card),
                affected_player: Some(affected_player),
                source_player: Some(source_player),
                cause: Some(cause),
                moved_card_sets: vec![crate::trigger_context::MovedCardSet {
                    cards: vec![card],
                    from: Some(crate::enums::Zone::Security),
                    to: Some(crate::enums::Zone::Trash),
                }],
                ..TriggerContext::default()
            },
        };
        if context.source_effect.is_none() {
            context.source_effect = self.current_effect_attribution();
        }
        context
    }

    fn current_effect_attribution(&self) -> Option<crate::trigger_context::EffectAttribution> {
        Some(crate::trigger_context::EffectAttribution {
            controller: self.effect_source_player?,
            source_card: self.effect_source_card,
            source_permanent: self.effect_source_permanent,
        })
    }

    fn top_card_handle(&self, handle: PermanentHandle) -> Option<CardHandle> {
        if handle.index == BREEDING_TARGET as u8 {
            return self
                .players
                .get(handle.player as usize)
                .and_then(|p| p.breeding_area.as_ref())
                .map(|perm| perm.top_card().handle());
        }
        self.players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
            .map(|perm| perm.top_card().handle())
    }

    fn enqueue_event_gated_delayed_options(
        &mut self,
        timing: EffectTiming,
        trigger_context: TriggerContext,
    ) {
        let mut candidates = Vec::new();
        for player in 0..self.players.len() {
            let player = player as PlayerId;
            for (index, perm) in self.player(player).battle_area.iter().enumerate() {
                let OptionState::Delayed {
                    trigger: DelayTrigger::OnEvent(event_timing),
                    placed_on_turn,
                    ..
                } = perm.option_state
                else {
                    continue;
                };
                if event_timing != timing || placed_on_turn >= self.turn_count {
                    continue;
                }
                candidates.push(PermanentHandle {
                    player,
                    index: index as u8,
                });
            }
        }

        for handle in candidates {
            self.enqueue_delayed_option_for_event(handle, timing, trigger_context.clone());
        }
    }

    fn enqueue_delayed_option_for_event(
        &mut self,
        handle: PermanentHandle,
        timing: EffectTiming,
        trigger_context: TriggerContext,
    ) {
        let Some(perm) = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
        else {
            return;
        };
        let OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(event_timing),
            placed_on_turn,
            ..
        } = perm.option_state
        else {
            return;
        };
        if event_timing != timing || placed_on_turn >= self.turn_count {
            return;
        }

        let top = perm.top_card();
        let source_card = top.handle();
        let card_id = top.card_id(&self.card_data).to_string();
        let source_kind = source_kind_for_card_kind(top.card_kind(&self.card_data));
        let Some(effects) = self.effects_for_card(&card_id, source_card) else {
            return;
        };
        let tp = self.turn_player();
        let is_turn_player = handle.player == tp;
        for (slot, effect) in effects.iter().enumerate() {
            if effect.timing != EffectTiming::DelayEffect
                || effect.delay_trigger != Some(DelayTrigger::OnEvent(timing))
            {
                continue;
            }
            if !self.event_gated_delay_condition_matches(
                effect,
                source_card,
                Some(handle),
                source_kind,
                handle.player,
                trigger_context.clone(),
            ) {
                continue;
            }
            self.effect_queue.push_back(QueuedEffect {
                source_card,
                source_permanent: Some(handle),
                source_kind,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                controller: handle.player,
                timing: EffectTiming::DelayEffect,
                trigger_context: Some(trigger_context.clone()),
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.clone(),
                allow_below_top_liveness: false,
                dna_origin_context: self.current_dna_origin,
                granted_effect_id: None,
            });
        }
    }

    fn event_gated_delay_condition_matches(
        &mut self,
        effect: &crate::effect::Effect,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        source_kind: EffectSourceKind,
        controller: PlayerId,
        trigger_context: TriggerContext,
    ) -> bool {
        let Some(cond) = &effect.condition else {
            return true;
        };

        let prev_trigger_context = self.current_trigger_context.clone();
        self.current_trigger_context = Some(trigger_context);
        let ctx = EffectReadContext::new_with_source_kind(
            self,
            source_card,
            source_permanent,
            source_kind,
            controller,
        );
        let passes = cond(&ctx);
        self.current_trigger_context = prev_trigger_context;
        passes
    }

    /// Collect `SecuritySkill` effects off a revealed security card. The
    /// card is expected to be parked in `Game.pending_security` (popped off
    /// the defender's security stack but not yet disposed). Only effects
    /// whose `security` flag is set are enqueued — matches Python's
    /// `is_security_effect` filter.
    fn enqueue_from_security_card(
        &mut self,
        timing: EffectTiming,
        defender: PlayerId,
        card: CardHandle,
        trigger_context: Option<TriggerContext>,
    ) {
        let Some(pending) = self.pending_security.as_ref() else {
            return;
        };
        if pending.card.handle() != card {
            return;
        }
        let card_id = pending.card.card_id(&self.card_data).to_string();
        let source_card = card;
        let source_kind = source_kind_for_card_kind(pending.card.card_kind(&self.card_data));

        let Some(effects) = self.effects_for_card(&card_id, source_card) else {
            return;
        };

        let tp = self.turn_player();
        let is_turn_player = defender == tp;

        for (slot, effect) in effects.iter().enumerate() {
            if security_activation_blocked_for_timing(self, defender, timing) {
                continue;
            }
            if !timing_flag_matches(effect, timing) {
                continue;
            }
            // Security trigger specifically: ignore effects that don't carry
            // the security flag. Matches Python's
            // `if getattr(effect, 'is_security_effect', False)` filter.
            if timing == EffectTiming::SecuritySkill && !effect.security {
                continue;
            }
            self.effect_queue.push_back(QueuedEffect {
                source_card,
                source_permanent: None,
                source_kind,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                controller: defender,
                timing,
                trigger_context: trigger_context.clone(),
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.clone(),
                allow_below_top_liveness: false,
                dna_origin_context: self.current_dna_origin,
                granted_effect_id: None,
            });
        }
    }

    /// Collect turn-boundary `[Security]` effects from a card that still
    /// lives in the persistent security stack. This is distinct from
    /// `SecuritySkill`, where combat has already popped the card into
    /// `pending_security`.
    fn enqueue_from_security_stack_card(
        &mut self,
        timing: EffectTiming,
        player: PlayerId,
        card: CardHandle,
        trigger_context: Option<TriggerContext>,
    ) {
        let Some(card_source) = self
            .player(player)
            .security
            .iter()
            .find(|source| source.handle() == card)
        else {
            return;
        };
        let card_id = card_source.card_id(&self.card_data).to_string();
        let source_kind = source_kind_for_card_kind(card_source.card_kind(&self.card_data));

        let Some(effects) = self.effects_for_card(&card_id, card) else {
            return;
        };

        let is_turn_player = player == self.turn_player();
        for (slot, effect) in effects.iter().enumerate() {
            if security_activation_blocked_for_timing(self, player, timing) {
                continue;
            }
            if !timing_flag_matches(effect, timing) {
                continue;
            }
            if !effect.security {
                continue;
            }
            self.effect_queue.push_back(QueuedEffect {
                source_card: card,
                source_permanent: None,
                source_kind,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                controller: player,
                timing,
                trigger_context: trigger_context.clone(),
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.clone(),
                allow_below_top_liveness: false,
                dna_origin_context: self.current_dna_origin,
                granted_effect_id: None,
            });
        }
    }

    /// Collect effects for a single permanent. Applies the same timing +
    /// flag filter as the legacy `fire_*` loops so enqueue is a drop-in
    /// replacement.
    ///
    /// Phase 8 Task 4: after the top card's own effects are collected, the
    /// host's `linked_cards` are scanned for `.linked()` effects at the same
    /// timing. A sideways-inherited effect fires attributed to the host's
    /// controller and source-permanent, but with `source_card` pointing at
    /// the linked card so the re-lookup in `run_queued_effect_inner` finds
    /// the right script.
    fn enqueue_from_permanent(
        &mut self,
        timing: EffectTiming,
        handle: PermanentHandle,
        trigger_context: Option<TriggerContext>,
    ) {
        // Track H §3 Phase 4i — push granted-triggered-effect entries
        // as QueuedEffect with `granted_effect_id` set. The drainer
        // recognizes these and fetches the body from
        // `Game::granted_effect_bodies` at fire time. Selection-driving
        // bodies park on `pending_selection` like printed effects;
        // inline-fire (Phase 4b) is now redundant for queue-routed
        // entries but kept as a fallback via `pending_granted_fires`.
        let granted_entries = self
            .modifiers
            .granted_triggered_for_timing_with_ids(handle, timing);
        let tp_for_granted = self.turn_player();
        for (body_id, source_card, source_player) in granted_entries {
            self.effect_queue.push_back(QueuedEffect {
                source_card,
                source_permanent: Some(handle),
                source_kind: source_kind_for_card_kind(
                    self.player(handle.player)
                        .battle_area
                        .get(handle.index as usize)
                        .map(|p| p.top_card().card_kind(&self.card_data))
                        .unwrap_or(crate::enums::CardKind::Digimon),
                ),
                controller: source_player,
                timing,
                trigger_context: trigger_context.clone(),
                effect_slot: 0,
                is_optional: false,
                is_turn_player: source_player == tp_for_granted,
                card_id: String::new(),
                allow_below_top_liveness: false,
                dna_origin_context: self.current_dna_origin,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                granted_effect_id: Some(body_id),
            });
        }
        let Some(perm) = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
        else {
            return;
        };
        let top = perm.top_card();
        let card_id = top.card_id(&self.card_data).to_string();
        let source_card = top.handle();
        let top_source_kind = source_kind_for_card_kind(top.card_kind(&self.card_data));

        let tp = self.turn_player();
        let is_turn_player = handle.player == tp;
        if permanent_activation_blocked_for_timing(self, handle, timing) {
            return;
        }

        // Defensive guard — not reachable through current dispatch paths but
        // prevents future double-enqueue if a caller fires a timing directly on
        // a Training permanent with an effect authored .inherited(). Today no
        // such card pattern exists; see Phase 8 Task 5 design for rationale.
        let skip_inherited = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
            .map(|p| {
                matches!(
                    p.option_state,
                    crate::permanent::OptionState::Training { .. }
                )
            })
            .unwrap_or(false);

        if let Some(effects) = self.effects_for_card(&card_id, source_card) {
            for (slot, effect) in effects.iter().enumerate() {
                if !timing_flag_matches(effect, timing) {
                    continue;
                }
                if effect.linked {
                    continue;
                }
                if skip_inherited && effect.inherited {
                    continue;
                }
                self.effect_queue.push_back(QueuedEffect {
                    source_card,
                    source_permanent: Some(handle),
                    source_kind: top_source_kind,
                    attribution_source_card: None,
                    attribution_source_kind: None,
                    bypass_once_per_turn: false,
                    controller: handle.player,
                    timing,
                    trigger_context: trigger_context.clone(),
                    effect_slot: slot as u8,
                    is_optional: effect.optional,
                    is_turn_player,
                    card_id: card_id.clone(),
                    allow_below_top_liveness: false,
                    dna_origin_context: self.current_dna_origin,
                    granted_effect_id: None,
                });
            }
        }

        // Phase 8 Task 4: sideways inheritance from linked cards. Snapshot
        // linked-card identity first to drop the `perm` borrow before
        // iterating (effects_for_card borrows &self).
        let linked_sources: Vec<(String, crate::card_source::CardHandle, EffectSourceKind)> = {
            let perm = self
                .players
                .get(handle.player as usize)
                .and_then(|p| p.battle_area.get(handle.index as usize));
            match perm {
                Some(p) => p
                    .linked_cards
                    .iter()
                    .map(|c| {
                        (
                            c.card_id(&self.card_data).to_string(),
                            c.handle(),
                            source_kind_for_card_kind(c.card_kind(&self.card_data)),
                        )
                    })
                    .collect(),
                None => Vec::new(),
            }
        };
        for (linked_card_id, linked_source, linked_source_kind) in linked_sources {
            let Some(effects) = self.effects_for_card(&linked_card_id, linked_source) else {
                continue;
            };
            for (slot, effect) in effects.iter().enumerate() {
                if !effect.linked {
                    continue;
                }
                if !timing_flag_matches(effect, timing) {
                    continue;
                }
                self.effect_queue.push_back(QueuedEffect {
                    source_card: linked_source,
                    source_permanent: Some(handle),
                    source_kind: linked_source_kind,
                    attribution_source_card: None,
                    attribution_source_kind: None,
                    bypass_once_per_turn: false,
                    controller: handle.player,
                    timing,
                    trigger_context: trigger_context.clone(),
                    effect_slot: slot as u8,
                    is_optional: effect.optional,
                    is_turn_player,
                    card_id: linked_card_id.clone(),
                    allow_below_top_liveness: false,
                    dna_origin_context: self.current_dna_origin,
                    granted_effect_id: None,
                });
            }
        }

        // Phase 8 Task 5/6 — Training sideways inheritance.
        //
        // When any permanent the owner controls fires a timing, also scan
        // the owner's battle_area for `OptionState::Training` permanents
        // and include their `inherited` effects at the same timing if the
        // Training is unbound or bound to this source permanent. The
        // training-card's permanent is never the source_permanent here —
        // source_permanent stays as the scanning perm (e.g. the hatched
        // digimon) so effect scripts can read the target via the normal
        // `ctx.source_permanent` path. Skip when the scanning perm is
        // itself a Training permanent to avoid self-attribution loops.
        //
        // Skipping the scan when `self_is_training` keeps Training's own
        // OnTrainingTrash firing single-shot from `move_from_breeding`'s
        // direct `TriggerSource::Permanent(training_handle)` dispatch.
        let self_is_training = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
            .map(|p| {
                matches!(
                    p.option_state,
                    crate::permanent::OptionState::Training { .. }
                )
            })
            .unwrap_or(false);
        if !self_is_training {
            let training_sources: Vec<(String, crate::card_source::CardHandle, EffectSourceKind)> = {
                let p = match self.players.get(handle.player as usize) {
                    Some(p) => p,
                    None => return,
                };
                let source_top_card = p
                    .battle_area
                    .get(handle.index as usize)
                    .map(|perm| perm.top_card().handle());
                p.battle_area
                    .iter()
                    .filter_map(|perm| {
                        if let crate::permanent::OptionState::Training { trained, .. } =
                            perm.option_state
                        {
                            let training_applies = trained
                                .map_or(true, |binding| source_top_card == Some(binding.top_card));
                            if training_applies {
                                let top = perm.top_card();
                                return Some((
                                    top.card_id(&self.card_data).to_string(),
                                    top.handle(),
                                    source_kind_for_card_kind(top.card_kind(&self.card_data)),
                                ));
                            }
                        }
                        None
                    })
                    .collect()
            };
            for (training_card_id, training_source, training_source_kind) in training_sources {
                let Some(effects) = self.effects_for_card(&training_card_id, training_source)
                else {
                    continue;
                };
                for (slot, effect) in effects.iter().enumerate() {
                    if !effect.inherited {
                        continue;
                    }
                    if !timing_flag_matches(effect, timing) {
                        continue;
                    }
                    self.effect_queue.push_back(QueuedEffect {
                        source_card: training_source,
                        source_permanent: Some(handle),
                        source_kind: training_source_kind,
                        attribution_source_card: None,
                        attribution_source_kind: None,
                        bypass_once_per_turn: false,
                        controller: handle.player,
                        timing,
                        trigger_context: trigger_context.clone(),
                        effect_slot: slot as u8,
                        is_optional: effect.optional,
                        is_turn_player,
                        card_id: training_card_id.clone(),
                        allow_below_top_liveness: false,
                        dna_origin_context: self.current_dna_origin,
                        granted_effect_id: None,
                    });
                }
            }
        }

        let inherited_sources: Vec<(String, CardHandle, EffectSourceKind)> = {
            let Some(perm) = self
                .players
                .get(handle.player as usize)
                .and_then(|p| p.battle_area.get(handle.index as usize))
            else {
                return;
            };
            let stack_len = perm.card_sources.len();
            perm.card_sources
                .iter()
                .take(stack_len.saturating_sub(1))
                .map(|c| {
                    (
                        c.card_id(&self.card_data).to_string(),
                        c.handle(),
                        source_kind_for_card_kind(c.card_kind(&self.card_data)),
                    )
                })
                .collect()
        };
        for (source_card_id, inherited_source, inherited_source_kind) in inherited_sources {
            let Some(effects) = self.effects_for_card(&source_card_id, inherited_source) else {
                continue;
            };
            for (slot, effect) in effects.iter().enumerate() {
                if !effect.inherited {
                    continue;
                }
                if !timing_flag_matches(effect, timing) {
                    continue;
                }
                self.effect_queue.push_back(QueuedEffect {
                    source_card: inherited_source,
                    source_permanent: Some(handle),
                    source_kind: inherited_source_kind,
                    attribution_source_card: None,
                    attribution_source_kind: None,
                    bypass_once_per_turn: false,
                    controller: handle.player,
                    timing,
                    trigger_context: trigger_context.clone(),
                    effect_slot: slot as u8,
                    is_optional: effect.optional,
                    is_turn_player,
                    card_id: source_card_id.clone(),
                    allow_below_top_liveness: true,
                    dna_origin_context: self.current_dna_origin,
                    granted_effect_id: None,
                });
            }
        }
    }

    fn enqueue_from_breeding_permanent(
        &mut self,
        timing: EffectTiming,
        handle: PermanentHandle,
        trigger_context: Option<TriggerContext>,
    ) {
        // Track H §3 Phase 4i — same queue-based granted dispatch as
        // `enqueue_from_permanent`; breeding-area carriers are valid
        // grant targets via the breeding permanent's handle.
        let granted_entries = self
            .modifiers
            .granted_triggered_for_timing_with_ids(handle, timing);
        let tp_for_granted = self.turn_player();
        for (body_id, source_card, source_player) in granted_entries {
            let source_kind = self
                .player(handle.player)
                .breeding_area
                .as_ref()
                .map(|p| source_kind_for_card_kind(p.top_card().card_kind(&self.card_data)))
                .unwrap_or(crate::enums::EffectSourceKind::Digimon);
            self.effect_queue.push_back(QueuedEffect {
                source_card,
                source_permanent: Some(handle),
                source_kind,
                controller: source_player,
                timing,
                trigger_context: trigger_context.clone(),
                effect_slot: 0,
                is_optional: false,
                is_turn_player: source_player == tp_for_granted,
                card_id: String::new(),
                allow_below_top_liveness: false,
                dna_origin_context: self.current_dna_origin,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                granted_effect_id: Some(body_id),
            });
        }
        let Some(perm) = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.breeding_area.as_ref())
        else {
            return;
        };
        let top = perm.top_card();
        let card_id = top.card_id(&self.card_data).to_string();
        let source_card = top.handle();
        let source_kind = source_kind_for_card_kind(top.card_kind(&self.card_data));
        let is_turn_player = handle.player == self.turn_player();

        if let Some(effects) = self.effects_for_card(&card_id, source_card) {
            for (slot, effect) in effects.iter().enumerate() {
                if !timing_flag_matches(effect, timing) {
                    continue;
                }
                if effect.inherited {
                    continue;
                }
                self.effect_queue.push_back(QueuedEffect {
                    source_card,
                    source_permanent: Some(handle),
                    source_kind,
                    attribution_source_card: None,
                    attribution_source_kind: None,
                    bypass_once_per_turn: false,
                    controller: handle.player,
                    timing,
                    trigger_context: trigger_context.clone(),
                    effect_slot: slot as u8,
                    is_optional: effect.optional,
                    is_turn_player,
                    card_id: card_id.clone(),
                    allow_below_top_liveness: false,
                    dna_origin_context: self.current_dna_origin,
                    granted_effect_id: None,
                });
            }
        }

        let inherited_sources: Vec<(String, CardHandle, EffectSourceKind)> = {
            let stack_len = perm.card_sources.len();
            perm.card_sources
                .iter()
                .take(stack_len.saturating_sub(1))
                .map(|c| {
                    (
                        c.card_id(&self.card_data).to_string(),
                        c.handle(),
                        source_kind_for_card_kind(c.card_kind(&self.card_data)),
                    )
                })
                .collect()
        };
        for (source_card_id, inherited_source, inherited_source_kind) in inherited_sources {
            let Some(effects) = self.effects_for_card(&source_card_id, inherited_source) else {
                continue;
            };
            for (slot, effect) in effects.iter().enumerate() {
                if !effect.inherited {
                    continue;
                }
                if !timing_flag_matches(effect, timing) {
                    continue;
                }
                self.effect_queue.push_back(QueuedEffect {
                    source_card: inherited_source,
                    source_permanent: Some(handle),
                    source_kind: inherited_source_kind,
                    attribution_source_card: None,
                    attribution_source_kind: None,
                    bypass_once_per_turn: false,
                    controller: handle.player,
                    timing,
                    trigger_context: trigger_context.clone(),
                    effect_slot: slot as u8,
                    is_optional: effect.optional,
                    is_turn_player,
                    card_id: source_card_id.clone(),
                    allow_below_top_liveness: true,
                    dna_origin_context: self.current_dna_origin,
                    granted_effect_id: None,
                });
            }
        }
    }

    /// Who gets to choose the next effect to resolve. Turn player first,
    /// then clockwise through the remaining players.
    fn next_chooser(&self) -> Option<PlayerId> {
        if self.effect_queue.is_empty() {
            return None;
        }
        let n = self.turn_order.len();
        for offset in 0..n {
            let idx = (self.turn_player_idx + offset) % n;
            let pid = self.turn_order[idx];
            if self.effect_queue.iter().any(|qe| qe.controller == pid) {
                return Some(pid);
            }
        }
        // Defensive fallback — if somehow no turn-order player owns any
        // queued effect (e.g. eliminated controller), use the front entry.
        self.effect_queue.front().map(|qe| qe.controller)
    }

    /// Execute a single queued effect: re-look-up, condition check,
    /// process. Exits silently on any validity gap (permanent deleted,
    /// effect removed from the registry, etc.) — same tolerance the legacy
    /// `fire_*` loops had.
    fn run_queued_effect(&mut self, qe: QueuedEffect) {
        // Set the effect-source attribution for replacement-cause inference.
        // Saved on entry, restored on exit — supports nested drains (an
        // effect queues another effect that recursively drains before this
        // one returns).
        let prev_effect_source = self.effect_source_player;
        let prev_effect_source_card = self.effect_source_card;
        let prev_effect_source_permanent = self.effect_source_permanent;
        let prev_trigger_context = self.current_trigger_context.clone();
        let prev_dna_origin = self.current_dna_origin;
        let attribution_source_card = qe.attribution_source_card.unwrap_or(qe.source_card);
        self.effect_source_player = Some(qe.controller);
        self.effect_source_card = Some(attribution_source_card);
        self.effect_source_permanent = qe.source_permanent;
        self.current_trigger_context = qe.trigger_context.clone();
        if qe.dna_origin_context.is_some() {
            self.current_dna_origin = qe.dna_origin_context;
        }
        let out = self.run_queued_effect_inner(qe);
        self.current_dna_origin = prev_dna_origin;
        self.current_trigger_context = prev_trigger_context;
        self.effect_source_permanent = prev_effect_source_permanent;
        self.effect_source_card = prev_effect_source_card;
        self.effect_source_player = prev_effect_source;
        out
    }

    fn run_queued_effect_inner(&mut self, qe: QueuedEffect) {
        // Track H §3 Phase 4i — granted-triggered-effect branch.
        // Granted bodies are inline closures with no Effect metadata
        // (no condition, pay_cost, optional, max_per_turn). We fetch
        // the body from `granted_effect_bodies` and invoke directly.
        // Selection-driving bodies that install a `PendingSelection`
        // park correctly — the standard selection resume path picks
        // them back up because the queue stays alive.
        if let Some(body_id) = qe.granted_effect_id {
            let body = self.granted_effect_bodies.get(body_id).cloned();
            let Some(body) = body else {
                return;
            };
            let mut ctx = EffectContext::new_with_source_kind(
                self,
                qe.source_card,
                qe.source_permanent,
                qe.source_kind,
                qe.controller,
            );
            body(&mut ctx);
            return;
        }
        // Track K cross-card refire support (from main): attribution
        // source falls back to the queued effect's primary source when
        // not overridden.
        let attribution_source_card = qe.attribution_source_card.unwrap_or(qe.source_card);
        let attribution_source_kind = qe.attribution_source_kind.unwrap_or(qe.source_kind);
        let Some(effects) = self.effects_for_card(&qe.card_id, qe.source_card) else {
            return;
        };
        let Some(effect) = effects.get(qe.effect_slot as usize) else {
            return;
        };

        if !self.queued_effect_source_is_live(&qe, effect) {
            return;
        }

        if effect.max_per_turn > 0 && !qe.bypass_once_per_turn {
            if let Some(perm_handle) = qe.source_permanent {
                let Some(activation_count) = self.source_permanent_activation_count(
                    perm_handle,
                    qe.source_card,
                    qe.effect_slot,
                ) else {
                    return;
                };
                if activation_count >= effect.max_per_turn {
                    return;
                }
            }
        }

        // Python parity (§2.5h): `_fire_security_skill` iterates
        // `effect_list(SecuritySkill)` and invokes the callback directly —
        // it never evaluates `effect.can_use_condition`. Matching that
        // behavior here so a conditional `[Security]` effect
        // (`[Security] If opp has a Digimon, delete it.`) fires with the
        // same semantics on both engines. The script is responsible for
        // any conditionality via an `if` inside its `process` closure.
        let skip_condition = qe.timing == EffectTiming::SecuritySkill;
        if !skip_condition {
            if let Some(cond) = &effect.condition {
                let ctx = EffectContext::new_with_source_kind(
                    self,
                    attribution_source_card,
                    qe.source_permanent,
                    attribution_source_kind,
                    qe.controller,
                );
                if !cond(&ctx.as_read()) {
                    return;
                }
            }
        }
        // Note: pay_cost_fn is NOT gated by skip_condition. For SecuritySkill
        // timing, pay_cost_fn still fires (intentional — pay-costs are
        // orthogonal to the condition-skipping behavior for security effects).
        //
        // Pay-cost hook — fires after condition passes, before process.
        // Triggered effects may install a PendingSelection while paying cost;
        // in that case park the queued effect and resume the process tail
        // after the selection chain resolves.
        if let Some(pay_cost) = &effect.pay_cost_fn {
            let mut ctx = EffectContext::new_with_source_kind(
                self,
                qe.attribution_source_card.unwrap_or(qe.source_card),
                qe.source_permanent,
                qe.attribution_source_kind.unwrap_or(qe.source_kind),
                qe.controller,
            );
            if !pay_cost(&mut ctx) {
                return; // cost not paid; skip process (silent abort, mirrors failed condition)
            }
            if self.pending_selection.is_some() {
                if let Some(outer) = self.pending_pay_cost_effect.take() {
                    self.pending_pay_cost_stack.push(outer);
                }
                self.pending_pay_cost_effect = Some(PendingPayCostEffect {
                    queued_effect: qe.clone(),
                    declined: false,
                });
                return;
            }
        }

        let event_delay_source = self.event_gated_delay_source(&qe);
        self.run_queued_effect_process_tail(&qe);
        self.trash_event_gated_delay_after_activation(event_delay_source);
    }

    fn run_queued_effect_process_tail(&mut self, qe: &QueuedEffect) {
        let Some(effects) = self.effects_for_card(&qe.card_id, qe.source_card) else {
            return;
        };
        let Some(effect) = effects.get(qe.effect_slot as usize) else {
            return;
        };

        if !self.queued_effect_source_is_live(qe, effect) {
            return;
        }

        if effect.max_per_turn > 0 && !qe.bypass_once_per_turn {
            if let Some(perm_handle) = qe.source_permanent {
                let Some(activation_count) = self.source_permanent_activation_count(
                    perm_handle,
                    qe.source_card,
                    qe.effect_slot,
                ) else {
                    return;
                };
                if activation_count >= effect.max_per_turn {
                    return;
                }
                self.record_source_permanent_activation(
                    perm_handle,
                    qe.source_card,
                    qe.effect_slot,
                );
            }
        }

        if let Some(process) = &effect.process {
            let mut ctx = EffectContext::new_with_source_kind(
                self,
                qe.attribution_source_card.unwrap_or(qe.source_card),
                qe.source_permanent,
                qe.attribution_source_kind.unwrap_or(qe.source_kind),
                qe.controller,
            );
            process(&mut ctx);
        }
    }

    fn event_gated_delay_source(&self, qe: &QueuedEffect) -> Option<(PlayerId, u16, EffectTiming)> {
        if qe.timing != EffectTiming::DelayEffect
            || !qe
                .trigger_context
                .as_ref()
                .is_some_and(|ctx| ctx.event_card.is_some())
        {
            return None;
        }
        let handle = qe.source_permanent?;
        let perm = self
            .players
            .get(handle.player as usize)?
            .battle_area
            .get(handle.index as usize)?;
        let OptionState::Delayed {
            owner,
            trigger: DelayTrigger::OnEvent(timing),
            ..
        } = perm.option_state
        else {
            return None;
        };
        if perm.top_card().handle() != qe.source_card {
            return None;
        }
        Some((owner, qe.source_card.0, timing))
    }

    fn trash_event_gated_delay_after_activation(
        &mut self,
        source: Option<(PlayerId, u16, EffectTiming)>,
    ) {
        let Some((owner, card_index, timing)) = source else {
            return;
        };
        if self.pending_selection.is_some() {
            self.park_delayed_option_lifecycle(DelayedOptionLifecycleResume {
                turn: u16::MAX,
                kind: DelayedOptionLifecycleResumeKind::Event { timing },
                pending_delete_key: Some((owner, card_index)),
                skip_key: None,
            });
            return;
        }
        let Some(handle) = self.find_event_gated_delay_permanent(owner, card_index, timing) else {
            return;
        };
        self.delete_permanent_with_cause(handle, ReplacementCause::Cost);
    }

    fn find_event_gated_delay_permanent(
        &self,
        owner: PlayerId,
        card_index: u16,
        timing: EffectTiming,
    ) -> Option<PermanentHandle> {
        for (index, perm) in self.player(owner).battle_area.iter().enumerate() {
            if perm.top_card().card_index != card_index {
                continue;
            }
            if matches!(
                perm.option_state,
                OptionState::Delayed {
                    owner: delayed_owner,
                    trigger: DelayTrigger::OnEvent(event_timing),
                    ..
                } if delayed_owner == owner && event_timing == timing
            ) {
                return Some(PermanentHandle {
                    player: owner,
                    index: index as u8,
                });
            }
        }
        None
    }

    fn queued_effect_source_is_live(
        &self,
        qe: &QueuedEffect,
        effect: &crate::effect::Effect,
    ) -> bool {
        // Source permanent may have been deleted by a prior effect in this
        // batch or by a parked pay-cost selection callback. Skip silently —
        // matches Python behavior.
        let Some(perm_handle) = qe.source_permanent else {
            return true;
        };
        if perm_handle.index == BREEDING_TARGET as u8 {
            return self
                .players
                .get(perm_handle.player as usize)
                .and_then(|p| p.breeding_area.as_ref())
                .is_some_and(|perm| {
                    let top_matches = !qe.allow_below_top_liveness
                        && perm.top_card().card_index == qe.source_card.0;
                    let below_top_source_matches = perm
                        .card_sources
                        .iter()
                        .take(perm.card_sources.len().saturating_sub(1))
                        .any(|c| c.card_index == qe.source_card.0);
                    let inherited_source_matches =
                        below_top_source_matches && qe.allow_below_top_liveness && effect.inherited;
                    top_matches || inherited_source_matches
                });
        }
        let Some(perm) = self
            .players
            .get(perm_handle.player as usize)
            .and_then(|p| p.battle_area.get(perm_handle.index as usize))
        else {
            return false;
        };
        // Also skip if the specific source card has been shuffled out of the
        // top-card slot (e.g. permanent digivolved mid-batch).
        // Phase 8 Task 4: a sideways-inherited effect's source_card is a card
        // in `linked_cards`, not the top card — accept either.
        // Phase 8 Task 5: a Training-sideways effect's source_card lives on a
        // different `OptionState::Training` permanent the same owner controls;
        // scan the owner's battle_area for it.
        let top_matches =
            !qe.allow_below_top_liveness && perm.top_card().card_index == qe.source_card.0;
        let linked_matches = perm
            .linked_cards
            .iter()
            .any(|c| c.card_index == qe.source_card.0);
        let below_top_source_matches = perm
            .card_sources
            .iter()
            .take(perm.card_sources.len().saturating_sub(1))
            .any(|c| c.card_index == qe.source_card.0);
        let inherited_source_matches =
            below_top_source_matches && qe.allow_below_top_liveness && effect.inherited;
        let training_matches = self
            .players
            .get(perm_handle.player as usize)
            .map(|p| {
                p.battle_area.iter().any(|pp| {
                    if pp.top_card().card_index != qe.source_card.0 {
                        return false;
                    }
                    let crate::permanent::OptionState::Training { trained, .. } = pp.option_state
                    else {
                        return false;
                    };
                    trained.map_or(true, |binding| {
                        binding.handle == perm_handle
                            && perm.top_card().handle() == binding.top_card
                    })
                })
            })
            .unwrap_or(false);
        top_matches || linked_matches || inherited_source_matches || training_matches
    }

    fn source_permanent_activation_count(
        &self,
        handle: PermanentHandle,
        source_card: CardHandle,
        effect_slot: u8,
    ) -> Option<u8> {
        if handle.index == BREEDING_TARGET as u8 {
            return self
                .players
                .get(handle.player as usize)
                .and_then(|p| p.breeding_area.as_ref())
                .map(|perm| perm.activation_count(source_card, effect_slot));
        }
        self.players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
            .map(|perm| perm.activation_count(source_card, effect_slot))
    }

    fn record_source_permanent_activation(
        &mut self,
        handle: PermanentHandle,
        source_card: CardHandle,
        effect_slot: u8,
    ) {
        if handle.index == BREEDING_TARGET as u8 {
            if let Some(perm) = self
                .players
                .get_mut(handle.player as usize)
                .and_then(|p| p.breeding_area.as_mut())
            {
                perm.record_activation(source_card, effect_slot);
            }
            return;
        }
        if let Some(perm) = self
            .players
            .get_mut(handle.player as usize)
            .and_then(|p| p.battle_area.get_mut(handle.index as usize))
        {
            perm.record_activation(source_card, effect_slot);
        }
    }

    fn resume_queued_effect_process_tail(&mut self, qe: QueuedEffect) {
        let prev_effect_source = self.effect_source_player;
        let prev_effect_source_card = self.effect_source_card;
        let prev_effect_source_permanent = self.effect_source_permanent;
        let prev_trigger_context = self.current_trigger_context.clone();
        let attribution_source_card = qe.attribution_source_card.unwrap_or(qe.source_card);
        self.effect_source_player = Some(qe.controller);
        self.effect_source_card = Some(attribution_source_card);
        self.effect_source_permanent = qe.source_permanent;
        self.current_trigger_context = qe.trigger_context.clone();
        let event_delay_source = self.event_gated_delay_source(&qe);
        self.run_queued_effect_process_tail(&qe);
        self.trash_event_gated_delay_after_activation(event_delay_source);
        self.current_trigger_context = prev_trigger_context;
        self.effect_source_permanent = prev_effect_source_permanent;
        self.effect_source_card = prev_effect_source_card;
        self.effect_source_player = prev_effect_source;
    }

    pub(crate) fn resume_pending_pay_cost_effect(&mut self) {
        while self.pending_selection.is_none() {
            if self.pending_pay_cost_effect.is_none() {
                self.pending_pay_cost_effect = self.pending_pay_cost_stack.pop();
            }
            let Some(pending) = self.pending_pay_cost_effect.take() else {
                return;
            };
            if !pending.declined {
                self.resume_queued_effect_process_tail(pending.queued_effect);
            }
        }
    }

    pub(crate) fn complete_effect_security_removal(
        &mut self,
        mut pending: PendingEffectSecurityRemoval,
    ) {
        let mut completed_digivolve: Option<(PermanentHandle, crate::card_source::CardHandle)> =
            None;
        let mut completed_security_placement: Option<(PlayerId, crate::card_source::CardHandle)> =
            None;
        let removed_card = self
            .pending_security
            .as_ref()
            .map(|security| security.card.handle());
        if matches!(pending.destination, SecurityRemovalDestination::Trash)
            && !pending.discard_security_fired
        {
            if let Some(security) = self.pending_security.as_ref() {
                if !security.played {
                    let card = security.card.handle();
                    pending.discard_security_fired = true;
                    self.enqueue_triggered(
                        EffectTiming::OnDiscardSecurity,
                        TriggerSource::SecurityDiscarded {
                            affected_player: pending.defender,
                            source_player: pending.source_player,
                            card,
                            cause: pending.cause,
                        },
                    );
                    self.drain_effect_queue();
                    if self.pending_selection.is_some() {
                        self.pending_effect_security_removal.push(pending);
                        return;
                    }
                }
            }
        }
        if let Some(security) = self.pending_security.take() {
            if !security.played {
                match pending.destination {
                    SecurityRemovalDestination::Trash => {
                        let owner = security.card.owner;
                        self.player_mut(owner).trash.push(security.card);
                    }
                    SecurityRemovalDestination::Hand(owner) => {
                        self.player_mut(owner).hand.push(security.card);
                    }
                    SecurityRemovalDestination::BottomSource(target) => {
                        if target.index == crate::action::space::BREEDING_TARGET as u8 {
                            if let Some(breeding) =
                                self.player_mut(target.player).breeding_area.as_mut()
                            {
                                breeding.push_under(security.card);
                            } else {
                                let owner = security.card.owner;
                                self.player_mut(owner).trash.push(security.card);
                            }
                        } else if let Some(perm) = self
                            .player_mut(target.player)
                            .battle_area
                            .get_mut(target.index as usize)
                        {
                            perm.push_under(security.card);
                        } else {
                            let owner = security.card.owner;
                            self.player_mut(owner).trash.push(security.card);
                        }
                    }
                    SecurityRemovalDestination::Digivolve {
                        player: _,
                        target,
                        turn,
                    } => {
                        if let Some(perm) = self
                            .player_mut(target.player)
                            .battle_area
                            .get_mut(target.index as usize)
                        {
                            let event_card = security.card.handle();
                            perm.digivolve(security.card, turn);
                            completed_digivolve = Some((target, event_card));
                        } else {
                            let owner = security.card.owner;
                            self.player_mut(owner).trash.push(security.card);
                        }
                    }
                    SecurityRemovalDestination::Security {
                        player,
                        position,
                        face_up,
                    } => {
                        let placed_card = security.card.handle();
                        let face_up_key = security.card.card_index;
                        match position {
                            crate::enums::StackPosition::Top => {
                                self.player_mut(player).security.push(security.card);
                            }
                            crate::enums::StackPosition::Bottom => {
                                self.player_mut(player).security.insert(0, security.card);
                            }
                            crate::enums::StackPosition::Random => {
                                use rand::Rng;
                                let len = self.player(player).security.len();
                                let idx = if len == 0 {
                                    0
                                } else {
                                    self.rng.gen_range(0..=len)
                                };
                                self.player_mut(player).security.insert(idx, security.card);
                            }
                        }
                        if face_up {
                            self.player_mut(player).face_up_security.insert(face_up_key);
                        }
                        completed_security_placement = Some((player, placed_card));
                    }
                }
            }
        }

        self.pending_security = pending.previous_pending_security;

        if let Some((player, placed_card)) = completed_security_placement {
            self.fire_on_place_security(player, pending.source_player, placed_card);
        }

        if let Some((target, event_card)) = completed_digivolve {
            self.enqueue_triggered(
                EffectTiming::WhenDigivolving,
                TriggerSource::Permanent(target),
            );
            self.drain_effect_queue();

            self.enqueue_triggered(
                EffectTiming::OnDigivolve,
                TriggerSource::Digivolved {
                    player: target.player,
                    permanent: target,
                    card: event_card,
                    effect_initiated: true,
                    dna_origin: false,
                },
            );
            self.drain_effect_queue();
        }

        self.enqueue_triggered(
            EffectTiming::OnOwnSecurityRemoved,
            TriggerSource::SecurityRemoved {
                affected_player: pending.defender,
                observer_player: pending.defender,
                source_player: pending.source_player,
                card: removed_card.unwrap_or(crate::card_source::CardHandle(0)),
                cause: pending.cause,
            },
        );
        self.drain_effect_queue();

        if pending.defender != pending.observer_player {
            self.enqueue_triggered(
                EffectTiming::OnOpponentSecurityRemoved,
                TriggerSource::SecurityRemoved {
                    affected_player: pending.defender,
                    observer_player: pending.observer_player,
                    source_player: pending.source_player,
                    card: removed_card.unwrap_or(crate::card_source::CardHandle(0)),
                    cause: pending.cause,
                },
            );
            self.drain_effect_queue();
        }
    }

    pub(crate) fn resume_pending_effect_security_removal(&mut self) {
        while self.pending_selection.is_none() {
            let Some(pending) = self.pending_effect_security_removal.pop() else {
                return;
            };
            self.complete_effect_security_removal(pending);
        }
    }

    pub(crate) fn fire_effect_security_removal(
        &mut self,
        defender: PlayerId,
        observer_player: PlayerId,
        source_player: PlayerId,
        cause: crate::trigger_context::EventCause,
        card: crate::card_source::CardSource,
        destination: SecurityRemovalDestination,
    ) {
        let card_handle = card.handle();
        let previous_pending = self.pending_security.replace(PendingSecurity {
            defender,
            card,
            played: false,
        });

        self.enqueue_triggered(
            EffectTiming::OnLoseSecurity,
            TriggerSource::SecurityRevealed {
                defender,
                card: card_handle,
            },
        );
        self.drain_effect_queue();

        let pending = PendingEffectSecurityRemoval {
            defender,
            observer_player,
            source_player,
            cause,
            destination,
            previous_pending_security: previous_pending,
            discard_security_fired: false,
        };

        if self.pending_selection.is_some() {
            self.pending_effect_security_removal.push(pending);
            return;
        }
        self.complete_effect_security_removal(pending);
    }

    /// Install a `TriggerOrder` selection offering `bundle` indices as
    /// resolution picks. `allow_decline_all` enables PASS = decline every
    /// remaining optional trigger controlled by `chooser`.
    ///
    /// Bundle size is capped at `HAND_MAIN_LIMIT` (30) to fit the reused
    /// 30-59 action ID range. If the caller passes more, the overflow
    /// entries are auto-fired in collection order after the prompt
    /// resolves — documented-worst-case behavior, not expected in practice.
    fn install_trigger_order_selection(
        &mut self,
        chooser: PlayerId,
        bundle: &[usize],
        allow_decline_all: bool,
    ) {
        debug_assert!(
            !bundle.is_empty(),
            "install_trigger_order_selection requires at least one trigger"
        );

        // Map each bundle position to an action ID in the 30-59 range.
        // action_id = HAND_EFFECT_START + position.
        if self.current_dna_origin.is_some() {
            for &qe_idx in bundle {
                if let Some(qe) = self.effect_queue.get_mut(qe_idx) {
                    qe.dna_origin_context = self.current_dna_origin;
                }
            }
        }
        let capped = bundle.len().min(HAND_MAIN_LIMIT);
        let mut valid_action_ids: Vec<u16> = Vec::with_capacity(capped);
        let mut choices: Vec<EffectChoiceEntry> = Vec::with_capacity(capped);
        for pos in 0..capped {
            let qe_idx = bundle[pos];
            let qe = &self.effect_queue[qe_idx];
            let action_id = HAND_EFFECT_START + pos as u16;
            let observation_metadata = self
                .effects_for_card(&qe.card_id, qe.source_card)
                .and_then(|effects| {
                    effects
                        .get(qe.effect_slot as usize)
                        .map(|effect| effect.observation_metadata)
                })
                .unwrap_or_default();
            debug_assert!(action_id < HAND_EFFECT_END);
            valid_action_ids.push(action_id);
            choices.push(EffectChoiceEntry {
                label: format!(
                    "{} slot {} ({})",
                    qe.card_id,
                    qe.effect_slot,
                    if qe.is_optional {
                        "optional"
                    } else {
                        "mandatory"
                    },
                ),
                action_id,
                source_card: Some(qe.source_card),
                source_kind: Some(qe.source_kind),
                timing: Some(qe.timing),
                is_optional: qe.is_optional,
                observation_metadata,
            });
        }

        // Provenance: point at the first queued effect's source. This is a
        // debug aid — the selection itself doesn't need a true source.
        let head_qe = &self.effect_queue[bundle[0]];
        let source_card = head_qe.source_card;
        let source_permanent = head_qe.source_permanent;
        let source_kind = head_qe.source_kind;

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;

        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::TriggerOrder,
            selecting_player: chooser,
            previous_phase,
            valid_action_ids,
            is_optional: allow_decline_all,
            prompt: format!(
                "Choose which triggered effect to resolve next ({} pending)",
                capped,
            ),
            effect_choices: Some(choices),
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let pos = action_id.saturating_sub(HAND_EFFECT_START) as usize;
                // Find the i-th entry in `game.effect_queue` controlled by
                // `chooser` — this is the same bundle position the prompt
                // offered. Recompute defensively; single-threaded + paused
                // selection guarantees the queue hasn't shifted.
                let target_idx = game
                    .effect_queue
                    .iter()
                    .enumerate()
                    .filter(|(_, qe)| qe.controller == chooser)
                    .nth(pos)
                    .map(|(i, _)| i);
                if let Some(idx) = target_idx {
                    if let Some(qe) = game.effect_queue.remove(idx) {
                        game.run_queued_effect(qe);
                    }
                }
                // Generic resolver will call `drain_effect_queue` after we
                // return — no need to drain from inside the callback.
            }),
            on_decline: if allow_decline_all {
                Some(Box::new(move |game: &mut Game| {
                    game.effect_queue
                        .retain(|qe| !(qe.controller == chooser && qe.is_optional));
                    // Drain is the generic resolver's responsibility.
                }))
            } else {
                None
            },
        });
    }

    /// Resolve any pending selection — `TriggerOrder`, `Target`, `Hand`,
    /// `OppField`, etc. Factored here so the effect-queue module owns the
    /// validate → take → invoke → drain sequence that every selection kind
    /// needs.
    ///
    /// Specifically:
    /// 1. Validate `player` matches `selecting_player`.
    /// 2. Validate `action_id` is either in `valid_action_ids` or is PASS
    ///    with `is_optional` set.
    /// 3. Take the selection out of `Game` and restore the previous phase
    ///    *before* invoking the callback, so the callback can inspect
    ///    `current_phase` or install a follow-up selection cleanly.
    /// 4. Fire the appropriate callback (main / `on_decline`).
    /// 5. Resume draining the effect queue — unless the callback installed
    ///    a new `pending_selection`, in which case draining is deferred
    ///    until that one resolves.
    pub(crate) fn resolve_generic_selection(
        &mut self,
        player: PlayerId,
        action_id: u16,
    ) -> Result<(), crate::selection::SelectionError> {
        use crate::selection::SelectionError;

        let sel = self
            .pending_selection
            .as_ref()
            .ok_or(SelectionError::NoPendingSelection)?;
        if sel.selecting_player != player {
            return Err(SelectionError::WrongPlayer);
        }
        let is_pass = action_id == PASS;
        if is_pass && !sel.is_optional {
            return Err(SelectionError::InvalidAction);
        }
        if !is_pass && !sel.valid_action_ids.contains(&action_id) {
            return Err(SelectionError::InvalidAction);
        }

        // Take the selection, restore phase, invoke the appropriate callback.
        let sel = self.pending_selection.take().expect("checked Some above");
        self.current_phase = sel.previous_phase;
        if is_pass {
            if let Some(on_decline) = sel.on_decline {
                on_decline(self);
            }
        } else {
            (sel.callback)(self, action_id);
        }

        // Phase C §4.4: drain parked-replacement slot (if any). If the
        // resolved selection was a callback inside a replacement-process,
        // its body wrote the outcome via EffectContext::cancel_leave() etc.;
        // commit it now via commit_deferred_outcome.
        //
        // Skip when the just-run callback installed a fresh selection — that
        // path is the OUTER accept callback parking the inner select; the
        // drain belongs to whichever callback resolves WITHOUT nesting again.
        if self.pending_selection.is_none() {
            crate::replacement::try_drain_parked_replacement_with_guard(self);
        }

        if self.pending_selection.is_none() {
            self.resume_pending_pay_cost_effect();
        }

        if self.pending_selection.is_none() {
            self.resume_pending_effect_security_removal();
        }

        if self.pending_selection.is_none() {
            crate::scheduled_effects::resume_scheduled_drain(self);
        }

        // If the callback parked a fresh selection, leave the drainer alone.
        // Otherwise resume — this covers both the normal post-callback case
        // and the `TriggerOrder` "continue picking the next bundle entry"
        // flow, so callers don't have to remember to drain.
        if self.pending_selection.is_none() {
            self.drain_effect_queue();
        }

        // Phase D Task 6: a deferred deletion (e.g. printed `<Save>` parked a
        // mid-OnDeletion Tamer-pick) resumes here once the parked selection
        // resolves AND the post-callback drain settled without re-parking.
        // `resume_pending_deletion` is a no-op when no deferral is parked.
        // Skip when the post-callback drain itself parked a new selection
        // (e.g. an OnAnyDeletion observer asking for a target) — the new
        // selection drives the resume of its own drain through this same
        // hook on its eventual resolution.
        if self.pending_selection.is_none() {
            self.resume_pending_deletion();
        }

        if self.pending_selection.is_none() {
            self.resume_pending_overclock_attack();
        }

        // After any post-callback draining, re-enter the security state
        // machine if a check is mid-resolve (RUST_PYTHON_PARITY §2.5j).
        // Idempotent when `security_resolution.is_none()`; safe to call
        // unconditionally. Nested selections (the callback installed a
        // further select) leave `pending_selection = Some(...)` so the
        // advance guards re-pause cleanly.
        if self.pending_selection.is_none() {
            self.advance_security_resolution();
        }
        // Phase 8 Task 2: re-enter Option dispose if the resolved selection
        // was parked inside an OptionMain body. Standard Options trash after
        // the body finishes; Delay/Link/Training hook here in Tasks 3-5.
        if self.pending_selection.is_none() {
            self.advance_pending_option();
        }
        if self.pending_selection.is_none() {
            self.resume_pending_option_placed_link();
        }
        if self.pending_selection.is_none() {
            self.finish_pending_option_placed_turn_check();
        }
        if self.pending_selection.is_none() {
            self.resume_pending_delayed_option_lifecycle();
        }
        // Some attack-time selections, especially optional pre-declare
        // replacements, park `pending_attack` before declaration commits. Once
        // the selection callback and any nested drains settle, resume the
        // attack state machine so callers using the normal action decoder do
        // not get a Main-phase mask while an attack is still pending.
        if self.pending_selection.is_none() && self.pending_attack.is_some() {
            self.advance_pending_attack();
        }
        Ok(())
    }

    /// Post-drain Option resolution hook, invoked from
    /// `resolve_generic_selection` after the effect queue is fully drained.
    ///
    /// Only the `MainEffectDrain` phase dispatches through this hook: when an
    /// OptionMain body parks a selection, this function is re-entered after
    /// selection resolution to commit the Option's disposal (trash, or subtype
    /// branch via `dispose_option`).
    ///
    /// Other phases are pass-through:
    /// - `LinkSelectHost`: host-select unwind happens in the
    ///   `install_link_host_selection` callback directly (calls
    ///   `attach_linked_card`). The arm here exists only to prevent silent
    ///   drops if a future path accidentally routes through advance.
    /// - `Disposing`: populated by Task 6 (WhenWouldBeTrashed replacement
    ///   window for Option self-trash).
    /// - `Done`: terminal.
    fn advance_pending_option(&mut self) {
        let Some(pending) = self.pending_option.as_ref() else {
            return;
        };
        if !self.effect_queue.is_empty() {
            return;
        }
        match pending.resolution_phase {
            crate::selection::OptionResolutionPhase::MainEffectDrain => {
                // Dispatch on the card's subtype flags. Standard → trash;
                // Delay → park on field (Task 3). Link / Training land in
                // Tasks 4-5 via the same dispatcher.
                if self.pending_option_can_arts_digivolve()
                    && self.install_arts_digivolve_selection()
                {
                    return;
                }
                self.dispose_option();
                if self.pending_selection.is_none() {
                    self.check_turn_end();
                }
            }
            crate::selection::OptionResolutionPhase::ArtsSelectTarget => {
                // Arts selection callbacks finish the flow directly.
            }
            crate::selection::OptionResolutionPhase::LinkSelectHost => {
                // Unwind happens in install_link_host_selection's callback
                // (calls attach_linked_card directly). This arm exists to
                // prevent silent drops if routing drifts; the normal flow
                // doesn't reach here.
            }
            crate::selection::OptionResolutionPhase::Disposing => {
                // Task 6: an optional `WhenWouldBeTrashed` replacement
                // installed a PendingSelection during `dispose_option`'s
                // Standard arm. Now that the selection has resolved, the
                // accept-side callback wrote the outcome into
                // `replacement_pending_outcome` (None if declined — meaning
                // the original trash should proceed). Take the parked
                // pending_option back and commit the outcome via the
                // shared helper, then cleanup and advance the turn state.
                let pending = self
                    .pending_option
                    .take()
                    .expect("parked by dispose_option");
                let outcome = self
                    .replacement_pending_outcome
                    .take()
                    .unwrap_or(crate::replacement::ReplacementOutcome::None);
                self.commit_option_trash_outcome(pending, outcome);
                self.check_turn_end();
            }
            crate::selection::OptionResolutionPhase::Done => {
                // Terminal; no-op.
            }
        }
    }
}

/// Match an effect's timing + legacy bool flags against the triggering
/// timing. Mirrors the filter the legacy `fire_*` loops applied.
fn timing_flag_matches(effect: &crate::effect::Effect, timing: EffectTiming) -> bool {
    match timing {
        EffectTiming::OnPlay => effect.on_play,
        EffectTiming::OnAttack => effect.on_attack,
        EffectTiming::OnDeletion => effect.on_deletion,
        _ => effect.timing == timing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_runner::{make_test_card, DebugRunner};

    /// Chain cap safety rail: if the chain_depth counter has reached the
    /// cap (simulating a long sequence of recursive triggers), the next
    /// drain iteration trips the guard, clears the remaining queue, and
    /// resets the counter. Prevents a pathological self-triggering chain
    /// from hanging the engine.
    #[test]
    fn chain_cap_terminates_runaway_queue() {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("TEST-006", "TestSix"))
            .start();
        r.place_on_field(0, "TEST-006", Some(0));

        // Simulate having just finished 50 chained resolutions — the next
        // resolution should be capped.
        r.game.effect_chain_depth = MAX_CHAIN_DEPTH;

        // Enqueue a fresh mandatory trigger; single-bundle, would normally
        // auto-fire. With the cap already reached, drain should abort.
        r.game.enqueue_triggered(
            EffectTiming::EndOfYourTurn,
            TriggerSource::PlayerBattleArea(0),
        );
        let memory_before = r.game.memory;

        r.game.drain_effect_queue();

        assert!(
            r.game.effect_queue.is_empty(),
            "runaway queue must be cleared after hitting the cap"
        );
        assert_eq!(
            r.game.effect_chain_depth, 0,
            "chain depth must reset after the cap clears the queue"
        );
        assert!(r.game.pending_selection.is_none());
        assert_eq!(
            r.game.memory, memory_before,
            "capped effect must not have fired"
        );
    }

    /// Stable intra-bundle ordering: when the turn player has multiple
    /// permanents each with one mandatory trigger, the bundle entries appear
    /// in battle_area order. Verified by inspecting `effect_queue` before
    /// the drainer consumes it.
    #[test]
    fn bundle_preserves_battle_area_order() {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("TEST-006", "TestSix"))
            .add_card(make_test_card("TEST-008", "TestEight"))
            .start();
        let _h0 = r.place_on_field(0, "TEST-006", Some(0));
        let _h1 = r.place_on_field(0, "TEST-008", Some(0));
        let _h2 = r.place_on_field(0, "TEST-006", Some(0));

        r.game.enqueue_triggered(
            EffectTiming::EndOfYourTurn,
            TriggerSource::PlayerBattleArea(0),
        );

        assert_eq!(r.game.effect_queue.len(), 3);
        assert_eq!(
            r.game.effect_queue[0].card_id, "TEST-006",
            "first slot of battle_area comes first"
        );
        assert_eq!(r.game.effect_queue[1].card_id, "TEST-008");
        assert_eq!(r.game.effect_queue[2].card_id, "TEST-006");
    }
}
