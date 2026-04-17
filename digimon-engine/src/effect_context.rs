//! EffectContext — the curated API surface for card effect scripts.
//!
//! Card scripts mutate the game through this context (never directly).
//! `EffectContext` wraps `&mut Game` for `process` closures; `EffectReadContext`
//! wraps `&Game` for `condition` closures and tensor-time effect inspection.
//! Both expose the same read-only query surface.

use crate::card_data::CardData;
use crate::card_source::{CardHandle, CardSource};
use crate::enums::{Expiry, Keyword, ModifierType, PlayerId};
use crate::game::Game;
use crate::modifiers::ModifierEntry;
use crate::permanent::{Permanent, PermanentHandle};
use crate::player::Player;
use crate::rules::Rules;

/// Read-only view of game state for effect condition closures.
///
/// Wraps `&Game` so conditions can be evaluated without a mutable borrow —
/// which is required at tensor-build time (§3.1 / §3.2 parity fixes) to
/// decide whether a conditional DP modifier currently contributes.
pub struct EffectReadContext<'a> {
    pub game: &'a Game,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub player: PlayerId,
}

impl<'a> EffectReadContext<'a> {
    pub fn new(
        game: &'a Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        player: PlayerId,
    ) -> Self {
        Self {
            game,
            source_card,
            source_permanent,
            player,
        }
    }

    pub fn memory(&self) -> i16 {
        self.game.memory
    }

    pub fn turn_count(&self) -> u16 {
        self.game.turn_count
    }

    pub fn rules(&self) -> &Rules {
        &self.game.rules
    }

    pub fn card_data(&self) -> &[CardData] {
        &self.game.card_data
    }

    pub fn player(&self, id: PlayerId) -> &Player {
        self.game.player(id)
    }

    pub fn my_player(&self) -> &Player {
        self.game.player(self.player)
    }

    pub fn opponent_id(&self) -> PlayerId {
        self.game.next_clockwise(self.player)
    }

    pub fn opponent(&self) -> &Player {
        self.game.player(self.opponent_id())
    }

    pub fn opponents(&self) -> Vec<PlayerId> {
        self.game.opponents(self.player)
    }

    pub fn battle_area(&self, id: PlayerId) -> &[Permanent] {
        &self.game.player(id).battle_area
    }

    pub fn hand(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).hand
    }

    pub fn trash(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).trash
    }

    pub fn security_count(&self, id: PlayerId) -> usize {
        self.game.player(id).security.len()
    }

    pub fn source_permanent(&self) -> Option<&Permanent> {
        let h = self.source_permanent?;
        let player = self.game.player(h.player);
        player.battle_area.get(h.index as usize)
    }
}

/// The context passed to every effect's `process` closure.
/// For `condition` closures see `EffectReadContext`.
pub struct EffectContext<'a> {
    pub game: &'a mut Game,
    /// Card whose effect is being resolved.
    pub source_card: CardHandle,
    /// The permanent containing the source card, if applicable.
    pub source_permanent: Option<PermanentHandle>,
    /// Player who controls the source.
    pub player: PlayerId,
}

impl<'a> EffectContext<'a> {
    pub fn new(
        game: &'a mut Game,
        source_card: CardHandle,
        source_permanent: Option<PermanentHandle>,
        player: PlayerId,
    ) -> Self {
        Self {
            game,
            source_card,
            source_permanent,
            player,
        }
    }

    // ─── Read-only queries ────────────────────────────────────────────

    pub fn memory(&self) -> i16 {
        self.game.memory
    }

    pub fn turn_count(&self) -> u16 {
        self.game.turn_count
    }

    pub fn rules(&self) -> &Rules {
        &self.game.rules
    }

    pub fn card_data(&self) -> &[CardData] {
        &self.game.card_data
    }

    pub fn player(&self, id: PlayerId) -> &Player {
        self.game.player(id)
    }

    pub fn my_player(&self) -> &Player {
        self.game.player(self.player)
    }

    /// First clockwise opponent (sugar for `opponents()[0]`).
    pub fn opponent_id(&self) -> PlayerId {
        self.game.next_clockwise(self.player)
    }

    pub fn opponent(&self) -> &Player {
        self.game.player(self.opponent_id())
    }

    pub fn opponents(&self) -> Vec<PlayerId> {
        self.game.opponents(self.player)
    }

    pub fn battle_area(&self, id: PlayerId) -> &[Permanent] {
        &self.game.player(id).battle_area
    }

    pub fn hand(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).hand
    }

    pub fn trash(&self, id: PlayerId) -> &[crate::card_source::CardSource] {
        &self.game.player(id).trash
    }

    pub fn security_count(&self, id: PlayerId) -> usize {
        self.game.player(id).security.len()
    }

    pub fn source_permanent(&self) -> Option<&Permanent> {
        let h = self.source_permanent?;
        let player = self.game.player(h.player);
        player.battle_area.get(h.index as usize)
    }

    /// Reborrow this mut context as a read-only context — for condition
    /// closures, which take `&EffectReadContext`.
    pub fn as_read(&self) -> EffectReadContext<'_> {
        EffectReadContext {
            game: self.game,
            source_card: self.source_card,
            source_permanent: self.source_permanent,
            player: self.player,
        }
    }

    // ─── Memory mutations ─────────────────────────────────────────────

    pub fn gain_memory(&mut self, amount: i16) {
        self.game.gain_memory(amount);
    }

    pub fn lose_memory(&mut self, amount: i16) {
        let new_memory = self.game.memory - amount;
        self.game.set_memory(new_memory);
    }

    pub fn set_memory(&mut self, value: i16) {
        self.game.set_memory(value);
    }

    // ─── Card draw / trash ────────────────────────────────────────────

    pub fn draw(&mut self, player: PlayerId, count: u8) -> u8 {
        self.game.player_mut(player).draw_many(count)
    }

    /// Trash the top N cards of a player's deck.
    pub fn trash_from_top(&mut self, player: PlayerId, count: u8) -> u8 {
        let p = self.game.player_mut(player);
        let mut trashed = 0;
        for _ in 0..count {
            if let Some(card) = p.deck.pop() {
                p.trash.push(card);
                trashed += 1;
            } else {
                break;
            }
        }
        trashed
    }

    // ─── Field mutations ──────────────────────────────────────────────

    pub fn delete_permanent(&mut self, target: PermanentHandle) {
        let player = self.game.player_mut(target.player);
        if (target.index as usize) < player.battle_area.len() {
            player.delete_permanent(target.index as usize);
            self.game.modifiers.clear_permanent(target);
        }
    }

    pub fn suspend(&mut self, target: PermanentHandle) {
        let player = self.game.player_mut(target.player);
        if let Some(perm) = player.battle_area.get_mut(target.index as usize) {
            perm.is_suspended = true;
        }
    }

    pub fn unsuspend(&mut self, target: PermanentHandle) {
        let player = self.game.player_mut(target.player);
        if let Some(perm) = player.battle_area.get_mut(target.index as usize) {
            perm.is_suspended = false;
        }
    }

    // ─── Modifier registration ────────────────────────────────────────

    pub fn add_dp_modifier(&mut self, target: PermanentHandle, value: i32, expiry: Expiry) {
        self.game.modifiers.add(
            target,
            ModifierEntry {
                modifier: ModifierType::ChangeDp,
                value,
                expiry,
                source_player: self.player,
            },
        );
    }

    pub fn add_modifier(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
    ) {
        self.game.modifiers.add(
            target,
            ModifierEntry {
                modifier,
                value,
                expiry,
                source_player: self.player,
            },
        );
    }

    pub fn grant_keyword(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        expiry: Expiry,
    ) {
        self.game
            .modifiers
            .grant_keyword(target, keyword, expiry, self.player);
    }

    // ─── Selection prompts ────────────────────────────────────────────
    //
    // After calling one of these helpers the effect's `process` closure must
    // return — the callback fires later (when the game resolves the
    // selection via `Game::resolve_selection`), and any further mutations
    // from within the original `process` would race with queue state.
    //
    // The filter runs **at install time** to produce `valid_action_ids`.
    // It does not re-run at resolution; the selection is validated against
    // that frozen set. Single-threaded engine, queue pauses while a
    // selection is parked, so state cannot drift.

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
            crate::selection::SelectionKind::OppField,
            crate::enums::GamePhase::SelectTarget,
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
            crate::selection::SelectionKind::OwnField,
            crate::enums::GamePhase::SelectTarget,
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
        self.game.current_phase = crate::enums::GamePhase::SelectHand;
        self.game.pending_selection = Some(crate::selection::PendingSelection {
            kind: crate::selection::SelectionKind::Hand,
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
        self.game.current_phase = crate::enums::GamePhase::SelectTrash;
        self.game.pending_selection = Some(crate::selection::PendingSelection {
            kind: crate::selection::SelectionKind::Trash,
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
        use crate::action::space::{
            SOURCES_PER_FIELD, SOURCE_SELECT_START,
        };

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
        self.game.current_phase = crate::enums::GamePhase::SelectMaterial;
        self.game.pending_selection = Some(crate::selection::PendingSelection {
            kind: crate::selection::SelectionKind::Material,
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
        use crate::selection::EffectChoiceEntry;

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
        self.game.current_phase = crate::enums::GamePhase::EffectChoice;
        self.game.pending_selection = Some(crate::selection::PendingSelection {
            kind: crate::selection::SelectionKind::EffectChoice,
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
        self.game.current_phase = crate::enums::GamePhase::SelectReveal;
        self.game.pending_selection = Some(crate::selection::PendingSelection {
            kind: crate::selection::SelectionKind::Reveal,
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
        self.game.current_phase = crate::enums::GamePhase::SelectSecurity;
        self.game.pending_selection = Some(crate::selection::PendingSelection {
            kind: crate::selection::SelectionKind::Security,
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
        kind: crate::selection::SelectionKind,
        phase: crate::enums::GamePhase,
        target_player: PlayerId,
        prompt: &str,
        is_optional: bool,
        filter: F,
        callback: C,
    ) where
        F: Fn(&Game, PermanentHandle) -> bool,
        C: FnOnce(&mut EffectContext<'_>, PermanentHandle) + Send + Sync + 'static,
    {
        use crate::action::space::{encode_attack, ATTACK_START};

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
        self.game.pending_selection = Some(crate::selection::PendingSelection {
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
                let target_index = (offset % crate::action::space::TARGETS_PER_ATTACKER) as u8;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card_data::CardData;
    use crate::rules::Rules;
    use std::collections::HashMap;

    fn min_db() -> HashMap<String, CardData> {
        let json = r#"{
            "BT1-001": {
                "card_id": "BT1-001", "card_name_eng": "Koromon",
                "card_effect_class_name": "BT1_001", "play_cost": 0, "dp": -1,
                "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
                "type_eng": [], "form_eng": [], "attribute_eng": [],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            }
        }"#;
        CardData::load_from_str(json).unwrap()
    }

    #[test]
    fn memory_mutations() {
        let db = min_db();
        let deck = vec!["BT1-001".to_string(); 10];
        let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(1)).unwrap();
        let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, 0);
        ctx.set_memory(0);
        ctx.gain_memory(3);
        assert_eq!(ctx.memory(), 3);
        ctx.lose_memory(2);
        assert_eq!(ctx.memory(), 1);
    }
}
