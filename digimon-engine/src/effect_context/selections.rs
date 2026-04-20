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
use crate::enums::{GamePhase, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};

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

        let selecting_player = self.player;
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

        let selecting_player = self.player;
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

        let selecting_player = self.player;
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

        let selecting_player = self.player;
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

        let selecting_player = self.player;
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

        let selecting_player = self.player;
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
        use crate::action::space::{HAND_MAIN_LIMIT, PLAY_HAND_START, TRASH_EFFECT_START, TRASH_MAIN_LIMIT};
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

        let selecting_player = self.player;
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

        let selecting_player = self.player;
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
}
