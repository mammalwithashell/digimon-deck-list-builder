//! Selection-prompt helpers on `EffectContext` — extracted from `mod.rs` for
//! readability. These are the 11 player-choice primitives card scripts use to
//! install a `PendingSelection`: field, hand, trash, material, reveal,
//! security, effect-choice, plus `mark_security_face_up` and
//! `play_pending_security`.
//!
//! The "no-approximations" contract (CLAUDE.md §17–18) lives here: every
//! optional card choice surfaces through one of these helpers — no
//! auto-selections, no silent drops. When the gap-closing roadmap adds
//! new selection kinds (multi-select, ordered permutation, cross-player,
//! budgeted multi-select), they land alongside the existing helpers in
//! this file.
//!
//! After calling any of these helpers the effect's `process` closure must
//! return — the callback fires later (when the game resolves the selection
//! via `Game::resolve_selection`), and further mutations from within the
//! original `process` would race with queue state.
//!
//! The filter runs **at install time** to produce `valid_action_ids`. It
//! does not re-run at resolution; the selection is validated against that
//! frozen set. Single-threaded engine, queue pauses while a selection is
//! parked, so state cannot drift.

use crate::card_source::{CardHandle, CardSource};
use crate::effect_context::{EffectContext, PartitionRequirement};
use crate::enums::{CardKind, EffectSourceKind, GamePhase, ModifierType, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::selection::{
    BreedingPermanentSelectionRef, EffectChoiceEntry, PendingSelection, SelectionKind,
    SourceSelectionRef,
};

/// Identifies which zone `select_count_capped_multi` draws candidates from.
///
/// - `Hand`  — action IDs map to `PLAY_HAND_START + i` (range 0–29)
/// - `Trash` — action IDs map to `TRASH_EFFECT_START + i` (range 1150–1194)
/// - `Material(PermanentHandle)` — action IDs map to
///   `SOURCE_SELECT_START + field_index * SOURCES_PER_FIELD + source_index`
///   (range 2000–2167). Candidates are the digivolution *sources* of the
///   named permanent — i.e., `card_sources[0..len-1]` — **excluding** the
///   top card (`card_sources.last()`). This matches DCGO's `DigivolutionCards`
///   which also excludes `TopCard`. Used by `Fragment(N)`, `ArmorPurge`, and
///   `MaterialSave(N)`.
///
/// # Stack indexing convention
/// `Permanent.card_sources` is a `Vec` where index 0 is the **bottom** card
/// and `last()` is the **top** card (confirmed by `Permanent::top_card()`
/// returning `card_sources.last()`). The `Material` variant therefore presents
/// indices `0..len-1` as candidates; the top card at `index = len-1` is never
/// included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountCappedZone {
    Hand,
    Trash,
    /// Digivolution sources of `perm`, excluding the top card.
    /// Action IDs: `SOURCE_SELECT_START + field_index * SOURCES_PER_FIELD + i`.
    Material(crate::permanent::PermanentHandle),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealBucketSelection {
    pub bind_as: String,
    pub min: u8,
    pub max: u8,
    pub candidates: Vec<CardHandle>,
}

/// Per-card-number / -level / -name uniqueness constraint applied to a
/// `select_count_capped_multi` selection. After each pick, candidates that
/// share the constrained attribute with any already-picked card are
/// removed from the next step's `valid_action_ids`.
///
/// Mirrors `digimon_dsl::compiled::CompiledDistinctBy`; the DSL lowering
/// in `dsl_cards::step::selections` translates the variant unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistinctByMode {
    CardNumber,
    Level,
    Name,
}

impl<'a> EffectContext<'a> {
    /// Prompt `self.player` to pick a Digimon from their clockwise-next
    /// opponent's battle area. The `filter` decides which field slots are
    /// legal targets (called once per slot at install time).
    ///
    /// If no slot passes and `is_optional = false`, this is a no-op
    /// (matches Python — a "delete target X" effect with no valid targets
    /// silently does nothing). If `is_optional = true` with no valid
    /// targets, also a no-op (no prompt installed, nothing to decline).
    ///
    /// The callback is invoked with a fresh `EffectContext` keyed to the
    /// same `(source_card, source_permanent, player)` tuple that installed
    /// the selection. This keeps card scripts ergonomic — they still have
    /// access to memory mutations, modifier grants, etc. inside the
    /// callback.
    ///
    /// **2-player scope (PR3).** Multiplayer target-opponent-selection
    /// requires richer action-ID encoding and lands alongside EDH; for now
    /// this helper scopes to `next_clockwise(self.player)`.
    pub fn select_opponent_permanent<F, C>(
        &mut self,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, PermanentHandle) -> bool,
        C: FnOnce(&mut EffectContext<'_>, PermanentHandle) + Send + Sync + 'static,
    {
        let target_player = self.game.next_clockwise(self.player);
        let source = Some(self.player);
        let composed = move |game: &Game, h: PermanentHandle| -> bool {
            if game.progress_excludes(h, source) {
                return false;
            }
            filter(game, h)
        };
        self.install_field_selection(
            SelectionKind::OppField,
            GamePhase::SelectTarget,
            target_player,
            prompt,
            is_optional,
            composed,
            callback,
        );
    }

    /// Prompt `self.player` to pick one of their own Digimon / Tamers.
    ///
    /// Same shape as `select_opponent_permanent`; see that method's docs
    /// for semantics around empty target lists and callback context.
    pub fn select_own_permanent<F, C>(
        &mut self,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, PermanentHandle) -> bool,
        C: FnOnce(&mut EffectContext<'_>, PermanentHandle) + Send + Sync + 'static,
    {
        let target_player = self.player;
        self.install_field_selection(
            SelectionKind::OwnField,
            GamePhase::SelectTarget,
            target_player,
            prompt,
            is_optional,
            filter,
            callback,
        );
    }

    /// Prompt `self.player` to pick a card from their own hand (or any
    /// player's hand — pass `of_player`). Filter runs once per hand
    /// position. The callback receives the chosen hand index.
    ///
    /// **Index stability.** The callback fires *after* the `process`
    /// closure has returned, so the hand may have mutated between install
    /// and resolution. In practice the queue is paused during a pending
    /// selection, so mutation is gated on the callback itself — but
    /// callback authors should re-check that the chosen index is still in
    /// range before using it.
    pub fn select_hand<F, C>(
        &mut self,
        of_player: PlayerId,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, usize) -> bool,
        C: FnOnce(&mut EffectContext<'_>, usize) + Send + Sync + 'static,
    {
        use crate::action::space::{HAND_MAIN_LIMIT, PLAY_HAND_START};

        let hand_len = self.game.player(of_player).hand.len();
        let cap = hand_len.min(HAND_MAIN_LIMIT);
        let mut valid_action_ids: Vec<u16> = Vec::with_capacity(cap);
        for i in 0..cap {
            if filter(self.game, i) {
                valid_action_ids.push(PLAY_HAND_START + i as u16);
            }
        }
        if valid_action_ids.is_empty() {
            return;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let user_callback: Box<dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync> =
            Box::new(callback);

        let previous_phase = self.game.current_phase;
        self.game.current_phase = GamePhase::SelectHand;
        self.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Hand,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let hand_index = action_id.saturating_sub(PLAY_HAND_START) as usize;
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                user_callback(&mut ctx, hand_index);
            }),
            on_decline: None,
        });
    }

    /// Prompt `self.player` to pick a card from the specified player's
    /// trash. See `select_hand` for general semantics around filter,
    /// optional, and index stability.
    pub fn select_trash<F, C>(
        &mut self,
        of_player: PlayerId,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, usize) -> bool,
        C: FnOnce(&mut EffectContext<'_>, usize) + Send + Sync + 'static,
    {
        use crate::action::space::{TRASH_EFFECT_START, TRASH_MAIN_LIMIT};

        let trash_len = self.game.player(of_player).trash.len();
        let cap = trash_len.min(TRASH_MAIN_LIMIT);
        let mut valid_action_ids: Vec<u16> = Vec::with_capacity(cap);
        for i in 0..cap {
            if filter(self.game, i) {
                valid_action_ids.push(TRASH_EFFECT_START + i as u16);
            }
        }
        if valid_action_ids.is_empty() {
            return;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let user_callback: Box<dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync> =
            Box::new(callback);

        let previous_phase = self.game.current_phase;
        self.game.current_phase = GamePhase::SelectTrash;
        self.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Trash,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let trash_index = action_id.saturating_sub(TRASH_EFFECT_START) as usize;
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                user_callback(&mut ctx, trash_index);
            }),
            on_decline: None,
        });
    }

    /// Prompt `self.player` to pick a card from a permanent's digivolution
    /// stack — i.e., one of the `card_sources` entries on `of_permanent`,
    /// **excluding** the top card (the active Digimon itself). Used for
    /// DNA / material-driven effects ("remove 1 of this Digimon's sources",
    /// "trash 2 materials to gain memory"). Parity §4.6d-residual.
    ///
    /// `filter(game, source_index)` runs once per source position at install
    /// time (index 0 = bottom card, index `card_sources.len()-2` = highest
    /// material). The top card at `card_sources.len()-1` is never offered —
    /// matches DCGO's `DigivolutionCards` (which excludes `TopCard`) and the
    /// `CountCappedZone::Material` contract on the multi-pick variant.
    /// Callback receives the chosen `source_index`.
    pub fn select_material<F, C>(
        &mut self,
        of_permanent: PermanentHandle,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, usize) -> bool,
        C: FnOnce(&mut EffectContext<'_>, usize) + Send + Sync + 'static,
    {
        use crate::action::space::{SOURCES_PER_FIELD, SOURCE_SELECT_START};

        let field_index = of_permanent.index as u16;
        let stack_len = match self
            .game
            .player(of_permanent.player)
            .battle_area
            .get(of_permanent.index as usize)
        {
            Some(perm) => perm.card_sources.len(),
            None => return,
        };
        // Mirror CountCappedZone::Material: top card is never a candidate.
        let source_count = stack_len.saturating_sub(1);
        let cap = source_count.min(SOURCES_PER_FIELD as usize);
        let mut valid_action_ids: Vec<u16> = Vec::with_capacity(cap);
        for i in 0..cap {
            if filter(self.game, i) {
                valid_action_ids
                    .push(SOURCE_SELECT_START + field_index * SOURCES_PER_FIELD + i as u16);
            }
        }
        if valid_action_ids.is_empty() {
            return;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let user_callback: Box<dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync> =
            Box::new(callback);

        let previous_phase = self.game.current_phase;
        self.game.current_phase = GamePhase::SelectMaterial;
        self.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Material,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let (_, source_idx) = crate::action::space::decode_source_select(action_id);
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                user_callback(&mut ctx, source_idx as usize);
            }),
            on_decline: None,
        });
    }

    pub fn select_own_sources<F, C>(
        &mut self,
        prompt: &str,
        min: u8,
        max: u8,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, SourceSelectionRef) -> bool + Send + Sync + 'static,
        C: FnOnce(&mut EffectContext<'_>, Vec<SourceSelectionRef>) + Send + Sync + 'static,
    {
        assert!(min <= max, "select_own_sources min must be <= max");
        assert!(max > 0, "select_own_sources max must be > 0");

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let previous_phase = self.game.current_phase;
        install_source_multi_selection(
            self.game,
            controller,
            selecting_player,
            override_pin,
            prompt.to_string(),
            min,
            max,
            Vec::new(),
            std::sync::Arc::new(filter),
            source_card,
            source_permanent,
            source_kind,
            previous_phase,
            Box::new(move |game, picked| {
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                callback(&mut ctx, picked);
            }),
        );
    }

    pub fn select_partition_sources<C>(
        &mut self,
        host: PermanentHandle,
        prompt: &str,
        requirements: Vec<PartitionRequirement>,
        callback: C,
    ) where
        C: FnOnce(&mut EffectContext<'_>, Vec<SourceSelectionRef>) + Send + Sync + 'static,
    {
        let count = requirements.len();
        if count == 0 || count > u8::MAX as usize {
            return;
        }
        if host.player != self.player {
            return;
        }
        let requirements = std::sync::Arc::new(requirements);
        if !partition_can_extend(self.game, host, &requirements, &[]) {
            return;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let previous_phase = self.game.current_phase;
        install_partition_source_selection(
            self.game,
            host,
            controller,
            selecting_player,
            override_pin,
            prompt.to_string(),
            requirements,
            Vec::new(),
            source_card,
            source_permanent,
            source_kind,
            previous_phase,
            Box::new(move |game, selected| {
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                callback(&mut ctx, selected);
            }),
        );
    }

    pub fn select_opponent_permanents_by_dp_budget<F, C>(
        &mut self,
        prompt: &str,
        dp_budget: i32,
        min_picks: u8,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, PermanentHandle) -> bool + Send + Sync + 'static,
        C: FnOnce(&mut EffectContext<'_>, Vec<PermanentHandle>) + Send + Sync + 'static,
    {
        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let opponent = self.game.next_clockwise(self.player);
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let previous_phase = self.game.current_phase;

        install_dp_budget_selection(
            self.game,
            opponent,
            controller,
            selecting_player,
            override_pin,
            prompt.to_string(),
            dp_budget,
            min_picks,
            Vec::new(),
            std::sync::Arc::new(filter),
            source_card,
            source_permanent,
            source_kind,
            previous_phase,
            Box::new(move |game, picked| {
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                callback(&mut ctx, picked);
            }),
        );
    }

    /// Play-cost-budget analog of `select_opponent_permanents_by_dp_budget`.
    /// The player picks `min_picks..N` opponent permanents whose running
    /// printed-play-cost sum never exceeds `play_cost_budget`; any single
    /// permanent whose individual play cost exceeds the budget is excluded.
    /// G-MULTI-SELECT-OPP-PLAY-COST-SUM.
    pub fn select_opponent_permanents_by_play_cost_budget<F, C>(
        &mut self,
        prompt: &str,
        play_cost_budget: i32,
        min_picks: u8,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, PermanentHandle) -> bool + Send + Sync + 'static,
        C: FnOnce(&mut EffectContext<'_>, Vec<PermanentHandle>) + Send + Sync + 'static,
    {
        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let opponent = self.game.next_clockwise(self.player);
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let previous_phase = self.game.current_phase;

        install_play_cost_budget_selection(
            self.game,
            opponent,
            controller,
            selecting_player,
            override_pin,
            prompt.to_string(),
            play_cost_budget,
            min_picks,
            Vec::new(),
            std::sync::Arc::new(filter),
            source_card,
            source_permanent,
            source_kind,
            previous_phase,
            Box::new(move |game, picked| {
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                callback(&mut ctx, picked);
            }),
        );
    }

    pub fn select_own_breeding_permanent<F, C>(&mut self, prompt: &str, filter: F, callback: C)
    where
        F: Fn(&Game, BreedingPermanentSelectionRef) -> bool + Send + Sync + 'static,
        C: FnOnce(&mut EffectContext<'_>, BreedingPermanentSelectionRef) + Send + Sync + 'static,
    {
        let Some(card) = self
            .game
            .player(self.player)
            .breeding_area
            .as_ref()
            .map(|p| p.top_card().handle())
        else {
            return;
        };
        let selection_ref = BreedingPermanentSelectionRef {
            player: self.player,
            card,
        };
        if !filter(self.game, selection_ref) {
            return;
        }
        let Some(action_id) = crate::action::space::encode_breeding_select(self.player) else {
            return;
        };

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let previous_phase = self.game.current_phase;

        self.game.current_phase = GamePhase::SelectBreedingPermanent;
        self.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::BreedingPermanent,
            selecting_player,
            previous_phase,
            valid_action_ids: vec![action_id],
            is_optional: false,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game, _| {
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                callback(&mut ctx, selection_ref);
            }),
            on_decline: None,
        });
    }

    /// Prompt `self.player` to pick one of several labeled branches
    /// ("choose one: A or B"). `labels` is the set of option labels in
    /// order; the callback receives the chosen 0-based index.
    ///
    /// Capped at `HAND_MAIN_LIMIT` (30) — matches Python's
    /// `SelectEffectChoice` ceiling and the `TriggerOrder` encoding.
    pub fn select_effect_choice<C>(&mut self, prompt: &str, labels: Vec<String>, callback: C)
    where
        C: FnOnce(&mut EffectContext<'_>, usize) + Send + Sync + 'static,
    {
        use crate::action::space::{HAND_EFFECT_START, HAND_MAIN_LIMIT};

        if labels.is_empty() {
            return;
        }
        let cap = labels.len().min(HAND_MAIN_LIMIT);

        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let mut valid_action_ids: Vec<u16> = Vec::with_capacity(cap);
        let mut choices: Vec<EffectChoiceEntry> = Vec::with_capacity(cap);
        for (i, label) in labels.iter().take(cap).enumerate() {
            let action_id = HAND_EFFECT_START + i as u16;
            valid_action_ids.push(action_id);
            choices.push(EffectChoiceEntry {
                label: label.clone(),
                action_id,
                source_card: Some(source_card),
                source_kind: Some(source_kind),
                timing: None,
                is_optional: false,
                observation_metadata: Default::default(),
            });
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let user_callback: Box<dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync> =
            Box::new(callback);

        let previous_phase = self.game.current_phase;
        self.game.current_phase = GamePhase::EffectChoice;
        self.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::EffectChoice,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional: false, // effect choice must pick a branch
            prompt: prompt.to_string(),
            effect_choices: Some(choices),
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let choice_index = action_id.saturating_sub(HAND_EFFECT_START) as usize;
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                user_callback(&mut ctx, choice_index);
            }),
            on_decline: None,
        });
    }

    /// Prompt `self.player` to pick one of the cards currently exposed in
    /// `Game.revealed_cards`. Filter runs once per reveal position at install
    /// time; callback receives the chosen index.
    ///
    /// Uses the `SelectReveal` sub-range (30-39) of the shared HAND_EFFECT
    /// action space — disambiguated by `GamePhase::SelectReveal`. Mirrors
    /// Python's `SEL_REVEALED_START`.
    pub fn select_reveal<F, C>(
        &mut self,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) -> bool
    where
        F: Fn(&Game, usize) -> bool,
        C: FnOnce(&mut EffectContext<'_>, usize) + Send + Sync + 'static,
    {
        use crate::action::space::{MAX_REVEALED, SEL_REVEAL_START};

        let revealed_len = self.game.revealed_cards.len();
        let cap = revealed_len.min(MAX_REVEALED);
        let mut valid_action_ids: Vec<u16> = Vec::with_capacity(cap);
        for i in 0..cap {
            if filter(self.game, i) {
                valid_action_ids.push(SEL_REVEAL_START + i as u16);
            }
        }
        if valid_action_ids.is_empty() {
            return false;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let user_callback: Box<dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync> =
            Box::new(callback);

        let previous_phase = self.game.current_phase;
        self.game.current_phase = GamePhase::SelectReveal;
        self.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Reveal,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let index = action_id.saturating_sub(SEL_REVEAL_START) as usize;
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                user_callback(&mut ctx, index);
            }),
            on_decline: None,
        });
        true
    }

    pub fn select_reveal_buckets<C>(
        &mut self,
        buckets: Vec<RevealBucketSelection>,
        prompt: &str,
        no_duplicate_cards: bool,
        callback: C,
    ) -> bool
    where
        C: FnOnce(&mut EffectContext<'_>, Vec<(String, Vec<CardHandle>)>) + Send + Sync + 'static,
    {
        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let previous_phase = self.game.current_phase;

        let final_callback: RevealBucketCallback =
            std::sync::Arc::new(std::sync::Mutex::new(Some(Box::new(
                move |game: &mut Game, picked: Vec<(String, Vec<CardHandle>)>| {
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        source_card,
                        source_permanent,
                        source_kind,
                        controller,
                        override_pin,
                    );
                    callback(&mut ctx, picked);
                },
            ))));

        install_reveal_bucket_step(
            self.game,
            buckets,
            0,
            Vec::new(),
            Vec::new(),
            prompt.to_string(),
            no_duplicate_cards,
            source_card,
            source_permanent,
            source_kind,
            selecting_player,
            previous_phase,
            final_callback,
        );
        self.game.pending_selection.is_some()
    }

    /// Prompt `self.player` to pick a card from a security stack. Set
    /// `of_player = self.player` to target own security, or
    /// `self.opponent_id()` to target the opponent's. Filter runs once per
    /// security position at install time; callback receives the chosen
    /// index.
    ///
    /// Uses sub-ranges 40-49 (own) and 50-59 (opp) of the shared
    /// HAND_EFFECT action space — disambiguated by `GamePhase::SelectSecurity`.
    /// Mirrors Python's `effect_select_own_security` / `effect_select_opponent_security`.
    pub fn select_security<F, C>(
        &mut self,
        of_player: PlayerId,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, usize) -> bool,
        C: FnOnce(&mut EffectContext<'_>, usize) + Send + Sync + 'static,
    {
        use crate::action::space::{MAX_SECURITY, SEL_MY_SECURITY_START, SEL_OPP_SECURITY_START};

        let base = if of_player == self.player {
            SEL_MY_SECURITY_START
        } else {
            SEL_OPP_SECURITY_START
        };

        let security_len = self.game.player(of_player).security.len();
        let cap = security_len.min(MAX_SECURITY);
        let mut valid_action_ids: Vec<u16> = Vec::with_capacity(cap);
        for i in 0..cap {
            if filter(self.game, i) {
                valid_action_ids.push(base + i as u16);
            }
        }
        if valid_action_ids.is_empty() {
            return;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let user_callback: Box<dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync> =
            Box::new(callback);

        let previous_phase = self.game.current_phase;
        self.game.current_phase = GamePhase::SelectSecurity;
        self.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Security,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let index = action_id.saturating_sub(base) as usize;
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                user_callback(&mut ctx, index);
            }),
            on_decline: None,
        });
    }

    /// Prompt `self.player` (or `of_player`) to pick one card from a union of
    /// zones — e.g. hand OR trash. The `zones` bitset selects which zones
    /// are in scope (`UnionZoneSet::HAND | UnionZoneSet::TRASH`). Filter runs
    /// once per card at install time; callback receives a `CardHandle` for the
    /// chosen card.
    ///
    /// Action IDs reuse existing ranges: hand slots map to `PLAY_HAND_START +
    /// i`; trash slots map to `TRASH_EFFECT_START + i`. The inner callback
    /// disambiguates by range before invoking the user callback. No new action
    /// range is introduced. This matches Python's `effect_play_from_zone`
    /// dual-range approach.
    ///
    /// If no card passes the filter (across all zones), this is a no-op —
    /// matching the silent-empty contract used by the other select_* helpers.
    ///
    /// **Filter signature:** unlike `select_hand`/`select_trash` (which pass a
    /// `usize` index), this helper's filter receives `&CardSource` so cross-zone
    /// predicates can inspect the card directly without branching on whether the
    /// index is a hand or trash index.
    /// Prompt for a card from the union of `zones` (hand ∪ trash). The
    /// `callback` receives the picked card's `CardHandle` **and** the
    /// [`UnionZoneOrigin`](crate::selection::UnionZoneOrigin) of the zone it
    /// came from, so a downstream consumer can act on the card's true origin
    /// (e.g. play it back from hand vs trash). The origin is recovered from
    /// the encoded `action_id` range (`PLAY_HAND` vs `TRASH_EFFECT`).
    pub fn select_union_zone<F, C>(
        &mut self,
        of_player: PlayerId,
        zones: crate::selection::UnionZoneSet,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, &CardSource) -> bool,
        C: FnOnce(
                &mut EffectContext<'_>,
                crate::card_source::CardHandle,
                crate::selection::UnionZoneOrigin,
            ) + Send
            + Sync
            + 'static,
    {
        use crate::action::space::{
            HAND_MAIN_LIMIT, PLAY_HAND_END, PLAY_HAND_START, TRASH_EFFECT_START, TRASH_MAIN_LIMIT,
        };
        use crate::selection::UnionZoneSet;

        let mut valid_action_ids: Vec<u16> = Vec::new();

        // Hand zone — collect lengths first, then filter per index to avoid
        // simultaneous `&game` + `&game.player.hand[i]` borrows.
        if zones.contains(UnionZoneSet::HAND) {
            let hand_len = self.game.player(of_player).hand.len();
            let cap = hand_len.min(HAND_MAIN_LIMIT);
            for i in 0..cap {
                // Clone the CardSource so we can release the borrow on
                // `self.game` before passing `&Game` to the filter.
                let card_clone = self.game.player(of_player).hand[i].clone();
                if filter(self.game, &card_clone) {
                    valid_action_ids.push(PLAY_HAND_START + i as u16);
                }
            }
        }

        // Trash zone — same pattern.
        if zones.contains(UnionZoneSet::TRASH) {
            let trash_len = self.game.player(of_player).trash.len();
            let cap = trash_len.min(TRASH_MAIN_LIMIT);
            for i in 0..cap {
                let card_clone = self.game.player(of_player).trash[i].clone();
                if filter(self.game, &card_clone) {
                    valid_action_ids.push(TRASH_EFFECT_START + i as u16);
                }
            }
        }

        if valid_action_ids.is_empty() {
            return;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let user_callback: Box<
            dyn FnOnce(
                    &mut EffectContext<'_>,
                    crate::card_source::CardHandle,
                    crate::selection::UnionZoneOrigin,
                ) + Send
                + Sync,
        > = Box::new(callback);

        let previous_phase = self.game.current_phase;
        self.game.current_phase = crate::enums::GamePhase::SelectUnion;
        self.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::UnionZone { zones },
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                debug_assert!(
                    action_id < PLAY_HAND_END || action_id >= TRASH_EFFECT_START,
                    "select_union_zone: action_id {} falls in gap between PLAY_HAND ({}..{}) and TRASH_EFFECT ({}..); valid_action_ids was populated incorrectly",
                    action_id, PLAY_HAND_START, PLAY_HAND_END, TRASH_EFFECT_START
                );
                // Disambiguate by range — the action_id range also records
                // the picked card's origin zone.
                let (handle, origin) = if action_id >= TRASH_EFFECT_START {
                    let idx = (action_id - TRASH_EFFECT_START) as usize;
                    (
                        game.player(of_player).trash[idx].handle(),
                        crate::selection::UnionZoneOrigin::Trash,
                    )
                } else {
                    // PLAY_HAND_START range (0-29)
                    let idx = action_id.saturating_sub(PLAY_HAND_START) as usize;
                    (
                        game.player(of_player).hand[idx].handle(),
                        crate::selection::UnionZoneOrigin::Hand,
                    )
                };
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                user_callback(&mut ctx, handle, origin);
            }),
            on_decline: None,
        });
    }

    /// Prompt `self.player` to place N `items` in a chosen order via sequential
    /// single-item picks. After the player selects all items the final
    /// `callback` fires with a `Vec<CardHandle>` in chosen order.
    ///
    /// **Encoding**: each step reuses the `SEL_REVEAL_START` (30–39) range.
    /// `valid_action_ids[i] = SEL_REVEAL_START + i` where `i` is an index
    /// into the *remaining* (not yet picked) list. After each pick the engine
    /// re-installs a fresh `PendingSelection` for the next step with the
    /// picked item removed. Phase is `GamePhase::SelectPermutation`;
    /// `kind = SelectionKind::OrderedPermutation { remaining: n }`.
    ///
    /// **No-approximations contract**: even singleton permutations surface as a
    /// one-choice selection so the RL agent sees every ordering decision.
    ///
    /// **Empty items**: final callback fires immediately; no selection installed.
    ///
    /// **Cap**: capped at 10 items (`debug_assert` in debug builds; clamp in
    /// release). This matches `SEL_REVEAL_START` range width (30–39 = 10 slots).
    pub fn select_ordered_permutation<C>(
        &mut self,
        items: Vec<crate::card_source::CardHandle>,
        prompt: &str,
        callback: C,
    ) where
        C: FnOnce(&mut EffectContext<'_>, Vec<crate::card_source::CardHandle>)
            + Send
            + Sync
            + 'static,
    {
        debug_assert!(items.len() <= 10, "ordered permutation capped at 10 items");
        let items = if items.len() > 10 {
            items.into_iter().take(10).collect()
        } else {
            items
        };

        // Empty: skip all selection steps, invoke the callback immediately.
        if items.is_empty() {
            callback(self, Vec::new());
            return;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let previous_phase = self.game.current_phase;
        let prompt_owned = prompt.to_string();

        let final_callback: Box<
            dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync,
        > = Box::new(
            move |game: &mut Game, ordered: Vec<crate::card_source::CardHandle>| {
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                callback(&mut ctx, ordered);
            },
        );

        install_permutation_step(
            self.game,
            items,
            Vec::new(),
            prompt_owned,
            source_card,
            source_permanent,
            source_kind,
            selecting_player,
            previous_phase,
            final_callback,
        );
    }

    /// Prompt `of_player` to pick *up to* `max` cards from `zone` via
    /// sequential single-item picks. Each pick reuses the existing zone action
    /// range (`PLAY_HAND_START + i` for hand, `TRASH_EFFECT_START + i` for
    /// trash). `PASS` (action 62) commits early; reaching `picked == max`
    /// auto-commits immediately. The final `callback` fires with a
    /// `Vec<CardHandle>` of picked items in pick order.
    ///
    /// **PASS availability** (no-approximations policy — every branch is RL-visible):
    /// - `is_optional_zero = false`: PASS becomes available only after `picked >= 1`.
    /// - `is_optional_zero = true`:  PASS is available even at `picked == 0`,
    ///   allowing the player to select nothing.
    /// Encoded via `PendingSelection::is_optional` so the mask builder gates
    /// PASS correctly on each step.
    ///
    /// **Empty filter**: if no card passes `filter` at install time, `callback`
    /// fires immediately with an empty `Vec`; no `PendingSelection` is installed.
    ///
    /// **Cap**: `max` is asserted `<= 10` in debug builds.
    pub fn select_count_capped_multi<F, C>(
        &mut self,
        of_player: PlayerId,
        zone: CountCappedZone,
        max: u8,
        prompt: &str,
        is_optional_zero: bool,
        distinct_by: Option<DistinctByMode>,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, &CardSource) -> bool + Send + Sync + 'static,
        C: FnOnce(&mut EffectContext<'_>, Vec<crate::card_source::CardHandle>)
            + Send
            + Sync
            + 'static,
    {
        self.select_count_capped_multi_min(
            of_player,
            zone,
            0,
            max,
            prompt,
            is_optional_zero,
            distinct_by,
            filter,
            callback,
        );
    }

    /// Like `select_count_capped_multi` but with a minimum-pick floor.
    /// `min` is the number of cards that MUST be picked before the player can
    /// finish (PASS becomes available only at `picked >= effective_min`,
    /// where `effective_min = max(min, if is_optional_zero {0} else {1})`).
    /// If fewer than `min` candidates pass the filter the selection silently
    /// no-ops WITHOUT firing `callback` — the required cost is unpayable.
    /// G-SELECT-MULTI-MIN.
    #[allow(clippy::too_many_arguments)]
    pub fn select_count_capped_multi_min<F, C>(
        &mut self,
        of_player: PlayerId,
        zone: CountCappedZone,
        min: u8,
        max: u8,
        prompt: &str,
        is_optional_zero: bool,
        distinct_by: Option<DistinctByMode>,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, &CardSource) -> bool + Send + Sync + 'static,
        C: FnOnce(&mut EffectContext<'_>, Vec<crate::card_source::CardHandle>)
            + Send
            + Sync
            + 'static,
    {
        debug_assert!(max <= 10, "select_count_capped_multi: max must be <= 10");
        debug_assert!(min <= max, "select_count_capped_multi: min must be <= max");

        use crate::action::space::{
            HAND_MAIN_LIMIT, PLAY_HAND_START, SOURCES_PER_FIELD, SOURCE_SELECT_START,
            TRASH_EFFECT_START, TRASH_MAIN_LIMIT,
        };

        // Collect valid candidates at install time using the filter.
        //
        // For Material: action IDs encode as
        //   SOURCE_SELECT_START + field_index * SOURCES_PER_FIELD + source_index
        // The range_start here is the base for source_index 0 of this permanent.
        // zone_len = number of sources excluding the top card (card_sources.len() - 1).
        let (zone_len, range_start) = match zone {
            CountCappedZone::Hand => {
                let len = self.game.player(of_player).hand.len().min(HAND_MAIN_LIMIT);
                (len, PLAY_HAND_START)
            }
            CountCappedZone::Trash => {
                let len = self
                    .game
                    .player(of_player)
                    .trash
                    .len()
                    .min(TRASH_MAIN_LIMIT);
                (len, TRASH_EFFECT_START)
            }
            CountCappedZone::Material(perm_handle) => {
                let stack_len = self
                    .game
                    .player(perm_handle.player)
                    .battle_area
                    .get(perm_handle.index as usize)
                    .map(|p| p.card_sources.len())
                    .unwrap_or(0);
                // Exclude top card: sources are indices 0..stack_len-1.
                let source_count = stack_len.saturating_sub(1);
                debug_assert!(
                    source_count <= SOURCES_PER_FIELD as usize,
                    "Material zone: source_count {} exceeds SOURCES_PER_FIELD {} for field_index {}",
                    source_count, SOURCES_PER_FIELD, perm_handle.index
                );
                let base = SOURCE_SELECT_START + perm_handle.index as u16 * SOURCES_PER_FIELD;
                (source_count, base)
            }
        };

        // Collect all indices whose card passes the filter. We clone each card
        // to avoid a simultaneous immutable + mutable borrow of self.game.
        let mut candidate_indices: Vec<usize> = Vec::with_capacity(zone_len);
        for i in 0..zone_len {
            let card_clone = match zone {
                CountCappedZone::Hand => self.game.player(of_player).hand[i].clone(),
                CountCappedZone::Trash => self.game.player(of_player).trash[i].clone(),
                CountCappedZone::Material(perm_handle) => {
                    // index i corresponds to card_sources[i] (0 = bottom, excludes top).
                    self.game.player(perm_handle.player).battle_area[perm_handle.index as usize]
                        .card_sources[i]
                        .clone()
                }
            };
            if filter(self.game, &card_clone) {
                candidate_indices.push(i);
            }
        }

        // Fewer candidates than the required minimum → the cost is unpayable.
        // Silently no-op WITHOUT firing the callback (no partial pick).
        // G-SELECT-MULTI-MIN.
        if min > 0 && candidate_indices.len() < min as usize {
            return;
        }

        // Empty filter → invoke final callback immediately; no selection installed.
        // (Only reachable when `min == 0` — the `min > 0` short-circuit above
        // already handled the unpayable case.)
        if candidate_indices.is_empty() {
            callback(self, Vec::new());
            return;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;
        let previous_phase = self.game.current_phase;
        let prompt_owned = prompt.to_string();

        let final_callback: Box<
            dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync,
        > = Box::new(
            move |game: &mut Game, picks: Vec<crate::card_source::CardHandle>| {
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                callback(&mut ctx, picks);
            },
        );

        install_count_capped_step(
            self.game,
            of_player,
            zone,
            range_start,
            min,
            max,
            is_optional_zero,
            distinct_by,
            candidate_indices,
            Vec::new(), // accum starts empty
            prompt_owned,
            source_card,
            source_permanent,
            source_kind,
            selecting_player,
            previous_phase,
            final_callback,
        );
    }

    /// Track E Tier 2 Task 10 — search the **own** security stack with a
    /// filter, surfacing a single-pick selection to the controller. Sugar
    /// over `select_security(self.player, ...)` for the common shape "look
    /// at your security stack and choose 1 card matching <filter>".
    ///
    /// `is_optional` controls whether the controller may PASS without
    /// picking. Empty-filter case is a silent no-op (no selection
    /// installed) — matches the contract of every other `select_*` helper.
    /// The callback receives the chosen security slot index. Caller is
    /// responsible for the post-pick mutation (e.g., add to hand, place in
    /// digivolution stack, return to deck).
    ///
    /// **Reveal semantics (no-approximations note):** the controller can
    /// inspect each candidate card's identity during the selection because
    /// the action mask exposes only own-side security indices that the
    /// controller is allowed to see. The opponent does NOT receive any
    /// information (the mask + observation tensor route own-security to
    /// the controller's observation only). Temporary face-up marking is
    /// not required — the visibility is implicit in the controller's view
    /// of their own stack.
    pub fn search_own_security_stack<F, C>(
        &mut self,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, &CardSource) -> bool,
        C: FnOnce(&mut EffectContext<'_>, usize) + Send + Sync + 'static,
    {
        let player = self.player;
        // Adapt the &CardSource filter to the existing `select_security`'s
        // filter signature (Game, usize).
        let filter_by_index = |game: &Game, idx: usize| -> bool {
            game.player(player)
                .security
                .get(idx)
                .map(|c| filter(game, c))
                .unwrap_or(false)
        };
        self.select_security(player, prompt, is_optional, filter_by_index, callback);
    }

    /// Mark a security slot face-up. Consumed by the observation tensor so
    /// the card's identity is exposed to the RL agent (it would otherwise be
    /// zeroed for hidden-information safety — §3.3). The slot is keyed by
    /// the card's instance index.
    pub fn mark_security_face_up(&mut self, of_player: PlayerId, card: &CardSource) {
        self.game
            .player_mut(of_player)
            .face_up_security
            .insert(card.card_index);
    }

    /// Play the card currently parked in `Game.pending_security` onto
    /// `self.player`'s field without paying cost. Called from a
    /// `SecuritySkill` effect's process closure to implement "Play this
    /// card without paying the cost" text.
    ///
    /// Raises the `played` bit on `Game.pending_security` so the
    /// security-resolution loop skips the default trash-after-check step.
    /// Fires `OnPlay` effects on the newly-created permanent — matches the
    /// `play_from_hand` flow. Does NOT pay memory.
    ///
    /// Silently no-ops if the field is full or no security check is in
    /// progress. Mirrors Python's `Game.effect_play_from_security`.
    ///
    /// Phase 6 flood-gate: if the revealed card is a Digimon and the
    /// defender has `CannotPlayDigimonByEffect` installed, the play is
    /// blocked — this method returns without raising `pending.played`.
    /// The security-resolution loop in `combat.rs` sees `played == false`
    /// and trashes the card via its normal "didn't stick" path.
    ///
    /// **Renamed in Phase 2f1 Task 3a** — formerly `play_from_security`.
    /// The 0-arg method was renamed to disambiguate from the new
    /// `EffectContext::play_from_security(player)` primitive (top-of-
    /// security-stack play, BT12-091 et al.). This method consumes the
    /// transient `pending_security` state set up by the security-check
    /// loop; the new primitive operates on a player's persistent
    /// `security` zone. Both are needed — they cover distinct card-text
    /// shapes — so a name disambiguation was required.
    pub fn play_pending_security(&mut self) {
        let turn = self.game.turn_count;
        let field_slots = self.game.rules.field_slots as usize;

        let Some(pending) = self.game.pending_security.as_ref() else {
            return;
        };
        if pending.played {
            return;
        }
        let defender = pending.defender;
        let card = pending.card.clone();

        if self.game.player(defender).battle_area.len() >= field_slots {
            return;
        }

        // Phase 6: CannotPlayDigimonByEffect gates effect-initiated plays,
        // including security triggers. If the revealed card is a Digimon
        // and the defending player has the modifier installed, block the
        // play by returning early without raising `pending.played`.
        // The security-resolution loop (combat.rs) will see `played == false`
        // and trash the card via the normal "didn't stick" path — no double
        // push needed here.
        // Tamer security triggers are NOT gated — only Digimon.
        let is_digimon = card.card_kind(&self.game.card_data) == CardKind::Digimon;
        if is_digimon
            && self
                .game
                .modifiers
                .player_has(defender, ModifierType::CannotPlayDigimonByEffect)
        {
            return;
        }

        let perm = crate::permanent::Permanent::new(card, turn);
        self.game.player_mut(defender).battle_area.push(perm);
        let field_index = self.game.player(defender).battle_area.len() - 1;

        // Raise the played bit so the security-resolution caller skips trash.
        if let Some(pending) = self.game.pending_security.as_mut() {
            pending.played = true;
        }

        self.game.fire_on_play(defender, field_index);
    }

    /// Shared implementation for the field-selection prompts. Encodes each
    /// valid field slot as `encode_attack(0, slot)` (reusing the ATTACK
    /// target half of the action space — matches Python's strategy).
    fn install_field_selection<F, C>(
        &mut self,
        kind: SelectionKind,
        phase: GamePhase,
        target_player: PlayerId,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, PermanentHandle) -> bool,
        C: FnOnce(&mut EffectContext<'_>, PermanentHandle) + Send + Sync + 'static,
    {
        use crate::action::space::{encode_attack, ATTACK_START, TARGETS_PER_ATTACKER};

        let target_count = self.game.player(target_player).battle_area.len();
        let mut valid_action_ids: Vec<u16> = Vec::new();
        for i in 0..target_count {
            let h = PermanentHandle {
                player: target_player,
                index: i as u8,
            };
            if filter(self.game, h) {
                valid_action_ids.push(encode_attack(0, i as u16));
            }
        }

        // Empty valid set → silently no-op. The RL policy never sees a
        // "mandatory prompt with no legal answer" state, matching Python.
        if valid_action_ids.is_empty() {
            return;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let controller = self.player;
        let override_pin = self.override_selecting_player;
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let source_kind = self.source_kind;

        // Wrap the user callback: at resolution time we need to build a
        // fresh EffectContext, decode the action ID to a PermanentHandle,
        // and hand both to the user.
        let user_callback: Box<dyn FnOnce(&mut EffectContext<'_>, PermanentHandle) + Send + Sync> =
            Box::new(callback);

        let previous_phase = self.game.current_phase;
        self.game.current_phase = phase;
        self.game.pending_selection = Some(PendingSelection {
            kind,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                // Decode: action - ATTACK_START gives (0-indexed) slot. Since
                // we always encode with attacker=0, the modulus isolates
                // target_field_index directly.
                let offset = action_id.saturating_sub(ATTACK_START);
                let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let h = PermanentHandle {
                    player: target_player,
                    index: target_index,
                };
                let mut ctx = EffectContext::new_with_source_kind_and_override(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                );
                user_callback(&mut ctx, h);
            }),
            on_decline: None,
        });
    }

    // ─── Opponent-as-selector builder ─────────────────────────────────────────

    /// Returns a scope that overrides `selecting_player` for any `select_*`
    /// call made through it. Use when card text reads "your opponent chooses"
    /// rather than "you choose".
    ///
    /// Example:
    /// ```ignore
    /// // "Your opponent chooses one of your Digimon and trashes it."
    /// ctx.as_selecting_player(opponent).select_own_permanent(
    ///     "Opponent: choose a Digimon to trash",
    ///     false,
    ///     |_g, _perm| true,
    ///     |ctx, handle| { ctx.delete_permanent(handle); },
    /// );
    /// ```
    ///
    /// The override is set immediately before the underlying helper runs and
    /// cleared before `select_*` returns — it does not persist past the
    /// method call.
    pub fn as_selecting_player(
        &mut self,
        player: crate::enums::PlayerId,
    ) -> EffectContextSelectorScope<'_, 'a> {
        EffectContextSelectorScope {
            ctx: self,
            selecting_player: player,
        }
    }
}

// ── EffectContextSelectorScope ───────────────────────────────────────────────

/// Scope returned by [`EffectContext::as_selecting_player`]. Each `select_*`
/// method on this scope temporarily sets `ctx.override_selecting_player` to
/// `self.selecting_player` before delegating to the underlying
/// `EffectContext::select_*` helper, then clears the override before returning.
///
/// This guarantees the override never outlives the method call — even if the
/// underlying helper is a no-op (empty filter → early return).
///
/// Not forwarded: `select_material`, `select_reveal`, `select_security` —
/// these are rarely routed to the opponent; add a forwarder here if a real card requires it.
pub struct EffectContextSelectorScope<'scope, 'g> {
    ctx: &'scope mut EffectContext<'g>,
    selecting_player: crate::enums::PlayerId,
}

impl<'scope, 'g> EffectContextSelectorScope<'scope, 'g> {
    /// Install a selection where `self.selecting_player` picks from the
    /// effect controller's own battle area. Forwards to
    /// `EffectContext::select_own_permanent` with the override applied.
    pub fn select_own_permanent<F, C>(
        &mut self,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&crate::game::Game, crate::permanent::PermanentHandle) -> bool,
        C: FnOnce(&mut EffectContext<'_>, crate::permanent::PermanentHandle)
            + Send
            + Sync
            + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx
            .select_own_permanent(prompt, is_optional, filter, callback);
        self.ctx.override_selecting_player = prev;
    }

    /// Install a selection where `self.selecting_player` picks from the
    /// effect controller's opponent's battle area. Forwards to
    /// `EffectContext::select_opponent_permanent` with the override applied.
    pub fn select_opponent_permanent<F, C>(
        &mut self,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&crate::game::Game, crate::permanent::PermanentHandle) -> bool,
        C: FnOnce(&mut EffectContext<'_>, crate::permanent::PermanentHandle)
            + Send
            + Sync
            + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx
            .select_opponent_permanent(prompt, is_optional, filter, callback);
        self.ctx.override_selecting_player = prev;
    }

    /// Install an effect-choice selection where `self.selecting_player` picks
    /// the branch. Forwards to `EffectContext::select_effect_choice`.
    pub fn select_effect_choice<C>(&mut self, prompt: &str, labels: Vec<String>, callback: C)
    where
        C: FnOnce(&mut EffectContext<'_>, usize) + Send + Sync + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx.select_effect_choice(prompt, labels, callback);
        self.ctx.override_selecting_player = prev;
    }

    /// Install a hand-pick selection where `self.selecting_player` picks from
    /// `of_player`'s hand. Forwards to `EffectContext::select_hand`.
    pub fn select_hand<F, C>(
        &mut self,
        of_player: crate::enums::PlayerId,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&crate::game::Game, usize) -> bool,
        C: FnOnce(&mut EffectContext<'_>, usize) + Send + Sync + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx
            .select_hand(of_player, prompt, is_optional, filter, callback);
        self.ctx.override_selecting_player = prev;
    }

    /// Install a trash-pick selection where `self.selecting_player` picks from
    /// `of_player`'s trash. Forwards to `EffectContext::select_trash`.
    pub fn select_trash<F, C>(
        &mut self,
        of_player: crate::enums::PlayerId,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&crate::game::Game, usize) -> bool,
        C: FnOnce(&mut EffectContext<'_>, usize) + Send + Sync + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx
            .select_trash(of_player, prompt, is_optional, filter, callback);
        self.ctx.override_selecting_player = prev;
    }

    /// Install a union-zone selection where `self.selecting_player` picks.
    /// Forwards to `EffectContext::select_union_zone`.
    pub fn select_union_zone<F, C>(
        &mut self,
        of_player: crate::enums::PlayerId,
        zones: crate::selection::UnionZoneSet,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&crate::game::Game, &crate::card_source::CardSource) -> bool,
        C: FnOnce(
                &mut EffectContext<'_>,
                crate::card_source::CardHandle,
                crate::selection::UnionZoneOrigin,
            ) + Send
            + Sync
            + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx
            .select_union_zone(of_player, zones, prompt, is_optional, filter, callback);
        self.ctx.override_selecting_player = prev;
    }

    /// Install a count-capped multi-select where `self.selecting_player` picks.
    /// Forwards to `EffectContext::select_count_capped_multi`.
    pub fn select_count_capped_multi<F, C>(
        &mut self,
        of_player: crate::enums::PlayerId,
        zone: crate::effect_context::CountCappedZone,
        max: u8,
        prompt: &str,
        is_optional_zero: bool,
        distinct_by: Option<crate::effect_context::DistinctByMode>,
        filter: F,
        callback: C,
    ) where
        F: Fn(&crate::game::Game, &crate::card_source::CardSource) -> bool + Send + Sync + 'static,
        C: FnOnce(&mut EffectContext<'_>, Vec<crate::card_source::CardHandle>)
            + Send
            + Sync
            + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx.select_count_capped_multi(
            of_player,
            zone,
            max,
            prompt,
            is_optional_zero,
            distinct_by,
            filter,
            callback,
        );
        self.ctx.override_selecting_player = prev;
    }

    /// Install an ordered-permutation selection where `self.selecting_player`
    /// picks. Forwards to `EffectContext::select_ordered_permutation`.
    pub fn select_ordered_permutation<C>(
        &mut self,
        items: Vec<crate::card_source::CardHandle>,
        prompt: &str,
        callback: C,
    ) where
        C: FnOnce(&mut EffectContext<'_>, Vec<crate::card_source::CardHandle>)
            + Send
            + Sync
            + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx.select_ordered_permutation(items, prompt, callback);
        self.ctx.override_selecting_player = prev;
    }
}

type RevealBucketCallback = std::sync::Arc<
    std::sync::Mutex<
        Option<Box<dyn FnOnce(&mut Game, Vec<(String, Vec<CardHandle>)>) + Send + Sync>>,
    >,
>;

// ── reveal bucket trampoline ────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn install_reveal_bucket_step(
    game: &mut Game,
    buckets: Vec<RevealBucketSelection>,
    bucket_index: usize,
    picked_buckets: Vec<(String, Vec<CardHandle>)>,
    current_bucket_picks: Vec<CardHandle>,
    prompt: String,
    no_duplicate_cards: bool,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: EffectSourceKind,
    selecting_player: PlayerId,
    previous_phase: GamePhase,
    final_callback: RevealBucketCallback,
) {
    use crate::action::space::{MAX_REVEALED, SEL_REVEAL_START};

    let Some(bucket) = buckets.get(bucket_index).cloned() else {
        if let Some(cb) = final_callback.lock().unwrap().take() {
            cb(game, picked_buckets);
        }
        return;
    };

    if bucket.max == 0 {
        let mut next_picked = picked_buckets;
        next_picked.push((bucket.bind_as, Vec::new()));
        install_reveal_bucket_step(
            game,
            buckets,
            bucket_index + 1,
            next_picked,
            Vec::new(),
            prompt,
            no_duplicate_cards,
            source_card,
            source_permanent,
            source_kind,
            selecting_player,
            previous_phase,
            final_callback,
        );
        return;
    }

    let already_chosen: std::collections::HashSet<CardHandle> = picked_buckets
        .iter()
        .flat_map(|(_, cards)| cards.iter().copied())
        .chain(current_bucket_picks.iter().copied())
        .collect();
    let cap = game.revealed_cards.len().min(MAX_REVEALED);
    let mut valid_action_ids = Vec::new();
    for idx in 0..cap {
        let handle = game.revealed_cards[idx].handle();
        if !bucket.candidates.contains(&handle) {
            continue;
        }
        if current_bucket_picks.contains(&handle) {
            continue;
        }
        if no_duplicate_cards && already_chosen.contains(&handle) {
            continue;
        }
        valid_action_ids.push(SEL_REVEAL_START + idx as u16);
    }

    if valid_action_ids.is_empty() {
        let mut next_picked = picked_buckets;
        next_picked.push((bucket.bind_as, current_bucket_picks));
        install_reveal_bucket_step(
            game,
            buckets,
            bucket_index + 1,
            next_picked,
            Vec::new(),
            prompt,
            no_duplicate_cards,
            source_card,
            source_permanent,
            source_kind,
            selecting_player,
            previous_phase,
            final_callback,
        );
        return;
    }

    let picked = current_bucket_picks.len() as u8;
    let is_optional = picked >= bucket.min;
    let callback_bucket = bucket.clone();
    let decline_bucket = bucket;
    let current_for_callback = current_bucket_picks.clone();
    let current_for_decline = current_bucket_picks;
    let picked_for_callback = picked_buckets.clone();
    let picked_for_decline = picked_buckets;

    game.current_phase = GamePhase::SelectReveal;
    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::RevealBucket {
            bucket_index: bucket_index as u8,
            min: callback_bucket.min,
            max: callback_bucket.max,
            picked,
        },
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional,
        prompt: prompt.clone(),
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new({
            let buckets = buckets.clone();
            let prompt = prompt.clone();
            let final_callback = std::sync::Arc::clone(&final_callback);
            move |game: &mut Game, action_id: u16| {
                let reveal_index = action_id.saturating_sub(SEL_REVEAL_START) as usize;
                let Some(card) = game.revealed_cards.get(reveal_index) else {
                    return;
                };
                let mut next_current = current_for_callback;
                next_current.push(card.handle());

                if next_current.len() >= callback_bucket.max as usize {
                    let mut next_picked = picked_for_callback;
                    next_picked.push((callback_bucket.bind_as.clone(), next_current));
                    install_reveal_bucket_step(
                        game,
                        buckets,
                        bucket_index + 1,
                        next_picked,
                        Vec::new(),
                        prompt,
                        no_duplicate_cards,
                        source_card,
                        source_permanent,
                        source_kind,
                        selecting_player,
                        previous_phase,
                        final_callback,
                    );
                    return;
                }

                install_reveal_bucket_step(
                    game,
                    buckets,
                    bucket_index,
                    picked_for_callback,
                    next_current,
                    prompt,
                    no_duplicate_cards,
                    source_card,
                    source_permanent,
                    source_kind,
                    selecting_player,
                    previous_phase,
                    final_callback,
                );
            }
        }),
        on_decline: Some(Box::new(move |game: &mut Game| {
            let mut next_picked = picked_for_decline;
            next_picked.push((decline_bucket.bind_as.clone(), current_for_decline));
            install_reveal_bucket_step(
                game,
                buckets,
                bucket_index + 1,
                next_picked,
                Vec::new(),
                prompt,
                no_duplicate_cards,
                source_card,
                source_permanent,
                source_kind,
                selecting_player,
                previous_phase,
                final_callback,
            );
        })),
    });
}

// ── ordered permutation trampoline ──────────────────────────────────────────

/// Install one step of an ordered-permutation selection into `game`.
///
/// Each step presents the player with `remaining.len()` choices encoded as
/// `SEL_REVEAL_START + i`. When the player picks index `i`, the chosen item
/// is appended to `accum`, removed from `remaining`, and this function is
/// called again for the next step — until `remaining` is empty, at which
/// point `final_callback` fires with `accum` (the full ordered result).
///
/// This is a free function (not a method on `EffectContext`) so it can be
/// called from inside a `Box<dyn FnOnce>` without any recursive closure
/// or `Arc` indirection. The pattern works because each step's closure is a
/// plain `FnOnce` that captures `remaining`, `accum`, and a new box for the
/// next step — no closure references itself.
#[allow(clippy::too_many_arguments)]
fn install_permutation_step(
    game: &mut Game,
    remaining: Vec<crate::card_source::CardHandle>,
    accum: Vec<crate::card_source::CardHandle>,
    prompt: String,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<crate::permanent::PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
    selecting_player: crate::enums::PlayerId,
    previous_phase: GamePhase,
    final_callback: Box<dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync>,
) {
    use crate::action::space::SEL_REVEAL_START;
    use crate::selection::{PendingSelection, SelectionKind};

    let n = remaining.len() as u8;
    let valid_action_ids: Vec<u16> = (0..remaining.len())
        .map(|i| SEL_REVEAL_START + i as u16)
        .collect();

    game.current_phase = GamePhase::SelectPermutation;
    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::OrderedPermutation { remaining: n },
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: false,
        prompt: prompt.clone(),
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game: &mut Game, action_id: u16| {
            debug_assert!(
                action_id >= SEL_REVEAL_START && action_id < SEL_REVEAL_START + remaining.len() as u16,
                "select_ordered_permutation: action_id {} outside expected range [{}, {}); valid_action_ids was populated incorrectly",
                action_id, SEL_REVEAL_START, SEL_REVEAL_START + remaining.len() as u16
            );
            let pick_idx = (action_id - SEL_REVEAL_START) as usize;
            let mut new_remaining = remaining;
            let mut new_accum = accum;
            let picked = new_remaining.remove(pick_idx);
            new_accum.push(picked);

            if new_remaining.is_empty() {
                // All items placed — invoke the final callback.
                final_callback(game, new_accum);
            } else {
                // More items remain — install the next step.
                install_permutation_step(
                    game,
                    new_remaining,
                    new_accum,
                    prompt,
                    source_card,
                    source_permanent,
                    source_kind,
                    selecting_player,
                    previous_phase,
                    final_callback,
                );
            }
        }),
        on_decline: None,
    });
}

// ── cross-permanent source multi-select trampoline ───────────────────────────

type SourceFilter = std::sync::Arc<dyn Fn(&Game, SourceSelectionRef) -> bool + Send + Sync>;
type SourceFinalCallback =
    Box<dyn FnOnce(&mut Game, Vec<SourceSelectionRef>) + Send + Sync + 'static>;

fn partition_source_refs(
    game: &Game,
    host: PermanentHandle,
    requirements: &[PartitionRequirement],
) -> Vec<SourceSelectionRef> {
    use crate::action::space::SOURCES_PER_FIELD;

    let Some(perm) = game
        .player(host.player)
        .battle_area
        .get(host.index as usize)
    else {
        return Vec::new();
    };
    let source_count = perm.card_sources.len().saturating_sub(1);
    let cap = source_count.min(SOURCES_PER_FIELD as usize);
    let mut out = Vec::new();
    for source_index in 0..cap {
        let source_ref = SourceSelectionRef {
            permanent: host,
            field_index: host.index,
            source_index: source_index as u8,
            card: perm.card_sources[source_index].handle(),
        };
        if requirements
            .iter()
            .any(|requirement| (requirement.matches)(game, source_ref))
        {
            out.push(source_ref);
        }
    }
    out
}

fn partition_requirements_match(
    game: &Game,
    requirements: &[PartitionRequirement],
    selected: &[SourceSelectionRef],
) -> bool {
    if selected.len() != requirements.len() {
        return false;
    }
    let mut used = vec![false; selected.len()];
    partition_match_from(game, requirements, selected, &mut used, 0)
}

fn partition_can_extend(
    game: &Game,
    host: PermanentHandle,
    requirements: &[PartitionRequirement],
    picked: &[SourceSelectionRef],
) -> bool {
    if picked.len() > requirements.len() {
        return false;
    }
    if picked.len() == requirements.len() {
        return partition_requirements_match(game, requirements, picked);
    }
    let candidates: Vec<SourceSelectionRef> = partition_source_refs(game, host, requirements)
        .into_iter()
        .filter(|candidate| !picked.iter().any(|p| p.card == candidate.card))
        .collect();
    partition_can_extend_from(game, requirements, picked.to_vec(), &candidates, 0)
}

fn partition_can_extend_from(
    game: &Game,
    requirements: &[PartitionRequirement],
    picked: Vec<SourceSelectionRef>,
    candidates: &[SourceSelectionRef],
    start: usize,
) -> bool {
    if picked.len() == requirements.len() {
        return partition_requirements_match(game, requirements, &picked);
    }
    for index in start..candidates.len() {
        let mut next = picked.clone();
        next.push(candidates[index]);
        if partition_can_extend_from(game, requirements, next, candidates, index + 1) {
            return true;
        }
    }
    false
}

fn partition_next_candidates(
    game: &Game,
    host: PermanentHandle,
    requirements: &[PartitionRequirement],
    picked: &[SourceSelectionRef],
) -> Vec<(u16, SourceSelectionRef)> {
    use crate::action::space::encode_source_select;

    partition_source_refs(game, host, requirements)
        .into_iter()
        .filter(|candidate| !picked.iter().any(|p| p.card == candidate.card))
        .filter(|candidate| {
            let mut next = picked.to_vec();
            next.push(*candidate);
            partition_can_extend(game, host, requirements, &next)
        })
        .filter_map(|source_ref| {
            encode_source_select(
                source_ref.field_index as u16,
                source_ref.source_index as u16,
            )
            .map(|action_id| (action_id, source_ref))
        })
        .collect()
}

fn partition_match_from(
    game: &Game,
    requirements: &[PartitionRequirement],
    selected: &[SourceSelectionRef],
    used: &mut [bool],
    requirement_index: usize,
) -> bool {
    if requirement_index == requirements.len() {
        return true;
    }
    for source_index in 0..selected.len() {
        if used[source_index] {
            continue;
        }
        if !(requirements[requirement_index].matches)(game, selected[source_index]) {
            continue;
        }
        used[source_index] = true;
        if partition_match_from(game, requirements, selected, used, requirement_index + 1) {
            return true;
        }
        used[source_index] = false;
    }
    false
}

#[allow(clippy::too_many_arguments)]
fn install_partition_source_selection(
    game: &mut Game,
    host: PermanentHandle,
    controller: PlayerId,
    selecting_player: PlayerId,
    override_pin: Option<PlayerId>,
    prompt: String,
    requirements: std::sync::Arc<Vec<PartitionRequirement>>,
    picked: Vec<SourceSelectionRef>,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
    previous_phase: GamePhase,
    final_callback: SourceFinalCallback,
) {
    use std::sync::{Arc, Mutex};

    if picked.len() == requirements.len() {
        if partition_requirements_match(game, &requirements, &picked) {
            final_callback(game, picked);
        }
        return;
    }

    let candidates = partition_next_candidates(game, host, &requirements, &picked);
    if candidates.is_empty() {
        return;
    }
    let valid_action_ids: Vec<u16> = candidates.iter().map(|(action_id, _)| *action_id).collect();

    let shared_cb: Arc<Mutex<Option<SourceFinalCallback>>> =
        Arc::new(Mutex::new(Some(final_callback)));
    let action_to_source = Arc::new(candidates);
    let prompt_for_next = prompt.clone();
    let requirements_for_next = std::sync::Arc::clone(&requirements);
    let picked_for_pick = picked.clone();

    game.current_phase = GamePhase::SelectSource;
    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::SourceMulti {
            min: requirements.len() as u8,
            max: requirements.len() as u8,
            picked: picked.len() as u8,
        },
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: false,
        prompt,
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game: &mut Game, action_id: u16| {
            let (_, source_ref) = action_to_source
                .iter()
                .find(|(candidate_action, _)| *candidate_action == action_id)
                .copied()
                .expect("partition source action must have been in valid_action_ids");
            let mut next_picked = picked_for_pick;
            next_picked.push(source_ref);

            let next_cb: SourceFinalCallback = Box::new(move |game, picks| {
                let cb_opt = shared_cb.lock().unwrap().take();
                debug_assert!(
                    cb_opt.is_some(),
                    "partition source final callback already consumed"
                );
                if let Some(cb) = cb_opt {
                    cb(game, picks);
                }
            });

            install_partition_source_selection(
                game,
                host,
                controller,
                selecting_player,
                override_pin,
                prompt_for_next,
                requirements_for_next,
                next_picked,
                source_card,
                source_permanent,
                source_kind,
                previous_phase,
                next_cb,
            );
        }),
        on_decline: None,
    });
}

fn source_multi_candidates(
    game: &Game,
    of_player: PlayerId,
    picked: &[SourceSelectionRef],
    filter: &SourceFilter,
) -> Vec<(u16, SourceSelectionRef)> {
    use crate::action::space::encode_source_select;

    let mut out = Vec::new();
    for (field_index, perm) in game.player(of_player).battle_area.iter().enumerate() {
        if perm.card_sources.len() <= 1 {
            continue;
        }
        for source_index in 0..(perm.card_sources.len() - 1) {
            let card = perm.card_sources[source_index].handle();
            let permanent = PermanentHandle {
                player: of_player,
                index: field_index as u8,
            };
            let source_ref = SourceSelectionRef {
                permanent,
                field_index: field_index as u8,
                source_index: source_index as u8,
                card,
            };
            if picked.iter().any(|p| p.card == card) {
                continue;
            }
            if !(filter)(game, source_ref) {
                continue;
            }
            if let Some(action_id) = encode_source_select(field_index as u16, source_index as u16) {
                out.push((action_id, source_ref));
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn install_source_multi_selection(
    game: &mut Game,
    of_player: PlayerId,
    selecting_player: PlayerId,
    override_pin: Option<PlayerId>,
    prompt: String,
    min: u8,
    max: u8,
    picked: Vec<SourceSelectionRef>,
    filter: SourceFilter,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
    previous_phase: GamePhase,
    final_callback: SourceFinalCallback,
) {
    use crate::action::space::PASS;
    use std::sync::{Arc, Mutex};

    let candidates = source_multi_candidates(game, of_player, &picked, &filter);
    if candidates.is_empty() || picked.len() == max as usize {
        if picked.len() >= min as usize {
            final_callback(game, picked);
        }
        return;
    }

    let mut valid_action_ids: Vec<u16> =
        candidates.iter().map(|(action_id, _)| *action_id).collect();
    if picked.len() >= min as usize {
        valid_action_ids.push(PASS);
    }

    let shared_cb: Arc<Mutex<Option<SourceFinalCallback>>> =
        Arc::new(Mutex::new(Some(final_callback)));
    let shared_cb_decline = Arc::clone(&shared_cb);
    let action_to_source = Arc::new(candidates);
    let prompt_for_next = prompt.clone();
    let filter_for_next = Arc::clone(&filter);
    let picked_for_pick = picked.clone();
    let picked_for_decline = picked.clone();

    game.current_phase = GamePhase::SelectSource;
    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::SourceMulti {
            min,
            max,
            picked: picked.len() as u8,
        },
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: picked.len() >= min as usize,
        prompt,
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game: &mut Game, action_id: u16| {
            let (_, source_ref) = action_to_source
                .iter()
                .find(|(candidate_action, _)| *candidate_action == action_id)
                .copied()
                .expect("source action must have been in valid_action_ids");
            let mut next_picked = picked_for_pick;
            next_picked.push(source_ref);

            let next_cb: SourceFinalCallback = Box::new(move |game, picks| {
                let cb_opt = shared_cb.lock().unwrap().take();
                debug_assert!(cb_opt.is_some(), "source final callback already consumed");
                if let Some(cb) = cb_opt {
                    cb(game, picks);
                }
            });

            install_source_multi_selection(
                game,
                of_player,
                selecting_player,
                override_pin,
                prompt_for_next,
                min,
                max,
                next_picked,
                filter_for_next,
                source_card,
                source_permanent,
                source_kind,
                previous_phase,
                next_cb,
            );
        }),
        on_decline: Some(Box::new(move |game: &mut Game| {
            let cb_opt = shared_cb_decline.lock().unwrap().take();
            debug_assert!(cb_opt.is_some(), "source final callback already consumed");
            if let Some(cb) = cb_opt {
                cb(game, picked_for_decline);
            }
        })),
    });
}

// ── opponent DP-budget permanent multi-select trampoline ─────────────────────

type DpBudgetFilter = std::sync::Arc<dyn Fn(&Game, PermanentHandle) -> bool + Send + Sync>;
type DpBudgetFinalCallback =
    Box<dyn FnOnce(&mut Game, Vec<PermanentHandle>) + Send + Sync + 'static>;

fn dp_budget_candidates(
    game: &Game,
    opponent: PlayerId,
    remaining_dp: i32,
    picked: &[PermanentHandle],
    filter: &DpBudgetFilter,
) -> Vec<(u16, PermanentHandle, i32)> {
    use crate::action::space::encode_attack;

    let mut out = Vec::new();
    for (index, _perm) in game.player(opponent).battle_area.iter().enumerate() {
        let handle = PermanentHandle {
            player: opponent,
            index: index as u8,
        };
        if picked.contains(&handle) {
            continue;
        }
        if !(filter)(game, handle) {
            continue;
        }
        let dp = game.effective_dp(handle).unwrap_or(0);
        if dp <= remaining_dp {
            out.push((encode_attack(0, index as u16), handle, dp));
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn install_dp_budget_selection(
    game: &mut Game,
    opponent: PlayerId,
    controller: PlayerId,
    selecting_player: PlayerId,
    override_pin: Option<PlayerId>,
    prompt: String,
    remaining_dp: i32,
    min_picks: u8,
    picked: Vec<PermanentHandle>,
    filter: DpBudgetFilter,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
    previous_phase: GamePhase,
    final_callback: DpBudgetFinalCallback,
) {
    use crate::action::space::PASS;
    use std::sync::{Arc, Mutex};

    let candidates = dp_budget_candidates(game, opponent, remaining_dp, &picked, &filter);
    if candidates.is_empty() {
        if picked.len() >= min_picks as usize {
            final_callback(game, picked);
        }
        return;
    }

    let mut valid_action_ids: Vec<u16> = candidates
        .iter()
        .map(|(action_id, _, _)| *action_id)
        .collect();
    if picked.len() >= min_picks as usize {
        valid_action_ids.push(PASS);
    }

    let shared_cb: Arc<Mutex<Option<DpBudgetFinalCallback>>> =
        Arc::new(Mutex::new(Some(final_callback)));
    let shared_cb_decline = Arc::clone(&shared_cb);
    let action_to_target = Arc::new(candidates);
    let prompt_for_next = prompt.clone();
    let filter_for_next = Arc::clone(&filter);
    let picked_for_pick = picked.clone();
    let picked_for_decline = picked.clone();

    game.current_phase = GamePhase::SelectBudgeted;
    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::DpBudget {
            remaining_dp,
            picked: picked.len() as u8,
        },
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: picked.len() >= min_picks as usize,
        prompt,
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game: &mut Game, action_id: u16| {
            let (_, chosen, dp) = action_to_target
                .iter()
                .find(|(candidate_action, _, _)| *candidate_action == action_id)
                .copied()
                .expect("DP-budget action must have been in valid_action_ids");
            let mut next_picked = picked_for_pick;
            next_picked.push(chosen);

            let next_cb: DpBudgetFinalCallback = Box::new(move |game, picks| {
                let cb_opt = shared_cb.lock().unwrap().take();
                debug_assert!(
                    cb_opt.is_some(),
                    "DP-budget final callback already consumed"
                );
                if let Some(cb) = cb_opt {
                    cb(game, picks);
                }
            });

            install_dp_budget_selection(
                game,
                opponent,
                controller,
                selecting_player,
                override_pin,
                prompt_for_next,
                remaining_dp - dp,
                min_picks,
                next_picked,
                filter_for_next,
                source_card,
                source_permanent,
                source_kind,
                previous_phase,
                next_cb,
            );
        }),
        on_decline: Some(Box::new(move |game: &mut Game| {
            let cb_opt = shared_cb_decline.lock().unwrap().take();
            debug_assert!(
                cb_opt.is_some(),
                "DP-budget final callback already consumed"
            );
            if let Some(cb) = cb_opt {
                cb(game, picked_for_decline);
            }
        })),
    });
}

// ── opponent play-cost-budget permanent multi-select trampoline ──────────────
//
// Play-cost analog of the DP-budget trampoline above. The only structural
// differences are: the per-candidate metric is printed play cost (not
// effective DP) and the SelectionKind variant carries `remaining_play_cost`.
// G-MULTI-SELECT-OPP-PLAY-COST-SUM.

type PlayCostBudgetFilter = std::sync::Arc<dyn Fn(&Game, PermanentHandle) -> bool + Send + Sync>;
type PlayCostBudgetFinalCallback =
    Box<dyn FnOnce(&mut Game, Vec<PermanentHandle>) + Send + Sync + 'static>;

fn play_cost_budget_candidates(
    game: &Game,
    opponent: PlayerId,
    remaining_play_cost: i32,
    picked: &[PermanentHandle],
    filter: &PlayCostBudgetFilter,
) -> Vec<(u16, PermanentHandle, i32)> {
    use crate::action::space::encode_attack;

    let mut out = Vec::new();
    for (index, perm) in game.player(opponent).battle_area.iter().enumerate() {
        let handle = PermanentHandle {
            player: opponent,
            index: index as u8,
        };
        if picked.contains(&handle) {
            continue;
        }
        if !(filter)(game, handle) {
            continue;
        }
        let play_cost = i32::from(perm.top_card().play_cost(&game.card_data));
        if play_cost <= remaining_play_cost {
            out.push((encode_attack(0, index as u16), handle, play_cost));
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn install_play_cost_budget_selection(
    game: &mut Game,
    opponent: PlayerId,
    controller: PlayerId,
    selecting_player: PlayerId,
    override_pin: Option<PlayerId>,
    prompt: String,
    remaining_play_cost: i32,
    min_picks: u8,
    picked: Vec<PermanentHandle>,
    filter: PlayCostBudgetFilter,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
    previous_phase: GamePhase,
    final_callback: PlayCostBudgetFinalCallback,
) {
    use crate::action::space::PASS;
    use std::sync::{Arc, Mutex};

    let candidates =
        play_cost_budget_candidates(game, opponent, remaining_play_cost, &picked, &filter);
    if candidates.is_empty() {
        if picked.len() >= min_picks as usize {
            final_callback(game, picked);
        }
        return;
    }

    let mut valid_action_ids: Vec<u16> = candidates
        .iter()
        .map(|(action_id, _, _)| *action_id)
        .collect();
    if picked.len() >= min_picks as usize {
        valid_action_ids.push(PASS);
    }

    let shared_cb: Arc<Mutex<Option<PlayCostBudgetFinalCallback>>> =
        Arc::new(Mutex::new(Some(final_callback)));
    let shared_cb_decline = Arc::clone(&shared_cb);
    let action_to_target = Arc::new(candidates);
    let prompt_for_next = prompt.clone();
    let filter_for_next = Arc::clone(&filter);
    let picked_for_pick = picked.clone();
    let picked_for_decline = picked.clone();

    game.current_phase = GamePhase::SelectBudgeted;
    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::PlayCostBudget {
            remaining_play_cost,
            picked: picked.len() as u8,
        },
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: picked.len() >= min_picks as usize,
        prompt,
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game: &mut Game, action_id: u16| {
            let (_, chosen, play_cost) = action_to_target
                .iter()
                .find(|(candidate_action, _, _)| *candidate_action == action_id)
                .copied()
                .expect("play-cost-budget action must have been in valid_action_ids");
            let mut next_picked = picked_for_pick;
            next_picked.push(chosen);

            let next_cb: PlayCostBudgetFinalCallback = Box::new(move |game, picks| {
                let cb_opt = shared_cb.lock().unwrap().take();
                debug_assert!(
                    cb_opt.is_some(),
                    "play-cost-budget final callback already consumed"
                );
                if let Some(cb) = cb_opt {
                    cb(game, picks);
                }
            });

            install_play_cost_budget_selection(
                game,
                opponent,
                controller,
                selecting_player,
                override_pin,
                prompt_for_next,
                remaining_play_cost - play_cost,
                min_picks,
                next_picked,
                filter_for_next,
                source_card,
                source_permanent,
                source_kind,
                previous_phase,
                next_cb,
            );
        }),
        on_decline: Some(Box::new(move |game: &mut Game| {
            let cb_opt = shared_cb_decline.lock().unwrap().take();
            debug_assert!(
                cb_opt.is_some(),
                "play-cost-budget final callback already consumed"
            );
            if let Some(cb) = cb_opt {
                cb(game, picked_for_decline);
            }
        })),
    });
}

// ── count-capped multi-select trampoline ─────────────────────────────────────

/// Install one step of a count-capped multi-select into `game`.
///
/// Each step presents the player with the remaining (not yet picked) candidate
/// action IDs. `PASS` (action 62) is available when `is_optional_zero` is set
/// **or** at least one pick has already been made (`accum.len() >= 1`); this is
/// encoded via `PendingSelection::is_optional` so `resolve_generic_selection`
/// routes PASS to `on_decline` and the mask builder gates PASS correctly.
///
/// PASS resolution fires `on_decline`, which commits by calling `final_callback`.
/// Non-PASS picks fire `callback`, which either auto-commits (at max) or
/// re-installs for the next step. The shared `final_callback` is wrapped in
/// `Arc<Mutex<Option<...>>>` so both closures can take ownership when fired.
///
/// `candidate_indices` lists the zone indices still eligible (already-picked
/// indices are removed after each step). `range_start` is `PLAY_HAND_START`
/// for hand, `TRASH_EFFECT_START` for trash, or
/// `SOURCE_SELECT_START + field_index * SOURCES_PER_FIELD` for material.
///
/// This is a free function (not an `EffectContext` method) so it can be called
/// recursively from inside a `Box<dyn FnOnce>` without any `Arc`/self-reference.
#[allow(clippy::too_many_arguments)]
fn install_count_capped_step(
    game: &mut Game,
    of_player: PlayerId,
    zone: CountCappedZone,
    range_start: u16,
    min: u8,
    max: u8,
    is_optional_zero: bool,
    distinct_by: Option<DistinctByMode>,
    candidate_indices: Vec<usize>, // zone indices still eligible
    accum: Vec<crate::card_source::CardHandle>, // handles picked so far (in order)
    prompt: String,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<crate::permanent::PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
    selecting_player: PlayerId,
    previous_phase: GamePhase,
    final_callback: Box<dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync>,
) {
    use crate::selection::{PendingSelection, SelectionKind};
    use std::sync::{Arc, Mutex};

    let picked = accum.len() as u8;

    // `is_optional` drives PASS gating in `resolve_generic_selection` and the
    // mask builder. True when the player is allowed to commit early at this
    // step — i.e. when `picked` has reached the required floor. The floor is
    // `min` (G-SELECT-MULTI-MIN) raised to at least 1 unless `is_optional_zero`
    // permits a zero-pick finish.
    let effective_min = min.max(if is_optional_zero { 0 } else { 1 });
    let is_optional = picked >= effective_min;

    // Build valid_action_ids from remaining candidate indices.
    let valid_action_ids: Vec<u16> = candidate_indices
        .iter()
        .map(|&i| range_start + i as u16)
        .collect();

    // SAFETY INVARIANT: Exactly one of `callback` or `on_decline` will ever fire
    // per installed PendingSelection. When `resolve_generic_selection` dispatches
    // one of them, the PendingSelection is taken and dropped — dropping the other
    // closure with it. So only one `.take()` ever executes; the Arc ref-count on
    // the non-firing side drops to zero unused.
    //
    // Why Arc<Mutex<Option<_>>> instead of moving `final_callback` into one closure:
    // `FnOnce` isn't `Clone`, and `Effect` requires `Send + Sync` (rules out `Rc`).
    // The Mutex is structurally uncontested (engine is single-threaded at this
    // layer); its only role is to satisfy the `Send + Sync` bound on the shared
    // storage. See `install_permutation_step` for the simpler `FnOnce`-per-closure
    // pattern used where there's no PASS-commit alternative.
    let shared_cb: Arc<
        Mutex<
            Option<Box<dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync>>,
        >,
    > = Arc::new(Mutex::new(Some(final_callback)));
    let shared_cb_decline = Arc::clone(&shared_cb);

    // Clone the accum for the on_decline path (the callback path moves accum).
    let accum_for_decline = accum.clone();

    game.current_phase = GamePhase::SelectBudgeted;
    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::CountCappedMultiSelect { max, picked },
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional,
        prompt: prompt.clone(),
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game: &mut Game, action_id: u16| {
            debug_assert!(
                action_id >= range_start
                    && candidate_indices.contains(&((action_id - range_start) as usize)),
                "select_count_capped_multi callback: action_id {} not in expected zone range (range_start={}, candidates={:?})",
                action_id, range_start, candidate_indices
            );

            // Identify picked zone index and resolve the CardHandle.
            let pick_zone_idx = (action_id - range_start) as usize;
            let card_handle = match zone {
                CountCappedZone::Hand => game.player(of_player).hand[pick_zone_idx].handle(),
                CountCappedZone::Trash => game.player(of_player).trash[pick_zone_idx].handle(),
                CountCappedZone::Material(perm_handle) => {
                    // pick_zone_idx is a card_sources index (0 = bottom; top excluded).
                    game.player(perm_handle.player).battle_area[perm_handle.index as usize]
                        .card_sources[pick_zone_idx]
                        .handle()
                }
            };

            // Build new state for the next step.
            let mut new_accum = accum;
            new_accum.push(card_handle);

            // Auto-commit when max reached.
            if new_accum.len() == max as usize {
                let cb_opt = shared_cb.lock().unwrap().take();
                debug_assert!(
                    cb_opt.is_some(),
                    "count_capped invariant violated: final_callback already consumed (both paths fired?)"
                );
                if let Some(cb) = cb_opt {
                    cb(game, new_accum);
                }
                return;
            }

            // Pre-resolve the data_indices for all accumulated picks so the
            // distinct_by filter below can look up card attributes without
            // holding a conflicting borrow over the zone slice.
            //
            // We iterate the zone once per accumulated handle. Because picks are
            // removed from `candidate_indices` at each step but NOT physically
            // moved in the zone, the CardHandle is still findable by its
            // `card_index` field (the handle identity).
            let accum_data_indices: Vec<usize> = if distinct_by.is_some() {
                new_accum
                    .iter()
                    .filter_map(|&h| {
                        let zone_slice: &[crate::card_source::CardSource] = match zone {
                            CountCappedZone::Hand => &game.player(of_player).hand,
                            CountCappedZone::Trash => &game.player(of_player).trash,
                            CountCappedZone::Material(ph) => {
                                &game.player(ph.player).battle_area[ph.index as usize].card_sources
                            }
                        };
                        zone_slice
                            .iter()
                            .find(|c| c.handle() == h)
                            .map(|c| c.data_index)
                    })
                    .collect()
            } else {
                Vec::new()
            };

            // Remove the picked index from candidates for the next step.
            // If `distinct_by` is set, also remove any remaining index whose
            // card shares the constrained attribute with any already-picked card.
            let new_candidates: Vec<usize> = candidate_indices
                .into_iter()
                .filter(|&i| i != pick_zone_idx)
                .filter(|&i| {
                    let Some(mode) = distinct_by else { return true };
                    // Look up the candidate's card data.
                    let cand_data_idx = match zone {
                        CountCappedZone::Hand => game.player(of_player).hand[i].data_index,
                        CountCappedZone::Trash => game.player(of_player).trash[i].data_index,
                        CountCappedZone::Material(ph) => {
                            game.player(ph.player).battle_area[ph.index as usize].card_sources[i]
                                .data_index
                        }
                    };
                    let cand_data = &game.card_data[cand_data_idx];
                    // Reject the candidate if it matches any accumulated pick.
                    !accum_data_indices.iter().any(|&picked_data_idx| {
                        let picked_data = &game.card_data[picked_data_idx];
                        match mode {
                            DistinctByMode::CardNumber => picked_data.card_id == cand_data.card_id,
                            DistinctByMode::Level => {
                                matches!(
                                    (picked_data.level, cand_data.level),
                                    (Some(p), Some(c)) if p == c
                                )
                            }
                            DistinctByMode::Name => picked_data.card_name == cand_data.card_name,
                        }
                    })
                })
                .collect();

            // If no candidates remain, commit early with what we have.
            if new_candidates.is_empty() {
                let cb_opt = shared_cb.lock().unwrap().take();
                debug_assert!(
                    cb_opt.is_some(),
                    "count_capped invariant violated: final_callback already consumed (both paths fired?)"
                );
                if let Some(cb) = cb_opt {
                    cb(game, new_accum);
                }
                return;
            }

            // Must unwrap the final_callback from the Arc to pass it to the next
            // step as a `Box<dyn FnOnce>`. Since exactly one branch fires, the
            // `take()` is guaranteed to succeed here (the on_decline path has not
            // fired yet).
            let next_cb: Box<
                dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync,
            > = Box::new(move |game, picks| {
                let cb_opt = shared_cb.lock().unwrap().take();
                debug_assert!(
                        cb_opt.is_some(),
                        "count_capped invariant violated: final_callback already consumed (both paths fired?)"
                    );
                if let Some(cb) = cb_opt {
                    cb(game, picks);
                }
            });

            // Install the next step.
            install_count_capped_step(
                game,
                of_player,
                zone,
                range_start,
                min,
                max,
                is_optional_zero,
                distinct_by,
                new_candidates,
                new_accum,
                prompt,
                source_card,
                source_permanent,
                source_kind,
                selecting_player,
                previous_phase,
                next_cb,
            );
        }),
        on_decline: Some(Box::new(move |game: &mut Game| {
            // PASS commit — fire final callback with whatever has been picked.
            let cb_opt = shared_cb_decline.lock().unwrap().take();
            debug_assert!(
                cb_opt.is_some(),
                "count_capped invariant violated: final_callback already consumed (both paths fired?)"
            );
            if let Some(cb) = cb_opt {
                cb(game, accum_for_decline);
            }
        })),
    });
}
