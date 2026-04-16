//! Modifier registry — typed effect modifiers with expiry.
//!
//! Modifiers are temporary or permanent effects applied to permanents.
//! Examples: +1000 DP this turn, can't be deleted by effects, granted blocker, etc.

use std::collections::HashMap;

use crate::enums::{Expiry, Keyword, ModifierType};
use crate::permanent::PermanentHandle;

/// A single modifier entry.
#[derive(Debug, Clone)]
pub struct ModifierEntry {
    pub modifier: ModifierType,
    pub value: i32,
    pub expiry: Expiry,
    /// Which player owned the source effect (for cleanup at end of their turn).
    pub source_player: u8,
}

/// Tracks all active modifiers in the game.
#[derive(Debug, Default)]
pub struct ModifierRegistry {
    /// Modifiers attached to specific permanents.
    permanent_modifiers: HashMap<PermanentHandle, Vec<ModifierEntry>>,
    /// Granted keywords on permanents (separate so duplicates are deduplicated).
    permanent_keywords: HashMap<PermanentHandle, Vec<(Keyword, Expiry, u8)>>,
}

impl ModifierRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a modifier to a permanent.
    pub fn add(&mut self, target: PermanentHandle, entry: ModifierEntry) {
        self.permanent_modifiers
            .entry(target)
            .or_default()
            .push(entry);
    }

    /// Grant a keyword to a permanent with the given expiry.
    pub fn grant_keyword(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        expiry: Expiry,
        source_player: u8,
    ) {
        self.permanent_keywords
            .entry(target)
            .or_default()
            .push((keyword, expiry, source_player));
    }

    /// Get all modifiers of a given type on a permanent.
    pub fn get(&self, target: PermanentHandle, modifier: ModifierType) -> Vec<&ModifierEntry> {
        self.permanent_modifiers
            .get(&target)
            .map(|entries| entries.iter().filter(|e| e.modifier == modifier).collect())
            .unwrap_or_default()
    }

    /// Sum of all `value` fields for a given modifier type on a permanent.
    pub fn sum(&self, target: PermanentHandle, modifier: ModifierType) -> i32 {
        self.get(target, modifier).iter().map(|e| e.value).sum()
    }

    /// Whether the permanent has any modifier of the given type.
    pub fn has(&self, target: PermanentHandle, modifier: ModifierType) -> bool {
        !self.get(target, modifier).is_empty()
    }

    /// Whether the permanent has the given granted keyword.
    pub fn has_keyword(&self, target: PermanentHandle, keyword: Keyword) -> bool {
        self.permanent_keywords
            .get(&target)
            .map(|kws| kws.iter().any(|(k, _, _)| *k == keyword))
            .unwrap_or(false)
    }

    /// Remove all modifiers attached to a permanent (e.g. when it leaves the field).
    pub fn clear_permanent(&mut self, target: PermanentHandle) {
        self.permanent_modifiers.remove(&target);
        self.permanent_keywords.remove(&target);
    }

    /// Expire modifiers at the end of a player's turn.
    pub fn expire_end_of_turn(&mut self, ending_player: u8) {
        for entries in self.permanent_modifiers.values_mut() {
            entries.retain(|e| !matches!(e.expiry, Expiry::EndOfTurn));
        }
        for kws in self.permanent_keywords.values_mut() {
            kws.retain(|(_, expiry, _)| !matches!(expiry, Expiry::EndOfTurn));
        }
        // EndOfOpponentsTurn: remove modifiers whose source_player != ending_player
        for entries in self.permanent_modifiers.values_mut() {
            entries.retain(|e| {
                !(matches!(e.expiry, Expiry::EndOfOpponentsTurn)
                    && e.source_player != ending_player)
            });
        }
        for kws in self.permanent_keywords.values_mut() {
            kws.retain(|(_, expiry, src)| {
                !(matches!(expiry, Expiry::EndOfOpponentsTurn) && *src != ending_player)
            });
        }
    }

    /// Expire modifiers at the end of an attack.
    pub fn expire_end_of_attack(&mut self) {
        for entries in self.permanent_modifiers.values_mut() {
            entries.retain(|e| {
                !matches!(e.expiry, Expiry::EndOfAttack | Expiry::EndOfBattle)
            });
        }
        for kws in self.permanent_keywords.values_mut() {
            kws.retain(|(_, expiry, _)| {
                !matches!(expiry, Expiry::EndOfAttack | Expiry::EndOfBattle)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(player: u8, index: u8) -> PermanentHandle {
        PermanentHandle { player, index }
    }

    #[test]
    fn add_and_query() {
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(target, ModifierEntry {
            modifier: ModifierType::ChangeDp,
            value: 1000,
            expiry: Expiry::EndOfTurn,
            source_player: 0,
        });
        assert_eq!(reg.sum(target, ModifierType::ChangeDp), 1000);
        assert!(reg.has(target, ModifierType::ChangeDp));
    }

    #[test]
    fn keyword_grant() {
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.grant_keyword(target, Keyword::Blocker, Expiry::EndOfTurn, 0);
        assert!(reg.has_keyword(target, Keyword::Blocker));
        assert!(!reg.has_keyword(target, Keyword::Rush));
    }

    #[test]
    fn end_of_turn_expiry() {
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(target, ModifierEntry {
            modifier: ModifierType::ChangeDp,
            value: 1000,
            expiry: Expiry::EndOfTurn,
            source_player: 0,
        });
        reg.add(target, ModifierEntry {
            modifier: ModifierType::ChangeDp,
            value: 500,
            expiry: Expiry::Permanent,
            source_player: 0,
        });
        reg.expire_end_of_turn(0);
        assert_eq!(reg.sum(target, ModifierType::ChangeDp), 500);
    }

    #[test]
    fn clear_on_leave_field() {
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(target, ModifierEntry {
            modifier: ModifierType::ChangeDp,
            value: 1000,
            expiry: Expiry::Permanent,
            source_player: 0,
        });
        reg.grant_keyword(target, Keyword::Rush, Expiry::Permanent, 0);
        reg.clear_permanent(target);
        assert_eq!(reg.sum(target, ModifierType::ChangeDp), 0);
        assert!(!reg.has_keyword(target, Keyword::Rush));
    }
}
