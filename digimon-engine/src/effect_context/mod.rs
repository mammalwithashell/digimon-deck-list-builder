//! EffectContext — the curated API surface for card effect scripts.
//!
//! Card scripts mutate the game through this context (never directly).
//! `EffectContext` wraps `&mut Game` for `process` closures; `EffectReadContext`
//! wraps `&Game` for `condition` closures and tensor-time effect inspection.
//! Both expose the same read-only query surface.
//!
//! **File layout.** Selection-prompt helpers (`select_*`, `play_from_security`,
//! `mark_security_face_up`, plus the private `install_field_selection`
//! shared implementation) live in `selections.rs` — they are numerous and
//! will grow substantially as the gap-closing roadmap adds multi-select,
//! ordered-permutation, cross-player, and budgeted-multi-select primitives.
//! Core mutations (memory, draw, trash, suspend, modifier grants) stay here.

mod selections;

use crate::card_data::CardData;
use crate::card_source::CardHandle;
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

    /// Move the top card of `player`'s security stack to their trash.
    /// No-op if the stack is empty. Returns true if a card was moved.
    pub fn trash_top_security(&mut self, player: PlayerId) -> bool {
        let p = self.game.player_mut(player);
        if let Some(card) = p.security.pop() {
            p.trash.push(card);
            true
        } else {
            false
        }
    }

    // ─── Field mutations ──────────────────────────────────────────────

    pub fn delete_permanent(&mut self, target: PermanentHandle) {
        let player = self.game.player_mut(target.player);
        if (target.index as usize) < player.battle_area.len() {
            player.delete_permanent(target.index as usize);
            self.game.modifiers.clear_permanent(target);
        }
    }

    /// Materialize a token on `controller`'s battle area.
    ///
    /// Looks up `token_name` in `game.token_registry`, synthesizes a
    /// `CardSource` with `is_token = true`, wraps it in a `Permanent`, and
    /// pushes onto `controller.battle_area`. No play cost, no OnPlay
    /// observer fan-out (tokens enter via effect, not via `play_from_hand`).
    ///
    /// Returns the spawned permanent's handle, or `None` if the token name
    /// is unknown or the field is full.
    pub fn play_token(
        &mut self,
        controller: crate::enums::PlayerId,
        token_name: &str,
    ) -> Option<crate::permanent::PermanentHandle> {
        use crate::card_source::CardSource;
        use crate::permanent::{Permanent, PermanentHandle};

        let def = self.game.token_registry.get(token_name)?;
        let target_card_id = def.card_id.clone();
        let data_index = self
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == target_card_id)?;
        debug_assert_eq!(
            self.game.card_data[data_index].card_kind,
            crate::enums::CardKind::Token,
            "token_registry entry must map to a CardKind::Token CardData row"
        );

        let slots = self.game.rules.field_slots as usize;
        if self.game.player(controller).battle_area.len() >= slots {
            return None;
        }

        let card_index = self.game.next_card_index();
        let mut card = CardSource::new_token(data_index, controller, card_index);
        card.card_index = card_index;
        let turn = self.game.turn_count;
        let perm = Permanent::new(card, turn);

        let player = self.game.player_mut(controller);
        player.battle_area.push(perm);
        let idx = player.battle_area.len() - 1;
        Some(PermanentHandle {
            player: controller,
            index: idx as u8,
        })
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

    #[test]
    fn play_token_unknown_name_returns_none() {
        let db = min_db();
        let deck = vec!["BT1-001".to_string(); 10];
        let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(1)).unwrap();
        let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, 0);
        assert!(ctx.play_token(0, "no-such-token-lol").is_none());
    }
}
