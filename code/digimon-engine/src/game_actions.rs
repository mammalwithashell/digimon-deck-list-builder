//! Player-driven game actions — split out of `game.rs` for readability.
//!
//! Everything here lives in `impl Game` blocks so the call surface is unchanged.
//! This is where `play_from_hand`, `digivolve_from_hand`, `initiate_dna_digivolve`,
//! and the `activate_*_main` [Main] effect dispatchers live. All three are invoked
//! by the action decoder and the Tauri/PyO3 bindings; none of them move here.

use crate::card_source::CardSource;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{
    CardKind, EffectSourceKind, EffectTiming, GamePhase, Keyword, ModifierType, PlaySource,
    PlayerId,
};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::selection::{
    OptionPlayResult, OptionResolutionPhase, OptionUseSource, PendingOption, PendingSelection,
    QueuedEffect, SelectionKind, TriggerSource,
};
use rand::seq::SliceRandom;

/// Source zone for `play_option_core`. Private to this module — the public
/// API is the pair of `play_option_from_hand` / `play_option_from_trash`
/// entry points.
#[derive(Clone, Copy)]
enum OptionSource {
    Hand(usize),
    Trash(usize),
}

impl OptionSource {
    fn use_source(self) -> OptionUseSource {
        match self {
            OptionSource::Hand(_) => OptionUseSource::Hand,
            OptionSource::Trash(_) => OptionUseSource::Trash,
        }
    }
}

/// Phase 8 Option subtype, inferred from effect flags. First-match-wins
/// inside `classify_option_subtype` — printed cards carry at most one
/// subtype per Option, so the ordering (Delay → Training → Link →
/// Standard) is rule-consistent but doesn't affect conforming data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionSubtype {
    Standard,
    Delay(crate::enums::DelayTrigger),
    Link,
    Training,
}

fn source_kind_for_card_kind(kind: CardKind) -> EffectSourceKind {
    match kind {
        CardKind::Digimon | CardKind::DigiEgg | CardKind::Dual => EffectSourceKind::Digimon,
        CardKind::Tamer => EffectSourceKind::Tamer,
        CardKind::Option => EffectSourceKind::Option,
        CardKind::Token => EffectSourceKind::Rule,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CostReductionKey {
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<crate::permanent::PermanentHandle>,
    controller: PlayerId,
    card_id: String,
    effect_slot: u8,
    is_under: bool,
}

struct CostReductionCandidate {
    key: CostReductionKey,
    label: String,
    amount: i32,
    optional: bool,
    has_pay_cost: bool,
}

struct BeforePayCostSourceInfo {
    source_permanent: Option<crate::permanent::PermanentHandle>,
    source_card: crate::card_source::CardHandle,
    card_id: String,
    is_under: bool,
    controller: PlayerId,
    effect_slot: u8,
}

#[derive(Debug, Clone, Copy)]
struct CostTargetContext {
    card: crate::card_source::CardHandle,
    from_hand: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayFromHandCostResult {
    Played(usize),
    Pending,
    Failed,
}

impl PlayFromHandCostResult {
    fn into_option(self) -> Option<usize> {
        match self {
            PlayFromHandCostResult::Played(index) => Some(index),
            PlayFromHandCostResult::Pending | PlayFromHandCostResult::Failed => None,
        }
    }
}

/// Inspect an Option's effect list to decide its play-path subtype.
/// Iterates effects; the first one carrying a subtype flag wins.
fn classify_option_subtype(effects: &[crate::effect::Effect]) -> OptionSubtype {
    for eff in effects {
        if let Some(trigger) = eff.delay_trigger {
            return OptionSubtype::Delay(trigger);
        }
        if eff.training {
            return OptionSubtype::Training;
        }
        if eff.link_cost.is_some() {
            return OptionSubtype::Link;
        }
    }
    OptionSubtype::Standard
}

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
        let player = self.player_mut(player_id);
        if player.battle_area.len() >= field_slots as usize {
            return false;
        }
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

            // Phase 8 Task 5: trash every Training permanent the owner
            // controls. Collect handles, then process in reverse so each
            // delete doesn't invalidate the indices of later ones.
            let training_handles: Vec<PermanentHandle> = self
                .player(player_id)
                .battle_area
                .iter()
                .enumerate()
                .filter_map(|(i, perm)| {
                    if let crate::permanent::OptionState::Training { owner } = perm.option_state {
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

    /// Play a card from hand to field for a player, paying the printed cost.
    ///
    /// Delegates to [`Self::play_from_hand_with_cost`] with
    /// `CostDelta::Reduce(0)` (pay the printed cost verbatim) and
    /// `PlaySource::ByHand` (standard player-action play).
    ///
    /// Does NOT call `check_turn_end`. Callers that want to end the turn when
    /// memory goes negative after OnPlay effects resolve should invoke
    /// `check_turn_end` explicitly.
    pub fn play_from_hand(&mut self, player_id: PlayerId, hand_index: usize) -> Option<usize> {
        self.play_from_hand_with_cost(
            player_id,
            hand_index,
            crate::enums::CostDelta::Reduce(0),
            PlaySource::ByHand,
        )
    }

    /// Generalization of `play_from_hand` — computes memory cost via the given
    /// `CostDelta` and plays the card. The caller's `CostDelta::Reduce(0)` is
    /// equivalent to paying the printed cost.
    ///
    /// Flow (matches Python):
    /// 1. Validate hand index and field capacity.
    /// 2. Read the card's printed play cost from `card_data`.
    /// 3. Apply `cost_delta.resolve(printed_cost)` to get the effective cost.
    /// 4. Call `pay_memory(effective_cost)`; if unaffordable, abort with `None`
    ///    and leave state unchanged.
    /// 5. Remove the card from hand, create a Permanent on the field.
    /// 6. Fire `OnPlay` effects via the registry.
    ///
    /// Returns `Some(field_index)` on success, `None` if the hand index is
    /// invalid, the battle area is full, or memory is insufficient.
    ///
    /// Does NOT call `check_turn_end`. Callers that want to end the turn when
    /// memory goes negative after OnPlay effects resolve should invoke
    /// `check_turn_end` explicitly.
    pub fn play_from_hand_with_cost(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
    ) -> Option<usize> {
        self.play_from_hand_with_cost_result(player_id, hand_index, cost_delta, source, true)
            .into_option()
    }

    pub(crate) fn play_from_hand_with_cost_result(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        cost_target_from_hand: bool,
    ) -> PlayFromHandCostResult {
        let field_slots = self.rules.field_slots;

        // Borrow-check-friendly pre-checks: gather everything we need from
        // immutable borrows before taking a mutable borrow.
        let card_kind = {
            let player = self.player(player_id);
            if hand_index >= player.hand.len() {
                return PlayFromHandCostResult::Failed;
            }
            if player.battle_area.len() >= field_slots as usize {
                return PlayFromHandCostResult::Failed;
            }
            let card = &player.hand[hand_index];
            card.card_kind(&self.card_data)
        };

        // Phase 6: CannotPlayDigimonByEffect — when source is ByEffect and the
        // card is a Digimon, gate on the player-scoped modifier.
        if source == PlaySource::ByEffect
            && card_kind == CardKind::Digimon
            && self
                .modifiers
                .player_has(player_id, ModifierType::CannotPlayDigimonByEffect)
        {
            return PlayFromHandCostResult::Failed;
        }

        // Phase 6: CannotPlayTamerByEffect — when source is ByEffect and the
        // card is a Tamer, gate on the player-scoped modifier.
        if source == PlaySource::ByEffect
            && card_kind == CardKind::Tamer
            && self
                .modifiers
                .player_has(player_id, ModifierType::CannotPlayTamerByEffect)
        {
            return PlayFromHandCostResult::Failed;
        }

        let target_card = self.player(player_id).hand[hand_index].handle();
        self.continue_play_from_hand_cost_reduction_chain(
            player_id,
            hand_index,
            CostTargetContext {
                card: target_card,
                from_hand: cost_target_from_hand,
            },
            cost_delta,
            source,
            0,
            Vec::new(),
        )
    }

    fn continue_play_from_hand_cost_reduction_chain(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: CostTargetContext,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        mut accumulated_reduction: i32,
        mut processed: Vec<CostReductionKey>,
    ) -> PlayFromHandCostResult {
        loop {
            let candidates =
                self.collect_before_pay_cost_reducers(player_id, Some(target), &processed);
            let Some(candidate) = candidates.into_iter().next() else {
                return self.finish_play_from_hand_after_reductions(
                    player_id,
                    hand_index,
                    target.card,
                    cost_delta,
                    source,
                    accumulated_reduction,
                );
            };

            if !candidate.optional && !candidate.has_pay_cost {
                let key = candidate.key.clone();
                if let Some(amount) = self.apply_cost_reduction_candidate(&key, target) {
                    accumulated_reduction += amount;
                }
                processed.push(key);
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
                        accumulated_reduction,
                        processed,
                    );
                }) as crate::selection::DeclineCallback
            });
            let previous_phase = self.current_phase;
            self.current_phase = GamePhase::EffectChoice;
            self.pending_selection = Some(PendingSelection {
                kind: SelectionKind::EffectChoice,
                selecting_player: player_id,
                previous_phase,
                valid_action_ids: vec![crate::action::space::HAND_EFFECT_START],
                is_optional: candidate.optional,
                prompt: format!("Use {} to reduce play cost?", candidate.label),
                effect_choices: Some(vec![crate::selection::EffectChoiceEntry {
                    label: format!("{} (-{})", candidate.label, candidate.amount),
                    action_id: crate::action::space::HAND_EFFECT_START,
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
                    let _ = game.continue_play_from_hand_cost_reduction_chain(
                        player_id, hand_index, target, cost_delta, source, reduction, processed,
                    );
                }),
                on_decline,
            });
            return PlayFromHandCostResult::Pending;
        }
    }

    fn finish_play_from_hand_after_reductions(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target_card: crate::card_source::CardHandle,
        cost_delta: crate::enums::CostDelta,
        _source: PlaySource,
        total_reduction: i32,
    ) -> PlayFromHandCostResult {
        let turn = self.turn_count;
        let field_slots = self.rules.field_slots;
        let printed_cost = {
            let player = self.player(player_id);
            if player.battle_area.len() >= field_slots as usize {
                return PlayFromHandCostResult::Failed;
            }
            let Some(card) = player.hand.get(hand_index) else {
                return PlayFromHandCostResult::Failed;
            };
            if card.handle() != target_card {
                return PlayFromHandCostResult::Failed;
            }
            card.play_cost(&self.card_data)
        };
        let base_cost = cost_delta.resolve(printed_cost) as i32;
        let effective_cost = (base_cost - total_reduction).max(0) as u16;

        if !self.pay_memory(effective_cost) {
            return PlayFromHandCostResult::Failed;
        }

        let player = self.player_mut(player_id);
        let card = player.hand.remove(hand_index);
        let perm = crate::permanent::Permanent::new(card, turn);
        player.battle_area.push(perm);
        let field_index = player.battle_area.len() - 1;
        let entered = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        let entered_card = self.players[player_id as usize].battle_area[field_index]
            .top_card()
            .handle();

        let emitted_card_id = self.players[player_id as usize].battle_area[field_index]
            .top_card()
            .card_id(&self.card_data)
            .to_string();
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Play {
            seq,
            player: player_id,
            card_id: emitted_card_id,
            field_index: field_index as u8,
        });

        self.fire_on_play(player_id, field_index);
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnEnterFieldAnyone,
            crate::selection::TriggerSource::EnteredField {
                player: player_id,
                permanent: entered,
                card: entered_card,
            },
        );
        self.drain_effect_queue();

        PlayFromHandCostResult::Played(field_index)
    }

    /// Play a card from `player`'s trash to field, paying the printed cost.
    ///
    /// Delegates to [`Self::play_from_trash_with_cost`] with
    /// `CostDelta::Reduce(0)` (pay the printed cost verbatim) and
    /// `PlaySource::ByEffect` (trash plays are always effect-driven).
    ///
    /// Does NOT call `check_turn_end`. Callers that want to end the turn when
    /// memory goes negative after OnPlay effects resolve should invoke
    /// `check_turn_end` explicitly.
    pub fn play_from_trash(&mut self, player_id: PlayerId, trash_index: usize) -> Option<usize> {
        self.play_from_trash_with_cost(
            player_id,
            trash_index,
            crate::enums::CostDelta::Reduce(0),
            PlaySource::ByEffect,
        )
    }

    /// Play a card from `player`'s trash. Like `play_from_hand_with_cost` but
    /// reads and removes from `player.trash`. Returns `Some(field_index)` on
    /// success, `None` if trash_index is invalid, battle area full, or memory
    /// insufficient.
    ///
    /// Does NOT call `check_turn_end`. Callers that want to end the turn when
    /// memory goes negative after OnPlay effects resolve should invoke
    /// `check_turn_end` explicitly.
    pub fn play_from_trash_with_cost(
        &mut self,
        player_id: PlayerId,
        trash_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
    ) -> Option<usize> {
        let field_slots = self.rules.field_slots;

        let card_kind = {
            let player = self.player(player_id);
            if trash_index >= player.trash.len() {
                return None;
            }
            if player.battle_area.len() >= field_slots as usize {
                return None;
            }
            let card = &player.trash[trash_index];
            card.card_kind(&self.card_data)
        };

        // Phase 6: CannotPlayDigimonByEffect — when source is ByEffect and the
        // card is a Digimon, gate on the player-scoped modifier.
        if source == PlaySource::ByEffect
            && card_kind == CardKind::Digimon
            && self
                .modifiers
                .player_has(player_id, ModifierType::CannotPlayDigimonByEffect)
        {
            return None;
        }

        // Phase 6: CannotPlayTamerByEffect — when source is ByEffect and the
        // card is a Tamer, gate on the player-scoped modifier.
        if source == PlaySource::ByEffect
            && card_kind == CardKind::Tamer
            && self
                .modifiers
                .player_has(player_id, ModifierType::CannotPlayTamerByEffect)
        {
            return None;
        }

        let card = self.player_mut(player_id).trash.remove(trash_index);
        self.player_mut(player_id).hand.push(card);
        let hand_index = self.player(player_id).hand.len() - 1;

        match self.play_from_hand_with_cost_result(player_id, hand_index, cost_delta, source, false)
        {
            PlayFromHandCostResult::Played(field_index) => Some(field_index),
            PlayFromHandCostResult::Pending => None,
            PlayFromHandCostResult::Failed => {
                let card = self
                    .player_mut(player_id)
                    .hand
                    .pop()
                    .expect("invariant: card was just pushed to hand");
                self.player_mut(player_id).trash.insert(trash_index, card);
                None
            }
        }
    }

    /// Play an Option card from `player`'s hand.
    ///
    /// Phase 8 Task 2 — Standard Option pipeline:
    /// 1. Validate phase / hand index / card kind / color match.
    /// 2. Compute + pay cost (honors BeforePayCost reductions).
    /// 3. Move card out of hand into `pending_option`.
    /// 4. Fire `OnUseOption` global observer (every battle area) + this
    ///    card's `OptionMain` body, drain the queue.
    /// 5. If a `PendingSelection` parked inside the body, return `Pending`
    ///    — caller drives the selection; `dispose_option` re-enters
    ///    via the post-resolution path once the selection resolves.
    /// 6. Otherwise dispose (Standard → trash; Delay → park on field) and
    ///    `check_turn_end`.
    ///
    /// Delay / Link / Training dispatch lands in Tasks 3/4/5. For Task 2
    /// every Option is treated as Standard — the specialized dispatch
    /// looks at the fired effect's flags in the `option_main` body and
    /// parks a different `OptionResolutionPhase`.
    pub fn play_option_from_hand(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
    ) -> OptionPlayResult {
        self.play_option_core(player_id, OptionSource::Hand(hand_index))
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
        self.play_option_core(player_id, OptionSource::Trash(trash_index))
    }

    /// Shared Option-play pipeline. Forks only on source zone — every other
    /// step (cost, OnUseOption + OptionMain, dispose) is identical between
    /// hand- and trash-sourced plays.
    fn play_option_core(&mut self, player_id: PlayerId, source: OptionSource) -> OptionPlayResult {
        debug_assert!(
            self.pending_option.is_none(),
            "reentrant Option play while another is mid-resolution"
        );

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
            if !crate::action::mask::option_use_requirement_or_color_available(
                card, self, player_id,
            ) {
                return OptionPlayResult::Invalid;
            }
            (
                card.handle(),
                card.option_use_cost(&self.card_data)
                    .unwrap_or_else(|| card.play_cost(&self.card_data)),
                card.card_id(&self.card_data).to_string(),
            )
        };

        // 3. Compute + pay cost (Phase 5 BeforePayCost hooks).
        let total_reduction = self.scan_before_pay_cost_reduction(player_id);
        let base_cost = printed_cost as i32;
        let effective_cost = (base_cost - total_reduction).max(0) as u16;
        if !self.pay_memory(effective_cost) {
            return OptionPlayResult::Invalid;
        }

        // 4. Remove from source zone, install PendingOption.
        let card = match source {
            OptionSource::Hand(i) => self.player_mut(player_id).hand.remove(i),
            OptionSource::Trash(i) => self.player_mut(player_id).trash.remove(i),
        };
        self.pending_option = Some(PendingOption {
            owner: player_id,
            card,
            source_kind,
            resolution_phase: OptionResolutionPhase::MainEffectDrain,
        });

        // 5. Fire OnUseOption (global observer across every battle area) +
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

        self.enqueue_option_main_from_pending(&card_id, card_handle, player_id);
        self.drain_effect_queue();

        // 6. If an effect parked a selection, suspend and let the caller drive.
        if self.pending_selection.is_some() {
            return OptionPlayResult::Pending;
        }

        if self.pending_option_can_arts_digivolve() && self.install_arts_digivolve_selection() {
            return OptionPlayResult::Pending;
        }

        // 7. Dispose per subtype (Standard → trash; Delay → park on field;
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

    pub(crate) fn pending_option_can_arts_digivolve(&self) -> bool {
        let Some(pending) = self.pending_option.as_ref() else {
            return false;
        };
        if pending.card.card_kind(&self.card_data) != CardKind::Dual {
            return false;
        }
        let data = &self.card_data[pending.card.data_index];
        data.dual
            .as_ref()
            .map(|dual| {
                data.keywords.contains(&Keyword::ArtsDigivolve)
                    || dual.option.keywords.contains(&Keyword::ArtsDigivolve)
                    || dual.digimon.keywords.contains(&Keyword::ArtsDigivolve)
            })
            .unwrap_or(false)
    }

    fn arts_digivolve_battle_targets(&self, owner: PlayerId) -> Vec<PermanentHandle> {
        let Some(pending) = self.pending_option.as_ref() else {
            return Vec::new();
        };
        let player = self.player(owner);
        player
            .battle_area
            .iter()
            .enumerate()
            .filter_map(|(i, perm)| {
                let handle = PermanentHandle {
                    player: owner,
                    index: i as u8,
                };
                if self.modifiers.has(handle, ModifierType::CannotDigivolve) {
                    return None;
                }
                if self.can_digivolve(&pending.card, perm) {
                    Some(handle)
                } else {
                    None
                }
            })
            .collect()
    }

    fn arts_digivolve_has_breeding_target(&self, owner: PlayerId) -> bool {
        let Some(pending) = self.pending_option.as_ref() else {
            return false;
        };
        let Some(breeding) = self.player(owner).breeding_area.as_ref() else {
            return false;
        };
        self.can_digivolve(&pending.card, breeding)
    }

    pub(crate) fn install_arts_digivolve_selection(&mut self) -> bool {
        use crate::action::space::encode_attack;

        let Some(pending) = self.pending_option.as_ref() else {
            return false;
        };
        let owner = pending.owner;
        let source_card = pending.card.handle();
        let targets = self.arts_digivolve_battle_targets(owner);
        let has_breeding = self.arts_digivolve_has_breeding_target(owner);
        if targets.is_empty() && !has_breeding {
            return false;
        }

        let mut valid_action_ids: Vec<u16> = targets
            .iter()
            .map(|h| encode_attack(0, h.index as u16))
            .collect();
        if has_breeding {
            valid_action_ids.push(crate::action::space::BREEDING_SELECTION_TARGET);
        }
        let target_snapshot = targets.clone();
        let previous_phase = self.current_phase;

        if let Some(pending) = self.pending_option.as_mut() {
            pending.resolution_phase = OptionResolutionPhase::ArtsSelectTarget;
        }
        self.current_phase = GamePhase::SelectTarget;
        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::OwnField,
            selecting_player: owner,
            previous_phase,
            valid_action_ids,
            is_optional: true,
            prompt: "Choose a card for Arts Digivolve, or pass to trash this Option".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Option,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
                if action_id == crate::action::space::BREEDING_SELECTION_TARGET {
                    let _ = game.arts_digivolve_pending_option_onto_breeding(owner);
                    return;
                }
                let offset = action_id.saturating_sub(ATTACK_START);
                let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
                if target_snapshot.iter().any(|h| h.index == target_index) {
                    let target = PermanentHandle {
                        player: owner,
                        index: target_index,
                    };
                    let _ = game.arts_digivolve_pending_option_onto_battle(target);
                }
            }),
            on_decline: Some(Box::new(|game: &mut Game| {
                game.dispose_option();
                game.check_turn_end();
            })),
        });
        true
    }

    pub(crate) fn arts_digivolve_pending_option_onto_battle(
        &mut self,
        target: PermanentHandle,
    ) -> bool {
        if !self.pending_option_can_arts_digivolve() {
            return false;
        }
        let Some(pending_ref) = self.pending_option.as_ref() else {
            return false;
        };
        if pending_ref.owner != target.player {
            return false;
        }
        let Some(perm) = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return false;
        };
        if self.modifiers.has(target, ModifierType::CannotDigivolve) {
            return false;
        }
        if !self.can_digivolve(&pending_ref.card, perm) {
            return false;
        }

        let pending = self.pending_option.take().expect("checked above");
        let arts_card_id = pending.card.card_id(&self.card_data).to_string();
        let arts_card_handle = pending.card.handle();
        let arts_owner = pending.owner;
        let turn = self.turn_count;
        self.player_mut(target.player).battle_area[target.index as usize]
            .digivolve(pending.card, turn);
        self.player_mut(target.player).draw();

        self.run_rule_check_after_arts();

        if self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .is_some()
        {
            self.enqueue_triggered(
                EffectTiming::WhenDigivolving,
                TriggerSource::Permanent(target),
            );
        } else {
            self.enqueue_when_digivolving_from_arts_card(
                &arts_card_id,
                arts_card_handle,
                arts_owner,
            );
        }
        self.drain_effect_queue();
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnDigivolve,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();
        self.check_turn_end();
        true
    }

    fn enqueue_when_digivolving_from_arts_card(
        &mut self,
        card_id: &str,
        card_handle: crate::card_source::CardHandle,
        owner: PlayerId,
    ) {
        let Some(effects) = self.effects_for_card(card_id, card_handle) else {
            return;
        };
        let tp = self.turn_player();
        let is_turn_player = owner == tp;
        for (slot, effect) in effects.iter().enumerate() {
            if effect.timing != EffectTiming::WhenDigivolving {
                continue;
            }
            self.effect_queue.push_back(QueuedEffect {
                source_card: card_handle,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                controller: owner,
                timing: EffectTiming::WhenDigivolving,
                trigger_context: None,
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.to_string(),
                allow_below_top_liveness: false,
            });
        }
    }

    pub(crate) fn arts_digivolve_pending_option_onto_breeding(&mut self, owner: PlayerId) -> bool {
        if !self.pending_option_can_arts_digivolve() {
            return false;
        }
        let Some(pending_ref) = self.pending_option.as_ref() else {
            return false;
        };
        if pending_ref.owner != owner {
            return false;
        }
        let Some(breeding) = self.player(owner).breeding_area.as_ref() else {
            return false;
        };
        if !self.can_digivolve(&pending_ref.card, breeding) {
            return false;
        }

        let pending = self.pending_option.take().expect("checked above");
        let turn = self.turn_count;
        if let Some(breeding) = self.player_mut(owner).breeding_area.as_mut() {
            breeding.digivolve(pending.card, turn);
        }
        self.player_mut(owner).draw();
        self.check_turn_end();
        true
    }

    pub(crate) fn run_rule_check_after_arts(&mut self) {
        let mut to_delete: Vec<PermanentHandle> = Vec::new();
        for pid in 0..self.players.len() {
            for (idx, perm) in self.players[pid].battle_area.iter().enumerate() {
                let handle = PermanentHandle {
                    player: pid as PlayerId,
                    index: idx as u8,
                };
                if perm.is_digimon(&self.card_data) && self.effective_dp(handle).unwrap_or(1) <= 0 {
                    to_delete.push(handle);
                }
            }
        }
        for handle in to_delete.into_iter().rev() {
            self.delete_permanent_with_effects(handle);
        }
    }

    /// Enqueue every `OptionMain` effect declared by `card_id` directly
    /// against the in-flight `pending_option` card. Options aren't on the
    /// battle area, so `TriggerSource::PlayerBattleArea` / `Permanent` can't
    /// find them — we push directly into `effect_queue` with the card handle
    /// and a `None` source permanent.
    fn enqueue_option_main_from_pending(
        &mut self,
        card_id: &str,
        card_handle: crate::card_source::CardHandle,
        owner: PlayerId,
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
            self.effect_queue.push_back(QueuedEffect {
                source_card: card_handle,
                source_permanent: None,
                source_kind: EffectSourceKind::Option,
                controller: owner,
                timing: EffectTiming::OptionMain,
                trigger_context: None,
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.to_string(),
                allow_below_top_liveness: false,
            });
        }
    }

    /// Phase 9 Task 3 — enqueue every `CounterEffect`-timing effect on the
    /// in-flight `pending_option` card. Mirrors
    /// `enqueue_option_main_from_pending` but filters on `CounterEffect`
    /// so a hand Counter Option's body fires BEFORE its `OptionMain` body.
    /// Called only when `in_counter_window` is set.
    fn enqueue_counter_effect_from_pending(
        &mut self,
        card_id: &str,
        card_handle: crate::card_source::CardHandle,
        owner: PlayerId,
    ) {
        let Some(effects) = self.effects_for_card(card_id, card_handle) else {
            return;
        };
        let tp = self.turn_player();
        let is_turn_player = owner == tp;
        for (slot, effect) in effects.iter().enumerate() {
            if effect.timing != EffectTiming::CounterEffect {
                continue;
            }
            if !effect.counter {
                continue;
            }
            self.effect_queue.push_back(QueuedEffect {
                source_card: card_handle,
                source_permanent: None,
                source_kind: EffectSourceKind::Option,
                controller: owner,
                timing: EffectTiming::CounterEffect,
                trigger_context: None,
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.to_string(),
                allow_below_top_liveness: false,
            });
        }
    }

    /// Dispose an Option that has finished resolving its `OptionMain`
    /// body. Branches on the card's subtype:
    ///
    /// - **Standard** — route to the owner's trash through Phase 7's
    ///   `WhenWouldBeTrashed` replacement window (cause=Cost). A mandatory
    ///   cancel keeps the card in the owner's hand; a redirect routes to
    ///   Deck (bottom) or Hand. An optional replacement installs a
    ///   `PendingSelection` and re-parks `pending_option` in `Disposing`
    ///   so `advance_pending_option` can commit once the selection
    ///   resolves.
    /// - **Delay** — park on the owner's battle_area as a Permanent with
    ///   `OptionState::Delayed`. The end-of-turn scan in
    ///   [`Game::scan_delayed_options_at_end_of_turn`] fires `DelayEffect`
    ///   and trashes via `delete_permanent_with_cause(Cost)` when
    ///   `turn_count == trash_on_turn`. That delete path fires
    ///   `WhenWouldLeaveBattleArea` + `WhenWouldBeDeleted` (Phase 7
    ///   integration for Delay flows through the Permanent fire-site, not
    ///   `WhenWouldBeTrashed`).
    /// - **Link** — install host-select prompt; the selection callback
    ///   calls `attach_linked_card` directly.
    /// - **Training** — park as `OptionState::Training` on the owner's
    ///   battle_area.
    pub(crate) fn dispose_option(&mut self) {
        let Some(pending) = self.pending_option.take() else {
            return;
        };

        let card_id = pending.card.card_id(&self.card_data).to_string();
        let effects = self
            .effects_for_card(&card_id, pending.card.handle())
            .unwrap_or_default();
        let subtype = classify_option_subtype(&effects);

        match subtype {
            OptionSubtype::Standard => {
                use crate::replacement::{ReplacementCause, ReplacementSubject};

                // Phase 8 Task 6: route the dispose-trash through
                // `try_replace(WhenWouldBeTrashed, ...)`. Cause is Cost
                // (the Option was played from hand/trash and is being
                // disposed as part of the play cost/resolution). Source
                // zone reflects where the Option was used from.
                let card_handle = pending.card.handle();
                let subject = ReplacementSubject::Card(card_handle, pending.source_kind.zone());
                self.pending_option = Some(pending);
                let outcome = self.try_replace(
                    EffectTiming::WhenWouldBeTrashed,
                    subject,
                    ReplacementCause::Cost,
                    Some(crate::enums::Zone::Trash),
                );
                let Some(pending) = self.pending_option.take() else {
                    return;
                };

                if self.pending_selection.is_some() {
                    // Optional replacement installed a selection. Re-park
                    // `pending_option` in `Disposing` so
                    // `advance_pending_option` can commit the trash
                    // outcome once the selection resolves.
                    self.pending_option = Some(PendingOption {
                        owner: pending.owner,
                        card: pending.card,
                        source_kind: pending.source_kind,
                        resolution_phase: OptionResolutionPhase::Disposing,
                    });
                    return;
                }

                self.commit_option_trash_outcome(pending, outcome);
            }
            OptionSubtype::Delay(trigger) => {
                let owner = pending.owner;
                let placed_card = pending.card.handle();
                let trash_turn = self.compute_delay_trash_turn(pending.owner, trigger);
                let turn = self.turn_count;
                let mut perm = crate::permanent::Permanent::new(pending.card, turn);
                perm.option_state = crate::permanent::OptionState::Delayed {
                    owner,
                    trash_on_turn: trash_turn,
                };
                self.player_mut(owner).battle_area.push(perm);
                let permanent = PermanentHandle {
                    player: owner,
                    index: (self.player(owner).battle_area.len() - 1) as u8,
                };
                self.enqueue_triggered(
                    EffectTiming::OnOptionPlaced,
                    TriggerSource::OptionPlaced {
                        player: owner,
                        permanent,
                        card: placed_card,
                    },
                );
                self.drain_effect_queue();
                if self.pending_selection.is_some() {
                    self.pending_option_placed_turn_check = true;
                }
            }
            OptionSubtype::Link => {
                // Phase 8 Task 4: evaluate link_filter against every
                // Standard-state Digimon on the owner's battle_area. If no
                // candidate passes, trash the card silently (mirrors "no
                // legal target" for other effect selections). Otherwise
                // install a PendingSelection routed to `attach_linked_card`
                // and park `pending_option` in `LinkSelectHost`.
                let owner = pending.owner;
                let source_card = pending.card.handle();
                let candidates = {
                    let owner_player = self.player(owner);
                    let mut out: Vec<PermanentHandle> = Vec::new();
                    for (i, perm) in owner_player.battle_area.iter().enumerate() {
                        if !perm.is_digimon(&self.card_data) {
                            continue;
                        }
                        if !matches!(perm.option_state, crate::permanent::OptionState::Standard) {
                            continue;
                        }
                        let handle = PermanentHandle {
                            player: owner,
                            index: i as u8,
                        };
                        // Find a link effect; evaluate its filter.
                        let filter_ok = effects.iter().find(|e| e.link_cost.is_some()).map_or(
                            true,
                            |link_effect| {
                                if let Some(f) = &link_effect.link_filter {
                                    let read_ctx =
                                        EffectReadContext::new(self, source_card, None, owner);
                                    f(&read_ctx, handle)
                                } else {
                                    true
                                }
                            },
                        );
                        if filter_ok {
                            out.push(handle);
                        }
                    }
                    out
                };

                if candidates.is_empty() {
                    self.player_mut(owner).trash.push(pending.card);
                    return;
                }

                // Re-install pending_option in LinkSelectHost and park a
                // field-selection prompt. The selection callback threads
                // straight into `attach_linked_card`.
                self.pending_option = Some(PendingOption {
                    owner,
                    card: pending.card,
                    source_kind: pending.source_kind,
                    resolution_phase: OptionResolutionPhase::LinkSelectHost,
                });
                self.install_link_host_selection(owner, source_card, candidates);
            }
            OptionSubtype::Training => {
                // Phase 8 Task 5: park as an `OptionState::Training` permanent on
                // the owner's battle_area. Stays there until the owner hatches
                // an egg via `move_from_breeding`, at which point every Training
                // permanent the owner controls fires `OnTrainingTrash` and is
                // trashed (see `Game::move_from_breeding`). Training sideways-
                // inheritance is dispatched in `enqueue_from_permanent`.
                let turn = self.turn_count;
                let mut perm = crate::permanent::Permanent::new(pending.card, turn);
                perm.option_state = crate::permanent::OptionState::Training {
                    owner: pending.owner,
                };
                self.player_mut(pending.owner).battle_area.push(perm);
            }
        }
    }

    /// Commit a Standard Option's dispose-trash given the
    /// `WhenWouldBeTrashed` outcome produced by `try_replace`. Shared by
    /// the synchronous path in `dispose_option` and the deferred path in
    /// `advance_pending_option::Disposing` (where an optional replacement
    /// installed a selection that has since resolved).
    ///
    /// Outcome routing (per Phase 7 spec §7.6):
    /// - `None` — commit the original event: trash the card.
    /// - `Cancelled` / `CustomHandled` — return the Option to the owner's
    ///   hand (cancel restores the original zone for Card subjects).
    /// - `Redirected(Deck)` — insert at the bottom of the owner's deck.
    /// - `Redirected(Hand)` — push to the owner's hand.
    /// - Other variants (`Redirected(other)`, `Substituted(_)`) are not
    ///   meaningful for Option trash in v1; debug_assert catches the
    ///   regression and falls back to trash.
    pub(crate) fn commit_option_trash_outcome(
        &mut self,
        pending: PendingOption,
        outcome: crate::replacement::ReplacementOutcome,
    ) {
        use crate::replacement::ReplacementOutcome;
        match outcome {
            ReplacementOutcome::None => {
                self.player_mut(pending.owner).trash.push(pending.card);
            }
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                self.player_mut(pending.owner).hand.push(pending.card);
            }
            ReplacementOutcome::Redirected(crate::enums::Zone::Deck) => {
                self.player_mut(pending.owner).deck.insert(0, pending.card);
            }
            ReplacementOutcome::Redirected(crate::enums::Zone::Hand) => {
                self.player_mut(pending.owner).hand.push(pending.card);
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected Redirected({:?}) for Option trash — only Deck/Hand supported in v1",
                    other
                );
                self.player_mut(pending.owner).trash.push(pending.card);
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "Substituted not supported for Option WhenWouldBeTrashed v1"
                );
                self.player_mut(pending.owner).trash.push(pending.card);
            }
        }
    }

    /// Install a field-selection prompt listing `candidates` as legal host
    /// Digimon for a Link Option. On resolve, the callback invokes
    /// `attach_linked_card(host)` which attaches the card + fires OnLink.
    fn install_link_host_selection(
        &mut self,
        owner: PlayerId,
        source_card: crate::card_source::CardHandle,
        candidates: Vec<PermanentHandle>,
    ) {
        use crate::action::space::{encode_attack, ATTACK_START, TARGETS_PER_ATTACKER};
        use crate::selection::SelectionKind;

        // Encode via attack-id space — same convention as
        // `select_own_permanent` / `install_field_selection`. The candidates
        // list restricts which indices are valid; no need for a reserved
        // action-ID namespace.
        let valid_action_ids: Vec<u16> = candidates
            .iter()
            .map(|h| encode_attack(0, h.index as u16))
            .collect();

        // Keep the candidate set in a closure-owned snapshot so the callback
        // decodes the picked index correctly even if new permanents are added
        // mid-selection (queue is paused, but this is the defensive choice).
        let candidate_snapshot = candidates.clone();

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectTarget;
        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::OwnField,
            selecting_player: owner,
            previous_phase,
            valid_action_ids,
            is_optional: false,
            prompt: "Choose a Digimon to link this Option to".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Option,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let offset = action_id.saturating_sub(ATTACK_START);
                let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let picked = candidate_snapshot
                    .iter()
                    .copied()
                    .find(|h| h.index == target_index)
                    .unwrap_or(PermanentHandle {
                        player: owner,
                        index: target_index,
                    });
                game.attach_linked_card(picked);
            }),
            on_decline: None,
        });
    }

    /// Complete a Link Option's attach: push the pending card into the
    /// host's `linked_cards`, fire `OnLink` globally, and clear
    /// `pending_option`. The caller has already validated that `host` was
    /// in the candidate list at selection install-time, but we re-check the
    /// handle is still live in case an intervening effect moved things.
    pub(crate) fn attach_linked_card(&mut self, host: PermanentHandle) {
        let Some(pending) = self.pending_option.take() else {
            return;
        };

        // If the host vanished (e.g. deleted mid-selection by an interposing
        // effect), fall back to trashing the Option — mirrors other
        // "target vanished" paths elsewhere in the engine.
        let host_live = self
            .player(host.player)
            .battle_area
            .get(host.index as usize)
            .map(|p| {
                p.is_digimon(&self.card_data)
                    && matches!(p.option_state, crate::permanent::OptionState::Standard)
            })
            .unwrap_or(false);
        if !host_live {
            self.player_mut(pending.owner).trash.push(pending.card);
            self.check_turn_end();
            return;
        }

        // Attach.
        self.player_mut(host.player).battle_area[host.index as usize]
            .linked_cards
            .push(pending.card);

        // Fire OnLink globally — every player's battle area scans for
        // OnLink-timed effects. Load-bearing for Appmon-trait cards.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnLink,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();

        // Link lifecycle complete — check if memory state demands turn transition.
        // The Standard Option path hits this via `advance_pending_option`; the
        // Link path bypasses that dispatcher (host-select callback calls this
        // directly), so we must invoke `check_turn_end` ourselves.
        self.check_turn_end();
    }

    /// Compute the absolute `turn_count` at which a delayed Option should
    /// self-trash. The rule is "end of the **owner**'s next turn" for
    /// `EndOfYourNextTurn`, and the current turn for `EndOfThisTurn`.
    ///
    /// In a 2-player round-robin:
    /// - If `owner == turn_player` (the common case — played on own turn),
    ///   "next own turn" lands `turn_count + 2` (skip the opponent's turn).
    /// - If `owner != turn_player` (played during opponent's turn, e.g. via
    ///   a Counter window), "next own turn" lands `turn_count + 1`.
    ///
    /// Multi-player extension is deferred — the plan locks 2-player
    /// semantics for v1.
    fn compute_delay_trash_turn(
        &self,
        owner: PlayerId,
        trigger: crate::enums::DelayTrigger,
    ) -> u16 {
        use crate::enums::DelayTrigger;
        // TODO(multi-player): generalize turn-rotation to >2 players.
        match trigger {
            DelayTrigger::EndOfThisTurn => self.turn_count,
            DelayTrigger::EndOfYourNextTurn => {
                if self.turn_player() == owner {
                    self.turn_count + 2
                } else {
                    self.turn_count + 1
                }
            }
        }
    }

    pub(crate) fn finish_pending_option_placed_turn_check(&mut self) {
        if !self.pending_option_placed_turn_check {
            return;
        }
        if self.pending_selection.is_some() || !self.effect_queue.is_empty() {
            return;
        }
        self.pending_option_placed_turn_check = false;
        self.check_turn_end();
    }

    /// Move a specific card from `player`'s deck to their hand. Returns false
    /// if the handle isn't in the deck. Does NOT shuffle — callers that mirror
    /// the printed "search then shuffle" rule must call `shuffle_deck` after.
    pub fn add_to_hand_from_deck(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(removed) = self.player_mut(player_id).remove_from_deck_by_handle(card) else {
            return false;
        };
        self.player_mut(player_id).add_to_hand(removed);
        true
    }

    /// Move a specific card from `player`'s trash to their hand.
    pub fn add_to_hand_from_trash(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(removed) = self.player_mut(player_id).remove_from_trash_by_handle(card) else {
            return false;
        };
        self.player_mut(player_id).add_to_hand(removed);
        true
    }

    /// Reveal up to `n` cards from the top of `player`'s deck. Cards move
    /// into `self.revealed_cards` (transient reveal pool, cleared on turn
    /// rotation). Returns the list of revealed card handles in top-first
    /// order.
    ///
    /// Does not fire `OnDraw` or modify hand. Callers that want to then
    /// move a revealed card to hand/deck/trash use the reveal-pool
    /// follow-up helpers added in Task 9.
    pub fn reveal_top_deck(
        &mut self,
        player_id: PlayerId,
        n: u8,
    ) -> Vec<crate::card_source::CardHandle> {
        let mut handles = Vec::new();
        for _ in 0..n {
            let p = self.player_mut(player_id);
            let Some(card) = p.deck.pop() else { break };
            handles.push(card.handle());
            self.revealed_cards.push(card);
        }
        handles
    }

    /// Shuffle `player`'s deck.
    pub fn shuffle_deck(&mut self, player_id: PlayerId) {
        // Split-borrow idiom: take deck out, shuffle, put back.
        let mut deck = std::mem::take(&mut self.player_mut(player_id).deck);
        deck.shuffle(&mut self.rng);
        self.player_mut(player_id).deck = deck;
    }

    /// Fire OnPlay effects for the permanent at `(player, field_index)`.
    /// Called by play_from_hand; can also be called directly by tests.
    ///
    /// Thin wrapper over the effect-queue drainer. Single-trigger cases fire
    /// in one step exactly like the old atomic loop; multi-trigger cases
    /// park on a `TriggerOrder` selection for the controller to order.
    pub fn fire_on_play(&mut self, player_id: PlayerId, field_index: usize) {
        if field_index >= self.players[player_id as usize].battle_area.len() {
            return;
        }
        let handle = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        self.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
        self.drain_effect_queue();
    }

    /// Activate a `[Main]` effect on the card at `player_id`'s hand slot
    /// `hand_index`. Returns `true` if a matching effect fired, `false` if no
    /// `EffectTiming::MainFromHand` effect on the card was legal.
    ///
    /// Consumes `HAND_EFFECT` action bits (30-59) that the mask emits. Memory
    /// cost, card movement, and any side effects are handled inside the
    /// effect's `process` closure — mirroring Python's
    /// `_execute_hand_main_effect`. First-match-wins: once an effect fires we
    /// stop iterating, matching the mask's own first-match-wins emission.
    ///
    /// Hand/Trash per-turn activation counters (§4.5c-residual 🟡) are not
    /// tracked here; see docs/RUST_PYTHON_PARITY.md §4.5c.
    pub fn activate_hand_main(&mut self, player_id: PlayerId, hand_index: usize) -> bool {
        let (card_id, handle) = {
            let player = match self.players.get(player_id as usize) {
                Some(p) => p,
                None => return false,
            };
            let card = match player.hand.get(hand_index) {
                Some(c) => c,
                None => return false,
            };
            (card.card_id(&self.card_data).to_string(), card.handle())
        };

        // Use `effects_for_card` rather than the raw registry so that
        // keyword-derived auto-installed effects are visible here. The
        // action mask uses the same accessor — without this, the mask
        // could emit a Hand [Main] bit for an auto-installed keyword
        // that this dispatcher could not honor.
        let effects = match self.effects_for_card(&card_id, handle) {
            Some(e) => e,
            None => return false,
        };

        for effect in &effects {
            if effect.timing != EffectTiming::MainFromHand {
                continue;
            }
            if let Some(cond) = &effect.condition {
                let ctx = EffectReadContext::new(self, handle, None, player_id);
                if !cond(&ctx) {
                    continue;
                }
            }
            if let Some(process) = &effect.process {
                let mut ctx = EffectContext::new(self, handle, None, player_id);
                process(&mut ctx);
            }
            return true;
        }
        false
    }

    /// Activate a `[Main]` effect on the permanent at `player_id`'s battle-area
    /// slot `field_index`. Returns `true` if a matching effect fired.
    ///
    /// Consumes `FIELD_EFFECT` bits at sub-slot `FIELD_EFFECT_SLOT_FOR_MAIN`
    /// (per-permanent base + 2). Walks the digivolution stack bottom-up,
    /// applying the inherited-vs-top filter used by
    /// [`Game::source_dp_contribution`] so a given Field [Main] effect only
    /// fires on the same source/position the mask emitted from. Honors OPT via
    /// [`Permanent::activation_count`] and records activation on success so a
    /// subsequent mask rebuild sees the bit suppressed.
    ///
    /// Mirrors Python's `_execute_field_main_effect`.
    pub fn activate_field_main(&mut self, player_id: PlayerId, field_index: usize) -> bool {
        // Phase F Task 6 — breeding-area Training dispatch.
        //
        // `field_index == BREEDING_TARGET (=14)` routes to the breeding-area
        // path. The mask only emits this bit for `<Training>`-bearing
        // breeding-area carriers (RULES_CONTEXT 16-40 — only Training
        // activates from breeding); the dispatcher here independently
        // re-checks the gate (printed Training keyword on top + carrier not
        // suspended) and runs only the `<Training>` effect (filtered by
        // `effect.name == "<Training>"`) so a stale or hand-rolled
        // `MainOnField` on the same card cannot leak through.
        if field_index == crate::action::space::BREEDING_TARGET as usize {
            return self.activate_breeding_main_training(player_id);
        }

        // Snapshot per-source identity without holding the battle_area borrow
        // across the effect closure invocations (which need `&mut self`).
        let (perm_handle, sources) = {
            let Some(player) = self.players.get(player_id as usize) else {
                return false;
            };
            let Some(perm) = player.battle_area.get(field_index) else {
                return false;
            };
            let stack_size = perm.card_sources.len();
            let handle = PermanentHandle {
                player: player_id,
                index: field_index as u8,
            };
            let mut infos: Vec<(bool, String, crate::card_source::CardHandle)> =
                Vec::with_capacity(stack_size);
            for (i, source) in perm.card_sources.iter().enumerate() {
                let is_under = i + 1 < stack_size;
                infos.push((
                    is_under,
                    source.card_id(&self.card_data).to_string(),
                    source.handle(),
                ));
            }
            (handle, infos)
        };

        for (is_under, card_id, source_handle) in sources {
            // Use `effects_for_card` rather than the raw registry so that
            // keyword-derived auto-installed effects (e.g. printed
            // `MaterialSave(N)`) are visible here. The action mask uses the
            // same accessor — without this, the mask would emit a Field
            // [Main] bit for an auto-installed keyword that this dispatcher
            // could not honor.
            let Some(effects) = self.effects_for_card(&card_id, source_handle) else {
                continue;
            };
            for (slot, effect) in effects.iter().enumerate() {
                if effect.timing != EffectTiming::MainOnField {
                    continue;
                }
                if is_under != effect.inherited {
                    continue;
                }
                if effect.max_per_turn > 0 {
                    let perm = &self.players[player_id as usize].battle_area[field_index];
                    if perm.activation_count(source_handle, slot as u8) >= effect.max_per_turn {
                        continue;
                    }
                }
                if let Some(cond) = &effect.condition {
                    let ctx =
                        EffectReadContext::new(self, source_handle, Some(perm_handle), player_id);
                    if !cond(&ctx) {
                        continue;
                    }
                }
                // Python records activation before invoking the callback so a
                // panic inside the process still counts toward OPT. Mirror that.
                if let Some(perm) = self.players[player_id as usize]
                    .battle_area
                    .get_mut(field_index)
                {
                    perm.record_activation(source_handle, slot as u8);
                }
                if let Some(process) = &effect.process {
                    let mut ctx =
                        EffectContext::new(self, source_handle, Some(perm_handle), player_id);
                    process(&mut ctx);
                }
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
    fn activate_breeding_main_training(&mut self, player_id: PlayerId) -> bool {
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

    /// Activate a `[Main]` effect on the card at `player_id`'s trash slot
    /// `trash_index`. Returns `true` if a matching effect fired.
    ///
    /// Consumes `TRASH_EFFECT` action bits (1150-1194). Mirrors Python's
    /// `_execute_trash_main_effect`: memory cost and any card movement happen
    /// inside the effect's process closure, and there is no per-turn
    /// activation counter (§4.5c-residual 🟡).
    pub fn activate_trash_main(&mut self, player_id: PlayerId, trash_index: usize) -> bool {
        let (card_id, handle) = {
            let player = match self.players.get(player_id as usize) {
                Some(p) => p,
                None => return false,
            };
            let card = match player.trash.get(trash_index) {
                Some(c) => c,
                None => return false,
            };
            (card.card_id(&self.card_data).to_string(), card.handle())
        };

        // Use `effects_for_card` rather than the raw registry so that
        // keyword-derived auto-installed effects are visible here. The
        // action mask uses the same accessor — without this, the mask
        // could emit a Trash [Main] bit for an auto-installed keyword
        // that this dispatcher could not honor.
        let effects = match self.effects_for_card(&card_id, handle) {
            Some(e) => e,
            None => return false,
        };

        for effect in &effects {
            if effect.timing != EffectTiming::MainFromTrash {
                continue;
            }
            if let Some(cond) = &effect.condition {
                let ctx = EffectReadContext::new(self, handle, None, player_id);
                if !cond(&ctx) {
                    continue;
                }
            }
            if let Some(process) = &effect.process {
                let mut ctx = EffectContext::new(self, handle, None, player_id);
                process(&mut ctx);
            }
            return true;
        }
        false
    }

    /// Trash a specific hand card by index. Returns the trashed card's handle
    /// on success, None if the index is out of range.
    pub fn trash_from_hand_by_index(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
    ) -> Option<crate::card_source::CardHandle> {
        let player = self.player_mut(player_id);
        if hand_index >= player.hand.len() {
            return None;
        }
        let card = player.hand.remove(hand_index);
        let h = card.handle();
        player.trash.push(card);
        Some(h)
    }

    /// Move a specific revealed card (identified by `card` handle) into
    /// `player`'s hand. Returns false if the handle is not in
    /// `self.revealed_cards`.
    pub fn add_to_hand_from_reveal(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(pos) = self.revealed_cards.iter().position(|c| c.handle() == card) else {
            return false;
        };
        let taken = self.revealed_cards.remove(pos);
        self.player_mut(player_id).hand.push(taken);
        true
    }

    /// Move a specific revealed card into `player`'s trash.
    pub fn trash_from_reveal(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(pos) = self.revealed_cards.iter().position(|c| c.handle() == card) else {
            return false;
        };
        let taken = self.revealed_cards.remove(pos);
        self.player_mut(player_id).trash.push(taken);
        true
    }

    /// Move a specific revealed card back to `player`'s deck at `position`.
    /// Returns false if the handle is not in the reveal pool.
    pub fn return_to_deck_from_reveal(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        let Some(pos_idx) = self.revealed_cards.iter().position(|c| c.handle() == card) else {
            return false;
        };
        let taken = self.revealed_cards.remove(pos_idx);
        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).deck.push(taken);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).deck.insert(0, taken);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let deck_len = self.player(player_id).deck.len();
                let idx = if deck_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=deck_len)
                };
                self.player_mut(player_id).deck.insert(idx, taken);
            }
        }
        true
    }

    /// Digivolve: push a card onto a permanent's stack.
    pub fn digivolve_onto(
        &mut self,
        player_id: PlayerId,
        field_index: usize,
        card: CardSource,
    ) -> bool {
        let turn = self.turn_count;
        let player = self.player_mut(player_id);
        if field_index >= player.battle_area.len() {
            return false;
        }
        player.battle_area[field_index].digivolve(card, turn);
        true
    }

    /// Bounce a permanent to its owner's hand: the top card moves to hand,
    /// every card beneath it goes to the owner's trash (per DCGO leave-field
    /// rules). Linked cards go to trash. Returns the handle of the card that
    /// ended up in hand.
    ///
    /// Does not fire OnLeaveField observers — that's Phase 1 timing-dispatch
    /// infrastructure. Modifiers targeting the returned permanent are cleared.
    ///
    /// Phase 7 Task 4: fires `WhenWouldLeaveBattleArea` + `WhenWouldBeReturnedToHand`
    /// replacement windows before committing. See spec §4.1, §7.
    pub fn return_to_hand(
        &mut self,
        handle: PermanentHandle,
    ) -> Option<crate::card_source::CardHandle> {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        {
            let player = self.player_mut(handle.player);
            if (handle.index as usize) >= player.battle_area.len() {
                return None;
            }
        }

        let cause = self.infer_effect_cause(handle.player);
        let subject = ReplacementSubject::Permanent(handle);

        // Super-timing first, then route-specific would.
        let leave_outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            subject,
            cause,
            Some(Zone::Hand),
        );
        if self.pending_selection.is_some() {
            return None;
        }
        let outcome = match leave_outcome {
            ReplacementOutcome::None => self.try_replace(
                EffectTiming::WhenWouldBeReturnedToHand,
                subject,
                cause,
                Some(Zone::Hand),
            ),
            other => other,
        };
        if self.pending_selection.is_some() {
            return None;
        }

        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return None;
            }
            ReplacementOutcome::Redirected(Zone::Deck) => {
                self.return_to_deck(handle, crate::enums::StackPosition::Bottom);
                return None;
            }
            ReplacementOutcome::Redirected(Zone::Trash) => {
                self.delete_permanent_with_cause(handle, cause);
                return None;
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected redirect destination for WhenWouldBeReturnedToHand: {:?}",
                    other
                );
                // Fall through and commit the original return-to-hand.
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)) => {
                return self.return_to_hand(other);
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "non-Permanent substitute subject for WhenWouldBeReturnedToHand"
                );
                // Fall through and commit the original.
            }
        }

        let perm = self
            .player_mut(handle.player)
            .battle_area
            .remove(handle.index as usize);

        let mut sources = perm.card_sources;
        let Some(top) = sources.pop() else {
            return None;
        };
        let top_handle = top.handle();
        self.player_mut(handle.player).hand.push(top);

        // Sources below the top go to trash and fire OnDigivolutionCardTrashed
        // (digivolution stack sources only — not linked_cards which are Tamer
        // equipment and separate semantic category).
        for card in sources {
            let source_card = card.handle();
            self.player_mut(handle.player).trash.push(card);
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnDigivolutionCardTrashed,
                crate::selection::TriggerSource::SourceTrashedFromStack {
                    player: handle.player,
                    host: handle,
                    host_card: top_handle,
                    card: source_card,
                },
            );
            self.drain_effect_queue();
        }
        let had_linked = !perm.linked_cards.is_empty();
        for card in perm.linked_cards {
            self.player_mut(handle.player).trash.push(card);
        }
        // Phase 8 Task 4: fire OnLinkedCardTrashed if the returning host was
        // carrying any linked cards (they cannot ride the host back to hand).
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnLinkedCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
            self.drain_effect_queue();
        }

        self.modifiers.clear_permanent(handle);
        // Phase 6: expire any player-scoped modifiers sourced from this permanent.
        self.modifiers.expire_player_on_permanent_leave(handle);
        Some(top_handle)
    }

    /// Return a permanent's top card to its owner's deck at `position`.
    /// Sources under the top go to trash; linked_cards go to trash.
    /// Modifiers targeting the permanent are cleared. Returns true on
    /// success, false if the handle is invalid or the stack is empty.
    ///
    /// Does not fire OnLeaveField observers.
    ///
    /// Phase 7 Task 4: fires `WhenWouldLeaveBattleArea` +
    /// `WhenWouldBeReturnedToDeck` replacement windows before committing.
    pub fn return_to_deck(
        &mut self,
        handle: PermanentHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        let player_id = handle.player;
        {
            let player = self.player_mut(player_id);
            if (handle.index as usize) >= player.battle_area.len() {
                return false;
            }
        }

        let cause = self.infer_effect_cause(player_id);
        let subject = ReplacementSubject::Permanent(handle);

        let leave_outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            subject,
            cause,
            Some(Zone::Deck),
        );
        if self.pending_selection.is_some() {
            return false;
        }
        let outcome = match leave_outcome {
            ReplacementOutcome::None => self.try_replace(
                EffectTiming::WhenWouldBeReturnedToDeck,
                subject,
                cause,
                Some(Zone::Deck),
            ),
            other => other,
        };
        if self.pending_selection.is_some() {
            return false;
        }

        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return false;
            }
            ReplacementOutcome::Redirected(Zone::Hand) => {
                return self.return_to_hand(handle).is_some();
            }
            ReplacementOutcome::Redirected(Zone::Trash) => {
                self.delete_permanent_with_cause(handle, cause);
                return false;
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected redirect destination for WhenWouldBeReturnedToDeck: {:?}",
                    other
                );
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)) => {
                return self.return_to_deck(other, position);
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "non-Permanent substitute subject for WhenWouldBeReturnedToDeck"
                );
            }
        }

        let mut perm = self
            .player_mut(player_id)
            .battle_area
            .remove(handle.index as usize);

        let Some(top) = perm.card_sources.pop() else {
            return false;
        };

        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).deck.push(top);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).deck.insert(0, top);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let deck_len = self.player(player_id).deck.len();
                let idx = if deck_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=deck_len)
                };
                self.player_mut(player_id).deck.insert(idx, top);
            }
        }

        // Sources below the top go to trash and fire OnDigivolutionCardTrashed
        // (digivolution stack sources only — not linked_cards which are Tamer
        // equipment and separate semantic category).
        for card in perm.card_sources {
            self.player_mut(player_id).trash.push(card);
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnDigivolutionCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
            self.drain_effect_queue();
        }
        let had_linked = !perm.linked_cards.is_empty();
        for card in perm.linked_cards {
            self.player_mut(player_id).trash.push(card);
        }
        // Phase 8 Task 4: fire OnLinkedCardTrashed if the returning host was
        // carrying any linked cards.
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnLinkedCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
            self.drain_effect_queue();
        }

        self.modifiers.clear_permanent(handle);
        // Phase 6: expire any player-scoped modifiers sourced from this permanent.
        self.modifiers.expire_player_on_permanent_leave(handle);
        true
    }

    /// Full "digivolve from hand" action — Python parity for
    /// `action_digivolve(field_idx, hand_idx)`. Validates phase, indices,
    /// `CannotDigivolve` modifier, and evo-cost fit; pays memory; removes
    /// the card from hand; stacks it onto the target permanent; draws 1;
    /// fires `WhenDigivolving` triggers and drains the effect queue;
    /// finally calls `check_turn_end`.
    ///
    /// Deferred (see RUST_PYTHON_PARITY.md):
    /// - Cost reductions (`WhenWouldDigivolve`, `CHANGE_DIGIVOLUTION_COST`)
    /// - `digivolve_observer` mechanism (no Rust equivalent yet)
    /// - Contextual modifier predicates (`{'digivolving_card': card}`)
    pub fn digivolve_from_hand(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        field_index: usize,
        _source: PlaySource,
    ) -> bool {
        if self.current_phase != GamePhase::Main {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: not in Main phase (phase={:?})",
                self.current_phase
            ));
            return false;
        }
        let player = self.player(player_id);
        if hand_index >= player.hand.len() {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: hand index {} out of range (hand size={})",
                hand_index,
                player.hand.len()
            ));
            return false;
        }
        if field_index >= player.battle_area.len() {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: field index {} out of range (battle_area size={})",
                field_index,
                player.battle_area.len()
            ));
            return false;
        }
        let handle = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        if self.modifiers.has(handle, ModifierType::CannotDigivolve) {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: permanent at field index {} blocked by CannotDigivolve modifier",
                field_index
            ));
            return false;
        }

        let card = player.hand[hand_index].clone();
        let perm = &player.battle_area[field_index];
        if !self.can_digivolve(&card, perm) {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: card {} cannot digivolve onto {} (evo-cost mismatch)",
                card.card_id(&self.card_data),
                perm.top_card().card_id(&self.card_data),
            ));
            return false;
        }
        let from_stack_top = perm.top_card().card_id(&self.card_data).to_string();
        let top_card_id = card.card_id(&self.card_data).to_string();

        let base_top = perm.top_card();
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

        let total_reduction = self.scan_before_pay_cost_reduction(player_id);
        let effective_cost = (printed_cost as i32 - total_reduction).max(0) as u16;

        if !self.pay_memory(effective_cost) {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: cannot pay memory cost {} (current memory={})",
                effective_cost, self.memory
            ));
            return false;
        }

        let turn = self.turn_count;
        let removed = self.player_mut(player_id).hand.remove(hand_index);
        self.player_mut(player_id).battle_area[field_index].digivolve(removed, turn);
        let event_card = self
            .player(player_id)
            .battle_area
            .get(field_index)
            .map(|perm| perm.top_card().handle())
            .expect("digivolve target remains in battle area after stack mutation");

        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Digivolve {
            seq,
            player: player_id,
            top_card_id,
            field_index: field_index as u8,
            from_stack_top,
        });

        self.player_mut(player_id).draw();

        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue();

        // OnDigivolve: global observer — fires in every player's battle area
        // after the evolving permanent's WhenDigivolving resolves. Distinct
        // from WhenDigivolving (self-timing on the evolving permanent).
        self.enqueue_triggered(
            EffectTiming::OnDigivolve,
            TriggerSource::Digivolved {
                player: player_id,
                permanent: handle,
                card: event_card,
            },
        );
        self.drain_effect_queue();

        self.check_turn_end();
        true
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

        let total_reduction = self.scan_before_pay_cost_reduction(player_id);
        let effective_cost = (printed_cost as i32 - total_reduction).max(0) as u16;

        if !self.pay_memory(effective_cost) {
            self.logger.log(&format!(
                "[Rejected] digivolve_breeding: cannot pay memory cost {} (current memory={})",
                effective_cost, self.memory
            ));
            return false;
        }

        let turn = self.turn_count;
        let removed = self.player_mut(player_id).hand.remove(hand_index);
        let player_mut = self.player_mut(player_id);
        if let Some(breeding) = player_mut.breeding_area.as_mut() {
            breeding.digivolve(removed, turn);
        }
        player_mut.draw();

        // Breeding digivolve does NOT fire WhenDigivolving (Python parity).
        self.check_turn_end();
        true
    }

    /// Insert a card at the bottom of `target`'s digivolution stack. The
    /// source card is taken from the zone specified by `source` (hand slot,
    /// trash slot, deck top, or reveal pool). Returns false if the source
    /// or target is invalid.
    ///
    /// On target-invalid after source-taken: the taken card is routed to
    /// the target player's trash as a safe-failure mode (source already
    /// mutated; no way to roll back).
    pub fn place_as_bottom_source(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
    ) -> bool {
        // Take the card out of its source zone.
        let taken = match source {
            crate::enums::CardSourceRef::Hand(p, i) => {
                let player = self.player_mut(p);
                if i >= player.hand.len() {
                    return false;
                }
                player.hand.remove(i)
            }
            crate::enums::CardSourceRef::Trash(p, i) => {
                let player = self.player_mut(p);
                if i >= player.trash.len() {
                    return false;
                }
                player.trash.remove(i)
            }
            crate::enums::CardSourceRef::DeckTop(p) => {
                let Some(c) = self.player_mut(p).deck.pop() else {
                    return false;
                };
                c
            }
            crate::enums::CardSourceRef::Reveal(h) => {
                let Some(idx) = self.revealed_cards.iter().position(|c| c.handle() == h) else {
                    return false;
                };
                self.revealed_cards.remove(idx)
            }
        };

        // Push under the target permanent.
        let target_player = self.player_mut(target.player);
        if (target.index as usize) >= target_player.battle_area.len() {
            // Source already mutated — safe-fail by routing to trash.
            target_player.trash.push(taken);
            return false;
        }
        target_player.battle_area[target.index as usize].push_under(taken);
        true
    }

    /// Search all zones of all players for a card matching `handle` and remove
    /// it, returning the `CardSource`. Returns `None` if the card is not found
    /// in any zone.
    ///
    /// Zones scanned (in order):
    ///   1. Each player's `hand`
    ///   2. Each player's `trash`
    ///   3. Each player's `deck`
    ///   4. Each player's `security`
    ///   5. Each player's `battle_area` permanent card stacks (all sources)
    ///   6. Each player's `breeding_area` card stack
    ///   7. The game-level `revealed_cards` transient pool
    ///
    /// Used by `EffectContext::place_card_under_permanent_bottom` to locate
    /// cards before tucking them under a permanent regardless of which zone
    /// they currently live in.
    pub(crate) fn remove_card_from_any_zone(
        &mut self,
        handle: crate::card_source::CardHandle,
    ) -> Option<crate::card_source::CardSource> {
        let player_count = self.players.len();

        for pid in 0..player_count {
            // --- hand ---
            if let Some(pos) = self.players[pid]
                .hand
                .iter()
                .position(|c| c.handle() == handle)
            {
                return Some(self.players[pid].hand.remove(pos));
            }
            // --- trash ---
            if let Some(pos) = self.players[pid]
                .trash
                .iter()
                .position(|c| c.handle() == handle)
            {
                return Some(self.players[pid].trash.remove(pos));
            }
            // --- deck ---
            if let Some(pos) = self.players[pid]
                .deck
                .iter()
                .position(|c| c.handle() == handle)
            {
                return Some(self.players[pid].deck.remove(pos));
            }
            // --- security ---
            if let Some(pos) = self.players[pid]
                .security
                .iter()
                .position(|c| c.handle() == handle)
            {
                return Some(self.players[pid].security.remove(pos));
            }
            // --- battle_area permanent stacks ---
            for perm_idx in 0..self.players[pid].battle_area.len() {
                let stack = &self.players[pid].battle_area[perm_idx].card_sources;
                if let Some(src_pos) = stack.iter().position(|c| c.handle() == handle) {
                    return Some(
                        self.players[pid].battle_area[perm_idx]
                            .card_sources
                            .remove(src_pos),
                    );
                }
            }
            // --- breeding_area ---
            if let Some(ref breeding) = self.players[pid].breeding_area {
                if let Some(src_pos) = breeding
                    .card_sources
                    .iter()
                    .position(|c| c.handle() == handle)
                {
                    return Some(
                        self.players[pid]
                            .breeding_area
                            .as_mut()
                            .unwrap()
                            .card_sources
                            .remove(src_pos),
                    );
                }
            }
        }

        // --- revealed_cards transient pool ---
        if let Some(pos) = self
            .revealed_cards
            .iter()
            .position(|c| c.handle() == handle)
        {
            return Some(self.revealed_cards.remove(pos));
        }

        None
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
    fn scan_before_pay_cost_reduction(&mut self, acting_player: crate::enums::PlayerId) -> i32 {
        let candidates = self.collect_before_pay_cost_reducers(acting_player, None, &[]);
        let mut total = 0;
        for candidate in candidates {
            if candidate.optional || candidate.has_pay_cost {
                self.logger.log(
                    "[Skipped] optional/paid BeforePayCost reducer requires explicit pending play-cost context",
                );
                continue;
            }
            if let Some(amount) = self.apply_cost_reduction_candidate(
                &candidate.key,
                CostTargetContext {
                    card: candidate.key.source_card,
                    from_hand: false,
                },
            ) {
                total += amount;
            }
        }
        total
    }

    fn collect_before_pay_cost_reducers(
        &mut self,
        acting_player: PlayerId,
        cost_target: Option<CostTargetContext>,
        processed: &[CostReductionKey],
    ) -> Vec<CostReductionCandidate> {
        if self
            .modifiers
            .player_has(acting_player, ModifierType::CannotReducePlayCost)
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
            if amount <= 0 {
                continue;
            }
            let Some(effects) = self.effects_for_card(&key.card_id, key.source_card) else {
                continue;
            };
            let Some(effect) = effects.get(key.effect_slot as usize) else {
                continue;
            };
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
            });
        }
        candidates
    }

    fn effect_source_kind_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> EffectSourceKind {
        self.card_kind_for_handle(handle)
            .map(source_kind_for_card_kind)
            .unwrap_or(EffectSourceKind::Rule)
    }

    fn inspect_cost_reduction_candidate(
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
            } else {
                EffectReadContext::new(self, key.source_card, key.source_permanent, key.controller)
            };
            reduction_fn(&ctx).max(0)
        } else {
            effect.cost_reduction.max(0)
        };
        Some(amount)
    }

    fn apply_cost_reduction_candidate(
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
            if !pay_cost_fn(&mut ctx) {
                return None;
            }
        }
        if effect.max_per_turn > 0 {
            self.record_cost_reducer_activation(key);
        }
        Some(amount)
    }

    fn cost_reducer_activation_count(&self, key: &CostReductionKey) -> u8 {
        let Some(source) = key.source_permanent else {
            return 0;
        };
        if source.index == crate::action::space::BREEDING_TARGET as u8 {
            return self
                .player(source.player)
                .breeding_area
                .as_ref()
                .map(|perm| perm.activation_count(key.source_card, key.effect_slot))
                .unwrap_or(0);
        }
        self.player(source.player)
            .battle_area
            .get(source.index as usize)
            .map(|perm| perm.activation_count(key.source_card, key.effect_slot))
            .unwrap_or(0)
    }

    fn record_cost_reducer_activation(&mut self, key: &CostReductionKey) {
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

    fn before_pay_cost_source_infos(
        &self,
        acting_player: PlayerId,
        cost_target_card: Option<crate::card_source::CardHandle>,
    ) -> Vec<BeforePayCostSourceInfo> {
        let mut infos = Vec::new();
        self.push_breeding_cost_sources(acting_player, &mut infos);
        for pid in 0..self.players.len() {
            let player_id = pid as PlayerId;
            let perm_count = self.player(player_id).battle_area.len();
            for perm_idx in 0..perm_count {
                let perm_handle = PermanentHandle {
                    player: player_id,
                    index: perm_idx as u8,
                };
                let stack_size = self.player(player_id).battle_area[perm_idx]
                    .card_sources
                    .len();
                for source_idx in 0..stack_size {
                    let source =
                        &self.player(player_id).battle_area[perm_idx].card_sources[source_idx];
                    self.push_cost_source_info(
                        &mut infos,
                        Some(perm_handle),
                        source,
                        source_idx + 1 < stack_size,
                        player_id,
                        false,
                    );
                }
            }
            if player_id != acting_player {
                self.push_breeding_cost_sources(player_id, &mut infos);
            }
        }
        if let Some(target) = cost_target_card {
            if let Some((card_id, controller)) = self.card_id_and_owner_for_handle(target) {
                let Some(effects) = self.effects_for_card(&card_id, target) else {
                    return infos;
                };
                for (slot, effect) in effects.iter().enumerate() {
                    if effect.timing == EffectTiming::BeforePayCost && effect.when_playing_this {
                        infos.push(BeforePayCostSourceInfo {
                            source_permanent: None,
                            source_card: target,
                            card_id: card_id.clone(),
                            is_under: false,
                            controller,
                            effect_slot: slot as u8,
                        });
                    }
                }
            }
        }
        infos
    }

    fn push_breeding_cost_sources(
        &self,
        player_id: PlayerId,
        infos: &mut Vec<BeforePayCostSourceInfo>,
    ) {
        let Some(perm) = self.player(player_id).breeding_area.as_ref() else {
            return;
        };
        let stack_size = perm.card_sources.len();
        let handle = PermanentHandle {
            player: player_id,
            index: crate::action::space::BREEDING_TARGET as u8,
        };
        for source_idx in 0..stack_size {
            let source = &perm.card_sources[source_idx];
            self.push_cost_source_info(
                infos,
                Some(handle),
                source,
                source_idx + 1 < stack_size,
                player_id,
                false,
            );
        }
    }

    fn push_cost_source_info(
        &self,
        infos: &mut Vec<BeforePayCostSourceInfo>,
        source_permanent: Option<PermanentHandle>,
        source: &CardSource,
        is_under: bool,
        controller: PlayerId,
        allow_when_playing_this: bool,
    ) {
        let card_id = source.card_id(&self.card_data).to_string();
        let source_card = source.handle();
        let Some(effects) = self.effects_for_card(&card_id, source_card) else {
            return;
        };
        for (slot, effect) in effects.iter().enumerate() {
            if effect.timing != EffectTiming::BeforePayCost {
                continue;
            }
            if effect.when_playing_this && !allow_when_playing_this {
                continue;
            }
            infos.push(BeforePayCostSourceInfo {
                source_permanent,
                source_card,
                card_id: card_id.clone(),
                is_under,
                controller,
                effect_slot: slot as u8,
            });
        }
    }

    fn card_id_and_owner_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> Option<(String, PlayerId)> {
        for player in &self.players {
            for card in &player.hand {
                if card.handle() == handle {
                    return Some((card.card_id(&self.card_data).to_string(), card.owner));
                }
            }
        }
        None
    }

    /// Install a `SelectMaterial` pending selection for DNA digivolve.
    /// Drives a two-stage resolution: stage 1 picks the first material;
    /// stage 2 (installed by the stage-1 callback) picks the second
    /// material. Stage 2 resolves into `Game::dna_digivolve_inner`,
    /// computes the matching `DnaCost` via `get_dna_stacking_order`,
    /// applies `BeforePayCost` reductions, and pays memory.
    pub fn initiate_dna_digivolve(&mut self, player_id: PlayerId, hand_index: usize) -> bool {
        if self.current_phase != GamePhase::Main {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: not in Main phase (phase={:?})",
                self.current_phase
            ));
            return false;
        }
        let player = self.player(player_id);
        if hand_index >= player.hand.len() {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: hand index {} out of range (hand size={})",
                hand_index,
                player.hand.len()
            ));
            return false;
        }
        let card = player.hand[hand_index].clone();
        let evo_meta = &self.card_data[card.data_index];
        if evo_meta.dna_costs.is_empty() {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: card {} has no DNA costs",
                card.card_id(&self.card_data)
            ));
            return false;
        }
        if !crate::dna_digivolve::has_valid_dna_targets(
            evo_meta,
            &player.battle_area,
            &self.card_data,
        ) {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: no valid DNA material pair for {}",
                card.card_id(&self.card_data)
            ));
            return false;
        }

        // Collect valid first-material battle_area indices: those that
        // appear in at least one valid pair (either ordering).
        let mut first_targets: Vec<u16> = Vec::new();
        for i in 0..player.battle_area.len() {
            for j in 0..player.battle_area.len() {
                if i == j {
                    continue;
                }
                if crate::dna_digivolve::can_dna_digivolve(
                    evo_meta,
                    &player.battle_area[i],
                    &player.battle_area[j],
                    &self.card_data,
                ) {
                    first_targets.push(i as u16);
                    break;
                }
            }
        }
        first_targets.sort();
        first_targets.dedup();
        if first_targets.is_empty() {
            self.logger.log(
                "[Rejected] initiate_dna_digivolve: no valid first-material indices after filter",
            );
            return false;
        }

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectMaterial;

        let selecting_player = player_id;
        let source_card = card.handle();

        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Material,
            selecting_player,
            previous_phase,
            valid_action_ids: first_targets,
            is_optional: false,
            prompt: "Select first DNA material".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                // Stage 1 resolution: action_id is the chosen first-material
                // battle_area index for `selecting_player`.
                let first_idx = action_id as usize;
                let first_player = selecting_player;
                let evo_hand_index = hand_index;

                // Defensive: validate the first index against the
                // controller's current battle_area (it could have shifted
                // since selection was installed if a triggered effect
                // removed a permanent during the install drain).
                if first_idx >= game.player(first_player).battle_area.len() {
                    game.logger.log(&format!(
                        "[Rejected] dna_digivolve stage 1: first index {} out of range (battle_area size={})",
                        first_idx,
                        game.player(first_player).battle_area.len()
                    ));
                    return;
                }
                if evo_hand_index >= game.player(first_player).hand.len() {
                    game.logger.log(&format!(
                        "[Rejected] dna_digivolve stage 1: evo hand index {} out of range (hand size={})",
                        evo_hand_index,
                        game.player(first_player).hand.len()
                    ));
                    return;
                }

                // Build valid second-material list for the chosen first.
                let evo_meta =
                    &game.card_data[game.player(first_player).hand[evo_hand_index].data_index];
                let second_targets = crate::dna_digivolve::get_valid_dna_second_targets(
                    evo_meta,
                    first_idx,
                    &game.player(first_player).battle_area,
                    &game.card_data,
                );
                if second_targets.is_empty() {
                    game.logger.log(&format!(
                        "[Rejected] dna_digivolve stage 1: no valid second-material targets for first index {}",
                        first_idx
                    ));
                    return;
                }

                // Install stage-2 selection. previous_phase was Main when
                // stage-1 was installed; we preserve it through the chain
                // so the final resolution returns to Main.
                game.pending_selection = Some(PendingSelection {
                    kind: SelectionKind::Material,
                    selecting_player: first_player,
                    previous_phase,
                    valid_action_ids: second_targets,
                    is_optional: false,
                    prompt: "Select second DNA material".to_string(),
                    effect_choices: None,
                    source_card,
                    source_permanent: None,
                    source_kind: EffectSourceKind::Digimon,
                    callback: Box::new(move |game: &mut Game, action_id: u16| {
                        let second_idx = action_id as usize;
                        game.resolve_dna_digivolve_stage2(
                            first_player,
                            first_idx,
                            second_idx,
                            evo_hand_index,
                        );
                    }),
                    on_decline: None,
                });
                // resolve_generic_selection restored current_phase to
                // previous_phase (Main) before invoking this callback. We
                // re-flip it back to SelectMaterial so the action mask
                // reflects the live stage-2 prompt.
                game.current_phase = GamePhase::SelectMaterial;
            }),
            on_decline: None,
        });
        true
    }

    /// Stage-2 resolution of `Game::initiate_dna_digivolve`'s two-stage
    /// selection chain. Receives the chosen second-material `battle_area`
    /// index and the captured stage-1 state. Re-resolves the matching
    /// `DnaCost` orientation, applies `BeforePayCost` reductions, calls
    /// `Game::dna_digivolve_inner`, and triggers the auto-end-of-turn
    /// check (mirroring `digivolve_from_hand`).
    ///
    /// Defensively re-validates indices because triggered effects fired
    /// during stage-1 install can mutate the battle area between selection
    /// install and resolution.
    ///
    /// Failure paths log `[Rejected] ...` via `self.logger` and return
    /// without mutating game state. The `pending_selection` was already
    /// consumed by `resolve_generic_selection` before this method ran.
    pub(crate) fn resolve_dna_digivolve_stage2(
        &mut self,
        first_player: PlayerId,
        first_idx: usize,
        second_idx: usize,
        evo_hand_index: usize,
    ) {
        if second_idx >= self.player(first_player).battle_area.len() {
            self.logger.log(&format!(
                "[Rejected] resolve_dna_digivolve_stage2: second index {} out of range (battle_area size={})",
                second_idx,
                self.player(first_player).battle_area.len()
            ));
            return;
        }
        if first_idx == second_idx {
            self.logger
                .log("[Rejected] resolve_dna_digivolve_stage2: first and second indices are equal");
            return;
        }
        if evo_hand_index >= self.player(first_player).hand.len() {
            self.logger.log(&format!(
                "[Rejected] resolve_dna_digivolve_stage2: evo hand index {} out of range (hand size={})",
                evo_hand_index,
                self.player(first_player).hand.len()
            ));
            return;
        }

        let evo_meta = &self.card_data[self.player(first_player).hand[evo_hand_index].data_index];
        let battle = &self.player(first_player).battle_area;
        let perm_first = &battle[first_idx];
        let perm_second = &battle[second_idx];

        let stacking = crate::dna_digivolve::get_dna_stacking_order(
            evo_meta,
            perm_first,
            perm_second,
            &self.card_data,
        );
        let Some((first_is_top, dna_cost)) = stacking else {
            self.logger.log(
                "[Rejected] resolve_dna_digivolve_stage2: no matching DnaCost for chosen pair",
            );
            return;
        };
        let printed_cost = dna_cost.memory_cost;

        let (target_a, target_b) = if first_is_top {
            (
                PermanentHandle {
                    player: first_player,
                    index: first_idx as u8,
                },
                PermanentHandle {
                    player: first_player,
                    index: second_idx as u8,
                },
            )
        } else {
            (
                PermanentHandle {
                    player: first_player,
                    index: second_idx as u8,
                },
                PermanentHandle {
                    player: first_player,
                    index: first_idx as u8,
                },
            )
        };

        let total_reduction = self.scan_before_pay_cost_reduction(first_player);
        let effective_cost = (printed_cost as i32 - total_reduction).max(0) as u16;

        let _ = self.dna_digivolve_inner(
            target_a,
            target_b,
            first_player,
            evo_hand_index,
            effective_cost,
            true,
        );

        self.check_turn_end();
    }

    /// Move a card from `source` to `player_id`'s security stack at the given
    /// `position` (Top, Bottom, Random). If `face_up` is true, the card's
    /// `card_index` is inserted into `face_up_security` so subsequent reveals
    /// know it was placed face-up. Returns false if the source index is invalid.
    ///
    /// Does not fire `OnLoseSecurity` or any security-related observers.
    ///
    /// Phase 7 Task 4: fires `WhenWouldPlaceInSecurity` at entry. Subject
    /// carries the card handle via the source zone; cause is inferred.
    /// v1 redirect accepts `Zone::Trash` only (card goes to trash instead of
    /// the security stack); other redirect destinations are a `debug_assert!`
    /// + fallthrough.
    pub fn place_on_security(
        &mut self,
        player_id: PlayerId,
        source: crate::enums::CardSourceRef,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Snapshot the source card's handle before the take so we can build
        // a meaningful ReplacementSubject. Return false early if the source
        // is invalid (matches the existing pre-flight behavior of the take).
        let source_card: crate::card_source::CardHandle = match source {
            crate::enums::CardSourceRef::Hand(p, i) => {
                let player = self.player(p);
                if i >= player.hand.len() {
                    return false;
                }
                player.hand[i].handle()
            }
            crate::enums::CardSourceRef::Trash(p, i) => {
                let player = self.player(p);
                if i >= player.trash.len() {
                    return false;
                }
                player.trash[i].handle()
            }
            crate::enums::CardSourceRef::DeckTop(p) => {
                let player = self.player(p);
                let Some(top) = player.deck.last() else {
                    return false;
                };
                top.handle()
            }
            crate::enums::CardSourceRef::Reveal(h) => {
                if !self.revealed_cards.iter().any(|c| c.handle() == h) {
                    return false;
                }
                h
            }
        };
        let source_zone = match source {
            crate::enums::CardSourceRef::Hand(_, _) => Zone::Hand,
            crate::enums::CardSourceRef::Trash(_, _) => Zone::Trash,
            crate::enums::CardSourceRef::DeckTop(_) => Zone::Deck,
            crate::enums::CardSourceRef::Reveal(_) => Zone::Reveal,
        };

        let cause = self.infer_effect_cause(player_id);
        let subject = ReplacementSubject::Card(source_card, source_zone);

        let outcome = self.try_replace(
            EffectTiming::WhenWouldPlaceInSecurity,
            subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() {
            return false;
        }
        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return false;
            }
            ReplacementOutcome::Redirected(Zone::Trash) => {
                // Redirect: card goes to its owner's trash instead of
                // security. Take the source and route it.
                let taken = match source {
                    crate::enums::CardSourceRef::Hand(p, i) => self.player_mut(p).hand.remove(i),
                    crate::enums::CardSourceRef::Trash(source_p, source_i) => {
                        // Task 4 v1: cross-player trash-to-trash redirects are rare
                        // in printed cards (a trash-to-security play being redirected
                        // TO trash is niche). For source_p == player_id this is a
                        // true no-op. For source_p != player_id, a strict reading
                        // would move the card from source_p.trash to player_id.trash;
                        // we preserve source location to avoid a hidden cross-player
                        // move. TODO(phase-7-followup): verify printed-card need.
                        debug_assert!(
                            source_p == player_id,
                            "redirect-to-Trash from cross-player trash is a v1 no-op; card stayed in source_p={} trash (player_id={}, source_i={})",
                            source_p, player_id, source_i
                        );
                        return false;
                    }
                    crate::enums::CardSourceRef::DeckTop(p) => {
                        let Some(c) = self.player_mut(p).deck.pop() else {
                            return false;
                        };
                        c
                    }
                    crate::enums::CardSourceRef::Reveal(h) => {
                        let Some(idx) = self.revealed_cards.iter().position(|c| c.handle() == h)
                        else {
                            return false;
                        };
                        self.revealed_cards.remove(idx)
                    }
                };
                self.player_mut(player_id).trash.push(taken);
                return false;
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected redirect destination for WhenWouldPlaceInSecurity: {:?}",
                    other
                );
                // Fallthrough and commit the original place.
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "substitute subject not supported for WhenWouldPlaceInSecurity v1"
                );
                // Fallthrough.
            }
        }

        // Take the card out of its source zone. Mirror the pattern from
        // place_as_bottom_source.
        let taken = match source {
            crate::enums::CardSourceRef::Hand(p, i) => {
                let player = self.player_mut(p);
                if i >= player.hand.len() {
                    return false;
                }
                player.hand.remove(i)
            }
            crate::enums::CardSourceRef::Trash(p, i) => {
                let player = self.player_mut(p);
                if i >= player.trash.len() {
                    return false;
                }
                player.trash.remove(i)
            }
            crate::enums::CardSourceRef::DeckTop(p) => {
                let Some(c) = self.player_mut(p).deck.pop() else {
                    return false;
                };
                c
            }
            crate::enums::CardSourceRef::Reveal(h) => {
                let Some(idx) = self.revealed_cards.iter().position(|c| c.handle() == h) else {
                    return false;
                };
                self.revealed_cards.remove(idx)
            }
        };

        // face_up_security is HashSet<u16> keyed by card_index.
        let face_up_key = taken.card_index;

        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).security.push(taken);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).security.insert(0, taken);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                // Split-borrow: read length from immutable borrow first, then
                // mutably insert — mirrors the pattern in return_to_deck.
                let sec_len = self.player(player_id).security.len();
                let idx = if sec_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=sec_len)
                };
                self.player_mut(player_id).security.insert(idx, taken);
            }
        }

        if face_up {
            self.player_mut(player_id)
                .face_up_security
                .insert(face_up_key);
        }
        true
    }

    /// Script-initiated digivolve: place the card at `hand_index` from
    /// `player_id`'s hand onto `target`, bypassing the phase check and
    /// optionally the color check. Memory is paid according to `cost_delta`.
    ///
    /// Unlike `digivolve_from_hand`, this does **not** check `GamePhase::Main`
    /// or fire `check_turn_end` — it is designed for use inside effect
    /// callbacks where those invariants don't apply. It also does **not**
    /// draw a card (that's a player-action benefit, not an effect mechanic).
    ///
    /// Returns `true` on success, `false` if validation fails (bad index,
    /// no matching evo cost, or insufficient memory).
    pub fn effect_initiated_digivolve(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
        _source: PlaySource,
    ) -> bool {
        // 1. Validate hand index and target index.
        {
            let player = self.player(player_id);
            if hand_index >= player.hand.len() {
                self.logger.log(&format!(
                    "[Rejected] effect_initiated_digivolve: hand index {} out of range (hand size={})",
                    hand_index,
                    player.hand.len()
                ));
                return false;
            }
        }
        {
            let target_player = self.player(target.player);
            if (target.index as usize) >= target_player.battle_area.len() {
                self.logger.log(&format!(
                    "[Rejected] effect_initiated_digivolve: target index {} out of range (battle_area size={})",
                    target.index,
                    target_player.battle_area.len()
                ));
                return false;
            }
        }

        // 2. Find a matching evo cost.
        let (evo_card_data_index, base_level, base_colors) = {
            let player = self.player(player_id);
            let card = &player.hand[hand_index];
            let target_player = self.player(target.player);
            let perm = &target_player.battle_area[target.index as usize];
            let Some(base_level) = perm.top_card().level(&self.card_data) else {
                self.logger
                    .log("[Rejected] effect_initiated_digivolve: target top card has no level");
                return false;
            };
            let base_colors = perm.top_card().colors(&self.card_data);
            (card.data_index, base_level, base_colors)
        };

        let evo_costs = &self.card_data[evo_card_data_index].evo_costs;
        let matching_cost = evo_costs.iter().find(|ec| {
            ec.level == base_level
                && (ignore_color
                    || crate::action::mask::evo_color(ec.card_color)
                        .map(|c| base_colors.contains(&c))
                        .unwrap_or(false))
        });
        let Some(matching) = matching_cost else {
            self.logger.log(&format!(
                "[Rejected] effect_initiated_digivolve: no matching evo cost (base_level={}, ignore_color={})",
                base_level, ignore_color
            ));
            return false;
        };
        let base_cost = cost_delta.resolve(matching.memory_cost);
        let total_reduction = self.scan_before_pay_cost_reduction(player_id);
        let effective_cost = (base_cost as i32 - total_reduction).max(0) as u16;

        // 3. Pay memory.
        if !self.pay_memory(effective_cost) {
            self.logger.log(&format!(
                "[Rejected] effect_initiated_digivolve: cannot pay memory cost {} (current memory={})",
                effective_cost, self.memory
            ));
            return false;
        }

        // 4. Move the card from hand onto the target permanent's stack.
        let turn = self.turn_count;
        let card = self.player_mut(player_id).hand.remove(hand_index);
        self.player_mut(target.player).battle_area[target.index as usize].digivolve(card, turn);

        // 5. Fire WhenDigivolving triggers.
        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(target),
        );
        self.drain_effect_queue();

        // OnDigivolve: global observer — fires in every player's battle area
        // after the evolving permanent's WhenDigivolving resolves. Distinct
        // from WhenDigivolving (self-timing on the evolving permanent).
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnDigivolve,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();

        true
    }
}
