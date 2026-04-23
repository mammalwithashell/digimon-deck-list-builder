//! Selection-prompt helpers on `EffectContext` — extracted from `mod.rs` for
//! readability. These are the 11 player-choice primitives card scripts use to
//! install a `PendingSelection`: field, hand, trash, material, reveal,
//! security, effect-choice, plus `mark_security_face_up` and
//! `play_from_security`.
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

use crate::card_source::CardSource;
use crate::effect_context::EffectContext;
use crate::enums::{CardKind, GamePhase, ModifierType, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};

/// Identifies which zone `select_count_capped_multi` draws candidates from.
///
/// - `Hand`  — action IDs map to `PLAY_HAND_START + i` (range 0–29)
/// - `Trash` — action IDs map to `TRASH_EFFECT_START + i` (range 1150–1194)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountCappedZone {
    Hand,
    Trash,
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
        self.install_field_selection(
            SelectionKind::OppField,
            GamePhase::SelectTarget,
            target_player,
            prompt,
            is_optional,
            filter,
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
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let user_callback: Box<
            dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync,
        > = Box::new(callback);

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
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let hand_index = action_id.saturating_sub(PLAY_HAND_START) as usize;
                let mut ctx =
                    EffectContext::new(game, source_card, source_permanent, selecting_player);
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
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let user_callback: Box<
            dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync,
        > = Box::new(callback);

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
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let trash_index = action_id.saturating_sub(TRASH_EFFECT_START) as usize;
                let mut ctx =
                    EffectContext::new(game, source_card, source_permanent, selecting_player);
                user_callback(&mut ctx, trash_index);
            }),
            on_decline: None,
        });
    }

    /// Prompt `self.player` to pick a card from a permanent's digivolution
    /// stack — i.e., one of the `card_sources` entries on `of_permanent`.
    /// Used for DNA / material-driven effects ("remove 1 of this Digimon's
    /// sources", "trash 2 materials to gain memory"). Parity §4.6d-residual.
    ///
    /// `filter(game, source_index)` runs once per source position at install
    /// time (index 0 = bottom card, index `card_sources.len()-1` = top).
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
        let source_count = match self
            .game
            .player(of_permanent.player)
            .battle_area
            .get(of_permanent.index as usize)
        {
            Some(perm) => perm.card_sources.len(),
            None => return,
        };
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
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let user_callback: Box<
            dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync,
        > = Box::new(callback);

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
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let (_, source_idx) = crate::action::space::decode_source_select(action_id);
                let mut ctx =
                    EffectContext::new(game, source_card, source_permanent, selecting_player);
                user_callback(&mut ctx, source_idx as usize);
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
    pub fn select_effect_choice<C>(
        &mut self,
        prompt: &str,
        labels: Vec<String>,
        callback: C,
    ) where
        C: FnOnce(&mut EffectContext<'_>, usize) + Send + Sync + 'static,
    {
        use crate::action::space::{HAND_EFFECT_START, HAND_MAIN_LIMIT};

        if labels.is_empty() {
            return;
        }
        let cap = labels.len().min(HAND_MAIN_LIMIT);

        let mut valid_action_ids: Vec<u16> = Vec::with_capacity(cap);
        let mut choices: Vec<EffectChoiceEntry> = Vec::with_capacity(cap);
        for (i, label) in labels.iter().take(cap).enumerate() {
            let action_id = HAND_EFFECT_START + i as u16;
            valid_action_ids.push(action_id);
            choices.push(EffectChoiceEntry {
                label: label.clone(),
                action_id,
            });
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let user_callback: Box<
            dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync,
        > = Box::new(callback);

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
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let choice_index = action_id.saturating_sub(HAND_EFFECT_START) as usize;
                let mut ctx =
                    EffectContext::new(game, source_card, source_permanent, selecting_player);
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
    ) where
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
            return;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let user_callback: Box<
            dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync,
        > = Box::new(callback);

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
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let index = action_id.saturating_sub(SEL_REVEAL_START) as usize;
                let mut ctx =
                    EffectContext::new(game, source_card, source_permanent, selecting_player);
                user_callback(&mut ctx, index);
            }),
            on_decline: None,
        });
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
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let user_callback: Box<
            dyn FnOnce(&mut EffectContext<'_>, usize) + Send + Sync,
        > = Box::new(callback);

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
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let index = action_id.saturating_sub(base) as usize;
                let mut ctx =
                    EffectContext::new(game, source_card, source_permanent, selecting_player);
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
        C: FnOnce(&mut EffectContext<'_>, crate::card_source::CardHandle) + Send + Sync + 'static,
    {
        use crate::action::space::{HAND_MAIN_LIMIT, PLAY_HAND_END, PLAY_HAND_START, TRASH_EFFECT_START, TRASH_MAIN_LIMIT};
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
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let user_callback: Box<
            dyn FnOnce(&mut EffectContext<'_>, crate::card_source::CardHandle) + Send + Sync,
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
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                debug_assert!(
                    action_id < PLAY_HAND_END || action_id >= TRASH_EFFECT_START,
                    "select_union_zone: action_id {} falls in gap between PLAY_HAND ({}..{}) and TRASH_EFFECT ({}..); valid_action_ids was populated incorrectly",
                    action_id, PLAY_HAND_START, PLAY_HAND_END, TRASH_EFFECT_START
                );
                // Disambiguate by range.
                let handle = if action_id >= TRASH_EFFECT_START {
                    let idx = (action_id - TRASH_EFFECT_START) as usize;
                    game.player(of_player).trash[idx].handle()
                } else {
                    // PLAY_HAND_START range (0-29)
                    let idx = action_id.saturating_sub(PLAY_HAND_START) as usize;
                    game.player(of_player).hand[idx].handle()
                };
                let mut ctx =
                    EffectContext::new(game, source_card, source_permanent, selecting_player);
                user_callback(&mut ctx, handle);
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
        C: FnOnce(&mut EffectContext<'_>, Vec<crate::card_source::CardHandle>) + Send + Sync + 'static,
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
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let previous_phase = self.game.current_phase;
        let prompt_owned = prompt.to_string();

        let final_callback: Box<
            dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync,
        > = Box::new(move |game: &mut Game, ordered: Vec<crate::card_source::CardHandle>| {
            let mut ctx = EffectContext::new(game, source_card, source_permanent, selecting_player);
            callback(&mut ctx, ordered);
        });

        install_permutation_step(
            self.game,
            items,
            Vec::new(),
            prompt_owned,
            source_card,
            source_permanent,
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
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, &CardSource) -> bool + Send + Sync + 'static,
        C: FnOnce(&mut EffectContext<'_>, Vec<crate::card_source::CardHandle>) + Send + Sync + 'static,
    {
        debug_assert!(max <= 10, "select_count_capped_multi: max must be <= 10");

        use crate::action::space::{HAND_MAIN_LIMIT, PLAY_HAND_START, TRASH_EFFECT_START, TRASH_MAIN_LIMIT};

        // Collect valid candidates at install time using the filter.
        let (zone_len, range_start) = match zone {
            CountCappedZone::Hand => {
                let len = self.game.player(of_player).hand.len().min(HAND_MAIN_LIMIT);
                (len, PLAY_HAND_START)
            }
            CountCappedZone::Trash => {
                let len = self.game.player(of_player).trash.len().min(TRASH_MAIN_LIMIT);
                (len, TRASH_EFFECT_START)
            }
        };

        // Collect all indices whose card passes the filter. We clone each card
        // to avoid a simultaneous immutable + mutable borrow of self.game.
        let mut candidate_indices: Vec<usize> = Vec::with_capacity(zone_len);
        for i in 0..zone_len {
            let card_clone = match zone {
                CountCappedZone::Hand => self.game.player(of_player).hand[i].clone(),
                CountCappedZone::Trash => self.game.player(of_player).trash[i].clone(),
            };
            if filter(self.game, &card_clone) {
                candidate_indices.push(i);
            }
        }

        // Empty filter → invoke final callback immediately; no selection installed.
        if candidate_indices.is_empty() {
            callback(self, Vec::new());
            return;
        }

        let selecting_player = self.override_selecting_player.unwrap_or(self.player);
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;
        let previous_phase = self.game.current_phase;
        let prompt_owned = prompt.to_string();

        let final_callback: Box<
            dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync,
        > = Box::new(move |game: &mut Game, picks: Vec<crate::card_source::CardHandle>| {
            let mut ctx = EffectContext::new(game, source_card, source_permanent, selecting_player);
            callback(&mut ctx, picks);
        });

        install_count_capped_step(
            self.game,
            of_player,
            zone,
            range_start,
            max,
            is_optional_zero,
            candidate_indices,
            Vec::new(), // accum starts empty
            prompt_owned,
            source_card,
            source_permanent,
            selecting_player,
            previous_phase,
            final_callback,
        );
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
    pub fn play_from_security(&mut self) {
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
        let source_card = self.source_card;
        let source_permanent = self.source_permanent;

        // Wrap the user callback: at resolution time we need to build a
        // fresh EffectContext, decode the action ID to a PermanentHandle,
        // and hand both to the user.
        let user_callback: Box<
            dyn FnOnce(&mut EffectContext<'_>, PermanentHandle) + Send + Sync,
        > = Box::new(callback);

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
                let mut ctx =
                    EffectContext::new(game, source_card, source_permanent, selecting_player);
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
        C: FnOnce(&mut EffectContext<'_>, crate::permanent::PermanentHandle) + Send + Sync + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx.select_own_permanent(prompt, is_optional, filter, callback);
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
        C: FnOnce(&mut EffectContext<'_>, crate::permanent::PermanentHandle) + Send + Sync + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx.select_opponent_permanent(prompt, is_optional, filter, callback);
        self.ctx.override_selecting_player = prev;
    }

    /// Install an effect-choice selection where `self.selecting_player` picks
    /// the branch. Forwards to `EffectContext::select_effect_choice`.
    pub fn select_effect_choice<C>(
        &mut self,
        prompt: &str,
        labels: Vec<String>,
        callback: C,
    ) where
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
        self.ctx.select_hand(of_player, prompt, is_optional, filter, callback);
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
        self.ctx.select_trash(of_player, prompt, is_optional, filter, callback);
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
        C: FnOnce(&mut EffectContext<'_>, crate::card_source::CardHandle) + Send + Sync + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx.select_union_zone(of_player, zones, prompt, is_optional, filter, callback);
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
        filter: F,
        callback: C,
    ) where
        F: Fn(&crate::game::Game, &crate::card_source::CardSource) -> bool + Send + Sync + 'static,
        C: FnOnce(&mut EffectContext<'_>, Vec<crate::card_source::CardHandle>) + Send + Sync + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx.select_count_capped_multi(of_player, zone, max, prompt, is_optional_zero, filter, callback);
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
        C: FnOnce(&mut EffectContext<'_>, Vec<crate::card_source::CardHandle>) + Send + Sync + 'static,
    {
        let prev = self.ctx.override_selecting_player.take();
        self.ctx.override_selecting_player = Some(self.selecting_player);
        self.ctx.select_ordered_permutation(items, prompt, callback);
        self.ctx.override_selecting_player = prev;
    }
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
    selecting_player: crate::enums::PlayerId,
    previous_phase: GamePhase,
    final_callback: Box<
        dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync,
    >,
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
                    selecting_player,
                    previous_phase,
                    final_callback,
                );
            }
        }),
        on_decline: None,
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
/// indices are removed after each step). `range_start` is `PLAY_HAND_START` for
/// hand or `TRASH_EFFECT_START` for trash.
///
/// This is a free function (not an `EffectContext` method) so it can be called
/// recursively from inside a `Box<dyn FnOnce>` without any `Arc`/self-reference.
#[allow(clippy::too_many_arguments)]
fn install_count_capped_step(
    game: &mut Game,
    of_player: PlayerId,
    zone: CountCappedZone,
    range_start: u16,
    max: u8,
    is_optional_zero: bool,
    candidate_indices: Vec<usize>, // zone indices still eligible
    accum: Vec<crate::card_source::CardHandle>, // handles picked so far (in order)
    prompt: String,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<crate::permanent::PermanentHandle>,
    selecting_player: PlayerId,
    previous_phase: GamePhase,
    final_callback: Box<
        dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync,
    >,
) {
    use std::sync::{Arc, Mutex};
    use crate::selection::{PendingSelection, SelectionKind};

    let picked = accum.len() as u8;

    // `is_optional` drives PASS gating in `resolve_generic_selection` and the
    // mask builder. True when the player is allowed to commit early at this step.
    let is_optional = is_optional_zero || picked >= 1;

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
    let shared_cb: Arc<Mutex<Option<Box<dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync>>>> =
        Arc::new(Mutex::new(Some(final_callback)));
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

            // Remove the picked index from candidates for the next step.
            let new_candidates: Vec<usize> = candidate_indices
                .into_iter()
                .filter(|&i| i != pick_zone_idx)
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
            let next_cb: Box<dyn FnOnce(&mut Game, Vec<crate::card_source::CardHandle>) + Send + Sync> =
                Box::new(move |game, picks| {
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
                max,
                is_optional_zero,
                new_candidates,
                new_accum,
                prompt,
                source_card,
                source_permanent,
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
