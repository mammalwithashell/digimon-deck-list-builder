//! Option-card play operations (Tier 2) — `impl Game`.

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
    /// Play an Option card from `player`'s hand.
    ///
    /// Pipeline:
    /// 1. Validate phase / hand index / card kind / color match.
    /// 2. Resolve the play mode — a dual-mode Plug-In Option (Standard
    ///    `[Main]` + Link) surfaces a mode-select prompt first.
    /// 3. Compute + pay the mode's cost (honors BeforePayCost reductions
    ///    for Standard; the flat link cost for Link).
    /// 4. Move card out of hand into `pending_option`.
    /// 5. Fire `OnUseOption`; for non-Link modes also fire the `OptionMain`
    ///    body. Drain the queue.
    /// 6. If a `PendingSelection` parked, return `Pending` — the caller
    ///    drives the selection; `dispose_option` re-enters via the
    ///    post-resolution path once it resolves.
    /// 7. Otherwise dispose per the resolved subtype and `check_turn_end`.
    pub fn play_option_from_hand(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
    ) -> OptionPlayResult {
        self.play_option_core(
            player_id,
            OptionSource::Hand(hand_index),
            None,
            OptionCostPolicy::Pay,
        )
    }

    /// Use an Option from hand without paying its use cost while preserving
    /// the normal Option lifecycle (OnUseOption, OptionMain/mode selection,
    /// and subtype disposal). Intended for card effects such as BT24-085.
    pub fn use_option_from_hand_without_paying_cost(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
    ) -> OptionPlayResult {
        self.play_option_core(
            player_id,
            OptionSource::Hand(hand_index),
            None,
            OptionCostPolicy::Free,
        )
    }

    /// Play an Option card from `player`'s trash (effect-driven).
    ///
    /// Same pipeline as `play_option_from_hand`, sourced from the trash zone.
    /// No `PlaySource`-gated flood gates apply to Options in v1 (Phase 6
    /// `CannotPlayDigimonByEffect` is Digimon-only).
    pub fn play_option_from_trash(
        &mut self,
        player_id: PlayerId,
        trash_index: usize,
    ) -> OptionPlayResult {
        if self
            .modifiers
            .player_has(player_id, ModifierType::CannotPlayFromTrash)
        {
            return OptionPlayResult::Invalid;
        }
        self.play_option_core(
            player_id,
            OptionSource::Trash(trash_index),
            None,
            OptionCostPolicy::Pay,
        )
    }

    /// Shared Option-play pipeline. Forks on source zone (hand vs trash)
    /// and on the resolved play mode (Standard / Delay / Link / Training).
    ///
    /// `chosen_mode` is `None` for a fresh play; for a dual-mode Plug-In
    /// Option the first call installs a mode-select `pending_selection` and
    /// returns `Pending`, and the selection callback re-enters with the
    /// chosen mode as `Some(_)`.
    pub(crate) fn play_option_core(
        &mut self,
        player_id: PlayerId,
        source: OptionSource,
        chosen_mode: Option<OptionPlayMode>,
        cost_policy: OptionCostPolicy,
    ) -> OptionPlayResult {
        // Always-fire (not gated on debug_assertions): re-entering
        // play_option_core with `pending_option` still set means the
        // single-occupancy slot is about to be overwritten, which
        // silently corrupts the prior in-flight Option's resolution.
        // Better to fail loudly so the crash recorder names both cards;
        // the wrapper converts the panic into a terminal step.
        {
            if let Some(pending) = self.pending_option.as_ref() {
                let pending_card_id = pending.card.card_id(&self.card_data).to_string();
                let incoming_card_id = match source {
                    OptionSource::Hand(i) => self
                        .player(player_id)
                        .hand
                        .get(i)
                        .map(|c| c.card_id(&self.card_data).to_string())
                        .unwrap_or_else(|| format!("hand[{}]:oob", i)),
                    OptionSource::Trash(i) => self
                        .player(player_id)
                        .trash
                        .get(i)
                        .map(|c| c.card_id(&self.card_data).to_string())
                        .unwrap_or_else(|| format!("trash[{}]:oob", i)),
                };
                panic!(
                    "reentrant Option play while another is mid-resolution: \
                     player={:?} incoming_card={} from_source={:?} \
                     in_flight_card={} in_flight_resolution_phase={:?} \
                     in_counter_window={}",
                    player_id,
                    incoming_card_id,
                    source,
                    pending_card_id,
                    pending.resolution_phase,
                    self.in_counter_window,
                );
            }
        }

        // 1. Phase gate. Counter-window Option plays bypass the Main-phase
        // gate — they fire during the defender's Counter window, which
        // can be any phase the turn player attacked from. Spec §5.2.
        if !self.in_counter_window && self.current_phase != GamePhase::Main {
            return OptionPlayResult::Invalid;
        }

        // 2. Source validation + Option kind + color match.
        let source_kind = source.use_source();
        let (card_handle, printed_cost, card_id) = {
            let player = self.player(player_id);
            let card = match source {
                OptionSource::Hand(i) => {
                    if i >= player.hand.len() {
                        return OptionPlayResult::Invalid;
                    }
                    &player.hand[i]
                }
                OptionSource::Trash(i) => {
                    if i >= player.trash.len() {
                        return OptionPlayResult::Invalid;
                    }
                    &player.trash[i]
                }
            };
            if card.card_kind(&self.card_data) != CardKind::Option
                && card.card_kind(&self.card_data) != CardKind::Dual
            {
                return OptionPlayResult::Invalid;
            }
            if self.in_counter_window {
                if !crate::action::mask::option_has_active_counter_effect(card, self, player_id) {
                    return OptionPlayResult::Invalid;
                }
            } else {
                let authored_effects = self
                    .effects_for_card(card.card_id(&self.card_data), card.handle())
                    .unwrap_or_default();
                if !authored_effects.is_empty()
                    && !crate::action::mask::option_has_active_main_effect(card, self, player_id)
                {
                    return OptionPlayResult::Invalid;
                }
            }
            if cost_policy == OptionCostPolicy::Pay
                && !crate::action::mask::option_use_requirement_or_color_available(
                    card, self, player_id,
                )
            {
                return OptionPlayResult::Invalid;
            }
            (
                card.handle(),
                card.option_use_cost(&self.card_data)
                    .unwrap_or_else(|| card.play_cost(&self.card_data)),
                card.card_id(&self.card_data).to_string(),
            )
        };

        // 3. Resolve the play mode. A dual-mode Plug-In Option (both a
        // Standard `[Main]` Option and a Link Option) surfaces a
        // mode-select prompt; its callback re-enters here with the chosen
        // mode. Cost, `OptionMain` firing, and disposal all fork on it.
        let mode = match chosen_mode {
            Some(mode) => mode,
            // Counter-window plays are always Standard counter Options;
            // dual-mode Plug-Ins are never counter Options.
            None if self.in_counter_window => OptionPlayMode::Standard,
            None => {
                let legal_modes = {
                    let player = self.player(player_id);
                    let card = match source {
                        OptionSource::Hand(i) => &player.hand[i],
                        OptionSource::Trash(i) => &player.trash[i],
                    };
                    self.option_legal_play_modes(card, player_id)
                };
                match legal_modes.as_slice() {
                    [] => return OptionPlayResult::Invalid,
                    [single] => *single,
                    _ => {
                        // Dual-mode: park a mode-select; the callback
                        // re-enters with the chosen mode.
                        self.install_option_mode_select(
                            player_id,
                            source,
                            card_handle,
                            legal_modes,
                            cost_policy,
                        );
                        return OptionPlayResult::Pending;
                    }
                }
            }
        };

        // 4. Compute + pay cost (Phase 5 BeforePayCost hooks). For a
        // dual-mode card the first call returns at the mode-select above,
        // so the BeforePayCost scan runs exactly once — on the re-entry
        // with the chosen mode.
        // Pass the option's hand/trash handle as the cost-target so target-
        // aware predicates and observers can fire — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET.
        let cost_target_ctx = CostTargetContext {
            card: card_handle,
            from_hand: matches!(source, OptionSource::Hand(_)),
            is_digivolve: false,
            target_permanents: [None, None],
        };
        let total_reduction = self.scan_before_pay_cost_reduction_with_target(
            player_id,
            CostReductionKind::OptionUse,
            Some(cost_target_ctx),
        );
        // Observer dispatch — G-BEFORE-PAY-COST-GAIN-MEMORY.
        self.scan_before_pay_cost_observers(player_id, Some(cost_target_ctx));
        let effective_cost = match mode {
            // Link Requirements: pay exactly the link cost (plus any
            // `ChangeLinkCost` modifier delta). The printed Option use cost
            // and BeforePayCost `OptionUse` reductions do not apply when
            // plugging the card in via Link Requirements.
            OptionPlayMode::Link { cost } => {
                (cost as i32 + self.modifiers.link_cost_delta_for_player(player_id)).max(0) as u16
            }
            // Standard / Delay / Training: effect-driven "without paying"
            // uses preserve the lifecycle but bypass the use-cost payment.
            _ if cost_policy == OptionCostPolicy::Free => 0,
            // Standard / Delay / Training: ordinary play pays the printed
            // use cost, less any BeforePayCost reduction.
            _ => ((printed_cost as i32) - total_reduction).max(0) as u16,
        };
        if !self.pay_memory(effective_cost) {
            return OptionPlayResult::Invalid;
        }

        // 5. Remove from source zone, install PendingOption with the
        // resolved subtype (so `dispose_option` need not re-classify).
        let card = match source {
            OptionSource::Hand(i) => self.player_mut(player_id).hand.remove(i),
            OptionSource::Trash(i) => self.player_mut(player_id).trash.remove(i),
        };
        self.pending_option = Some(PendingOption {
            owner: player_id,
            card,
            source_kind,
            resolution_phase: OptionResolutionPhase::MainEffectDrain,
            subtype: mode.subtype(),
        });

        // 6. Fire OnUseOption (global observer across every battle area) +
        // OptionMain (this card's body). Drain between — OnUseOption fires
        // first per spec §4 (global observer fires before body).
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnUseOption,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }

        // Phase 9 Task 3 — Counter-window overlay: when this Option is
        // being played as a defender's counter, the `.counter()` +
        // CounterEffect-timing body fires BEFORE OptionMain. Drain each
        // bundle separately so the two fire in order (Counter then Main)
        // without prompting the defender with a synthetic TriggerOrder
        // choice. Spec §5.2.
        if self.in_counter_window {
            self.enqueue_counter_effect_from_pending(&card_id, card_handle, player_id);
            self.drain_effect_queue();
            // If the counter body parked a selection, yield — the
            // resumption path re-enters via `advance_pending_option`
            // which will fire OptionMain after the body's selection
            // resolves.
            if self.pending_selection.is_some() {
                return OptionPlayResult::Pending;
            }
        }

        // Fire the `OptionMain` body for the resolved mode. A Standard /
        // Delay / Training play runs the non-link `[Main]` body; a Link play
        // runs only the link-declaration effect (which may itself carry a
        // body — e.g. a Link Option whose `.link(..).process(..)` does the
        // plug-in's work). This split is what keeps a dual-mode Plug-In's
        // Standard `[Main]` body from firing on a Link play, and vice versa.
        self.enqueue_option_main_from_pending(&card_id, card_handle, player_id, mode.is_link());
        self.drain_effect_queue();

        // 7. If an effect parked a selection, suspend and let the caller drive.
        if self.pending_selection.is_some() {
            return OptionPlayResult::Pending;
        }

        if self.pending_option_can_arts_digivolve() && self.install_arts_digivolve_selection() {
            return OptionPlayResult::Pending;
        }

        // 8. Dispose per subtype (Standard → trash; Delay → park on field;
        // Link → install host-selection). `dispose_option` may install a
        // PendingSelection (Link flow); if so, return Pending and defer
        // check_turn_end until `attach_linked_card` finishes the attach.
        self.dispose_option();
        if self.pending_selection.is_some() {
            return OptionPlayResult::Pending;
        }
        self.check_turn_end();
        OptionPlayResult::Trashed
    }

    /// Install the dual-mode mode-select prompt for a Plug-In Option that
    /// is both a Standard `[Main]` Option and a Link Option. `legal_modes`
    /// holds the modes the player may legally choose right now — this is
    /// reached only when there are two (a single legal mode plays
    /// directly). The selection callback re-enters `play_option_core` with
    /// the chosen mode. The mode-select surfaces as a normal
    /// `pending_selection`, so every legal play mode is exposed to the
    /// action space (no-approximations policy).
    pub(crate) fn install_option_mode_select(
        &mut self,
        player_id: PlayerId,
        source: OptionSource,
        card_handle: crate::card_source::CardHandle,
        legal_modes: Vec<OptionPlayMode>,
        cost_policy: OptionCostPolicy,
    ) {
        use crate::action::space::HAND_EFFECT_START;

        let mut valid_action_ids: Vec<u16> = Vec::with_capacity(legal_modes.len());
        let mut choices: Vec<crate::selection::EffectChoiceEntry> =
            Vec::with_capacity(legal_modes.len());
        for (i, mode) in legal_modes.iter().enumerate() {
            let action_id = HAND_EFFECT_START + i as u16;
            valid_action_ids.push(action_id);
            let label = match mode {
                OptionPlayMode::Link { cost } => {
                    format!("Plug in via Link Requirements (Cost {cost})")
                }
                _ => "Play as a [Main] Option".to_string(),
            };
            choices.push(crate::selection::EffectChoiceEntry {
                label,
                action_id,
                source_card: Some(card_handle),
                source_kind: Some(EffectSourceKind::Option),
                timing: None,
                is_optional: false,
                observation_metadata: Default::default(),
            });
        }

        let modes = legal_modes;
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;
        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::EffectChoice,
            selecting_player: player_id,
            previous_phase,
            valid_action_ids,
            is_optional: false,
            prompt: "Choose how to play this Plug-In Option".to_string(),
            effect_choices: Some(choices),
            source_card: card_handle,
            source_permanent: None,
            source_kind: EffectSourceKind::Option,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let index = action_id.saturating_sub(HAND_EFFECT_START) as usize;
                let mode = modes
                    .get(index)
                    .copied()
                    .unwrap_or(OptionPlayMode::Standard);
                let _ = game.play_option_core(player_id, source, Some(mode), cost_policy);
            }),
            on_decline: None,
        });
    }

    /// Enqueue every `OptionMain` effect declared by `card_id` directly
    /// against the in-flight `pending_option` card. Options aren't on the
    /// battle area, so `TriggerSource::PlayerBattleArea` / `Permanent` can't
    /// find them — we push directly into `effect_queue` with the card handle
    /// and a `None` source permanent.
    pub(crate) fn enqueue_option_main_from_pending(
        &mut self,
        card_id: &str,
        card_handle: crate::card_source::CardHandle,
        owner: PlayerId,
        link_mode: bool,
    ) {
        let Some(effects) = self.effects_for_card(card_id, card_handle) else {
            return;
        };
        let tp = self.turn_player();
        let is_turn_player = owner == tp;
        for (slot, effect) in effects.iter().enumerate() {
            if effect.timing != EffectTiming::OptionMain {
                continue;
            }
            // Fork on the resolved play mode. A link-declaration effect
            // (`link_cost.is_some()` — DSL `link_requirement` or a
            // hand-written `.link(..).process(..)` Link Option) belongs to
            // the Link play; a non-link `OptionMain` effect is the Standard
            // `[Main]` body. Enqueuing only the matching set keeps a
            // dual-mode Plug-In's two bodies from cross-firing — and avoids
            // a spurious `TriggerOrder` prompt between them.
            if effect.link_cost.is_some() != link_mode {
                continue;
            }
            self.effect_queue.push_back(QueuedEffect {
                source_card: card_handle,
                source_permanent: None,
                source_kind: EffectSourceKind::Option,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                controller: owner,
                timing: EffectTiming::OptionMain,
                trigger_context: None,
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.to_string(),
                allow_below_top_liveness: false,
                dna_origin_context: self.current_dna_origin,
                granted_effect_id: None,
            });
        }
    }

    pub(crate) fn fire_on_link_after_option_placed(&mut self) {
        // Fire OnLink globally - every player's battle area scans for
        // OnLink-timed effects. Load-bearing for Appmon-trait cards.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnLink,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        // `maybe_drain` defers when inside a select-callback or outer-tail
        // scope (post-2026-05-23 G-DSL-OUTER-TAIL-NESTED-PARK fix). The
        // scope's exit hook flushes the queue at a safe checkpoint.
        // Behavior is unchanged at top-level callers (counter is 0).
        self.maybe_drain_effect_queue();
        if self.pending_selection.is_some() {
            self.pending_option_placed_turn_check = true;
            return;
        }

        // Link lifecycle complete — check if memory state demands turn transition.
        // The Standard Option path hits this via `advance_pending_option`; the
        // Link path bypasses that dispatcher (host-select callback calls this
        // directly), so we must invoke `check_turn_end` ourselves.
        self.check_turn_end();
    }
}
