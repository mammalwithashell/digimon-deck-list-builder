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

    /// Play a card from `player`'s hand at `hand_index`, deducting memory
    /// according to `cost_delta`. OnPlay effects fire.
    ///
    /// Returns the `PermanentHandle` of the new field permanent, or `None`
    /// if the hand index is invalid, the battle area is full, or memory is
    /// insufficient.
    pub fn play_from_hand_with_cost(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
    ) -> Option<PermanentHandle> {
        let field_index = self.game.play_from_hand_with_cost(player, hand_index, cost_delta)?;
        Some(PermanentHandle {
            player,
            index: field_index as u8,
        })
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
}
