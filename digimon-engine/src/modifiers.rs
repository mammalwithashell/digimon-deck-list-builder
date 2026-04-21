//! Modifier registry — typed effect modifiers with expiry.
//!
//! Modifiers are temporary or permanent effects applied to permanents.
//! Examples: +1000 DP this turn, can't be deleted by effects, granted blocker, etc.

use std::collections::HashMap;

use crate::enums::{Expiry, Keyword, ModifierType, PlayerId};
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

/// A player-scoped modifier entry (Phase 6 flood gates).
///
/// Unlike `ModifierEntry` (which is keyed to a permanent), this modifier
/// applies to a whole player — e.g. "opponent cannot gain memory by effect".
///
/// NOTE: no closure-valued condition in v1. Card scripts gate WHEN they install
/// the modifier via the Effect's `.condition` closure. Phase 7 may add a
/// `condition` field.
#[derive(Debug, Clone)]
pub struct PlayerModifierEntry {
    pub modifier: ModifierType,
    /// For future parametric variants; ignored for boolean flags.
    pub value: i32,
    /// Reuses the existing `Expiry` enum.
    pub expiry: Expiry,
    /// Required when `expiry == Expiry::UntilLeaveField`.
    pub source_permanent: Option<PermanentHandle>,
    /// Who installed it (used for `EndOfOpponentsTurn` expiry).
    pub source_player: PlayerId,
}

/// Tracks all active modifiers in the game.
#[derive(Debug, Default)]
pub struct ModifierRegistry {
    /// Modifiers attached to specific permanents.
    permanent_modifiers: HashMap<PermanentHandle, Vec<ModifierEntry>>,
    /// Granted keywords on permanents (separate so duplicates are deduplicated).
    permanent_keywords: HashMap<PermanentHandle, Vec<(Keyword, Expiry, u8)>>,
    /// Player-scoped modifiers (Phase 6 flood gates).
    player_modifiers: HashMap<PlayerId, Vec<PlayerModifierEntry>>,
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

    /// Whether ANY permanent in the registry has a modifier of the given
    /// type. Mirrors Python's "global modifier query" pattern — e.g.
    /// `_is_play_blocked_by_modifier` iterates every active modifier of
    /// type `CannotPlayFromHand` without keying by target.
    pub fn any_with_type(&self, modifier: ModifierType) -> bool {
        self.permanent_modifiers
            .values()
            .any(|entries| entries.iter().any(|e| e.modifier == modifier))
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

    // ── Player-scoped modifier methods (Phase 6 flood gates) ─────────────

    /// Install a player-scoped modifier.
    pub fn add_player_modifier(&mut self, target_player: PlayerId, entry: PlayerModifierEntry) {
        self.player_modifiers
            .entry(target_player)
            .or_default()
            .push(entry);
    }

    /// Whether `target_player` currently has a modifier of the given type.
    pub fn player_has(&self, target_player: PlayerId, modifier: ModifierType) -> bool {
        self.player_modifiers
            .get(&target_player)
            .map(|entries| entries.iter().any(|e| e.modifier == modifier))
            .unwrap_or(false)
    }

    /// Sum of all `value` fields for a given modifier type on a player.
    /// Returns 0 if no matching entries exist.
    pub fn player_modifier_value(&self, target_player: PlayerId, modifier: ModifierType) -> i32 {
        self.player_modifiers
            .get(&target_player)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|e| e.modifier == modifier)
                    .map(|e| e.value)
                    .sum()
            })
            .unwrap_or(0)
    }

    /// Iterate over all player-scoped modifier entries for `target_player`.
    pub fn player_modifiers_iter(&self, target_player: PlayerId) -> impl Iterator<Item = &PlayerModifierEntry> {
        self.player_modifiers
            .get(&target_player)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .iter()
    }

    /// Expire player-scoped modifiers at the end of a turn.
    ///
    /// - `Expiry::EndOfTurn` — always removed.
    /// - `Expiry::EndOfOpponentsTurn` — removed when `ending_player != entry.source_player`
    ///   (i.e. the turn ending is the opponent of whoever installed the modifier).
    pub fn expire_player_end_of_turn(&mut self, ending_player: PlayerId) {
        for entries in self.player_modifiers.values_mut() {
            entries.retain(|e| match e.expiry {
                Expiry::EndOfTurn => false,
                Expiry::EndOfOpponentsTurn => e.source_player == ending_player,
                _ => true,
            });
        }
    }

    /// Expire player-scoped modifiers whose `source_permanent` matches `handle`.
    /// Called whenever a permanent leaves the battle area.
    pub fn expire_player_on_permanent_leave(&mut self, handle: PermanentHandle) {
        for entries in self.player_modifiers.values_mut() {
            entries.retain(|e| {
                !(matches!(e.expiry, Expiry::UntilLeaveField)
                    && e.source_permanent == Some(handle))
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
