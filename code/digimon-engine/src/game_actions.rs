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
    PlayerId, Zone,
};
use crate::game::Game;
use crate::game::{
    PendingWouldDigivolveResume, PendingWouldLinkResume, PendingWouldPlayOrigin,
    PendingWouldPlayResume,
};
use crate::permanent::PermanentHandle;
use crate::selection::{
    OptionPlayResult, OptionResolutionPhase, OptionSubtype, OptionUseSource, PendingOption,
    PendingSelection, QueuedEffect, SelectionKind, TriggerSource,
};
use rand::seq::SliceRandom;

/// Source zone for `play_option_core`. Private to this module — the public
/// API is the pair of `play_option_from_hand` / `play_option_from_trash`
/// entry points.
#[derive(Clone, Copy, Debug)]
enum OptionSource {
    Hand(usize),
    Trash(usize),
}

struct TakenCardSource {
    card: CardSource,
    restore_face_up_security_for: Option<PlayerId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CostReductionKind {
    Play,
    Digivolve,
    OptionUse,
}

impl OptionSource {
    fn use_source(self) -> OptionUseSource {
        match self {
            OptionSource::Hand(_) => OptionUseSource::Hand,
            OptionSource::Trash(_) => OptionUseSource::Trash,
        }
    }
}

/// One available play mode for an Option card. `classify_option_modes`
/// derives the set of modes from the card's effect list. Most Options have
/// exactly one mode; a dual-mode Plug-In Option that is both a Standard
/// `[Main]` Option and a Link Option has two (`Standard`, then `Link`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionPlayMode {
    /// Played as a normal `[Main]` Option — pay the printed use cost,
    /// resolve the `OptionMain` body, dispose to trash.
    Standard,
    /// A Delay Option — parks on the field until its delay trigger.
    Delay(crate::enums::DelayTrigger),
    /// Plugged in via Link Requirements — pay `cost`, attach sideways to a
    /// host Digimon. No `[Main]` / `[Security]` effect runs.
    Link { cost: u16 },
    /// A Training Option.
    Training,
}

impl OptionPlayMode {
    /// The disposal subtype this play mode resolves to.
    fn subtype(self) -> OptionSubtype {
        match self {
            OptionPlayMode::Standard => OptionSubtype::Standard,
            OptionPlayMode::Delay(trigger) => OptionSubtype::Delay(trigger),
            OptionPlayMode::Link { .. } => OptionSubtype::Link,
            OptionPlayMode::Training => OptionSubtype::Training,
        }
    }

    fn is_link(self) -> bool {
        matches!(self, OptionPlayMode::Link { .. })
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
    /// True when this cost is a DIGIVOLVE cost (normal or DNA). Surfaced to
    /// predicates via `EffectReadContext::cost_is_digivolve` so the
    /// `when_any_ally_digivolves_into` cost-reduction trigger fires only for
    /// digivolutions. `G-COST-REDUCTION-DIGIVOLVE-INTO`.
    is_digivolve: bool,
    /// Permanents being digivolved (single entry for normal digivolve,
    /// two for DNA digivolve; both `None` for play-from-hand / option
    /// use). Fixed-size to preserve `Copy`; surfaced to predicates via
    /// `EffectReadContext::cost_target_permanents` as a `Vec`. Used by
    /// the `source_is_cost_target_permanent` predicate
    /// (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET).
    target_permanents: [Option<crate::permanent::PermanentHandle>; 2],
}

impl CostTargetContext {
    fn target_permanents_vec(&self) -> Vec<crate::permanent::PermanentHandle> {
        self.target_permanents.iter().filter_map(|h| *h).collect()
    }
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

/// Inspect an Option's effect list to derive every available play mode.
///
/// Returns a 1- or 2-element list. `Delay` and `Training` are exclusive
/// whole-card subtypes (a card carrying either has exactly that one mode).
/// Otherwise the card may have a Standard `[Main]` mode (it carries a
/// non-link `OptionMain` body) and/or a Link mode (it carries a
/// `link_requirement` effect with a `link_cost`). A Plug-In Option that
/// has both is **dual-mode**: the list is `[Standard, Link]`, in that
/// order, and the player picks via a mode-select prompt.
fn classify_option_modes(effects: &[crate::effect::Effect]) -> Vec<OptionPlayMode> {
    let mut delay = None;
    let mut training = false;
    let mut link_cost: Option<u16> = None;
    let mut has_standard_main = false;
    for eff in effects {
        if let Some(trigger) = eff.delay_trigger {
            delay = Some(trigger);
        }
        if eff.training {
            training = true;
        }
        if let Some(cost) = eff.link_cost {
            link_cost = Some(cost);
        } else if eff.timing == EffectTiming::OptionMain {
            // A non-link `OptionMain` effect is a Standard `[Main]` body.
            has_standard_main = true;
        }
    }
    // Delay / Training are exclusive whole-card subtypes.
    if let Some(trigger) = delay {
        return vec![OptionPlayMode::Delay(trigger)];
    }
    if training {
        return vec![OptionPlayMode::Training];
    }
    let mut modes = Vec::new();
    // A card with no link effect is always Standard (the fallback for
    // `[Security]`-only Options too); a card with a link effect is
    // Standard only when it additionally carries a `[Main]` body.
    if has_standard_main || link_cost.is_none() {
        modes.push(OptionPlayMode::Standard);
    }
    if let Some(cost) = link_cost {
        modes.push(OptionPlayMode::Link { cost });
    }
    modes
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
        self.play_from_hand_with_cost_result_from_origin(
            player_id,
            hand_index,
            cost_delta,
            source,
            cost_target_from_hand,
            PendingWouldPlayOrigin::Hand,
        )
    }

    pub(crate) fn play_from_hand_with_cost_result_from_origin(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        cost_target_from_hand: bool,
        origin: PendingWouldPlayOrigin,
    ) -> PlayFromHandCostResult {
        self.play_from_hand_with_cost_result_from_origin_suppress(
            player_id,
            hand_index,
            cost_delta,
            source,
            cost_target_from_hand,
            origin,
            false,
        )
    }

    /// As [`Self::play_from_hand_with_cost_result_from_origin`], but threads a
    /// `suppress_on_play` flag (PUPPETS-G030). When `true`, the just-played
    /// permanent's own `[On Play]` effects are skipped for this play event;
    /// every other timing and every other permanent are unaffected.
    pub(crate) fn play_from_hand_with_cost_result_from_origin_suppress(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        cost_target_from_hand: bool,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
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
                is_digivolve: false,
                target_permanents: [None, None],
            },
            cost_delta,
            source,
            origin,
            suppress_on_play,
            0,
            Vec::new(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn continue_play_from_hand_cost_reduction_chain(
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
                return self.finish_play_from_hand_after_reductions(
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

    #[allow(clippy::too_many_arguments)]
    fn finish_play_from_hand_after_reductions(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target_card: crate::card_source::CardHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
        total_reduction: i32,
    ) -> PlayFromHandCostResult {
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
        // Observer dispatch (G-BEFORE-PAY-COST-GAIN-MEMORY) — fires AFTER
        // the cost-reduction chain finishes (`total_reduction` is the sum
        // of accepted reducers) but BEFORE the WhenPermanentWouldPlay
        // replacement and final memory deduction.
        let cost_target_ctx = CostTargetContext {
            card: target_card,
            from_hand: true,
            is_digivolve: false,
            target_permanents: [None, None],
        };
        self.scan_before_pay_cost_observers(player_id, Some(cost_target_ctx));
        let effective_cost = (base_cost - total_reduction).max(0) as u16;

        self.pending_would_play_resume = Some(PendingWouldPlayResume {
            player: player_id,
            card: target_card,
            effective_cost,
            origin,
            effect_initiated: source == PlaySource::ByEffect,
            suppress_on_play,
        });
        let cause = match source {
            PlaySource::ByEffect => crate::replacement::ReplacementCause::OwnEffect,
            PlaySource::ByHand | PlaySource::ByDigivolve => {
                crate::replacement::ReplacementCause::OwnEffect
            }
        };
        let outcome = self.try_replace(
            EffectTiming::WhenPermanentWouldPlay,
            crate::replacement::ReplacementSubject::Card(target_card, Zone::Hand),
            cause,
            Some(Zone::BattleArea),
        );
        if self.pending_selection.is_some() {
            return PlayFromHandCostResult::Pending;
        }
        match outcome {
            crate::replacement::ReplacementOutcome::None => {
                self.pending_would_play_resume = None;
            }
            crate::replacement::ReplacementOutcome::Cancelled
            | crate::replacement::ReplacementOutcome::CustomHandled => {
                self.pending_would_play_resume = None;
                return PlayFromHandCostResult::Failed;
            }
            crate::replacement::ReplacementOutcome::Redirected(_)
            | crate::replacement::ReplacementOutcome::Substituted(_) => {
                self.pending_would_play_resume = None;
                return PlayFromHandCostResult::Failed;
            }
        }

        self.commit_play_from_hand_card_no_replace(
            player_id,
            target_card,
            effective_cost,
            source == PlaySource::ByEffect,
            suppress_on_play,
        )
        .map(PlayFromHandCostResult::Played)
        .unwrap_or(PlayFromHandCostResult::Failed)
    }

    pub(crate) fn commit_pending_would_play(
        &mut self,
        outcome: crate::replacement::ReplacementOutcome,
    ) {
        let Some(resume) = self.pending_would_play_resume.take() else {
            return;
        };
        match outcome {
            crate::replacement::ReplacementOutcome::None => {
                let _ = self.commit_play_from_hand_card_no_replace(
                    resume.player,
                    resume.card,
                    resume.effective_cost,
                    resume.effect_initiated,
                    resume.suppress_on_play,
                );
            }
            crate::replacement::ReplacementOutcome::Cancelled
            | crate::replacement::ReplacementOutcome::CustomHandled => {
                self.restore_pending_would_play_origin(resume);
            }
            crate::replacement::ReplacementOutcome::Redirected(_)
            | crate::replacement::ReplacementOutcome::Substituted(_) => {
                self.restore_pending_would_play_origin(resume);
            }
        }
    }

    fn restore_pending_would_play_origin(&mut self, resume: PendingWouldPlayResume) {
        let Some(hand_index) = self
            .player(resume.player)
            .hand
            .iter()
            .position(|card| card.handle() == resume.card)
        else {
            return;
        };
        match resume.origin {
            PendingWouldPlayOrigin::Hand => {}
            PendingWouldPlayOrigin::Trash { index } => {
                let card = self.player_mut(resume.player).hand.remove(hand_index);
                let insert_at = index.min(self.player(resume.player).trash.len());
                self.player_mut(resume.player).trash.insert(insert_at, card);
            }
            PendingWouldPlayOrigin::SecurityTop { was_face_up } => {
                let card = self.player_mut(resume.player).hand.remove(hand_index);
                let card_index = card.card_index;
                self.player_mut(resume.player).security.push(card);
                if was_face_up {
                    self.player_mut(resume.player)
                        .face_up_security
                        .insert(card_index);
                }
            }
            PendingWouldPlayOrigin::Source {
                permanent,
                source_index,
            } => {
                if permanent.player != resume.player {
                    return;
                }
                let card = self.player_mut(resume.player).hand.remove(hand_index);
                let Some(perm) = self
                    .player_mut(resume.player)
                    .battle_area
                    .get_mut(permanent.index as usize)
                else {
                    self.player_mut(resume.player).trash.push(card);
                    return;
                };
                let insert_at = source_index.min(perm.card_sources.len());
                perm.card_sources.insert(insert_at, card);
            }
        }
    }

    fn commit_play_from_hand_card_no_replace(
        &mut self,
        player_id: PlayerId,
        target_card: crate::card_source::CardHandle,
        effective_cost: u16,
        effect_initiated: bool,
        suppress_on_play: bool,
    ) -> Option<usize> {
        if self.player(player_id).battle_area.len() >= self.rules.field_slots as usize {
            return None;
        }
        let hand_index = self
            .player(player_id)
            .hand
            .iter()
            .position(|card| card.handle() == target_card)?;
        if !self.pay_memory(effective_cost) {
            return None;
        }

        let turn = self.turn_count;
        let card = self.player_mut(player_id).hand.remove(hand_index);
        let perm = crate::permanent::Permanent::new(card, turn);
        self.player_mut(player_id).battle_area.push(perm);
        let field_index = self.player(player_id).battle_area.len() - 1;
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

        // PUPPETS-G030 — `suppress_on_play` skips ONLY the just-played
        // permanent's own `[On Play]` enqueue, and only for this play event.
        // `OnEnterFieldAnyone` / `OnAllyPlayed` broadcasts below, and every
        // other permanent's triggers, are untouched. Used by BT5-106's
        // [Security] clause ("Any [On Play] effects on Digimon played with
        // this effect don't activate.").
        if !suppress_on_play {
            self.fire_on_play(player_id, field_index);
        }
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnEnterFieldAnyone,
            crate::selection::TriggerSource::EnteredField {
                player: player_id,
                permanent: entered,
                card: entered_card,
                effect_initiated,
            },
        );
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnAllyPlayed,
            crate::selection::TriggerSource::EnteredField {
                player: player_id,
                permanent: entered,
                card: entered_card,
                effect_initiated,
            },
        );
        self.drain_effect_queue();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();

        Some(field_index)
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
        self.play_from_trash_with_cost_suppress(player_id, trash_index, cost_delta, source, false)
    }

    /// As [`Self::play_from_trash_with_cost`], but threads a `suppress_on_play`
    /// flag (PUPPETS-G030). When `true`, the just-played permanent's own
    /// `[On Play]` effects are skipped for this play event only. Used by
    /// BT5-106's [Security] clause.
    pub fn play_from_trash_with_cost_suppress(
        &mut self,
        player_id: PlayerId,
        trash_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        suppress_on_play: bool,
    ) -> Option<usize> {
        if self
            .modifiers
            .player_has(player_id, ModifierType::CannotPlayFromTrash)
        {
            return None;
        }
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

        match self.play_from_hand_with_cost_result_from_origin_suppress(
            player_id,
            hand_index,
            cost_delta,
            source,
            false,
            PendingWouldPlayOrigin::Trash { index: trash_index },
            suppress_on_play,
        ) {
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

    /// Candidate host Digimon for a Link Option attach: every Standard-state
    /// own Digimon that is below its link cap and passes the card's
    /// `link_filter`. Shared by `dispose_option`'s Link arm and the
    /// dual-mode legality check in `option_legal_play_modes`.
    pub(crate) fn link_host_candidates(
        &self,
        owner: PlayerId,
        source_card: crate::card_source::CardHandle,
        effects: &[crate::effect::Effect],
    ) -> Vec<PermanentHandle> {
        let mut out: Vec<PermanentHandle> = Vec::new();
        for (i, perm) in self.player(owner).battle_area.iter().enumerate() {
            let handle = PermanentHandle {
                player: owner,
                index: i as u8,
            };
            if !self.permanent_is_digimon_for_rules(handle) {
                continue;
            }
            if !matches!(perm.option_state, crate::permanent::OptionState::Standard) {
                continue;
            }
            let link_max =
                (5 + self.modifiers.link_max_delta(handle)).clamp(0, u8::MAX as i32) as usize;
            if perm.linked_cards.len() >= link_max {
                continue;
            }
            let filter_ok =
                effects
                    .iter()
                    .find(|e| e.link_cost.is_some())
                    .map_or(true, |link_effect| {
                        if let Some(f) = &link_effect.link_filter {
                            let read_ctx = EffectReadContext::new(self, source_card, None, owner);
                            f(&read_ctx, handle)
                        } else {
                            true
                        }
                    });
            if filter_ok {
                out.push(handle);
            }
        }
        out
    }

    /// The set of play modes `player_id` may **afford** for `card` right
    /// now. A dual-mode Plug-In Option yields `[Standard, Link]` when both
    /// fit the memory budget (the player then picks via the mode-select);
    /// a single affordable mode plays directly; an empty result means the
    /// Option cannot be played at all.
    ///
    /// Only affordability is filtered here — host availability for a Link
    /// play is resolved later by `dispose_option` (a Link play with no
    /// eligible host trashes the card, identical to a single-mode Link
    /// Option). This keeps `PLAY_HAND` masking and the mode-select offer
    /// consistent with the engine's existing Link-Option contract.
    pub(crate) fn option_legal_play_modes(
        &self,
        card: &CardSource,
        player_id: PlayerId,
    ) -> Vec<OptionPlayMode> {
        let effects = self
            .effects_for_card(card.card_id(&self.card_data), card.handle())
            .unwrap_or_default();
        let use_cost = card
            .option_use_cost(&self.card_data)
            .unwrap_or_else(|| card.play_cost(&self.card_data));
        let memory_min = self.rules.memory_range.0;
        classify_option_modes(&effects)
            .into_iter()
            .filter(|mode| {
                let cost = match mode {
                    OptionPlayMode::Link { cost } => (*cost as i32
                        + self.modifiers.link_cost_delta_for_player(player_id))
                    .max(0) as i16,
                    _ => use_cost as i16,
                };
                (self.memory - cost) >= memory_min
            })
            .collect()
    }

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
        self.play_option_core(player_id, OptionSource::Hand(hand_index), None)
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
        self.play_option_core(player_id, OptionSource::Trash(trash_index), None)
    }

    /// Shared Option-play pipeline. Forks on source zone (hand vs trash)
    /// and on the resolved play mode (Standard / Delay / Link / Training).
    ///
    /// `chosen_mode` is `None` for a fresh play; for a dual-mode Plug-In
    /// Option the first call installs a mode-select `pending_selection` and
    /// returns `Pending`, and the selection callback re-enters with the
    /// chosen mode as `Some(_)`.
    fn play_option_core(
        &mut self,
        player_id: PlayerId,
        source: OptionSource,
        chosen_mode: Option<OptionPlayMode>,
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
            // Standard / Delay / Training: the printed use cost, less any
            // BeforePayCost reduction.
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
    fn install_option_mode_select(
        &mut self,
        player_id: PlayerId,
        source: OptionSource,
        card_handle: crate::card_source::CardHandle,
        legal_modes: Vec<OptionPlayMode>,
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
                let _ = game.play_option_core(player_id, source, Some(mode));
            }),
            on_decline: None,
        });
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
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                controller: owner,
                timing: EffectTiming::WhenDigivolving,
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
            for idx in 0..self.players[pid].battle_area.len() {
                let handle = PermanentHandle {
                    player: pid as PlayerId,
                    index: idx as u8,
                };
                if self.permanent_is_digimon_for_rules(handle)
                    && self.effective_dp(handle).unwrap_or(1) <= 0
                {
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
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                controller: owner,
                timing: EffectTiming::CounterEffect,
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
        // The disposal subtype was fixed at play time (`play_option_core`
        // stores the resolved mode on `pending_option`) — a dual-mode
        // Plug-In Option must not be re-classified here.
        let subtype = pending.subtype;

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
                        subtype: pending.subtype,
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
                    trigger,
                    placed_on_turn: turn,
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
                        permanent: Some(permanent),
                        linked_host: None,
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
                // Standard-state Digimon on the owner's battle_area (shared
                // helper `link_host_candidates`). If no candidate passes,
                // trash the card silently (mirrors "no legal target" for
                // other effect selections). Otherwise install a
                // PendingSelection routed to `attach_linked_card` and park
                // `pending_option` in `LinkSelectHost`.
                let owner = pending.owner;
                let source_card = pending.card.handle();
                let candidates = self.link_host_candidates(owner, source_card, &effects);

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
                    subtype: pending.subtype,
                });
                self.install_link_host_selection(owner, source_card, candidates, false);
            }
            OptionSubtype::Training => {
                // Phase 8 Task 5: park as an `OptionState::Training` permanent on
                // the owner's battle_area. Stays there until the owner hatches
                // an egg via `move_from_breeding`, at which point every Training
                // permanent the owner controls fires `OnTrainingTrash` and is
                // trashed (see `Game::move_from_breeding`). Training sideways-
                // inheritance is dispatched in `enqueue_from_permanent`.
                let owner = pending.owner;
                let placed_card = pending.card.handle();
                let turn = self.turn_count;
                let mut perm = crate::permanent::Permanent::new(pending.card, turn);
                perm.option_state = crate::permanent::OptionState::Training {
                    owner,
                    trained: None,
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
                        permanent: Some(permanent),
                        linked_host: None,
                        card: placed_card,
                    },
                );
                self.drain_effect_queue();
                if self.pending_selection.is_some() {
                    self.pending_option_placed_turn_check = true;
                }
            }
        }
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
    pub(crate) fn install_link_host_selection(
        &mut self,
        owner: PlayerId,
        source_card: crate::card_source::CardHandle,
        candidates: Vec<PermanentHandle>,
        optional: bool,
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
            is_optional: optional,
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
            on_decline: optional.then(|| {
                Box::new(move |game: &mut Game| {
                    if let Some(pending) = game.pending_option.take() {
                        game.player_mut(pending.owner).trash.push(pending.card);
                        game.check_turn_end();
                    }
                }) as Box<dyn FnOnce(&mut Game) + Send + Sync>
            }),
        });
    }

    /// Complete a Link Option's attach: push the pending card into the
    /// host's `linked_cards`, fire `OnLink` globally, and clear
    /// `pending_option`. The caller has already validated that `host` was
    /// in the candidate list at selection install-time, but we re-check the
    /// handle is still live in case an intervening effect moved things.
    pub(crate) fn attach_linked_card(&mut self, host: PermanentHandle) {
        let Some(pending_card_handle) = self
            .pending_option
            .as_ref()
            .map(|pending| pending.card.handle())
        else {
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
                self.permanent_is_digimon_for_rules(host)
                    && matches!(p.option_state, crate::permanent::OptionState::Standard)
            })
            .unwrap_or(false);
        if !host_live {
            let Some(pending) = self.pending_option.take() else {
                return;
            };
            self.player_mut(pending.owner).trash.push(pending.card);
            self.check_turn_end();
            return;
        }

        self.pending_would_link_resume = Some(PendingWouldLinkResume {
            host,
            card: pending_card_handle,
        });
        let outcome = self.try_replace(
            EffectTiming::WhenWouldLink,
            crate::replacement::ReplacementSubject::Card(pending_card_handle, Zone::Reveal),
            crate::replacement::ReplacementCause::OwnEffect,
            Some(Zone::BattleArea),
        );
        if self.pending_selection.is_some() {
            return;
        }
        self.commit_pending_would_link(outcome);
    }

    pub(crate) fn commit_pending_would_link(
        &mut self,
        outcome: crate::replacement::ReplacementOutcome,
    ) {
        let Some(resume) = self.pending_would_link_resume.take() else {
            return;
        };
        match outcome {
            crate::replacement::ReplacementOutcome::None => {
                self.commit_linked_card_no_replace(resume);
            }
            crate::replacement::ReplacementOutcome::Cancelled
            | crate::replacement::ReplacementOutcome::CustomHandled
            | crate::replacement::ReplacementOutcome::Redirected(_)
            | crate::replacement::ReplacementOutcome::Substituted(_) => {
                if let Some(pending) = self.pending_option.take() {
                    self.player_mut(pending.owner).trash.push(pending.card);
                }
                self.check_turn_end();
            }
        }
    }

    fn commit_linked_card_no_replace(&mut self, resume: PendingWouldLinkResume) {
        let Some(pending) = self.pending_option.take() else {
            return;
        };

        if pending.card.handle() != resume.card {
            self.player_mut(pending.owner).trash.push(pending.card);
            self.check_turn_end();
            return;
        }

        let host = resume.host;
        let host_live = self
            .player(host.player)
            .battle_area
            .get(host.index as usize)
            .map(|p| {
                self.permanent_is_digimon_for_rules(host)
                    && matches!(p.option_state, crate::permanent::OptionState::Standard)
            })
            .unwrap_or(false);
        if !host_live {
            self.player_mut(pending.owner).trash.push(pending.card);
            self.check_turn_end();
            return;
        }

        // Attach.
        let linked_card = pending.card.handle();
        self.player_mut(host.player).battle_area[host.index as usize]
            .linked_cards
            .push(pending.card);

        self.enqueue_triggered(
            EffectTiming::OnOptionPlaced,
            TriggerSource::OptionPlaced {
                player: pending.owner,
                permanent: None,
                linked_host: Some(host),
                card: linked_card,
            },
        );
        self.drain_effect_queue();
        if self.pending_selection.is_some() {
            self.pending_option_placed_link_resume = Some(host);
            return;
        }

        self.fire_on_link_after_option_placed();
    }

    fn fire_on_link_after_option_placed(&mut self) {
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

    pub(crate) fn resume_pending_option_placed_link(&mut self) {
        if self.pending_option_placed_link_resume.is_none() {
            return;
        }
        if self.pending_selection.is_some() || !self.effect_queue.is_empty() {
            return;
        }
        self.pending_option_placed_link_resume = None;
        self.fire_on_link_after_option_placed();
    }

    /// Compute the absolute `turn_count` at which a delayed Option should
    /// self-trash. The rule is "end/start of the **owner**'s next turn" for
    /// next-turn triggers, and the current turn for `EndOfThisTurn`.
    ///
    /// In a 2-player round-robin:
    /// - If `owner == turn_player` (the common case — played on own turn),
    ///   "next own turn" lands `turn_count + 2` (skip the opponent's turn).
    /// - If `owner != turn_player` (played during opponent's turn, e.g. via
    ///   a Counter window), "next own turn" lands `turn_count + 1`.
    ///
    /// Multi-player extension is deferred — the plan locks 2-player
    /// semantics for v1.
    pub(crate) fn compute_delay_trash_turn(
        &self,
        owner: PlayerId,
        trigger: crate::enums::DelayTrigger,
    ) -> u16 {
        use crate::enums::DelayTrigger;
        match trigger {
            DelayTrigger::EndOfThisTurn => self.turn_count,
            DelayTrigger::EndOfYourNextTurn | DelayTrigger::StartOfYourNextTurn => {
                self.next_owner_turn_count(owner)
            }
            // Standard `<Delay>` is activated by a player `[Main]`-phase
            // action, not a turn-keyed auto-trash scan. `OnEvent` likewise
            // has no scheduled turn — both park indefinitely.
            DelayTrigger::MainPhaseActivated | DelayTrigger::OnEvent(_) => u16::MAX,
        }
    }

    fn next_owner_turn_count(&self, owner: PlayerId) -> u16 {
        let Some(owner_idx) = self.turn_order.iter().position(|&p| p == owner) else {
            return self.turn_count;
        };
        let turn_delta = if owner_idx > self.turn_player_idx {
            owner_idx - self.turn_player_idx
        } else {
            owner_idx + self.turn_order.len() - self.turn_player_idx
        };
        self.turn_count + turn_delta as u16
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
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
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
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    /// Move a specific card from `player_id`'s security stack to their hand.
    /// Returns false if the handle is not in that player's security stack.
    pub fn add_to_hand_from_security(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(idx) = self
            .player(player_id)
            .security
            .iter()
            .position(|c| c.handle() == card)
        else {
            return false;
        };
        let removed = self.player_mut(player_id).security.remove(idx);
        let owner = removed.owner;
        self.player_mut(player_id)
            .face_up_security
            .remove(&removed.card_index);
        self.player_mut(owner).add_to_hand(removed);
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
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

    pub fn reveal_top_digitama(
        &mut self,
        player_id: PlayerId,
        n: u8,
    ) -> Vec<crate::card_source::CardHandle> {
        let mut handles = Vec::new();
        for _ in 0..n {
            let p = self.player_mut(player_id);
            let Some(card) = p.digitama_deck.pop() else {
                break;
            };
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

    /// Shuffle `player_id`'s security stack without changing its contents.
    pub fn shuffle_security(&mut self, player_id: PlayerId) {
        let mut security = std::mem::take(&mut self.player_mut(player_id).security);
        security.shuffle(&mut self.rng);
        self.player_mut(player_id).security = security;
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
        // See G-DSL-OUTER-TAIL-NESTED-PARK fix note in
        // `fire_on_link_after_option_placed`.
        self.maybe_drain_effect_queue();
    }

    /// Activate a `[Main]` effect on the card at `player_id`'s hand slot
    /// `hand_index`. Returns `true` if a matching effect fired, `false` if no
    /// `EffectTiming::MainFromHand`/`OptionMain` effect on the card was legal.
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
        let (card_id, handle, card_kind) = {
            let player = match self.players.get(player_id as usize) {
                Some(p) => p,
                None => return false,
            };
            let card = match player.hand.get(hand_index) {
                Some(c) => c,
                None => return false,
            };
            (
                card.card_id(&self.card_data).to_string(),
                card.handle(),
                card.card_kind(&self.card_data),
            )
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

        let main_timing_matches = |timing: EffectTiming| {
            timing == EffectTiming::MainFromHand
                || (matches!(card_kind, CardKind::Option | CardKind::Dual)
                    && timing == EffectTiming::OptionMain)
        };

        for effect in &effects {
            if !main_timing_matches(effect.timing) {
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

        // PUPPETS-G009 — standard `<Delay>` `[Main]`-phase activation. A
        // parked `DelayTrigger::MainPhaseActivated` Option whose placing turn
        // has passed is activated by trashing it as the cost and running its
        // stored `<Delay>` body. Dispatched before the ordinary `MainOnField`
        // scan because the Delay body lives at `EffectTiming::DelayEffect`,
        // not `MainOnField`.
        let delay_handle = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        if self.delayed_option_main_activation_available(delay_handle) {
            return self.activate_delayed_option_main(delay_handle);
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
        let mut taken = self.revealed_cards.remove(pos);
        taken.clear_reveal_overlay();
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
        let mut taken = self.revealed_cards.remove(pos);
        taken.clear_reveal_overlay();
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
        let mut taken = self.revealed_cards.remove(pos_idx);
        taken.clear_reveal_overlay();
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
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
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

        // Snapshot the leaving permanent's identity BEFORE removal so the
        // OnLeaveField observer's `event_target_*` predicates resolve.
        let leave_snapshot = self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .and_then(|p| {
                let top_handle = p.top_card().handle();
                let data = self.card_data_for_handle(top_handle)?;
                let mut digisources: Vec<crate::card_source::CardHandle> = Vec::new();
                for src in p.card_sources.iter() {
                    let h = src.handle();
                    if h != top_handle {
                        digisources.push(h);
                    }
                }
                let source_count = digisources.len();
                let dp_now = self.effective_dp(handle);
                Some(crate::trigger_context::DeletedObjectSnapshot {
                    former_controller: handle.player,
                    top_card: top_handle,
                    card_kind: data.card_kind,
                    traits: data.traits.clone(),
                    level: data.level,
                    dp: dp_now,
                    cause: crate::trigger_context::EventCause::Return,
                    dp_just_before: dp_now,
                    level_just_before: data.level,
                    cost_just_before: Some(data.play_cost),
                    names_just_before: vec![data.card_name.clone()],
                    traits_just_before: data.traits.clone(),
                    source_count_just_before: source_count,
                    digisources_just_before: digisources,
                })
            });

        let perm = self
            .player_mut(handle.player)
            .battle_area
            .remove(handle.index as usize);

        let mut sources = perm.card_sources;
        let Some(top) = sources.pop() else {
            return None;
        };
        let top_handle = top.handle();
        let top_owner = top.owner;
        let mut leaving_sources = sources.clone();
        leaving_sources.push(top.clone());
        self.apply_ace_overflow_for_sources(&leaving_sources);
        // Owner-routed: top card returns to its owner's hand, not the
        // controller's. Track E correctness rule. Identical when
        // `top_owner == handle.player` (the common case today).
        self.player_mut(top_owner).hand.push(top);

        // Sources below the top go to each source's owner's trash and fire
        // OnDigivolutionCardTrashed (digivolution stack sources only — not
        // linked_cards which are Tamer equipment and separate semantic
        // category). Owner-routed.
        for card in sources {
            let source_card = card.handle();
            // Owner-routed: each source returns to its OWN owner's trash,
            // not the controller's (Track E correctness rule). The trigger
            // attribution still uses `handle.player` (the host's controller)
            // for event-source-player binding.
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
            self.fire_digivolution_card_trashed(
                handle.player,
                handle,
                top_handle,
                source_card,
                crate::trigger_context::EventCause::Return,
            );
        }
        let had_linked = !perm.linked_cards.is_empty();
        for card in perm.linked_cards {
            // Owner-routed: linked cards return to their own owner's trash.
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
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

        self.clear_permanent_full(handle);
        // Phase 6: expire any player-scoped modifiers sourced from this permanent.
        self.modifiers.expire_player_on_permanent_leave(handle);
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        // OnLeaveField: the permanent left the battle area by return-to-hand.
        if let Some(snapshot) = leave_snapshot {
            self.fire_on_leave_field(handle, snapshot);
        }
        Some(top_handle)
    }

    /// Low-level source-attribution helper for tests and engine internals.
    ///
    /// The underlying movement still routes through `return_to_hand`, including
    /// replacement windows and source-disposition triggers; this wrapper only
    /// supplies effect source attribution so opponent-only protection can
    /// distinguish own effects from opponent effects. Production card effects
    /// should prefer `EffectContext::return_to_hand`, which also enforces
    /// `can_affect_permanent` gates and uses real source metadata.
    #[doc(hidden)]
    pub fn return_to_hand_from_effect(
        &mut self,
        handle: PermanentHandle,
        effect_player: PlayerId,
    ) -> bool {
        let previous = self.effect_source_player;
        self.effect_source_player = Some(effect_player);
        let moved = self.return_to_hand(handle).is_some();
        self.effect_source_player = previous;
        moved
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
        self.return_to_deck_inner(handle, position, false)
    }

    /// Low-level source-attribution helper for tests and engine internals.
    ///
    /// This is the source-attributed companion to `return_to_deck`; callers
    /// that need production effect semantics should prefer
    /// `EffectContext::return_to_deck`, which also enforces
    /// `can_affect_permanent` gates and uses real source metadata.
    #[doc(hidden)]
    pub fn return_to_deck_from_effect(
        &mut self,
        handle: PermanentHandle,
        effect_player: PlayerId,
    ) -> bool {
        let previous = self.effect_source_player;
        self.effect_source_player = Some(effect_player);
        let moved = self.return_to_deck(handle, crate::enums::StackPosition::Bottom);
        self.effect_source_player = previous;
        moved
    }

    /// Fire the global `OnLeaveField` observer for a permanent that has just
    /// left the battle area by a non-deletion route (return-to-hand,
    /// return-to-deck). The deletion route fires `OnLeaveField` from
    /// `finalize_permanent_deletion_with_event_card`. `snapshot` carries the
    /// leaving permanent's identity so `event_target_*` predicates resolve
    /// against it, exactly as the deletion path does. Called AFTER the
    /// permanent is removed from `battle_area`.
    pub(crate) fn fire_on_leave_field(
        &mut self,
        handle: PermanentHandle,
        snapshot: crate::trigger_context::DeletedObjectSnapshot,
    ) {
        let card = snapshot.top_card;
        let queue_start = self.effect_queue.len();
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnLeaveField,
            crate::selection::TriggerSource::EventObserved {
                player: handle.player,
                permanent: handle,
                card,
            },
        );
        for queued in self.effect_queue.iter_mut().skip(queue_start) {
            if queued.timing != crate::enums::EffectTiming::OnLeaveField {
                continue;
            }
            if let Some(trigger) = queued.trigger_context.as_mut() {
                trigger.deleted_object = Some(snapshot.clone());
                trigger.cause = Some(snapshot.cause);
                trigger.affected_player = Some(snapshot.former_controller);
                trigger.subject = Some(crate::trigger_context::EventSubject::Permanent(handle));
            }
        }
        // G-DSL-OUTER-TAIL-NESTED-PARK: maybe_drain defers when inside
        // a select-callback / outer-tail scope.
        self.maybe_drain_effect_queue();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    pub(crate) fn fire_digivolution_card_trashed(
        &mut self,
        player: PlayerId,
        host: PermanentHandle,
        host_card: crate::card_source::CardHandle,
        card: crate::card_source::CardHandle,
        cause: crate::trigger_context::EventCause,
    ) {
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnDigivolutionCardTrashed,
            crate::selection::TriggerSource::SourceTrashedFromStack {
                player,
                host,
                host_card,
                card,
                cause,
            },
        );
        // Intentionally NOT routed through maybe_drain: EX10-036 (and
        // similar multi-source trash chains) rely on observers firing
        // synchronously between source trashes so secondary clauses can
        // pick up the just-trashed cards mid-resolution. Behavioral test
        // `ex10_036_clause_a_after_source_trash_prompts_opp_field_delete`
        // documents the expected interleaving. Other observer fires
        // (place_security, leave_field, link, attack, play) are deferred.
        self.drain_effect_queue();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Low-level source-attribution helper for tests and engine internals.
    ///
    /// Uses the standard De-Digivolve floor (`stop_at_level = Some(3)`) and
    /// returns whether at least one card was popped. Replacement windows are
    /// resolved by `EffectContext::de_digivolve` under the supplied source
    /// attribution. Production card effects should prefer an existing
    /// `EffectContext` so `can_affect_permanent` and source-kind metadata come
    /// from the real resolving card.
    #[doc(hidden)]
    pub fn de_digivolve_from_effect(
        &mut self,
        handle: PermanentHandle,
        effect_player: PlayerId,
        amount: u8,
    ) -> bool {
        let previous = self.effect_source_player;
        self.effect_source_player = Some(effect_player);
        let popped = {
            let mut ctx =
                EffectContext::new(self, crate::card_source::CardHandle(0), None, effect_player);
            ctx.de_digivolve(handle, Some(3), Some(amount))
        };
        self.effect_source_player = previous;
        popped > 0
    }

    /// Return a permanent's full stack to its owner's deck at `position`.
    /// Preserves bottom-to-top source order in the deck instead of trashing
    /// lower digivolution sources.
    pub fn return_stack_to_deck(
        &mut self,
        handle: PermanentHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        self.return_to_deck_inner(handle, position, true)
    }

    fn return_to_deck_inner(
        &mut self,
        handle: PermanentHandle,
        position: crate::enums::StackPosition,
        include_sources: bool,
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
                return self.return_to_deck_inner(other, position, include_sources);
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "non-Permanent substitute subject for WhenWouldBeReturnedToDeck"
                );
            }
        }

        // Snapshot the leaving permanent's identity BEFORE removal so the
        // OnLeaveField observer's `event_target_*` predicates resolve.
        let leave_snapshot = self
            .player(player_id)
            .battle_area
            .get(handle.index as usize)
            .and_then(|p| {
                let top_handle = p.top_card().handle();
                let data = self.card_data_for_handle(top_handle)?;
                let mut digisources: Vec<crate::card_source::CardHandle> = Vec::new();
                for src in p.card_sources.iter() {
                    let h = src.handle();
                    if h != top_handle {
                        digisources.push(h);
                    }
                }
                let source_count = digisources.len();
                let dp_now = self.effective_dp(handle);
                Some(crate::trigger_context::DeletedObjectSnapshot {
                    former_controller: player_id,
                    top_card: top_handle,
                    card_kind: data.card_kind,
                    traits: data.traits.clone(),
                    level: data.level,
                    dp: dp_now,
                    cause: match position {
                        crate::enums::StackPosition::Bottom => {
                            crate::trigger_context::EventCause::DeckBottom
                        }
                        _ => crate::trigger_context::EventCause::Return,
                    },
                    dp_just_before: dp_now,
                    level_just_before: data.level,
                    cost_just_before: Some(data.play_cost),
                    names_just_before: vec![data.card_name.clone()],
                    traits_just_before: data.traits.clone(),
                    source_count_just_before: source_count,
                    digisources_just_before: digisources,
                })
            });

        let mut perm = self
            .player_mut(player_id)
            .battle_area
            .remove(handle.index as usize);

        let Some(top) = perm.card_sources.pop() else {
            return false;
        };
        let mut leaving_sources = perm.card_sources.clone();
        leaving_sources.push(top.clone());
        self.apply_ace_overflow_for_sources(&leaving_sources);

        if include_sources {
            perm.card_sources.push(top);
            self.insert_stack_into_owners_decks(perm.card_sources, position);
        } else {
            // Owner-routed: top card returns to its OWN owner's deck.
            // Track E correctness rule. Identical to controller-routed
            // when owner == controller (the common case today); diverges
            // if a future control-transfer effect sets owner != controller.
            let top_owner = top.owner;
            let host_card = top.handle();
            self.insert_card_into_deck(top_owner, top, position);

            // Sources below the top → each source's OWN owner's trash.
            // Trigger uses Track A's fire_digivolution_card_trashed helper
            // which carries the EventCause for downstream payload binding.
            for card in perm.card_sources {
                let source_card = card.handle();
                let owner = card.owner;
                self.player_mut(owner).trash.push(card);
                self.fire_digivolution_card_trashed(
                    handle.player,
                    handle,
                    host_card,
                    source_card,
                    match position {
                        crate::enums::StackPosition::Bottom => {
                            crate::trigger_context::EventCause::DeckBottom
                        }
                        _ => crate::trigger_context::EventCause::Return,
                    },
                );
            }
        }

        let had_linked = !perm.linked_cards.is_empty();
        for card in perm.linked_cards {
            // Owner-routed: linked cards return to their own owner's trash.
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
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

        self.clear_permanent_full(handle);
        // Phase 6: expire any player-scoped modifiers sourced from this permanent.
        self.modifiers.expire_player_on_permanent_leave(handle);
        // OnLeaveField: the permanent left the battle area by return-to-deck.
        if let Some(snapshot) = leave_snapshot {
            self.fire_on_leave_field(handle, snapshot);
        }
        true
    }

    fn insert_card_into_deck(
        &mut self,
        player_id: PlayerId,
        card: CardSource,
        position: crate::enums::StackPosition,
    ) {
        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).deck.push(card);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).deck.insert(0, card);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let deck_len = self.player(player_id).deck.len();
                let idx = if deck_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=deck_len)
                };
                self.player_mut(player_id).deck.insert(idx, card);
            }
        }
    }

    fn insert_stack_into_deck(
        &mut self,
        player_id: PlayerId,
        stack: Vec<CardSource>,
        position: crate::enums::StackPosition,
    ) {
        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).deck.extend(stack);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).deck.splice(0..0, stack);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let deck_len = self.player(player_id).deck.len();
                let idx = if deck_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=deck_len)
                };
                self.player_mut(player_id).deck.splice(idx..idx, stack);
            }
        }
    }

    fn insert_stack_into_owners_decks(
        &mut self,
        stack: Vec<CardSource>,
        position: crate::enums::StackPosition,
    ) {
        let player_count = self.players.len();
        for player_id in 0..player_count {
            let owner = player_id as PlayerId;
            let owned_stack: Vec<CardSource> = stack
                .iter()
                .filter(|card| card.owner == owner)
                .cloned()
                .collect();
            if !owned_stack.is_empty() {
                self.insert_stack_into_deck(owner, owned_stack, position);
            }
        }
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
        source: PlaySource,
    ) -> bool {
        self.digivolve_from_hand_inner(player_id, hand_index, field_index, source, false)
    }

    /// As [`Self::digivolve_from_hand`], but threads a `player_reducer_resolved`
    /// flag (`G-COST-REDUCE-ALLY-DIGIVOLVE`). When `false` (the default user
    /// call), the function first consults `Game::player_digivolve_cost_reducers`
    /// and — if a reducer qualifies and is payable — installs an interactive
    /// accept/decline prompt, returning `false` without performing the
    /// digivolution; the accept/decline callbacks re-invoke this function with
    /// the flag `true` and a pre-resolved `pending_player_digivolve_reduction`.
    /// When `true`, the player-scoped reducer prompt is skipped (it has already
    /// been resolved this attempt).
    fn digivolve_from_hand_inner(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
        player_reducer_resolved: bool,
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
        let Some(route) = self.normal_digivolve_route_for_hand_card(player_id, hand_index, handle)
        else {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: card {} cannot digivolve onto {} (evo-cost mismatch)",
                card.card_id(&self.card_data),
                perm.top_card().card_id(&self.card_data),
            ));
            return false;
        };
        let printed_cost = route.memory_cost;

        // G-COST-REDUCE-ALLY-DIGIVOLVE — player-scoped one-shot future-digivolve
        // cost reducer prompt. Runs BEFORE the synchronous field-hosted
        // BeforePayCost scan; if a reducer qualifies and is payable, this
        // installs an interactive accept/decline PendingSelection and returns
        // — the accept/decline callbacks re-enter `digivolve_from_hand_inner`
        // with `player_reducer_resolved = true`.
        if !player_reducer_resolved
            && self.try_prompt_player_digivolve_cost_reducer(
                player_id,
                handle,
                hand_index,
                field_index,
                source,
            )
        {
            return false;
        }
        // Pre-resolved player-scoped reduction (set by the accept callback,
        // 0 on decline). Consumed once here.
        let player_reduction = std::mem::take(&mut self.pending_player_digivolve_reduction);

        // Pass the hand card being digivolved into as the cost-target so
        // target-aware predicates (`cost_target: { trait_has: Free }`)
        // can fire — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET. `handle` is the
        // permanent being digivolved into; threaded as a target permanent
        // so `source_is_cost_target_permanent` gates self-targeted
        // observers.
        let target = CostTargetContext {
            card: card.handle(),
            from_hand: true,
            is_digivolve: true,
            target_permanents: [Some(handle), None],
        };
        let total_reduction = self.scan_before_pay_cost_reduction_with_target(
            player_id,
            CostReductionKind::Digivolve,
            Some(target),
        ) + player_reduction;
        // Fire BeforePayCost observers (e.g. gain_memory) AFTER reduction
        // is computed but BEFORE pay_memory — G-BEFORE-PAY-COST-GAIN-MEMORY.
        self.scan_before_pay_cost_observers(player_id, Some(target));
        let effective_cost = (printed_cost as i32 - total_reduction).max(0) as u16;

        self.pending_would_digivolve_resume = Some(PendingWouldDigivolveResume {
            player: player_id,
            permanent: handle,
            card: card.handle(),
            effective_cost,
        });
        let cause = match source {
            PlaySource::ByEffect => crate::replacement::ReplacementCause::OwnEffect,
            PlaySource::ByHand | PlaySource::ByDigivolve => {
                crate::replacement::ReplacementCause::OwnEffect
            }
        };
        let outcome = self.try_replace(
            EffectTiming::WhenPermanentWouldDigivolve,
            crate::replacement::ReplacementSubject::Permanent(handle),
            cause,
            Some(Zone::BattleArea),
        );
        if self.pending_selection.is_some() {
            return false;
        }
        match outcome {
            crate::replacement::ReplacementOutcome::None => {
                self.pending_would_digivolve_resume = None;
            }
            crate::replacement::ReplacementOutcome::Cancelled
            | crate::replacement::ReplacementOutcome::CustomHandled => {
                self.pending_would_digivolve_resume = None;
                return false;
            }
            crate::replacement::ReplacementOutcome::Redirected(_)
            | crate::replacement::ReplacementOutcome::Substituted(_) => {
                self.pending_would_digivolve_resume = None;
                return false;
            }
        }

        self.commit_digivolve_from_hand_no_replace(PendingWouldDigivolveResume {
            player: player_id,
            permanent: handle,
            card: card.handle(),
            effective_cost,
        })
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
    fn try_prompt_player_digivolve_cost_reducer(
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

    /// Battle-area field indices of `player`'s unsuspended Digimon — the
    /// legal suspend-cost targets for `G-PAY-COST-SELECT-ARBITRARY-SUSPEND`.
    fn suspendable_own_digimon(&self, player: PlayerId) -> Vec<usize> {
        self.player(player)
            .battle_area
            .iter()
            .enumerate()
            .filter(|(_, perm)| !perm.is_suspended && perm.is_digimon(&self.card_data))
            .map(|(i, _)| i)
            .collect()
    }

    /// Install the accept/decline `PendingSelection` for a player-scoped
    /// digivolve cost reducer. On accept → install the suspend-cost
    /// selection, then re-enter the digivolve with the reduction applied.
    /// On decline → re-enter the digivolve at full cost (reducer stays
    /// armed). `G-COST-REDUCE-ALLY-DIGIVOLVE`.
    fn install_player_digivolve_reducer_prompt(
        &mut self,
        reducer_idx: usize,
        reducer: crate::player_cost_reducer::PlayerDigivolveCostReducer,
        acting_player: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
    ) {
        use crate::action::space::HAND_EFFECT_START;
        use crate::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};

        let source_card = reducer.source_card;
        let source_kind = self.effect_source_kind_for_handle(source_card);
        let amount = reducer.amount;

        // Accept branch — pay the suspend cost (if any), apply the reduction,
        // consume the reducer if single-fire, then re-enter the digivolve.
        let accept = {
            let reducer = reducer.clone();
            move |game: &mut Game, _action_id: u16| {
                game.player_digivolve_reducer_accept(
                    reducer_idx,
                    reducer,
                    acting_player,
                    hand_index,
                    field_index,
                    source,
                );
            }
        };
        // Decline branch — leave the reducer armed, re-enter the digivolve
        // at the unreduced cost.
        let decline = move |game: &mut Game| {
            game.pending_player_digivolve_reduction = 0;
            game.digivolve_from_hand_inner(acting_player, hand_index, field_index, source, true);
        };

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;
        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::EffectChoice,
            selecting_player: acting_player,
            previous_phase,
            valid_action_ids: vec![HAND_EFFECT_START],
            is_optional: true,
            prompt: format!(
                "Suspend 1 of your Digimon to reduce the digivolution cost by {}?",
                amount
            ),
            effect_choices: Some(vec![EffectChoiceEntry {
                label: format!("Suspend 1 Digimon (digivolution cost -{})", amount),
                action_id: HAND_EFFECT_START,
                source_card: Some(source_card),
                source_kind: Some(source_kind),
                timing: Some(crate::enums::EffectTiming::BeforePayCost),
                is_optional: true,
                observation_metadata: Default::default(),
            }]),
            source_card,
            source_permanent: None,
            source_kind,
            callback: Box::new(accept),
            on_decline: Some(Box::new(decline)),
        });
    }

    /// Accept-branch continuation for a player-scoped digivolve cost
    /// reducer: install the suspend-cost selection (`select 1 unsuspended
    /// own Digimon`). On suspend resolution → suspend it, apply the
    /// reduction, consume the reducer if single-fire, and re-enter the
    /// digivolve. `G-COST-REDUCE-ALLY-DIGIVOLVE` / `G-PAY-COST-SELECT-ARBITRARY-SUSPEND`.
    fn player_digivolve_reducer_accept(
        &mut self,
        reducer_idx: usize,
        reducer: crate::player_cost_reducer::PlayerDigivolveCostReducer,
        acting_player: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
    ) {
        use crate::action::space::{encode_attack, ATTACK_START, TARGETS_PER_ATTACKER};
        use crate::selection::{PendingSelection, SelectionKind};

        let amount = reducer.amount;
        let single_fire = reducer.single_fire;
        let source_card = reducer.source_card;
        let source_kind = self.effect_source_kind_for_handle(source_card);

        if !reducer.suspend_cost {
            // No suspend cost — apply the reduction directly.
            self.consume_player_digivolve_reducer(reducer_idx, &reducer, single_fire);
            self.pending_player_digivolve_reduction = amount;
            self.digivolve_from_hand_inner(acting_player, hand_index, field_index, source, true);
            return;
        }

        let suspendable = self.suspendable_own_digimon(acting_player);
        if suspendable.is_empty() {
            // Cost became unpayable between prompt-install and accept — leave
            // the reducer armed and continue at the unreduced cost.
            self.pending_player_digivolve_reduction = 0;
            self.digivolve_from_hand_inner(acting_player, hand_index, field_index, source, true);
            return;
        }

        let valid_action_ids: Vec<u16> = suspendable
            .iter()
            .map(|i| encode_attack(0, *i as u16))
            .collect();

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectTarget;
        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::OwnField,
            selecting_player: acting_player,
            previous_phase,
            valid_action_ids,
            is_optional: false,
            prompt: "Suspend 1 of your Digimon (digivolution cost reduction)".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let offset = action_id.saturating_sub(ATTACK_START);
                let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let suspend_target = PermanentHandle {
                    player: acting_player,
                    index: target_index,
                };
                game.suspend(suspend_target);
                game.consume_player_digivolve_reducer(reducer_idx, &reducer, single_fire);
                game.pending_player_digivolve_reduction = amount;
                game.digivolve_from_hand_inner(
                    acting_player,
                    hand_index,
                    field_index,
                    source,
                    true,
                );
            }),
            on_decline: None,
        });
    }

    /// Remove a single-fire player-scoped digivolve cost reducer after a
    /// successful application. The reducer is located by identity (player +
    /// source card + amount) rather than by stale index, since the vector
    /// may have shifted between prompt-install and resolution.
    fn consume_player_digivolve_reducer(
        &mut self,
        reducer_idx: usize,
        reducer: &crate::player_cost_reducer::PlayerDigivolveCostReducer,
        single_fire: bool,
    ) {
        if !single_fire {
            return;
        }
        // Prefer the recorded index when it still points at the same reducer;
        // otherwise re-locate by identity.
        if self
            .player_digivolve_cost_reducers
            .get(reducer_idx)
            .is_some_and(|r| {
                r.player == reducer.player
                    && r.source_card == reducer.source_card
                    && r.amount == reducer.amount
            })
        {
            self.player_digivolve_cost_reducers.remove(reducer_idx);
            return;
        }
        if let Some(pos) = self.player_digivolve_cost_reducers.iter().position(|r| {
            r.player == reducer.player
                && r.source_card == reducer.source_card
                && r.amount == reducer.amount
        }) {
            self.player_digivolve_cost_reducers.remove(pos);
        }
    }

    pub(crate) fn commit_pending_would_digivolve(
        &mut self,
        outcome: crate::replacement::ReplacementOutcome,
    ) {
        let Some(resume) = self.pending_would_digivolve_resume.take() else {
            return;
        };
        match outcome {
            crate::replacement::ReplacementOutcome::None => {
                let _ = self.commit_digivolve_from_hand_no_replace(resume);
            }
            crate::replacement::ReplacementOutcome::Cancelled
            | crate::replacement::ReplacementOutcome::CustomHandled => {}
            crate::replacement::ReplacementOutcome::Redirected(_)
            | crate::replacement::ReplacementOutcome::Substituted(_) => {}
        }
    }

    fn commit_digivolve_from_hand_no_replace(
        &mut self,
        resume: PendingWouldDigivolveResume,
    ) -> bool {
        let field_index = resume.permanent.index as usize;
        if resume.permanent.player != resume.player {
            return false;
        }
        if self
            .player(resume.player)
            .battle_area
            .get(field_index)
            .is_none()
        {
            return false;
        }
        if self
            .modifiers
            .has(resume.permanent, ModifierType::CannotDigivolve)
        {
            return false;
        }

        let Some(hand_index) = self
            .player(resume.player)
            .hand
            .iter()
            .position(|card| card.handle() == resume.card)
        else {
            return false;
        };
        let Some(_route) =
            self.normal_digivolve_route_for_hand_card(resume.player, hand_index, resume.permanent)
        else {
            return false;
        };

        let (from_stack_top, top_card_id) = {
            let player = self.player(resume.player);
            let perm = &player.battle_area[field_index];
            let card = &player.hand[hand_index];
            (
                perm.top_card().card_id(&self.card_data).to_string(),
                card.card_id(&self.card_data).to_string(),
            )
        };

        if !self.pay_memory(resume.effective_cost) {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: cannot pay memory cost {} (current memory={})",
                resume.effective_cost, self.memory
            ));
            return false;
        }

        let turn = self.turn_count;
        let removed = self.player_mut(resume.player).hand.remove(hand_index);
        self.player_mut(resume.player).battle_area[field_index].digivolve(removed, turn);
        let event_card = self
            .player(resume.player)
            .battle_area
            .get(field_index)
            .map(|perm| perm.top_card().handle())
            .expect("digivolve target remains in battle area after stack mutation");

        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Digivolve {
            seq,
            player: resume.player,
            top_card_id,
            field_index: field_index as u8,
            from_stack_top,
        });

        self.player_mut(resume.player).draw();

        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(resume.permanent),
        );
        self.drain_effect_queue();

        // OnDigivolve: global observer — fires in every player's battle area
        // after the evolving permanent's WhenDigivolving resolves. Distinct
        // from WhenDigivolving (self-timing on the evolving permanent).
        self.enqueue_triggered(
            EffectTiming::OnDigivolve,
            TriggerSource::Digivolved {
                player: resume.player,
                permanent: resume.permanent,
                card: event_card,
                effect_initiated: false,
                dna_origin: false,
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

    fn card_source_ref_snapshot(
        &self,
        source: crate::enums::CardSourceRef,
    ) -> Option<(crate::card_source::CardHandle, usize, crate::enums::Zone)> {
        use crate::enums::{CardSourceRef, Zone};
        match source {
            CardSourceRef::Hand(p, i) => self
                .player(p)
                .hand
                .get(i)
                .map(|c| (c.handle(), c.data_index, Zone::Hand)),
            CardSourceRef::Trash(p, i) => self
                .player(p)
                .trash
                .get(i)
                .map(|c| (c.handle(), c.data_index, Zone::Trash)),
            CardSourceRef::DeckTop(p) => self
                .player(p)
                .deck
                .last()
                .map(|c| (c.handle(), c.data_index, Zone::Deck)),
            CardSourceRef::Security(p, i) => self
                .player(p)
                .security
                .get(i)
                .map(|c| (c.handle(), c.data_index, Zone::Security)),
            CardSourceRef::Material(h, i) => self
                .player(h.player)
                .battle_area
                .get(h.index as usize)
                .and_then(|perm| perm.card_sources.get(i))
                .map(|c| (c.handle(), c.data_index, Zone::BattleArea)),
            CardSourceRef::Reveal(h) => self
                .revealed_cards
                .iter()
                .find(|c| c.handle() == h)
                .map(|c| (c.handle(), c.data_index, Zone::Reveal)),
        }
    }

    fn take_card_source_ref(
        &mut self,
        source: crate::enums::CardSourceRef,
    ) -> Option<TakenCardSource> {
        use crate::enums::CardSourceRef;
        let mut face_up_security = None;
        let card = match source {
            CardSourceRef::Hand(p, i) => {
                let player = self.player_mut(p);
                if i >= player.hand.len() {
                    return None;
                }
                player.hand.remove(i)
            }
            CardSourceRef::Trash(p, i) => {
                let player = self.player_mut(p);
                if i >= player.trash.len() {
                    return None;
                }
                player.trash.remove(i)
            }
            CardSourceRef::DeckTop(p) => self.player_mut(p).deck.pop()?,
            CardSourceRef::Security(p, i) => {
                let player = self.player_mut(p);
                if i >= player.security.len() {
                    return None;
                }
                let card = player.security.remove(i);
                if player.face_up_security.remove(&card.card_index) {
                    face_up_security = Some(p);
                }
                card
            }
            CardSourceRef::Material(h, i) => {
                let perm = self
                    .player_mut(h.player)
                    .battle_area
                    .get_mut(h.index as usize)?;
                if i >= perm.card_sources.len() {
                    return None;
                }
                perm.card_sources.remove(i)
            }
            CardSourceRef::Reveal(h) => {
                let idx = self.revealed_cards.iter().position(|c| c.handle() == h)?;
                let mut taken = self.revealed_cards.remove(idx);
                taken.clear_reveal_overlay();
                taken
            }
        };
        Some(TakenCardSource {
            card,
            restore_face_up_security_for: face_up_security,
        })
    }

    fn restore_card_source_ref(
        &mut self,
        source: crate::enums::CardSourceRef,
        taken: TakenCardSource,
    ) -> bool {
        use crate::enums::CardSourceRef;
        let card_index = taken.card.card_index;
        let restored = match source {
            CardSourceRef::Hand(p, i) => {
                let player = self.player_mut(p);
                let idx = i.min(player.hand.len());
                player.hand.insert(idx, taken.card);
                true
            }
            CardSourceRef::Trash(p, i) => {
                let player = self.player_mut(p);
                let idx = i.min(player.trash.len());
                player.trash.insert(idx, taken.card);
                true
            }
            CardSourceRef::DeckTop(p) => {
                self.player_mut(p).deck.push(taken.card);
                true
            }
            CardSourceRef::Security(p, i) => {
                let player = self.player_mut(p);
                let idx = i.min(player.security.len());
                player.security.insert(idx, taken.card);
                true
            }
            CardSourceRef::Material(h, i) => {
                let Some(perm) = self
                    .player_mut(h.player)
                    .battle_area
                    .get_mut(h.index as usize)
                else {
                    return false;
                };
                let idx = i.min(perm.card_sources.len());
                perm.card_sources.insert(idx, taken.card);
                true
            }
            CardSourceRef::Reveal(_) => {
                self.revealed_cards.push(taken.card);
                true
            }
        };
        if restored {
            if let Some(player) = taken.restore_face_up_security_for {
                self.player_mut(player).face_up_security.insert(card_index);
            }
        }
        restored
    }

    /// Insert a card at the bottom of `target`'s digivolution stack. The
    /// source card is taken from the zone specified by `source` (hand slot,
    /// trash slot, deck top, security slot, material stack slot, or reveal
    /// pool). Returns false if the source or target is invalid.
    ///
    /// `face_down` sets the inserted `CardSource.face_down` flag (the DCGO
    /// `IsFlipped` analog for digivolution-stack sources). Pass `true` to
    /// stash a face-down source (e.g. a Tamer face-down stash); `false`
    /// preserves the ordinary face-up placement.
    ///
    /// NOTE: `face_down` is honored only for hand / trash / deck-top /
    /// material / reveal sources placed into the breeding or battle area; it
    /// is **not** honored for `CardSourceRef::Security` sources, which are
    /// always placed face-up (DCGO parity).
    pub fn place_as_bottom_source(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        face_down: bool,
    ) -> bool {
        self.place_as_bottom_source_observed(source, target, target.player, face_down)
    }

    pub(crate) fn place_as_bottom_source_observed(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        observer_player: PlayerId,
        face_down: bool,
    ) -> bool {
        if let crate::enums::CardSourceRef::Security(defender, index) = source {
            if target.index == crate::action::space::BREEDING_TARGET as u8 {
                if self.player(target.player).breeding_area.is_none() {
                    return false;
                }
            } else if self
                .player(target.player)
                .battle_area
                .get(target.index as usize)
                .is_none()
            {
                return false;
            }

            let player = self.player_mut(defender);
            if index >= player.security.len() {
                return false;
            }
            let card = player.security.remove(index);
            player.face_up_security.remove(&card.card_index);
            let cause = crate::trigger_context::EventCause::from(self.infer_effect_cause(defender));
            self.fire_effect_security_removal(
                defender,
                observer_player,
                observer_player,
                cause,
                card,
                crate::selection::SecurityRemovalDestination::BottomSource(target),
            );
            return true;
        }

        let Some(taken) = self.take_card_source_ref(source) else {
            return false;
        };

        if target.index == crate::action::space::BREEDING_TARGET as u8 {
            let Some(breeding) = self.player_mut(target.player).breeding_area.as_mut() else {
                let _ = self.restore_card_source_ref(source, taken);
                return false;
            };
            let mut card = taken.card;
            card.face_down = face_down;
            breeding.push_under(card);
            return true;
        }

        let target_player = self.player_mut(target.player);
        if (target.index as usize) >= target_player.battle_area.len() {
            let _ = self.restore_card_source_ref(source, taken);
            return false;
        }
        let mut card = taken.card;
        card.face_down = face_down;
        target_player.battle_area[target.index as usize].push_under(card);
        true
    }

    pub fn place_permanent_as_bottom_sources(
        &mut self,
        source: PermanentHandle,
        target: PermanentHandle,
    ) -> bool {
        if source.index == crate::action::space::BREEDING_TARGET as u8 {
            return false;
        }
        if source == target {
            return false;
        }
        if self
            .player(source.player)
            .battle_area
            .get(source.index as usize)
            .is_none()
        {
            return false;
        }

        let mut adjusted_target = target;
        if target.index == crate::action::space::BREEDING_TARGET as u8 {
            if self.player(target.player).breeding_area.is_none() {
                return false;
            }
        } else {
            if self
                .player(target.player)
                .battle_area
                .get(target.index as usize)
                .is_none()
            {
                return false;
            }
            if source.player == target.player && source.index < target.index {
                adjusted_target.index = adjusted_target.index.saturating_sub(1);
            }
        }

        let removed = self
            .player_mut(source.player)
            .battle_area
            .remove(source.index as usize);
        let cards = removed.card_sources;

        if adjusted_target.index == crate::action::space::BREEDING_TARGET as u8 {
            let Some(breeding) = self
                .player_mut(adjusted_target.player)
                .breeding_area
                .as_mut()
            else {
                return false;
            };
            breeding.card_sources.splice(0..0, cards);
            return true;
        }

        let Some(target_perm) = self
            .player_mut(adjusted_target.player)
            .battle_area
            .get_mut(adjusted_target.index as usize)
        else {
            return false;
        };
        target_perm.card_sources.splice(0..0, cards);
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
            let mut taken = self.revealed_cards.remove(pos);
            taken.clear_reveal_overlay();
            return Some(taken);
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
    fn scan_before_pay_cost_reduction_with_target(
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

    fn collect_before_pay_cost_reducers(
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

    // ── BeforePayCostObserve dispatch (G-BEFORE-PAY-COST-GAIN-MEMORY) ──
    //
    // Walks the same source list as the cost-reduction scan but matches
    // effects with timing `BeforePayCostObserve` and fires their `process`
    // bodies. Observer bodies typically gain memory or otherwise mutate
    // state during cost calculation; they MUST NOT install a pending
    // selection in v1 (no-approximations §17: surface choices through
    // pending_selection — observer-with-selection support is planned but
    // out of scope for Phase 2 Track H, since BG Imperial's six refs are
    // all scalar `gain_memory` bodies).

    fn before_pay_cost_observer_infos(
        &self,
        acting_player: PlayerId,
        cost_target_card: Option<crate::card_source::CardHandle>,
    ) -> Vec<BeforePayCostSourceInfo> {
        let mut infos = Vec::new();
        self.push_breeding_observer_sources(acting_player, &mut infos);
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
                    self.push_observer_source_info(
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
                self.push_breeding_observer_sources(player_id, &mut infos);
            }
        }
        if let Some(target) = cost_target_card {
            if let Some((card_id, controller)) = self.card_id_and_owner_for_handle(target) {
                if let Some(effects) = self.effects_for_card(&card_id, target) {
                    for (slot, effect) in effects.iter().enumerate() {
                        if effect.timing == EffectTiming::BeforePayCostObserve
                            && effect.when_playing_this
                        {
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
        }
        infos
    }

    fn push_breeding_observer_sources(
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
            self.push_observer_source_info(
                infos,
                Some(handle),
                source,
                source_idx + 1 < stack_size,
                player_id,
                false,
            );
        }
    }

    fn push_observer_source_info(
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
            if effect.timing != EffectTiming::BeforePayCostObserve {
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
    fn scan_before_pay_cost_observers(
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

    fn observer_activation_count(&self, info: &BeforePayCostSourceInfo) -> u8 {
        let Some(source) = info.source_permanent else {
            return 0;
        };
        if source.index == crate::action::space::BREEDING_TARGET as u8 {
            return self
                .player(source.player)
                .breeding_area
                .as_ref()
                .map(|perm| perm.activation_count(info.source_card, info.effect_slot))
                .unwrap_or(0);
        }
        self.player(source.player)
            .battle_area
            .get(source.index as usize)
            .map(|perm| perm.activation_count(info.source_card, info.effect_slot))
            .unwrap_or(0)
    }

    fn record_observer_activation(&mut self, info: &BeforePayCostSourceInfo) {
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
        let Some(route_window) = self.current_dna_route_window() else {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: not in DNA action phase (phase={:?})",
                self.current_phase
            ));
            return false;
        };
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

        // Collect valid first-material battle_area indices: those that
        // appear in at least one valid pair (either ordering).
        let mut first_targets: Vec<u16> = self
            .valid_dna_first_targets_for_hand_card(player_id, hand_index, route_window)
            .collect();
        first_targets.sort();
        first_targets.dedup();
        if first_targets.is_empty() {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: no valid DNA material pair for {}",
                card.card_id(&self.card_data)
            ));
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
                let second_targets: Vec<u16> = game
                    .valid_dna_second_targets_for_hand_card(
                        first_player,
                        evo_hand_index,
                        first_idx,
                        route_window,
                    )
                    .collect();
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
                        game.resolve_dna_digivolve_stage2_with_window(
                            first_player,
                            first_idx,
                            second_idx,
                            evo_hand_index,
                            route_window,
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
    /// DNA route orientation, applies `BeforePayCost` reductions, calls
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
    pub fn resolve_dna_digivolve_stage2_with_window(
        &mut self,
        first_player: PlayerId,
        first_idx: usize,
        second_idx: usize,
        evo_hand_index: usize,
        route_window: crate::dna_digivolve::DnaRouteWindow,
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

        let Some(route_match) = self.dna_route_for_hand_card(
            first_player,
            evo_hand_index,
            first_idx,
            second_idx,
            route_window,
        ) else {
            self.logger.log(
                "[Rejected] resolve_dna_digivolve_stage2: no matching DNA route for chosen pair",
            );
            return;
        };
        let printed_cost = route_match.memory_cost;

        let (target_a, target_b) = if route_match.first_is_top {
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

        // Pass the DNA-result hand card as the cost-target so target-aware
        // predicates (`cost_target: { color_is: green }`) can fire for the
        // DNA digivolve path — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET. Both
        // DNA materials get threaded as target permanents so per-material
        // self-scoped observers ("when THIS Digimon would DNA digivolve")
        // can fire on either side.
        let evo_card_target = self.player(first_player).hand[evo_hand_index].handle();
        let target = CostTargetContext {
            card: evo_card_target,
            from_hand: true,
            is_digivolve: true,
            target_permanents: [Some(target_a), Some(target_b)],
        };
        // Set the DNA-origin context so cost-calc-time predicates like
        // `dna_origin: true` evaluate true for both the reducer scan and
        // the observer scan. Restored after both scans complete so the
        // marker doesn't leak into downstream effect-queue drains (those
        // re-set it themselves per-effect).
        let prev_dna_origin = self.current_dna_origin;
        self.current_dna_origin = Some(true);
        let total_reduction = self.scan_before_pay_cost_reduction_with_target(
            first_player,
            CostReductionKind::Digivolve,
            Some(target),
        );
        // Observer dispatch — G-BEFORE-PAY-COST-GAIN-MEMORY.
        self.scan_before_pay_cost_observers(first_player, Some(target));
        self.current_dna_origin = prev_dna_origin;
        let effective_cost = (printed_cost as i32 - total_reduction).max(0) as u16;

        let _ = self.dna_digivolve_inner(
            target_a,
            target_b,
            first_player,
            evo_hand_index,
            effective_cost,
            true,
            false,
        );

        if route_window == crate::dna_digivolve::DnaRouteWindow::EndOfTurnAction {
            if self.memory < 0 && !self.game_over {
                self.pass_end_of_turn_action();
            }
        } else {
            self.check_turn_end();
        }
    }

    /// Move a card from `source` to `player_id`'s security stack at the given
    /// `position` (Top, Bottom, Random). If `face_up` is true, the card's
    /// `card_index` is inserted into `face_up_security` so subsequent reveals
    /// know it was placed face-up. Returns false if the source index is invalid.
    ///
    /// Does not fire `OnLoseSecurity`; successful placements fire
    /// `OnPlaceSecurity` observers after the card reaches the security stack.
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
        self.place_on_security_observed(player_id, source, position, face_up, player_id)
    }

    /// Generalized "move this permanent into the security stack" primitive.
    /// Routes the permanent's top card to `player_id`'s security at `position`
    /// (face-up if `face_up`); routes sources-below-top to each source's
    /// owner's trash, firing `OnDigivolutionCardTrashed` per source; routes
    /// linked cards to the controller's trash, firing `OnLinkedCardTrashed`
    /// once if any were present. Mirrors the source-disposition shape used by
    /// `Game::return_to_deck` and `EffectContext::attach_tamer_to_digimon`.
    ///
    /// Gates on `CannotAddSecurityByEffect` (player-scoped, checked against
    /// `observer_player`). Routes through `WhenWouldLeaveBattleArea` then
    /// `WhenWouldPlaceInSecurity` replacements; bails (`false`) on any
    /// non-`None` outcome or installed pending selection.
    ///
    /// Used by `EffectContext::place_self_at_security` (Track E) — printed
    /// text "place this Digimon at the bottom of your security stack face
    /// down" (EX4-060), "place this Digimon as your top security card"
    /// (EX9-021), etc. DCGO `IPutSecurityPermanent` covers the same shape.
    ///
    /// **Engine divergence vs DCGO:** DCGO bundles the entire permanent
    /// (top + sources + linked) under a single security slot. The Rust
    /// engine's `Player.security: Vec<CardSource>` is flat (one card per
    /// slot), so the bundle is unrepresentable. We route sources to trash
    /// instead, matching the rules-default behavior for permanents leaving
    /// the field to a non-stack destination. Documented in
    /// `docs/RUST_PYTHON_PARITY.md` (Track E divergence note).
    pub(crate) fn place_permanent_on_security_observed(
        &mut self,
        player_id: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer_player: PlayerId,
    ) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        if self
            .modifiers
            .player_has(observer_player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }

        let Some(permanent) = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return false;
        };
        if permanent.card_sources.is_empty() {
            return false;
        }

        let source_card = permanent.top_card().handle();
        let cause = self.infer_effect_cause(player_id);
        let leave_subject = ReplacementSubject::Permanent(target);
        let leave_outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            leave_subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() || !matches!(leave_outcome, ReplacementOutcome::None) {
            return false;
        }

        let place_subject = ReplacementSubject::Card(source_card, Zone::BattleArea);
        let place_outcome = self.try_replace(
            EffectTiming::WhenWouldPlaceInSecurity,
            place_subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() || !matches!(place_outcome, ReplacementOutcome::None) {
            return false;
        }

        let mut permanent = self
            .player_mut(target.player)
            .battle_area
            .remove(target.index as usize);

        // Pop top card; if somehow empty (shouldn't happen — we checked
        // above), bail without further state changes.
        let Some(card) = permanent.card_sources.pop() else {
            return false;
        };

        // Modifier cleanup BEFORE the source-trash dispatch — modifiers are
        // keyed on `PermanentHandle`, which becomes invalid after `remove()`
        // shifts indices. Mirrors `attach_tamer_to_digimon`.
        self.clear_permanent_full(target);
        self.modifiers.expire_player_on_permanent_leave(target);

        // Sources-below-top → each source's owner's trash. Per source: push,
        // enqueue OnDigivolutionCardTrashed for each player, drain queue.
        // Mirrors `EffectContext::attach_tamer_to_digimon`.
        for source in permanent.card_sources.drain(..) {
            let owner = source.owner;
            self.player_mut(owner).trash.push(source);
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnDigivolutionCardTrashed,
                    TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
            // Intentionally inline-drain (see `fire_digivolution_card_trashed`):
            // EX10-036's behavioral test depends on synchronous between-source
            // observer firing for chained trash-pickup clauses.
            self.drain_effect_queue();
        }

        // Linked cards → controller's trash; fire OnLinkedCardTrashed once
        // if any were present. Mirrors `attach_tamer_to_digimon`.
        let had_linked = !permanent.linked_cards.is_empty();
        for linked in permanent.linked_cards.drain(..) {
            let owner = linked.owner;
            self.player_mut(owner).trash.push(linked);
        }
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnLinkedCardTrashed,
                    TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
            // Intentionally inline-drain — same rationale as above.
            self.drain_effect_queue();
        }

        // Place top card in security at the requested position.
        let face_up_key = card.card_index;
        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).security.push(card);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).security.insert(0, card);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let sec_len = self.player(player_id).security.len();
                let idx = if sec_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=sec_len)
                };
                self.player_mut(player_id).security.insert(idx, card);
            }
        }
        if face_up {
            self.player_mut(player_id)
                .face_up_security
                .insert(face_up_key);
        }
        true
    }

    pub(crate) fn place_sourceless_permanent_on_security_bottom(
        &mut self,
        player_id: PlayerId,
        target: PermanentHandle,
        observer_player: PlayerId,
    ) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        if self
            .modifiers
            .player_has(observer_player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }

        let Some(permanent) = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return false;
        };
        if permanent.card_sources.len() != 1 {
            return false;
        }

        let source_card = permanent.top_card().handle();
        let cause = self.infer_effect_cause(player_id);
        let leave_subject = ReplacementSubject::Permanent(target);
        let leave_outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            leave_subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() || !matches!(leave_outcome, ReplacementOutcome::None) {
            return false;
        }

        let place_subject = ReplacementSubject::Card(source_card, Zone::BattleArea);
        let place_outcome = self.try_replace(
            EffectTiming::WhenWouldPlaceInSecurity,
            place_subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() || !matches!(place_outcome, ReplacementOutcome::None) {
            return false;
        }

        let mut permanent = self
            .player_mut(target.player)
            .battle_area
            .remove(target.index as usize);
        let Some(card) = permanent.card_sources.pop() else {
            return false;
        };

        self.clear_permanent_full(target);
        self.modifiers.expire_player_on_permanent_leave(target);

        let had_linked = !permanent.linked_cards.is_empty();
        for linked in permanent.linked_cards {
            self.player_mut(target.player).trash.push(linked);
        }
        if had_linked {
            self.enqueue_triggered(
                EffectTiming::OnLinkedCardTrashed,
                TriggerSource::PlayerBattleArea(observer_player),
            );
            self.drain_effect_queue();
        }

        self.player_mut(player_id).security.insert(0, card);
        self.fire_on_place_security(player_id, observer_player, source_card);
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    pub(crate) fn fire_on_place_security(
        &mut self,
        affected_player: PlayerId,
        source_player: PlayerId,
        card: crate::card_source::CardHandle,
    ) {
        self.enqueue_triggered(
            EffectTiming::OnPlaceSecurity,
            TriggerSource::SecurityPlaced {
                affected_player,
                source_player,
                card,
                cause: crate::trigger_context::EventCause::SecurityPlacement,
            },
        );
        // G-DSL-OUTER-TAIL-NESTED-PARK fix: this was previously the dominant
        // collision site — `place_on_security` called from inside a Lamiamon
        // clause-2 inner-tail callback would inline-drain a second copy of
        // the same triggered effect, parking on top of the first's outer
        // tail. `maybe_drain` defers the drain to the outer-tail scope's
        // exit.
        self.maybe_drain_effect_queue();
    }

    pub(crate) fn place_permanent_on_security(
        &mut self,
        player_id: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer_player: PlayerId,
    ) -> bool {
        use crate::enums::{EffectTiming, StackPosition, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        if self
            .modifiers
            .player_has(observer_player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }

        if self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .is_none()
        {
            return false;
        }

        let cause = self.infer_effect_cause(target.player);
        let leave_outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            ReplacementSubject::Permanent(target),
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() {
            return false;
        }
        match leave_outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => return false,
            ReplacementOutcome::Redirected(Zone::Security) => {}
            ReplacementOutcome::Redirected(Zone::Trash) => {
                self.delete_permanent_with_cause(target, cause);
                return false;
            }
            ReplacementOutcome::Redirected(Zone::Hand) => {
                return self.return_to_hand(target).is_some()
            }
            ReplacementOutcome::Redirected(Zone::Deck) => {
                return self.return_to_deck(target, StackPosition::Bottom);
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected redirect destination for permanent-to-security: {:?}",
                    other
                );
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)) => {
                return self.place_permanent_on_security(
                    player_id,
                    other,
                    position,
                    face_up,
                    observer_player,
                );
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "non-permanent substitute is unsupported for permanent-to-security"
                );
            }
        }

        self.place_permanent_on_security_without_leave_replacement(
            player_id,
            target,
            position,
            face_up,
            observer_player,
        )
    }

    pub(crate) fn place_permanent_on_security_without_leave_replacement(
        &mut self,
        player_id: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer_player: PlayerId,
    ) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        if self
            .modifiers
            .player_has(observer_player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }

        let Some(permanent) = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return false;
        };
        let source_card = permanent.top_card().handle();
        let cause = self.infer_effect_cause(player_id);
        let place_subject = ReplacementSubject::Card(source_card, Zone::BattleArea);
        let place_outcome = self.try_replace(
            EffectTiming::WhenWouldPlaceInSecurity,
            place_subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() || !matches!(place_outcome, ReplacementOutcome::None) {
            return false;
        }

        let mut permanent = self
            .player_mut(target.player)
            .battle_area
            .remove(target.index as usize);
        let Some(top) = permanent.card_sources.pop() else {
            return false;
        };
        let top_handle = top.handle();
        let face_up_key = top.card_index;

        let mut leaving_sources = permanent.card_sources.clone();
        leaving_sources.push(top.clone());
        self.apply_ace_overflow_for_sources(&leaving_sources);

        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).security.push(top);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).security.insert(0, top);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let sec_len = self.player(player_id).security.len();
                let idx = if sec_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=sec_len)
                };
                self.player_mut(player_id).security.insert(idx, top);
            }
        }

        if face_up {
            self.player_mut(player_id)
                .face_up_security
                .insert(face_up_key);
        }

        for card in permanent.card_sources {
            let source_card = card.handle();
            // Owner-routed (Track E correctness): each source returns to
            // its OWN owner's trash. Identical to controller-routed when
            // owner == controller (the common case).
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
            self.enqueue_triggered(
                EffectTiming::OnDigivolutionCardTrashed,
                TriggerSource::SourceTrashedFromStack {
                    player: target.player,
                    host: target,
                    host_card: top_handle,
                    card: source_card,
                    cause: crate::trigger_context::EventCause::SecurityPlacement,
                },
            );
            self.drain_effect_queue();
        }

        let had_linked = !permanent.linked_cards.is_empty();
        for linked in permanent.linked_cards {
            // Owner-routed: linked cards return to their own owner's trash.
            let owner = linked.owner;
            self.player_mut(owner).trash.push(linked);
        }
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnLinkedCardTrashed,
                    TriggerSource::PlayerBattleArea(pid as PlayerId),
                );
            }
            self.drain_effect_queue();
        }

        self.clear_permanent_full(target);
        self.modifiers.expire_player_on_permanent_leave(target);
        self.fire_on_place_security(player_id, observer_player, top_handle);
        true
    }

    pub(crate) fn place_on_security_observed(
        &mut self,
        player_id: PlayerId,
        source: crate::enums::CardSourceRef,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer_player: PlayerId,
    ) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Snapshot the source card's handle before the take so we can build
        // a meaningful ReplacementSubject. Return false early if the source
        // is invalid (matches the existing pre-flight behavior of the take).
        let Some((source_card, _, source_zone)) = self.card_source_ref_snapshot(source) else {
            return false;
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
                    crate::enums::CardSourceRef::Security(defender, index) => {
                        let player = self.player_mut(defender);
                        if index >= player.security.len() {
                            return false;
                        }
                        let card = player.security.remove(index);
                        player.face_up_security.remove(&card.card_index);
                        let cause = crate::trigger_context::EventCause::from(
                            self.infer_effect_cause(defender),
                        );
                        self.fire_effect_security_removal(
                            defender,
                            observer_player,
                            observer_player,
                            cause,
                            card,
                            crate::selection::SecurityRemovalDestination::Trash,
                        );
                        return false;
                    }
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
                    other => {
                        let Some(taken) = self.take_card_source_ref(other) else {
                            return false;
                        };
                        taken.card
                    }
                };
                let owner = taken.owner;
                self.player_mut(owner).trash.push(taken);
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
        if let crate::enums::CardSourceRef::Security(defender, index) = source {
            let player = self.player_mut(defender);
            if index >= player.security.len() {
                return false;
            }
            let card = player.security.remove(index);
            player.face_up_security.remove(&card.card_index);
            let cause = crate::trigger_context::EventCause::from(self.infer_effect_cause(defender));
            self.fire_effect_security_removal(
                defender,
                observer_player,
                observer_player,
                cause,
                card,
                crate::selection::SecurityRemovalDestination::Security {
                    player: player_id,
                    position,
                    face_up,
                },
            );
            return true;
        }
        let Some(taken) = self.take_card_source_ref(source) else {
            return false;
        };
        let taken = taken.card;

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
        self.fire_on_place_security(player_id, observer_player, source_card);
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
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id,
            crate::enums::CardSourceRef::Hand(player_id, hand_index),
            target,
            cost_delta,
            ignore_color,
            false,
            source,
        )
    }

    pub fn effect_initiated_digivolve_ignore_requirements(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id,
            crate::enums::CardSourceRef::Hand(player_id, hand_index),
            target,
            cost_delta,
            true,
            true,
            source,
        )
    }

    /// Source-general script-initiated digivolve. The result card is taken
    /// from any `CardSourceRef`, placed on top of `target`, and restored to
    /// its source zone if a post-take failure occurs.
    pub fn effect_initiated_digivolve_from_source(
        &mut self,
        player_id: PlayerId,
        source_ref: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id,
            source_ref,
            target,
            cost_delta,
            ignore_color,
            false,
            source,
        )
    }

    pub fn effect_initiated_digivolve_from_source_ignore_requirements(
        &mut self,
        player_id: PlayerId,
        source_ref: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id, source_ref, target, cost_delta, true, true, source,
        )
    }

    fn effect_initiated_digivolve_from_source_inner(
        &mut self,
        player_id: PlayerId,
        source_ref: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
        ignore_requirements: bool,
        source: PlaySource,
    ) -> bool {
        if source == PlaySource::ByEffect
            && self
                .modifiers
                .player_has(player_id, ModifierType::CannotDigivolveDigimonByEffect)
        {
            self.logger.log(
                "[Rejected] effect_initiated_digivolve: blocked by CannotDigivolveDigimonByEffect",
            );
            return false;
        }

        let Some((evo_card_handle, evo_card_data_index, _)) =
            self.card_source_ref_snapshot(source_ref)
        else {
            self.logger
                .log("[Rejected] effect_initiated_digivolve: source ref out of range");
            return false;
        };

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
        let (base_level, base_colors) = {
            let target_player = self.player(target.player);
            let perm = &target_player.battle_area[target.index as usize];
            let identity = perm.synth_identity(&self.card_data, &self.modifiers, target);
            let Some(base_level) = identity.level else {
                self.logger
                    .log("[Rejected] effect_initiated_digivolve: target top card has no level");
                return false;
            };
            (base_level, identity.colors)
        };

        let evo_costs = &self.card_data[evo_card_data_index].evo_costs;
        let matching_memory_cost = if ignore_requirements {
            Some(0)
        } else {
            evo_costs
                .iter()
                .find(|ec| {
                    ec.level == base_level
                        && (ignore_color
                            || crate::action::mask::evo_color(ec.card_color)
                                .map(|c| base_colors.contains(&c))
                                .unwrap_or(false))
                })
                .map(|ec| ec.memory_cost)
        };
        let Some(matching_memory_cost) = matching_memory_cost else {
            self.logger.log(&format!(
                "[Rejected] effect_initiated_digivolve: no matching evo cost (base_level={}, ignore_color={}, ignore_requirements={})",
                base_level, ignore_color, ignore_requirements
            ));
            return false;
        };
        let base_cost = cost_delta.resolve(matching_memory_cost);
        // Pass the evolving card as the cost-target so target-aware
        // predicates can fire — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET. The
        // `target` permanent here is the one being digivolved into
        // (effect-initiated digivolves stack a hand/trash/security card
        // onto an existing battle-area permanent).
        let from_hand = matches!(source_ref, crate::enums::CardSourceRef::Hand(_, _));
        let cost_target_ctx = CostTargetContext {
            card: evo_card_handle,
            from_hand,
            is_digivolve: true,
            target_permanents: [Some(target), None],
        };
        let total_reduction = self.scan_before_pay_cost_reduction_with_target(
            player_id,
            CostReductionKind::Digivolve,
            Some(cost_target_ctx),
        );
        // Observer dispatch — G-BEFORE-PAY-COST-GAIN-MEMORY.
        self.scan_before_pay_cost_observers(player_id, Some(cost_target_ctx));
        let effective_cost = (base_cost as i32 - total_reduction).max(0) as u16;

        // 3. Remove the card from its source and pay memory. If payment fails,
        // restore the source exactly where it came from.
        if let crate::enums::CardSourceRef::Security(defender, index) = source_ref {
            if !self.pay_memory(effective_cost) {
                self.logger.log(&format!(
                    "[Rejected] effect_initiated_digivolve: cannot pay memory cost {} (current memory={})",
                    effective_cost, self.memory
                ));
                return false;
            }

            let player = self.player_mut(defender);
            if index >= player.security.len() {
                self.logger
                    .log("[Rejected] effect_initiated_digivolve: source ref changed before take");
                return false;
            }
            let card = player.security.remove(index);
            player.face_up_security.remove(&card.card_index);
            let cause = crate::trigger_context::EventCause::from(self.infer_effect_cause(defender));
            self.fire_effect_security_removal(
                defender,
                player_id,
                player_id,
                cause,
                card,
                crate::selection::SecurityRemovalDestination::Digivolve {
                    player: player_id,
                    target,
                    turn: self.turn_count,
                },
            );
            return true;
        }

        let Some(taken) = self.take_card_source_ref(source_ref) else {
            self.logger
                .log("[Rejected] effect_initiated_digivolve: source ref changed before take");
            return false;
        };
        if !self.pay_memory(effective_cost) {
            self.logger.log(&format!(
                "[Rejected] effect_initiated_digivolve: cannot pay memory cost {} (current memory={})",
                effective_cost, self.memory
            ));
            let _ = self.restore_card_source_ref(source_ref, taken);
            return false;
        }

        // 4. Move the card onto the target permanent's stack.
        let turn = self.turn_count;
        self.player_mut(target.player).battle_area[target.index as usize]
            .digivolve(taken.card, turn);

        // 4a. Soft-remove an emptied Material source. If `source_ref` was
        // `Material(src, _)`, `take_card_source_ref` may have removed the
        // source permanent's only card. Left in `battle_area`, that "zombie"
        // permanent panics any trigger fan-out that iterates all permanents
        // and calls `top_card()`. NOT a deletion: no OnDeletion fires, no
        // replacement window, no trash for the body card (already moved to
        // target). Matches DCGO's caller-side `RemoveField(permanent)`
        // pattern (e.g. Jogress at
        // `DCGO/Assets/Scripts/Script/CardController.cs:1509`). See
        // `Game::soft_remove_if_emptied` doc + the `G-PERMANENT-EMPTY-…`
        // entry in `qa/archetype-qa/engine-gaps.md`.
        let mut target = target;
        if let crate::enums::CardSourceRef::Material(src_handle, _) = source_ref {
            if self.soft_remove_if_emptied(src_handle) {
                target = Self::shift_handle_after_soft_remove(src_handle, target);
            }
        }

        let event_card = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .map(|perm| perm.top_card().handle())
            .expect("effect digivolve target remains in battle area after stack mutation");

        // 5. Fire WhenDigivolving triggers.
        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(target),
        );
        self.drain_effect_queue();

        // OnDigivolve: global observer — carries the evolved permanent/card
        // plus effect-origin provenance for "digivolved by an effect" gates.
        self.enqueue_triggered(
            EffectTiming::OnDigivolve,
            TriggerSource::Digivolved {
                player: player_id,
                permanent: target,
                card: event_card,
                effect_initiated: true,
                dna_origin: false,
            },
        );
        self.drain_effect_queue();

        true
    }
}
