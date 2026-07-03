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
    BREEDING_TARGET, HAND_EFFECT_END, HAND_EFFECT_START, HAND_MAIN_LIMIT, PASS, REPLACEMENT_ACCEPT,
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

/// RAII guard that installs a `TriggerContext` onto `Game.current_trigger_context`
/// for the lifetime of the guard and restores the previous value on drop —
/// including on panic. Used to evaluate a triggered effect's condition with the
/// queued effect's trigger context outside the normal `run_queued_effect`
/// drain path (e.g. the pre-cost-prompt decision in `drain_effect_queue`).
///
/// The guard borrows `&mut Game` for its whole lifetime, so callers reach the
/// game through `guard.game` while the trigger context is installed. The
/// previous value is restored on `Drop`, so an early return or panic during
/// condition evaluation cannot leak the temporary context.
struct TriggerContextGuard<'g> {
    game: &'g mut Game,
    previous: Option<TriggerContext>,
}

impl<'g> TriggerContextGuard<'g> {
    /// Install `trigger_context`, saving whatever was there before.
    fn install(game: &'g mut Game, trigger_context: Option<TriggerContext>) -> Self {
        let previous = game.current_trigger_context.take();
        game.current_trigger_context = trigger_context;
        Self { game, previous }
    }
}

impl Drop for TriggerContextGuard<'_> {
    fn drop(&mut self) {
        self.game.current_trigger_context = self.previous.take();
    }
}

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
        EffectTiming::OnPlay => game
            .modifiers
            .has(handle, ModifierType::CannotActivateOnPlayEffects),
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
        // make-engine-cloneable: capture the candidates for the data frame before
        // the closure moves them.
        let effects_for_resume = effects.clone();
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;
        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
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
        // Park the data frame: resolves via `run_refire_effect_choice_step`.
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RefireEffectChoice(
                crate::resume::RefireEffectChoiceState {
                    effects: effects_for_resume,
                    outer_conts: Vec::new(),
                },
            )],
        });
    }

    /// Resolve a parked refire-effect choice (resumable VM). Mirrors
    /// `install_refire_effect_selection`'s callback/on_decline: accept → decode
    /// the picked effect index and run it; PASS (optional decline) → no-op. Then
    /// run any composed outer-clause tails.
    pub(crate) fn run_refire_effect_choice_step(
        &mut self,
        state: crate::resume::RefireEffectChoiceState,
        action_id: u16,
        is_pass: bool,
    ) {
        if !is_pass {
            let pos = action_id.saturating_sub(HAND_EFFECT_START) as usize;
            if let Some(effect) = state.effects.get(pos).cloned() {
                self.run_refired_effect(effect);
            }
        }
        crate::dsl_cards::step::selections::run_outer_conts(self, state.outer_conts);
    }

    /// Collect every effect on `source` whose timing matches `timing` and
    /// whose `is_*` flag matches the timing, append them to `effect_queue`.
    ///
    /// **Does not drive execution.** Call `drain_effect_queue()` afterward
    /// to resolve the collected effects, or call `enqueue_triggered` for
    /// multiple sources first and drain once at the end.
    /// Enqueue the `OnAddToHand` observer for an EFFECT-driven hand gain by
    /// `player`. Call this from every effect-initiated "add to hand" sink
    /// (return-to-hand, security/trash/deck/reveal-to-hand, effect-draw, …) — NOT
    /// from the normal turn/mulligan draw. The observer is enqueued (drained by
    /// the surrounding effect-resolution loop, like `OnPlay`/`OnEnterField`); the
    /// gaining player is carried in `affected_player` so a controller's observer
    /// can gate on "the opponent's hand gained cards." See G-ON-ADD-TO-HAND-OBSERVER.
    pub(crate) fn fire_on_add_to_hand_by_effect(&mut self, player: PlayerId) {
        self.enqueue_triggered(
            EffectTiming::OnAddToHand,
            TriggerSource::HandGained {
                player,
                effect_initiated: true,
            },
        );
    }

    pub fn enqueue_triggered(&mut self, timing: EffectTiming, source: TriggerSource) {
        match source {
            TriggerSource::Permanent(handle) => {
                let trigger_context =
                    self.trigger_context_for_source(&source, Some(handle), timing);
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
                    let trigger_context =
                        self.trigger_context_for_source(&source, Some(handle), timing);
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
            }
            TriggerSource::Linked { player, .. } => {
                // Same battle-area scan as `PlayerBattleArea`, but the trigger
                // context carries the just-linked card so a `WhenLinked`
                // self-filter (`event_card == source_card`) is possible. The
                // `.linked()` OnLink effect on the just-linked card is reached
                // through `enqueue_from_permanent`'s linked-card scan.
                let count = self.player(player).battle_area.len();
                for i in 0..count {
                    let handle = PermanentHandle {
                        player,
                        index: i as u8,
                    };
                    let trigger_context =
                        self.trigger_context_for_source(&source, Some(handle), timing);
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
            }
            TriggerSource::PlayerBreedingArea(player) => {
                let handle = PermanentHandle {
                    player,
                    index: BREEDING_TARGET as u8,
                };
                let trigger_context =
                    self.trigger_context_for_source(&source, Some(handle), timing);
                self.enqueue_from_breeding_permanent(timing, handle, Some(trigger_context));
            }
            TriggerSource::SecurityRevealed { defender, card } => {
                let trigger_context = self.trigger_context_for_source(&source, None, timing);
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
                let trigger_context = self.trigger_context_for_source(&source, None, timing);
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
                    let trigger_context =
                        self.trigger_context_for_source(&source, Some(handle), timing);
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
            }
            TriggerSource::OnCheckFaceUpSecurity { attacker, .. } => {
                let count = self.player(attacker.player).battle_area.len();
                for i in 0..count {
                    let handle = PermanentHandle {
                        player: attacker.player,
                        index: i as u8,
                    };
                    let trigger_context =
                        self.trigger_context_for_source(&source, Some(handle), timing);
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
            }
            TriggerSource::MovedFromBreeding { .. } => {
                for player in 0..self.players.len() {
                    let player = player as PlayerId;
                    let count = self.player(player).battle_area.len();
                    for i in 0..count {
                        let handle = PermanentHandle {
                            player,
                            index: i as u8,
                        };
                        let trigger_context =
                            self.trigger_context_for_source(&source, Some(handle), timing);
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
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
                            self.trigger_context_for_source(&source, Some(handle), timing);
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                }
            }
            TriggerSource::HandGained { .. } => {
                // Fan out to EVERY player's battle area so a controller's
                // observer (e.g. BT11-033 watching the opponent's hand) sees a
                // hand-gain on any player. The gaining player is carried in
                // `affected_player` (set in `trigger_context_for_source`).
                for player in 0..self.players.len() {
                    let player = player as PlayerId;
                    let count = self.player(player).battle_area.len();
                    for i in 0..count {
                        let handle = PermanentHandle {
                            player,
                            index: i as u8,
                        };
                        let trigger_context =
                            self.trigger_context_for_source(&source, Some(handle), timing);
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
                            self.trigger_context_for_source(&source, Some(handle), timing);
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                }
                if timing == EffectTiming::OnAllyPlayed {
                    let trigger_context = self.trigger_context_for_source(&source, None, timing);
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
                            self.trigger_context_for_source(&source, Some(handle), timing);
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                    let breeding_handle = PermanentHandle {
                        player,
                        index: BREEDING_TARGET as u8,
                    };
                    let trigger_context =
                        self.trigger_context_for_source(&source, Some(breeding_handle), timing);
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
                            self.trigger_context_for_source(&source, Some(handle), timing);
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                    let breeding_handle = PermanentHandle {
                        player,
                        index: BREEDING_TARGET as u8,
                    };
                    let trigger_context =
                        self.trigger_context_for_source(&source, Some(breeding_handle), timing);
                    self.enqueue_from_breeding_permanent(
                        timing,
                        breeding_handle,
                        Some(trigger_context),
                    );
                }
            }
            TriggerSource::PlayerBattleAreaAttack { player, .. } => {
                let count = self.player(player).battle_area.len();
                for i in 0..count {
                    let handle = PermanentHandle {
                        player,
                        index: i as u8,
                    };
                    let trigger_context =
                        self.trigger_context_for_source(&source, Some(handle), timing);
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
            }
            TriggerSource::EventObserved { .. }
            | TriggerSource::AttackTargetChanged { .. }
            | TriggerSource::BlockDeclared { .. } => {
                for player in 0..self.players.len() {
                    let player = player as PlayerId;
                    let count = self.player(player).battle_area.len();
                    for i in 0..count {
                        let handle = PermanentHandle {
                            player,
                            index: i as u8,
                        };
                        let trigger_context =
                            self.trigger_context_for_source(&source, Some(handle), timing);
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                }
            }
            TriggerSource::SourceTrashedFromStack { player, card, .. } => {
                let trigger_context = self.trigger_context_for_source(&source, None, timing);
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
                            self.trigger_context_for_source(&source, Some(handle), timing);
                        self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                    }
                }
            }
            TriggerSource::SourceReturnedToDeckBottom { .. } => {
                // Sibling of SourceTrashedFromStack, but the card moved to the
                // DECK (not trash), so there is no trashed-source zone to scan —
                // only battle-area permanents observe it. The host-scoped
                // observer (BT21-058 / BT18-065 inherited "from THIS Digimon's
                // digivolution cards") lives on the permanent whose stack lost
                // the source; its `event_host_permanent_is_source` predicate is
                // satisfied by the carried `host` in the trigger context.
                for player in 0..self.players.len() {
                    let player = player as PlayerId;
                    let count = self.player(player).battle_area.len();
                    for i in 0..count {
                        let handle = PermanentHandle {
                            player,
                            index: i as u8,
                        };
                        let trigger_context =
                            self.trigger_context_for_source(&source, Some(handle), timing);
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
                    let trigger_context =
                        self.trigger_context_for_source(&source, Some(handle), timing);
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
                let breeding_handle = PermanentHandle {
                    player: observer_player,
                    index: BREEDING_TARGET as u8,
                };
                let trigger_context =
                    self.trigger_context_for_source(&source, Some(breeding_handle), timing);
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
                    let trigger_context =
                        self.trigger_context_for_source(&source, Some(handle), timing);
                    self.enqueue_from_permanent(timing, handle, Some(trigger_context));
                }
                let breeding_handle = PermanentHandle {
                    player: affected_player,
                    index: BREEDING_TARGET as u8,
                };
                let trigger_context =
                    self.trigger_context_for_source(&source, Some(breeding_handle), timing);
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
                let trigger_context = self.trigger_context_for_source(&source, None, timing);
                self.enqueue_from_security_card(
                    timing,
                    affected_player,
                    card,
                    Some(trigger_context),
                );
            }
        }
        // Fan event dispatches out to placed event-gated Delay Options.
        // `EnteredField` covers `OnEnterFieldAnyone` / `OnAllyPlayed` plays:
        // P-229's `<Delay>` is keyed to `OnEvent(OnAllyPlayed)` and must fire
        // when a [Mirai Kinosaki] is played (PUPPETS-G004). The candidate scan
        // in `enqueue_event_gated_delayed_options` only matches Options whose
        // `OnEvent(event_timing)` equals `timing`, so dispatching for both the
        // `OnEnterFieldAnyone` and `OnAllyPlayed` broadcasts is harmless.
        if matches!(
            source,
            TriggerSource::EventObserved { .. }
                | TriggerSource::PlayerBattleAreaAttack { .. }
                | TriggerSource::AttackTargetChanged { .. }
                | TriggerSource::BlockDeclared { .. }
                | TriggerSource::EnteredField { .. }
        ) {
            let trigger_context = self.trigger_context_for_source(&source, None, timing);
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

    /// Open a deferred-drain scope. While `draining_deferred > 0`, every
    /// `fire_on_*` observer helper that previously inline-drained should
    /// route through `maybe_drain_effect_queue()` instead — enqueue but
    /// don't drain, leaving the flush to whichever outer scope exits last.
    ///
    /// Always pair with `exit_deferred_drain_and_flush()`. The post-fix
    /// flush happens only on the final exit (counter going 1 → 0).
    ///
    /// Used to prevent the nested-park collision documented at
    /// `qa/archetype-qa/engine-gaps.md` `G-DSL-OUTER-TAIL-NESTED-PARK`:
    /// previously, an inline drain inside a select-resolution callback
    /// could fire a second copy of the calling triggered effect while
    /// the first's `dsl_outer_tail` was still parked.
    pub(crate) fn enter_deferred_drain(&mut self) {
        self.draining_deferred = self.draining_deferred.saturating_add(1);
    }

    /// Close a deferred-drain scope opened by `enter_deferred_drain()`.
    /// On the final exit (counter going 1 → 0), drain anything that
    /// accumulated in the queue during the scope.
    pub(crate) fn exit_deferred_drain_and_flush(&mut self) {
        self.draining_deferred = self.draining_deferred.saturating_sub(1);
        if self.draining_deferred == 0 {
            self.drain_effect_queue();
        }
    }

    /// Drain the effect queue unless we're inside a deferred-drain scope.
    /// `fire_on_*` observer helpers should call this instead of
    /// `drain_effect_queue()` directly, so a select-resolution callback
    /// or outer-tail run can hold the queue back until it exits.
    pub(crate) fn maybe_drain_effect_queue(&mut self) {
        if self.draining_deferred == 0 {
            self.drain_effect_queue();
        }
    }

    /// Play the SECOND extracted `<Partition>` source once the CURRENT
    /// pending-selection chain fully drains (judge-quiz Q30: the second
    /// partition source must be played only after the FIRST play's would-play
    /// interrupt chain — e.g. MedievalGallantmon's "suspend 2 Digimon" cost
    /// reduction — completes, so the second card is not yet in the battle
    /// area while the first's interrupts resolve, "played out
    /// simultaneously").
    ///
    /// If no selection is pending, the play happens immediately. Otherwise a
    /// data [`AfterSelectionHook::PartitionSecondPlay`] is armed; the drain
    /// in `resolve_generic_selection` re-enters here after each resolution,
    /// so the hook RE-ARMS until the last selection in the chain resolves —
    /// it fires exactly once. Being plain data, an armed hook survives
    /// `Game::clone` faithfully.
    pub(crate) fn queue_partition_second_play(
        &mut self,
        controller: PlayerId,
        source_card: crate::card_source::CardHandle,
        card: crate::card_source::CardHandle,
    ) {
        if self.pending_selection.is_none() {
            let mut ctx =
                crate::effect_context::EffectContext::new(self, source_card, None, controller);
            let _ = ctx.play_from_trash_free_unsuspended(card);
            return;
        }
        self.after_selection_resume_hooks.0.push(
            crate::resume::AfterSelectionHook::PartitionSecondPlay {
                controller,
                source_card,
                card,
            },
        );
    }

    /// Dispatch one drained [`AfterSelectionHook`] to its owning module's
    /// resume method. Called by `resolve_generic_selection` right after a
    /// resume-driven resolution (the data analog of the retired
    /// closure-composition wrappers).
    pub(crate) fn run_after_selection_hook(&mut self, hook: crate::resume::AfterSelectionHook) {
        use crate::resume::AfterSelectionHook as Hook;
        match hook {
            Hook::InteractiveDigivolveReducer {
                amount,
                acting_player,
                hand_index,
                field_index,
                source,
            } => self.resume_interactive_digivolve_reducer_after_pending(
                amount,
                acting_player,
                hand_index,
                field_index,
                source,
            ),
            Hook::DigiXrosLeaveContinuation { cont } => {
                self.continue_digixros_after_parked_leave(cont)
            }
            Hook::PlayCostContinuation {
                player_id,
                hand_index,
                target,
                cost_delta,
                source,
                origin,
                suppress_on_play,
                accumulated_reduction,
                processed,
            } => self.resume_play_cost_continuation_after_pending(
                player_id,
                hand_index,
                target,
                cost_delta,
                source,
                origin,
                suppress_on_play,
                accumulated_reduction,
                processed,
            ),
            Hook::InteractiveOptionUseReducer {
                amount,
                player_id,
                source,
                mode,
                cost_policy,
            } => self.resume_interactive_option_use_reducer_after_pending(
                amount,
                player_id,
                source,
                mode,
                cost_policy,
            ),
            // Re-arms itself while a selection is still pending; plays once
            // the chain has drained.
            Hook::PartitionSecondPlay {
                controller,
                source_card,
                card,
            } => self.queue_partition_second_play(controller, source_card, card),
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
    /// Public drain entrypoint. Wraps the inner queue loop with the general
    /// state-based ≤0-DP rules-check (Gap 1 / `G-NO-GENERAL-ZERO-DP-RULES-CHECK`).
    ///
    /// The rules-check runs only at the OUTERMOST drain (`effect_drain_depth == 1`)
    /// — re-entrant drains (an effect body draining via `EffectContext`, or the
    /// batched-deletion deferred-drain flush) observe depth > 1 and skip it, so it
    /// never fires between the sub-steps of one resolving effect (the judge rule:
    /// "rule checks don't happen until an ongoing effect or rule action finishes" —
    /// Q6/Q13/Q14). The primary site is BETWEEN top-level queued effects
    /// (`rules_check_between_queued_effects`, after each `run_queued_effect`), so a
    /// Digimon driven to ≤0 DP by one effect is deleted before the next queued
    /// trigger resolves (Q24). The wrapper adds a final fixpoint sweep for any
    /// ≤0-DP Digimon left when the queue emptied. A parked selection means
    /// resolution is NOT finished; the check defers to the drain that resumes when
    /// the selection resolves.
    pub fn drain_effect_queue(&mut self) {
        self.effect_drain_depth = self.effect_drain_depth.saturating_add(1);
        // Official timing: complete rule processing BEFORE activating queued
        // triggered effects (judge-quiz Q24 — a Digimon driven to ≤0 DP
        // while its trigger was parked is deleted by the rules check first,
        // so the trigger's source is gone when activation is attempted).
        // Outermost entries only; nested drains are mid-effect (Q6/Q13/Q14).
        if self.effect_drain_depth == 1 && self.pending_selection.is_none() {
            self.run_state_based_rules_check();
        }
        self.drain_effect_queue_inner();
        if self.effect_drain_depth == 1 {
            let mut guard: u16 = 0;
            loop {
                if self.pending_selection.is_some() {
                    break;
                }
                if !self.run_state_based_rules_check() {
                    break;
                }
                guard += 1;
                if guard > MAX_CHAIN_DEPTH {
                    break;
                }
                // The batched deletion drains its own OnDeletion handlers, but
                // those (or auras expiring with the deleted carrier) may have
                // enqueued further triggers — flush them before the next pass.
                if self.pending_selection.is_none() && !self.effect_queue.is_empty() {
                    self.drain_effect_queue_inner();
                }
            }
        }
        self.effect_drain_depth = self.effect_drain_depth.saturating_sub(1);
    }

    /// State-based ≤0-DP rules-check run BETWEEN top-level queued effects (after
    /// each `run_queued_effect`). Q24: a Digimon driven to ≤0 DP by one effect is
    /// deleted by the rules-check before the next queued trigger resolves. Runs
    /// only at the outermost drain (`effect_drain_depth == 1`) and only when
    /// resolution isn't parked on a selection — so it never fires between the
    /// sub-steps of one resolving effect (Q6/Q13/Q14).
    fn rules_check_between_queued_effects(&mut self) {
        if self.effect_drain_depth == 1 && self.pending_selection.is_none() {
            self.run_state_based_rules_check();
        }
    }

    /// Inner queue-drain loop (the historical `drain_effect_queue` body).
    /// Resolves triggered effects until the queue empties or a selection parks.
    /// Runs the state-based rules-check between top-level queued effects via
    /// `rules_check_between_queued_effects`; the final fixpoint sweep is the
    /// wrapper's job.
    fn drain_effect_queue_inner(&mut self) {
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

            // Filter queued triggers whose clause-level condition would
            // currently fail. They would resolve as no-ops if fired
            // (condition check in `run_queued_effect_inner` skips the
            // body), so they shouldn't inflate `bundle.len()` and force a
            // spurious `TriggerOrder` prompt over a single fireable
            // trigger. Mirrors DCGO's "collect ICardEffects with
            // CanUseCondition passing" semantic — the trigger queue
            // should only surface user-visible choices for triggers
            // that have at least one viable effect path.
            //
            // **Non-destructive filter (2026-05-24, may-dna-digivolve-now
            // follow-up).** The exclusion is transient to this iteration:
            // entries stay in `effect_queue` so a later iteration can
            // re-evaluate their condition after a sibling trigger's body
            // has mutated state. Canonical case: BT22-008's `[EoT]`
            // inherited DNA digivolve creates an Omnimon-named Digimon
            // mid-drain → BT17-081's slot 2 `[EoT]` clause (gated on
            // `any_permanent: { name_contains: Omnimon }`) now passes its
            // condition and fires the "1 of your Omnimon may attack a
            // player" prompt in the SAME EoT batch, matching DCGO.
            //
            // Source-liveness and once-per-turn checks are NOT applied
            // here: source-liveness can become false mid-drain due to a
            // sibling trigger's body trashing the source, and the
            // run-time check handles that case; OPT lockout is
            // accounting-only and doesn't change user-visible choice.
            let non_firing: Vec<usize> = self.non_firing_queued_effect_indices_for(chooser);

            let bundle: Vec<usize> = self
                .effect_queue
                .iter()
                .enumerate()
                .filter_map(|(i, qe)| {
                    (qe.controller == chooser && !non_firing.contains(&i)).then_some(i)
                })
                .collect();

            if bundle.is_empty() {
                // Every queued effect for this chooser is currently
                // non-firing. They can't fire on their own — and no
                // OTHER chooser's body can mutate state to satisfy them
                // (the queue is per-chooser-batched). Drop them and
                // continue with the next chooser.
                //
                // Reverse-order removal so earlier indices stay valid.
                for idx in non_firing.into_iter().rev() {
                    self.effect_queue.remove(idx);
                }
                continue;
            }

            if bundle.len() == 1 {
                let idx = bundle[0];
                // Single trigger with activation_cost_fn + optional: true →
                // expose a TriggerOrder selection (with PASS) BEFORE running
                // the cost closure. This lets the player decline the
                // activation cost itself (e.g. "By returning this Tamer to
                // the bottom of the deck, you may …"). Without this branch
                // the cost would fire automatically and the player could
                // not avoid it.
                //
                // For optional triggers without activation_cost_fn,
                // optionality is exposed by the body's first actionable
                // pending selection; auto-fire is intentional there so we
                // don't prompt when filters produce no legal follow-up.
                let needs_pre_cost_prompt = {
                    let (
                        is_optional,
                        card_id,
                        source_card,
                        source_permanent,
                        source_kind,
                        controller,
                        effect_slot,
                        trigger_context,
                    ) = {
                        let qe = &self.effect_queue[idx];
                        (
                            qe.is_optional,
                            qe.card_id.clone(),
                            qe.source_card,
                            qe.source_permanent,
                            qe.source_kind,
                            qe.controller,
                            qe.effect_slot as usize,
                            qe.trigger_context.clone(),
                        )
                    };
                    if is_optional {
                        if let Some(effects) = self.effects_for_card(&card_id, source_card) {
                            if let Some(eff) = effects.get(effect_slot) {
                                // Only install a pre-cost prompt when the
                                // effect has an activation_cost AND the
                                // condition (if any) currently passes. If the
                                // condition would suppress the effect anyway,
                                // auto-fire so it silently skips — no prompt.
                                let has_cost = eff.activation_cost_fn.is_some();
                                let condition_passes = if has_cost {
                                    if let Some(cond) = &eff.condition {
                                        // The condition must see the queued
                                        // effect's trigger context — DSL
                                        // predicates like `event_target_owner`
                                        // / `event_target_trait_has` and
                                        // deleted-object snapshots read
                                        // `current_trigger_context`. The real
                                        // evaluation path (`run_queued_effect`
                                        // → `run_queued_effect_inner`) sets it
                                        // before its condition check; mirror
                                        // that here so the pre-cost decision
                                        // is faithful. The RAII guard restores
                                        // the previous value even on panic.
                                        let trigger_guard =
                                            TriggerContextGuard::install(self, trigger_context);
                                        let ctx = EffectContext::new_with_source_kind(
                                            &mut *trigger_guard.game,
                                            source_card,
                                            source_permanent,
                                            source_kind,
                                            controller,
                                        );
                                        cond(&ctx.as_read())
                                    } else {
                                        true
                                    }
                                } else {
                                    false
                                };
                                has_cost && condition_passes
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };
                if needs_pre_cost_prompt {
                    self.install_trigger_order_selection(chooser, &bundle, true);
                    return;
                }
                let qe = self
                    .effect_queue
                    .remove(idx)
                    .expect("bundle index in-bounds by construction");
                // Composition note: `needs_pre_cost_prompt` (cost-bearing
                // optional trigger) and `needs_outer_optional_prompt` below
                // are mutually exclusive in practice. The pre-cost branch
                // above `return`s and its TriggerOrder callback runs the
                // effect directly via `run_queued_effect`, bypassing this
                // outer-prompt check — so a cost-bearing optional effect
                // gets exactly one decline gate (the pre-cost PASS), and a
                // cost-free optional effect gets exactly one (the outer
                // prompt). No double-prompt.
                //
                // G-OUTER-OPTIONAL-NOT-INSTALLED: a lone OPTIONAL triggered
                // effect ("you may …") whose body's first step is a mandatory
                // selection must surface an explicit outer accept/decline
                // prompt BEFORE its body runs — otherwise the body forces an
                // action the printed "you may" says the player can refuse.
                // The DSL lowering sets `needs_outer_optional_prompt` only for
                // that case (when the first body step already exposes a
                // declinable PASS, the inner PASS is the decline path and the
                // flag stays false). The outer prompt is skipped entirely
                // when the effect's preconditions already fail.
                if qe.is_optional && self.queued_effect_wants_outer_optional_prompt(&qe) {
                    self.install_outer_optional_trigger_selection(qe);
                    return;
                }
                self.run_queued_effect(qe);
                self.rules_check_between_queued_effects();
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
                self.rules_check_between_queued_effects();
                continue;
            }

            // Multi-trigger bundle — install a TriggerOrder selection.
            // Cap at HAND_MAIN_LIMIT (30) to fit the reused 30-59 action
            // range. Overflow auto-fires in collection order after the prompt
            // completes (rare; see the cap handling inside install_*).
            //
            // Note: the pre-cost decline prompt above is only installed for a
            // single-trigger bundle (`bundle.len() == 1`). For a multi-copy
            // bundle (`bundle.len() >= 2`), once the player picks the first
            // trigger here, that trigger's `activation_cost_fn` fires inside
            // `run_queued_effect` without a separate per-trigger decline.
            // This is a pre-existing limitation, kept intentional: the
            // TriggerOrder PASS bit declines the whole bundle, not a single
            // copy mid-resolution.
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
        timing: EffectTiming,
    ) -> TriggerContext {
        let mut context = match *source {
            TriggerSource::Permanent(handle) => TriggerContext {
                target_permanent: Some(handle),
                target_card: self.top_card_handle(handle),
                source_player: Some(handle.player),
                // `cause` is only meaningful for the OnDeletion timing: it
                // forwards the active deletion cause (set by
                // delete_permanent_with_cause before enqueueing OnDeletion,
                // override-first via observed_deletion_event_cause) so
                // event_cause predicates in on_deletion YAML clauses can
                // inspect it (e.g. "not battle"). `TriggerSource::Permanent`
                // is shared by non-deletion timings (OnPlay, OnDigivolve,
                // CounterEffect, …) — leave `cause` at `None` for those.
                cause: (timing == EffectTiming::OnDeletion)
                    .then(|| self.observed_deletion_event_cause())
                    .flatten(),
                ..TriggerContext::default()
            },
            TriggerSource::PlayerBattleArea(player) => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                source_player: Some(player),
                ..TriggerContext::default()
            },
            TriggerSource::PlayerBattleAreaAttack {
                player,
                attacker,
                card,
            } => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_permanent: Some(attacker),
                event_card: Some(card),
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
            TriggerSource::OnCheckFaceUpSecurity {
                attacker,
                defender,
                revealed_card,
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
            TriggerSource::HandGained {
                player,
                effect_initiated,
            } => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                // The player whose hand gained cards. `event_add_to_hand_player`
                // predicates compare this against the observer's controller.
                affected_player: Some(player),
                source_player: Some(player),
                effect_initiated,
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
            TriggerSource::Linked { player, host, card } => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_permanent: Some(host),
                event_card: Some(card),
                event_source_card: Some(card),
                event_host_card: self.top_card_handle(host),
                event_host_permanent: Some(host),
                source_player: Some(player),
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
            TriggerSource::BlockDeclared {
                attacker,
                blocker,
                card,
            } => TriggerContext {
                target_permanent: source_permanent,
                target_card: source_permanent.and_then(|h| self.top_card_handle(h)),
                event_permanent: Some(attacker),
                event_card: Some(card),
                event_host_permanent: Some(blocker),
                event_host_card: self.top_card_handle(blocker),
                source_player: Some(attacker.player),
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
            // Sibling of SourceTrashedFromStack; the returned card moved to the
            // DECK, so the `moved_card_sets` destination is `Deck` (bottom).
            // Same host / event-card context so the host-scoped
            // (`event_host_permanent_is_source`) and event-card-name
            // (`event_card_name_contains`) observer gates resolve identically.
            TriggerSource::SourceReturnedToDeckBottom {
                player,
                host,
                host_card,
                card,
                cause,
            } => TriggerContext {
                subject: Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Deck,
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
                    to: Some(crate::enums::Zone::Deck),
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

    /// Resolve a `PermanentHandle` to its top card. Returns `None` when:
    /// - the slot doesn't exist (out-of-range index, missing breeding perm), OR
    /// - the slot exists but its `card_sources` is empty (a "zombie" permanent
    ///   left over from a body-moving operation that hasn't finished cleanup).
    ///
    /// All callers wrap this in `Option::and_then`, treating None as "no card
    /// for trigger context purposes" — which is the right semantics for both
    /// missing-slot and zombie cases. Defending here (rather than at every
    /// iteration site that uses this for observer fan-out) means future
    /// zombie-producing code paths cannot trip the `permanent.rs:134` assert
    /// via this code path. See `G-PERMANENT-EMPTY-DURING-BATCH-DELETION` in
    /// `qa/archetype-qa/engine-gaps.md` for the original surfacing.
    ///
    /// Note: this is intentionally `.and_then(card_sources.last())` rather
    /// than `.map(top_card())` — `Permanent::top_card()` asserts non-empty,
    /// which is correct for direct accesses but wrong for handle-resolution
    /// scans across all battle-area slots.
    fn top_card_handle(&self, handle: PermanentHandle) -> Option<CardHandle> {
        if handle.index == BREEDING_TARGET as u8 {
            return self
                .players
                .get(handle.player as usize)
                .and_then(|p| p.breeding_area.as_ref())
                .and_then(|perm| perm.card_sources.last().map(|c| c.handle()));
        }
        self.players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
            .and_then(|perm| perm.card_sources.last().map(|c| c.handle()))
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
        let Some(perm) = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
        else {
            return;
        };
        // Layer-2 defensive guard: a "zombie" permanent (slot exists in
        // battle_area but `card_sources` is empty, left over from some
        // body-moving operation that hasn't finished cleanup) has no
        // effects to enqueue. Skip immediately rather than tripping
        // `Permanent::top_card()`'s assertion inside `source_kind_for_card_kind`
        // or `effect_list`. Matches DCGO's `if (TopCard == null) skip`
        // pattern (e.g. Permanent.cs:2093 `CanAttack` guard). See
        // `G-PERMANENT-EMPTY-DURING-BATCH-DELETION` in
        // `qa/archetype-qa/engine-gaps.md`.
        if perm.card_sources.is_empty() {
            return;
        }
        let top = perm.top_card();
        let card_id = top.card_id(&self.card_data).to_string();
        let source_card = top.handle();
        let top_source_kind = source_kind_for_card_kind(top.card_kind(&self.card_data));

        let tp = self.turn_player();
        let is_turn_player = handle.player == tp;
        if permanent_activation_blocked_for_timing(self, handle, timing) {
            return;
        }

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
            // Cause attribution: a granted triggered effect is the GRANTOR's
            // effect from the carrier's perspective. If the carrier is
            // unaffected by that player's effects, the granted clause does NOT
            // fire. Two flavors:
            //   • `<Progress>` / `ImmunityToOpponentEffects` while the carrier
            //     is the current attacker (judge-quiz Q2) — gated at enqueue
            //     time while `pending_attack` is still set.
            //   • A general `CannotBeAffected` opponent-effect immunity (e.g.
            //     Magnamon (X Antibody)'s "isn't affected by your opponent's
            //     effects") removes the opponent-granted clause — judge-quiz
            //     Q17. The granted effect is the grantor's, so we test
            //     immunity against `source_player`, keyed to the grantor's
            //     effect-source kind.
            // No-op for same-controller grants.
            let grantor_kind = self
                .card_data_for_handle(source_card)
                .map(|d| source_kind_for_card_kind(d.card_kind))
                .unwrap_or(crate::enums::EffectSourceKind::Digimon);
            if self.progress_excludes(handle, Some(source_player))
                || self.permanent_is_unaffected_by_effect(handle, source_player, grantor_kind)
            {
                continue;
            }
            // D4 / DCGO: a granted effect is the GRANTEE's own effect once
            // installed (DCGO sources the granted ActivateClass from the
            // carrier's top card). Run the body with controller = carrier's
            // controller so a deletion it causes is the carrier's OwnEffect
            // (e.g. AD1-… <Partition> does NOT fire on a granted self-delete —
            // judge-quiz Q16). The grantor (`source_player`) is used only for
            // the immunity gate above, never for body attribution.
            let carrier_controller = handle.player;
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
                controller: carrier_controller,
                timing,
                trigger_context: trigger_context.clone(),
                effect_slot: 0,
                is_optional: false,
                is_turn_player: carrier_controller == tp_for_granted,
                card_id: String::new(),
                allow_below_top_liveness: false,
                dna_origin_context: self.current_dna_origin,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                granted_effect_id: Some(body_id),
            });
        }

        if let Some(effects) = self.effects_for_card(&card_id, source_card) {
            for (slot, effect) in effects.iter().enumerate() {
                if !timing_flag_matches(effect, timing) {
                    continue;
                }
                if effect.linked {
                    continue;
                }
                // An inherited effect (the lower portion of a digi card) is
                // active ONLY while that card is a digivolution source beneath
                // another card — the top Digimon activates it (RULES 15-3-1).
                // The card scanned here is the carrier's *top* card, never a
                // source, so its own inherited clauses stay dormant. Below-top
                // sources dispatch their inherited effects through the dedicated
                // `inherited_sources` scan further down. Mirrors
                // `enqueue_from_breeding_permanent`, which likewise skips
                // inherited effects unconditionally on its top-card scan.
                if effect.inherited {
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

        // Phase 2 Track D: walk every below-top digivolution source on the
        // carrier. `card_sources` is `[base, evo1, evo2, ..., top]`; the
        // top-card slot was already scanned above. Each below-top source
        // dispatches its INHERITED effects through this carrier permanent,
        // with `source_card` set to the stacked card's handle so OPT-slot
        // keying (Track C) and predicates that read source identity remain
        // per-source-slot stable. `allow_below_top_liveness: true` lets the
        // liveness gate accept the stacked source as a valid origin.
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

        if permanent_activation_blocked_for_timing(self, handle, timing) {
            return;
        }

        // Track H §3 Phase 4i — same queue-based granted dispatch as
        // `enqueue_from_permanent`; breeding-area carriers are valid
        // grant targets via the breeding permanent's handle.
        let granted_entries = self
            .modifiers
            .granted_triggered_for_timing_with_ids(handle, timing);
        let tp_for_granted = self.turn_player();
        for (body_id, source_card, source_player) in granted_entries {
            // D4 / DCGO: granted body runs as the carrier's own effect (see the
            // battle-area dispatch above). `source_player` (grantor) gates only
            // the opponent-effect immunity check (Q17) — a breeding carrier
            // can't be the current attacker, so the Progress branch is moot.
            let grantor_kind = self
                .card_data_for_handle(source_card)
                .map(|d| source_kind_for_card_kind(d.card_kind))
                .unwrap_or(crate::enums::EffectSourceKind::Digimon);
            if self.permanent_is_unaffected_by_effect(handle, source_player, grantor_kind) {
                continue;
            }
            let carrier_controller = handle.player;
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
                controller: carrier_controller,
                timing,
                trigger_context: trigger_context.clone(),
                effect_slot: 0,
                is_optional: false,
                is_turn_player: carrier_controller == tp_for_granted,
                card_id: String::new(),
                allow_below_top_liveness: false,
                dna_origin_context: self.current_dna_origin,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                granted_effect_id: Some(body_id),
            });
        }

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
        if let Some(qe) = self.effect_queue.iter().find(|qe| qe.is_turn_player) {
            return Some(qe.controller);
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

    /// Return the indices of queued effects owned by `chooser` whose
    /// clause-level `condition` would currently fail — i.e. those that
    /// should be EXCLUDED from this drain iteration's bundle.
    ///
    /// **Non-destructive (2026-05-24, may-dna-digivolve-now follow-up).**
    /// Earlier versions removed these entries from `effect_queue`
    /// outright. That broke the canonical "BT22-008 EoT DNA digivolve
    /// creates Omnimon → BT17-081 [EoT] `name_contains: Omnimon` clause
    /// now fires" chain: BT17-081's slot 2 condition currently fails
    /// (no Omnimon on field), so it was pruned at queue time — even
    /// though BT22-008's body, scheduled to run earlier in the same
    /// drain, would create the Omnimon that makes BT17-081's condition
    /// pass. The DNA-digivolved Omnimon never got attacked because
    /// BT17-081 slot 2 was already gone from the queue.
    ///
    /// The fix: filter the bundle each iteration (read-only) so a
    /// trigger whose condition currently fails gets a fresh re-check
    /// after its sibling triggers' bodies mutate state. Entries whose
    /// condition will never pass cleanly drain at the bottom of the
    /// loop (`drain_effect_queue` advances past them via the next
    /// chooser).
    ///
    /// Does NOT filter on source-liveness (can change mid-drain due to
    /// sibling-trigger trashing) or on once-per-turn caps (accounting
    /// only, doesn't change visible choice). Those keep their existing
    /// run-time checks inside `run_queued_effect_inner`.
    ///
    /// Conditions are evaluated with each queued effect's own
    /// `trigger_context` installed via `TriggerContextGuard`, mirroring
    /// the pre-cost-prompt branch's evaluation in `drain_effect_queue`.
    ///
    /// Granted-triggered-effect entries (`granted_effect_id.is_some()`)
    /// are NEVER excluded — their bodies have no `Effect` metadata and
    /// no clause-level condition, so the "would no-op" predicate is
    /// undefined for them. They keep their existing run-time path.
    fn non_firing_queued_effect_indices_for(&mut self, chooser: PlayerId) -> Vec<usize> {
        let mut to_skip: Vec<usize> = Vec::new();
        for i in 0..self.effect_queue.len() {
            let qe = &self.effect_queue[i];
            if qe.controller != chooser {
                continue;
            }
            if qe.granted_effect_id.is_some() {
                // Granted effects have no clause condition — keep them.
                continue;
            }
            // Snapshot the fields we need; release the borrow before
            // entering the trigger-context guard (which needs &mut self).
            let card_id = qe.card_id.clone();
            let source_card = qe.source_card;
            let source_permanent = qe.source_permanent;
            let source_kind = qe.source_kind;
            let controller = qe.controller;
            let effect_slot = qe.effect_slot as usize;
            let trigger_context = qe.trigger_context.clone();
            let dna_origin_context = qe.dna_origin_context;

            // Look up the effect's condition closure. If the effect
            // doesn't exist anymore (carrier removed, registry mutated),
            // the run-time path returns silently — treat the same here:
            // keep the queued entry, let run-time handle it.
            //
            // `effects_for_card` returns an OWNED `Vec<Effect>` so the
            // condition closure can be evaluated against `&mut self`
            // without lifetime conflict — same idiom as the pre-cost
            // prompt branch's evaluation in `drain_effect_queue`.
            //
            // Also temporarily install the queued effect's
            // `dna_origin_context` onto `Game::current_dna_origin` so
            // conditions that branch on DNA-origin (e.g. shared
            // `[When Digivolving]` clauses with DNA-only rider arms)
            // see the same value the run-time path would. Mirrors the
            // `prev_dna_origin` save/restore in `run_queued_effect`.
            let prev_dna_origin = self.current_dna_origin;
            if dna_origin_context.is_some() {
                self.current_dna_origin = dna_origin_context;
            }
            let condition_passes =
                if let Some(effects) = self.effects_for_card(&card_id, source_card) {
                    if let Some(eff) = effects.get(effect_slot) {
                        if let Some(cond) = &eff.condition {
                            let trigger_guard = TriggerContextGuard::install(self, trigger_context);
                            let ctx = EffectContext::new_with_source_kind(
                                &mut *trigger_guard.game,
                                source_card,
                                source_permanent,
                                source_kind,
                                controller,
                            );
                            cond(&ctx.as_read())
                        } else {
                            true // no condition → keep
                        }
                    } else {
                        true // slot missing → keep, let run-time handle it
                    }
                } else {
                    true // effects missing → keep
                };
            self.current_dna_origin = prev_dna_origin;
            if !condition_passes {
                to_skip.push(i);
            }
        }
        to_skip
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
            if !self.modifiers.contains_granted_triggered_body_id(body_id) {
                self.granted_effect_bodies.remove(body_id);
            }
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

        // DCGO `CardEffectCommons.CanActivateOnDeletion` (OnDeletion.cs): the
        // [On Deletion] bundle activates only while the deleted carrier's
        // top-most card is still in its former controller's trash. This is a
        // CanActivate check — re-evaluated at activation, not at queue time —
        // so if an earlier effect (the deleting effect's own body, or an
        // earlier OnDeletion clause) moves the top card out of trash before
        // this clause resolves, the clause is suppressed. Tokens leave no card
        // in trash but always activate (`if (card.IsToken) return true;`).
        if qe.timing == EffectTiming::OnDeletion {
            if let Some(snap) = qe
                .trigger_context
                .as_ref()
                .and_then(|t| t.deleted_object.as_ref())
            {
                if !snap.is_token {
                    let top = snap.top_card;
                    let owner = snap.former_controller;
                    let in_trash = self.player(owner).trash.iter().any(|c| c.handle() == top);
                    if !in_trash {
                        return; // suppressed: top card no longer in trash
                    }
                }
            }
        }

        if effect.max_per_turn > 0 && !qe.bypass_once_per_turn {
            if let Some(perm_handle) = qe.source_permanent {
                let opt_key = Self::opt_slot_key(effect, qe.effect_slot);
                let Some(activation_count) =
                    self.source_permanent_activation_count(perm_handle, qe.source_card, opt_key)
                else {
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

        // Activation-cost hook — distinct from pay_cost_fn (Phase 2 Track B).
        // Runs on triggered-ability resolution between the condition gate
        // and the body process closure. Failure path:
        //   - body process does NOT run
        //   - OPT slot IS consumed for the same activation key, so a card
        //     that can't pay its cost this trigger doesn't get to retry on
        //     a fresh trigger event in the same turn (Working Rule 17 — no
        //     silent retries; Track C may reshape OPT keying in a follow-up
        //     but the semantic is "cost-failed firings count toward the OPT
        //     cap identically to successful firings").
        // Success path: the body MUST run even when the activation cost
        // legitimately removed the source permanent (e.g. "return this
        // Tamer to the bottom of the deck"). The standard source-liveness
        // re-check in `run_queued_effect_process_tail` would reject this
        // case, so we route through
        // `run_queued_effect_process_tail_after_activation_cost` which
        // keeps OPT bookkeeping but skips the source-is-live gate (the
        // cost just established that the trigger fired and the source
        // intentionally left).
        let mut activation_cost_paid = false;
        if effect.activation_cost_fn.is_some() {
            let max_per_turn = effect.max_per_turn;
            let opt_key = Self::opt_slot_key(effect, qe.effect_slot);
            // Re-lookup is necessary because invoking the closure needs
            // `&mut self`, which conflicts with the borrow into `effects`.
            let cost_outcome = {
                let Some(effects) = self.effects_for_card(&qe.card_id, qe.source_card) else {
                    return;
                };
                let Some(effect) = effects.get(qe.effect_slot as usize) else {
                    return;
                };
                let Some(activation_cost) = &effect.activation_cost_fn else {
                    return;
                };
                let mut ctx = EffectContext::new_with_source_kind(
                    self,
                    qe.attribution_source_card.unwrap_or(qe.source_card),
                    qe.source_permanent,
                    qe.attribution_source_kind.unwrap_or(qe.source_kind),
                    qe.controller,
                );
                activation_cost(&mut ctx)
            };
            if !cost_outcome {
                if max_per_turn > 0 && !qe.bypass_once_per_turn {
                    if let Some(perm_handle) = qe.source_permanent {
                        self.record_source_permanent_activation(
                            perm_handle,
                            qe.source_card,
                            opt_key,
                        );
                    }
                }
                return;
            }
            activation_cost_paid = true;
        }

        let event_delay_source = self.event_gated_delay_source(&qe);
        if activation_cost_paid {
            self.run_queued_effect_process_tail_after_activation_cost(&qe);
        } else {
            self.run_queued_effect_process_tail(&qe);
        }
        self.trash_event_gated_delay_after_activation(event_delay_source);
    }

    /// Variant of [`Self::run_queued_effect_process_tail`] used after a
    /// successful `activation_cost_fn`. Skips the source-liveness re-check
    /// because the cost may have intentionally removed the source (e.g.
    /// `return_self_to_deck_bottom_as_cost`); OPT bookkeeping is preserved
    /// so a successful firing counts toward the cap.
    fn run_queued_effect_process_tail_after_activation_cost(&mut self, qe: &QueuedEffect) {
        let Some(effects) = self.effects_for_card(&qe.card_id, qe.source_card) else {
            return;
        };
        let Some(effect) = effects.get(qe.effect_slot as usize) else {
            return;
        };
        let opt_key = Self::opt_slot_key(effect, qe.effect_slot);
        if effect.max_per_turn > 0 && !qe.bypass_once_per_turn {
            if let Some(perm_handle) = qe.source_permanent {
                let Some(activation_count) =
                    self.source_permanent_activation_count(perm_handle, qe.source_card, opt_key)
                else {
                    // Source removed by the cost — OPT bookkeeping not
                    // applicable, body still runs.
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
                    return;
                };
                if activation_count >= effect.max_per_turn {
                    return;
                }
                self.record_source_permanent_activation(perm_handle, qe.source_card, opt_key);
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

        let opt_key = Self::opt_slot_key(effect, qe.effect_slot);
        if effect.max_per_turn > 0 && !qe.bypass_once_per_turn {
            if let Some(perm_handle) = qe.source_permanent {
                let Some(activation_count) =
                    self.source_permanent_activation_count(perm_handle, qe.source_card, opt_key)
                else {
                    return;
                };
                if activation_count >= effect.max_per_turn {
                    return;
                }
                self.record_source_permanent_activation(perm_handle, qe.source_card, opt_key);
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
        // Layer-2 zombie-permanent guard. A transient zombie (e.g. a
        // carrier between `play_from_materials`'s `Pending` park and its
        // resume) trips `top_card()`'s panic; gracefully report no source.
        // Mirrors `enqueue_from_permanent` (effect_queue.rs ~line 1550) and
        // `queued_effect_source_is_live` (effect_queue.rs ~line 2444). See
        // `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
        // `qa/archetype-qa/engine-gaps.md`.
        if perm.card_sources.is_empty() {
            return None;
        }
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
            // Layer-2 zombie-permanent guard. The iter scans EVERY perm in
            // battle_area; a single zombie (empty `card_sources`) trips
            // `top_card()`'s panic. Skip the zombie and continue. Mirrors
            // `enqueue_from_permanent` and `queued_effect_source_is_live`.
            // See `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
            // `qa/archetype-qa/engine-gaps.md`.
            if perm.card_sources.is_empty() {
                continue;
            }
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
        // **Post-batched-deletion bypass (2026-05-23).** `OnDeletion`
        // entries enqueued by `delete_permanents_batch` carry the
        // `deleted_object` snapshot in their trigger context. The
        // batched flow trashes the carrier *before* the drain runs, so
        // the standard "is permanent still on field" check fails for
        // every batched OnDeletion entry. Bypass when the trigger
        // context proves this is a post-deletion fire: handler bodies
        // either read live state (gracefully bail when the carrier is
        // gone) or read the snapshot via `ctx.deleted_self_*()`.
        if qe.timing == EffectTiming::OnDeletion
            && qe
                .trigger_context
                .as_ref()
                .is_some_and(|t| t.deleted_object.is_some())
        {
            return true;
        }
        if perm_handle.index == BREEDING_TARGET as u8 {
            return self
                .players
                .get(perm_handle.player as usize)
                .and_then(|p| p.breeding_area.as_ref())
                .is_some_and(|perm| {
                    // Defensive: empty `card_sources` (zombie permanent) →
                    // no top match. See `G-PERMANENT-EMPTY-DURING-BATCH-DELETION`.
                    let top_matches = !qe.allow_below_top_liveness
                        && perm
                            .card_sources
                            .last()
                            .is_some_and(|c| c.card_index == qe.source_card.0);
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
        // A zombie permanent (`card_sources` empty) has no live source — bail.
        // Matches the same pattern in `top_card_handle` /
        // `enqueue_from_permanent`. See `G-PERMANENT-EMPTY-DURING-BATCH-DELETION`
        // for the original surfacing.
        let Some(perm_top) = perm.card_sources.last() else {
            return false;
        };
        // Also skip if the specific source card has been shuffled out of the
        // top-card slot (e.g. permanent digivolved mid-batch).
        // Phase 8 Task 4: a sideways-inherited effect's source_card is a card
        // in `linked_cards`, not the top card — accept either.
        // Phase 8 Task 5: a Training-sideways effect's source_card lives on a
        // different `OptionState::Training` permanent the same owner controls;
        // scan the owner's battle_area for it.
        let top_matches = !qe.allow_below_top_liveness && perm_top.card_index == qe.source_card.0;
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
                    // Defensive: skip zombie permanents in the Training scan.
                    let Some(pp_top) = pp.card_sources.last() else {
                        return false;
                    };
                    if pp_top.card_index != qe.source_card.0 {
                        return false;
                    }
                    let crate::permanent::OptionState::Training { trained, .. } = pp.option_state
                    else {
                        return false;
                    };
                    trained.map_or(true, |binding| {
                        binding.handle == perm_handle && perm_top.handle() == binding.top_card
                    })
                })
            })
            .unwrap_or(false);
        top_matches || linked_matches || inherited_source_matches || training_matches
    }

    /// Resolve the once-per-turn counter key for an effect. A multi-timing
    /// OPT cluster sets `Effect::shared_opt_group` so all of its timings draw
    /// on one counter; otherwise the per-slot `effect_slot` is the key.
    /// `G-OPT-MULTI-TIMING-SHARED-LOCKOUT`.
    fn opt_slot_key(effect: &crate::effect::Effect, effect_slot: u8) -> u8 {
        effect.shared_opt_group.unwrap_or(effect_slot)
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

    /// Refund one recorded activation — DCGO `ActivateClass.RemoveUse()`.
    /// Driven by the DSL `refund_opt` step when an optional once-per-turn
    /// body executed nothing (G-OPT-REFUND-ON-DECLINE).
    pub(crate) fn unrecord_source_permanent_activation(
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
                perm.unrecord_activation(source_card, effect_slot);
            }
            return;
        }
        if let Some(perm) = self
            .players
            .get_mut(handle.player as usize)
            .and_then(|p| p.battle_area.get_mut(handle.index as usize))
        {
            perm.unrecord_activation(source_card, effect_slot);
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
                    SecurityRemovalDestination::Deck { owner, to_bottom } => {
                        // Route to the owner's deck. Digi-Eggs go to the
                        // digitama deck. Deck top = Vec end (drawn first);
                        // deck bottom = index 0 — matching `move_card_to_deck`.
                        let is_egg = security.card.card_kind(&self.card_data)
                            == crate::enums::CardKind::DigiEgg;
                        let player = self.player_mut(owner);
                        let deck = if is_egg {
                            &mut player.digitama_deck
                        } else {
                            &mut player.deck
                        };
                        if to_bottom {
                            deck.insert(0, security.card);
                        } else {
                            deck.push(security.card);
                        }
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
    /// True when a lone OPTIONAL queued effect should install an explicit
    /// outer accept/decline prompt (`G-OUTER-OPTIONAL-NOT-INSTALLED`). Two
    /// conditions must both hold: the DSL lowering flagged this effect with
    /// `needs_outer_optional_prompt` (its body's first step is mandatory),
    /// AND the effect would actually do something if run — i.e. its source is
    /// live, its OPT counter is not exhausted, and its `condition` passes.
    /// An optional effect whose preconditions already fail must not prompt.
    fn queued_effect_wants_outer_optional_prompt(&mut self, qe: &QueuedEffect) -> bool {
        // Granted inline bodies have no Effect metadata and never carry the
        // outer-prompt flag — never prompt for them.
        if qe.granted_effect_id.is_some() {
            return false;
        }
        let Some(effects) = self.effects_for_card(&qe.card_id, qe.source_card) else {
            return false;
        };
        let Some(effect) = effects.get(qe.effect_slot as usize) else {
            return false;
        };
        if !effect.needs_outer_optional_prompt {
            return false;
        }
        if !self.queued_effect_source_is_live(qe, effect) {
            return false;
        }
        if effect.max_per_turn > 0 && !qe.bypass_once_per_turn {
            if let Some(perm_handle) = qe.source_permanent {
                let opt_key = Self::opt_slot_key(effect, qe.effect_slot);
                match self.source_permanent_activation_count(perm_handle, qe.source_card, opt_key) {
                    Some(count) if count >= effect.max_per_turn => return false,
                    None => return false,
                    _ => {}
                }
            }
        }
        // SecuritySkill effects skip the condition gate (Python parity, see
        // `run_queued_effect_inner`). For every other timing, an unsatisfied
        // condition means the body would no-op — do not prompt.
        let attribution_source_card = qe.attribution_source_card.unwrap_or(qe.source_card);
        let attribution_source_kind = qe.attribution_source_kind.unwrap_or(qe.source_kind);
        // Install the queued effect's trigger context BEFORE evaluating the
        // condition / outer_optional_guard closures. DSL predicates like
        // `event_target_owner`, `event_target_kind`, `event_card_color_has`,
        // and deleted-object snapshots read `current_trigger_context`. The
        // real evaluation path (`run_queued_effect` → `run_queued_effect_inner`)
        // sets it before the body's condition gate; if the outer-prompt
        // decision uses the ambient (likely `None` or stale) context, the
        // condition fails here even though it would pass at body run time,
        // and the optional prompt is silently skipped — the body then
        // auto-fires inside `run_queued_effect`. The RAII guard restores
        // the previous value even on panic. See
        // `G-OUTER-OPTIONAL-TRIGGER-CTX` (proposal
        // `fix-outer-optional-prompt-trigger-ctx`).
        let guard = TriggerContextGuard::install(self, qe.trigger_context.clone());
        let rctx = EffectReadContext::new_with_source_kind(
            &*guard.game,
            attribution_source_card,
            qe.source_permanent,
            attribution_source_kind,
            qe.controller,
        );
        if qe.timing != EffectTiming::SecuritySkill {
            if let Some(cond) = &effect.condition {
                if !cond(&rctx) {
                    return false;
                }
            }
        }
        // Body-actionability guard: when the body's first step is a
        // selection, only prompt if it has at least one candidate — DCGO
        // does not prompt for an optional ability with no legal target.
        // Must see the same trigger context as the condition above so a
        // guard predicate that itself references event_* fields evaluates
        // consistently.
        if let Some(outer_guard) = &effect.outer_optional_guard {
            if !outer_guard(&rctx) {
                return false;
            }
        }
        true
    }

    /// Install an outer accept/decline `PendingSelection` for a single
    /// OPTIONAL triggered effect. ACCEPT runs the queued effect's body;
    /// PASS/decline skips it cleanly. `G-OUTER-OPTIONAL-NOT-INSTALLED`.
    fn install_outer_optional_trigger_selection(&mut self, qe: QueuedEffect) {
        let chooser = qe.controller;
        let source_card = qe.source_card;
        let source_permanent = qe.source_permanent;
        let source_kind = qe.source_kind;
        let prompt = format!("You may activate {}'s triggered effect", qe.card_id);
        let qe_for_resume = qe.clone();

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;

        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::Replacement,
            selecting_player: chooser,
            previous_phase,
            valid_action_ids: vec![REPLACEMENT_ACCEPT],
            is_optional: true,
            prompt,
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            // ACCEPT: the parked QueuedEffect is moved into this `FnOnce`
            // and its body runs. The generic resolver drains the queue after.
            callback: Box::new(move |game: &mut Game, _action_id: u16| {
                game.run_queued_effect(qe);
            }),
            // DECLINE: do nothing — the QueuedEffect (already removed from
            // `effect_queue`) is simply dropped; its body never runs. The
            // generic resolver resumes draining the rest of the queue.
            on_decline: Some(Box::new(|_game: &mut Game| {})),
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::OuterOptionalTrigger(
                crate::resume::OuterOptionalTriggerState {
                    queued_effect: qe_for_resume,
                    outer_conts: Vec::new(),
                },
            )],
        });
    }

    pub(crate) fn run_outer_optional_trigger_step(
        &mut self,
        state: crate::resume::OuterOptionalTriggerState,
        is_pass: bool,
    ) {
        if !is_pass {
            self.run_queued_effect(state.queued_effect);
        }
    }

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
            zone_owner: None,
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
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::TriggerOrderSelection(
                crate::resume::TriggerOrderSelectionState {
                    chooser,
                    allow_decline_all,
                    outer_conts: Vec::new(),
                },
            )],
        });
    }

    pub(crate) fn run_trigger_order_selection_step(
        &mut self,
        state: crate::resume::TriggerOrderSelectionState,
        action_id: u16,
        is_pass: bool,
    ) {
        if is_pass {
            if state.allow_decline_all {
                self.effect_queue
                    .retain(|qe| !(qe.controller == state.chooser && qe.is_optional));
            }
            return;
        }

        let pos = action_id.saturating_sub(HAND_EFFECT_START) as usize;
        let target_idx = self
            .effect_queue
            .iter()
            .enumerate()
            .filter(|(_, qe)| qe.controller == state.chooser)
            .nth(pos)
            .map(|(i, _)| i);
        if let Some(idx) = target_idx {
            if let Some(qe) = self.effect_queue.remove(idx) {
                self.run_queued_effect(qe);
            }
        }
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
        let was_attack_pending = self.pending_attack.is_some();

        // Take the selection, restore phase, invoke the appropriate callback.
        let sel = self.pending_selection.take().expect("checked Some above");
        let resume = self.pending_selection_resume.take();
        self.current_phase = sel.previous_phase;
        // Wrap the callback in a deferred-drain scope (post-2026-05-23
        // G-DSL-OUTER-TAIL-NESTED-PARK fix). While the callback runs,
        // `fire_on_*` observer helpers go through `maybe_drain_effect_queue`
        // and only enqueue; the exit below flushes any accumulated queue.
        // Matches DCGO's pattern of deferring trigger drains until after
        // the resolving coroutine returns to its caller.
        self.enter_deferred_drain();
        if let Some(stack) = resume {
            // Coexistence switch (make-engine-cloneable Phase 2): a card ported
            // to the resumable VM carries its continuation as data — run it
            // instead of the legacy `sel.callback`. The abort-flag scoping
            // mirrors the on_decline path below.
            let prev_aborted = std::mem::replace(&mut self.dsl_clause_aborted, false);
            crate::dsl_cards::step::selections::run_resume(self, stack, action_id, is_pass);
            self.dsl_clause_aborted = prev_aborted;
            // Run continuations that wrappers (play-cost / reducer /
            // digixros-leave / partition-second-play) deferred because this
            // selection was resume-driven — their closure was bypassed by
            // run_resume. Each hook is plain data (`AfterSelectionHook`);
            // a hook that installs another resume-driven selection re-arms by
            // pushing onto the (now-drained) channel for the next resolution.
            let hooks = std::mem::take(&mut self.after_selection_resume_hooks.0);
            for hook in hooks {
                self.run_after_selection_hook(hook);
            }
        } else if is_pass {
            if let Some(on_decline) = sel.on_decline {
                // Scope the cost-pay abort flag to this on_decline. Save
                // and clear on entry, restore on exit — so a NESTED decline
                // (rare: inner clause's decline fires from inside an outer
                // accept callback) doesn't leak its abort state up to the
                // outer continuation, and a stale flag from a prior decline
                // cannot suppress this one. The flag itself is set by the
                // install_select_* on_decline closures (cost-pay aborts) and
                // checked by the DSL step runner.
                let prev_aborted = std::mem::replace(&mut self.dsl_clause_aborted, false);
                on_decline(self);
                self.dsl_clause_aborted = prev_aborted;
            }
        } else {
            (sel.callback)(self, action_id);
        }
        self.exit_deferred_drain_and_flush();

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
        if self.pending_selection.is_none() {
            self.resume_pending_end_turn();
        }
        // Some attack-time selections, especially optional pre-declare
        // replacements, park `pending_attack` before declaration commits. Once
        // the selection callback and any nested drains settle, resume the
        // attack state machine so callers using the normal action decoder do
        // not get a Main-phase mask while an attack is still pending.
        if self.pending_selection.is_none() && self.pending_attack.is_some() {
            self.advance_pending_attack();
        }
        if self.pending_selection.is_none()
            && self.current_phase == crate::enums::GamePhase::Main
            && !was_attack_pending
        {
            self.check_turn_end();
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
