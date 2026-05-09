//! Modifier registry — typed effect modifiers with expiry.
//!
//! Modifiers are temporary or permanent effects applied to permanents.
//! Examples: +1000 DP this turn, can't be deleted by effects, granted blocker, etc.

use std::collections::HashMap;

use crate::enums::{EffectSourceKind, EffectTiming, Expiry, Keyword, ModifierType, PlayerId};
use crate::permanent::PermanentHandle;

/// A single modifier entry.
///
/// Not `Clone` — `replacement_condition` is a `Box<dyn Fn + Send + Sync>`,
/// which cannot be cloned in general. Consumers should share via reference
/// or rebuild via `ModifierEntry::simple(...)` / literal construction.
pub struct ModifierEntry {
    pub modifier: ModifierType,
    pub value: i32,
    pub expiry: Expiry,
    /// Permanent that materialized this entry, when source-scoped cleanup matters.
    pub source_permanent: Option<PermanentHandle>,
    /// Which player owned the source effect (for cleanup at end of their turn).
    pub source_player: u8,
    /// True for process-backed declaratives refreshed by `tick_declarative_effects`.
    pub materialized_declarative: bool,
    /// Cause filter for replacement-backed modifiers. None = cause-agnostic.
    pub cause_filter: Option<crate::replacement::ReplacementCause>,
    /// Optional runtime condition for passive replacements. None = always applies.
    pub replacement_condition: Option<crate::replacement::ReplacementConditionFn>,
    /// Optional source-kind/controller filter for CannotBeAffected-style gates.
    pub effect_immunity_filter: Option<EffectImmunityFilter>,
    /// For `ModifierType::DisableEffect` entries — the specific
    /// `EffectTiming` that is suppressed on this permanent. None for
    /// every other variant. Read by the observer dispatch hook before
    /// firing per-permanent observers (Track A's responsibility).
    pub disable_effect_timing: Option<EffectTiming>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectControllerFilter {
    Any,
    OpponentOnly,
    OwnOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectImmunityFilter {
    pub source_kind: Option<EffectSourceKind>,
    pub controller: EffectControllerFilter,
}

impl std::fmt::Debug for ModifierEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModifierEntry")
            .field("modifier", &self.modifier)
            .field("value", &self.value)
            .field("expiry", &self.expiry)
            .field("source_permanent", &self.source_permanent)
            .field("source_player", &self.source_player)
            .field("materialized_declarative", &self.materialized_declarative)
            .field("cause_filter", &self.cause_filter)
            .field("effect_immunity_filter", &self.effect_immunity_filter)
            .field("disable_effect_timing", &self.disable_effect_timing)
            .finish_non_exhaustive()
    }
}

impl ModifierEntry {
    /// Back-compat constructor: no cause filter, no replacement condition.
    pub fn simple(modifier: ModifierType, value: i32, expiry: Expiry, source_player: u8) -> Self {
        Self {
            modifier,
            value,
            expiry,
            source_permanent: None,
            source_player,
            materialized_declarative: false,
            cause_filter: None,
            replacement_condition: None,
            effect_immunity_filter: None,
            disable_effect_timing: None,
        }
    }

    pub fn materialized_declarative(
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
        source_permanent: Option<PermanentHandle>,
        source_player: u8,
    ) -> Self {
        Self {
            modifier,
            value,
            expiry,
            source_permanent,
            source_player,
            materialized_declarative: true,
            cause_filter: None,
            replacement_condition: None,
            effect_immunity_filter: None,
            disable_effect_timing: None,
        }
    }

    /// Constructor for a `DisableEffect` entry that suppresses dispatch of
    /// `timing` on this permanent. The dispatcher reads
    /// `entry.disable_effect_timing` and skips per-permanent observers
    /// whose `effect.timing == timing`. Other timings on the same
    /// permanent are unaffected.
    pub fn disable_effect(timing: EffectTiming, expiry: Expiry, source_player: PlayerId) -> Self {
        Self {
            modifier: ModifierType::DisableEffect,
            value: 0,
            expiry,
            source_permanent: None,
            source_player,
            materialized_declarative: false,
            cause_filter: None,
            replacement_condition: None,
            effect_immunity_filter: None,
            disable_effect_timing: Some(timing),
        }
    }

    /// Constructor for a passive replacement-backed modifier (Phase 7 Task 5).
    ///
    /// Uses `default_passive_cause_filter` to pick a sensible default
    /// `cause_filter` for the `modifier` variant. E.g. `CannotBeReturnedToDeck`
    /// defaults to `OpponentEffect` (printed text is "cannot be returned to
    /// the deck by your opponent's effects"), while `CannotBeDestroyed`
    /// defaults to `None` (cause-agnostic).
    pub fn passive_replacement(modifier: ModifierType, expiry: Expiry, source_player: u8) -> Self {
        Self {
            modifier,
            value: 0,
            expiry,
            source_permanent: None,
            source_player,
            materialized_declarative: false,
            cause_filter: default_passive_cause_filter(modifier),
            replacement_condition: None,
            effect_immunity_filter: None,
            disable_effect_timing: None,
        }
    }

    /// Builder variant: force `cause_filter = Some(OpponentEffect)` for
    /// scripts that want "cannot be X'd by opponent's effects".
    pub fn opponent_only(mut self) -> Self {
        self.cause_filter = Some(crate::replacement::ReplacementCause::OpponentEffect);
        self
    }

    /// Builder variant: attach a runtime `replacement_condition` closure.
    pub fn with_condition(mut self, cond: crate::replacement::ReplacementConditionFn) -> Self {
        self.replacement_condition = Some(cond);
        self
    }

    pub fn with_effect_immunity_filter(mut self, filter: EffectImmunityFilter) -> Self {
        self.effect_immunity_filter = Some(filter);
        self
    }

    pub fn cannot_be_affected_by_opponents_source_kind(
        source_kind: EffectSourceKind,
        expiry: Expiry,
        source_player: PlayerId,
    ) -> Self {
        Self::simple(ModifierType::CannotBeAffected, 0, expiry, source_player)
            .with_effect_immunity_filter(EffectImmunityFilter {
                source_kind: Some(source_kind),
                controller: EffectControllerFilter::OpponentOnly,
            })
    }

    pub fn cannot_be_affected_by_any_source_kind(
        source_kind: EffectSourceKind,
        expiry: Expiry,
        source_player: PlayerId,
    ) -> Self {
        Self::simple(ModifierType::CannotBeAffected, 0, expiry, source_player)
            .with_effect_immunity_filter(EffectImmunityFilter {
                source_kind: Some(source_kind),
                controller: EffectControllerFilter::Any,
            })
    }

    /// Builder variant: attach a `disable_effect_timing` parameter for
    /// `DisableEffect` entries. Calling this on a non-`DisableEffect`
    /// modifier is meaningless and will be ignored by consult sites.
    pub fn with_disable_effect_timing(mut self, timing: EffectTiming) -> Self {
        self.disable_effect_timing = Some(timing);
        self
    }
}

/// A player-scoped modifier entry (Phase 6 flood gates).
///
/// Unlike `ModifierEntry` (which is keyed to a permanent), this modifier
/// applies to a whole player — e.g. "opponent cannot gain memory by effect".
///
/// NOTE: no closure-valued condition in v1. Card scripts gate WHEN they install
/// the modifier via the Effect's `.condition` closure. Phase 7 may add a
/// `condition` field.
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
    /// True for process-backed declaratives refreshed by `tick_declarative_effects`.
    pub materialized_declarative: bool,
    /// Cause filter for replacement-backed modifiers. None = cause-agnostic.
    pub cause_filter: Option<crate::replacement::ReplacementCause>,
    /// Optional runtime condition for passive replacements. None = always applies.
    pub replacement_condition: Option<crate::replacement::ReplacementConditionFn>,
    pub effect_immunity_filter: Option<EffectImmunityFilter>,
}

impl std::fmt::Debug for PlayerModifierEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerModifierEntry")
            .field("modifier", &self.modifier)
            .field("value", &self.value)
            .field("expiry", &self.expiry)
            .field("source_permanent", &self.source_permanent)
            .field("source_player", &self.source_player)
            .field("materialized_declarative", &self.materialized_declarative)
            .field("cause_filter", &self.cause_filter)
            .field("effect_immunity_filter", &self.effect_immunity_filter)
            .finish_non_exhaustive()
    }
}

impl PlayerModifierEntry {
    /// Back-compat constructor: no cause filter, no replacement condition.
    pub fn simple(
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
        source_permanent: Option<PermanentHandle>,
        source_player: PlayerId,
    ) -> Self {
        Self {
            modifier,
            value,
            expiry,
            source_permanent,
            source_player,
            materialized_declarative: false,
            cause_filter: None,
            replacement_condition: None,
            effect_immunity_filter: None,
        }
    }

    pub fn materialized_declarative(
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
        source_permanent: Option<PermanentHandle>,
        source_player: PlayerId,
    ) -> Self {
        Self {
            modifier,
            value,
            expiry,
            source_permanent,
            source_player,
            materialized_declarative: true,
            cause_filter: None,
            replacement_condition: None,
            effect_immunity_filter: None,
        }
    }

    /// Constructor for a passive replacement-backed player-scoped modifier
    /// (Phase 7 Task 5). Uses `default_passive_cause_filter` for the
    /// `cause_filter` default.
    pub fn passive_replacement(
        modifier: ModifierType,
        expiry: Expiry,
        source_permanent: Option<PermanentHandle>,
        source_player: PlayerId,
    ) -> Self {
        Self {
            modifier,
            value: 0,
            expiry,
            source_permanent,
            source_player,
            materialized_declarative: false,
            cause_filter: default_passive_cause_filter(modifier),
            replacement_condition: None,
            effect_immunity_filter: None,
        }
    }

    /// Builder variant: force `cause_filter = Some(OpponentEffect)`.
    pub fn opponent_only(mut self) -> Self {
        self.cause_filter = Some(crate::replacement::ReplacementCause::OpponentEffect);
        self
    }

    /// Builder variant: attach a runtime `replacement_condition` closure.
    pub fn with_condition(mut self, cond: crate::replacement::ReplacementConditionFn) -> Self {
        self.replacement_condition = Some(cond);
        self
    }
}

/// Default `cause_filter` for a passive replacement-backed modifier.
///
/// - "Cannot be X'd by opponent's effects" family → `Some(OpponentEffect)`
///   (matches printed text on most protection cards).
/// - `CannotBeDestroyedByBattle` → `Some(Battle)`.
/// - `CannotBeDestroyed` / `CannotBeDestroyedByEffect` → `None`
///   (cause-agnostic: applies to all causes / all effect causes).
/// - Everything else → `None`.
pub(crate) fn default_passive_cause_filter(
    modifier: ModifierType,
) -> Option<crate::replacement::ReplacementCause> {
    use crate::replacement::ReplacementCause;
    match modifier {
        ModifierType::CannotBeReturnedToDeck
        | ModifierType::CannotBeReturnedToHand
        | ModifierType::CannotBeTrashedByEffect
        | ModifierType::CannotBeDeDigivolved => Some(ReplacementCause::OpponentEffect),
        ModifierType::CannotBeDestroyedByBattle => Some(ReplacementCause::Battle),
        // CannotBeDestroyed / CannotBeDestroyedByEffect are cause-agnostic in
        // printed text (the latter covers both own and opponent effects).
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KeywordEntry {
    keyword: Keyword,
    expiry: Expiry,
    source_player: u8,
    source_permanent: Option<PermanentHandle>,
    materialized_declarative: bool,
}

impl KeywordEntry {
    fn simple(keyword: Keyword, expiry: Expiry, source_player: u8) -> Self {
        Self {
            keyword,
            expiry,
            source_player,
            source_permanent: None,
            materialized_declarative: false,
        }
    }

    fn materialized_declarative(
        keyword: Keyword,
        expiry: Expiry,
        source_permanent: Option<PermanentHandle>,
        source_player: u8,
    ) -> Self {
        Self {
            keyword,
            expiry,
            source_player,
            source_permanent,
            materialized_declarative: true,
        }
    }
}

/// Tracks all active modifiers in the game.
#[derive(Debug, Default)]
pub struct ModifierRegistry {
    /// Modifiers attached to specific permanents.
    permanent_modifiers: HashMap<PermanentHandle, Vec<ModifierEntry>>,
    /// Granted keywords on permanents.
    permanent_keywords: HashMap<PermanentHandle, Vec<KeywordEntry>>,
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
            .push(KeywordEntry::simple(keyword, expiry, source_player));
    }

    pub fn grant_declarative_keyword(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        expiry: Expiry,
        source_permanent: Option<PermanentHandle>,
        source_player: u8,
    ) {
        self.permanent_keywords.entry(target).or_default().push(
            KeywordEntry::materialized_declarative(
                keyword,
                expiry,
                source_permanent,
                source_player,
            ),
        );
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

    /// Whether `modifier` blocks an opponent-controlled effect from affecting
    /// `target`. This is intentionally narrow: only passive replacement entries
    /// with `cause_filter = Some(OpponentEffect)` participate, so own effects
    /// and broader protection families keep their existing semantics.
    pub fn blocks_opponent_effect(
        &self,
        target: PermanentHandle,
        modifier: ModifierType,
        effect_player: PlayerId,
    ) -> bool {
        self.get(target, modifier).into_iter().any(|entry| {
            matches!(
                entry.cause_filter,
                Some(crate::replacement::ReplacementCause::OpponentEffect)
            ) && effect_player != target.player
        })
    }

    /// Iterate over ALL `ModifierEntry` values attached to `target`
    /// (regardless of `ModifierType`). Used by the Phase 7 replacement
    /// dispatcher to scan for `CannotBe*` entries across all modifier types
    /// without issuing one `get(target, ...)` call per variant.
    pub fn permanent_modifiers_iter(
        &self,
        target: PermanentHandle,
    ) -> impl Iterator<Item = &ModifierEntry> {
        self.permanent_modifiers
            .get(&target)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .iter()
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
            .map(|kws| kws.iter().any(|entry| entry.keyword == keyword))
            .unwrap_or(false)
    }

    /// Whether dispatch of `timing` should be suppressed for `target`.
    ///
    /// True iff there is at least one `ModifierType::DisableEffect` entry on
    /// `target` whose `disable_effect_timing == Some(timing)`. Read by
    /// Track A's observer dispatch hook before firing per-permanent
    /// observers — when this returns `true`, the dispatcher skips the
    /// permanent's effects at that specific timing without disabling the
    /// permanent's other effects.
    pub fn is_timing_disabled(&self, target: PermanentHandle, timing: EffectTiming) -> bool {
        self.permanent_modifiers
            .get(&target)
            .map(|entries| {
                entries.iter().any(|e| {
                    e.modifier == ModifierType::DisableEffect
                        && e.disable_effect_timing == Some(timing)
                })
            })
            .unwrap_or(false)
    }

    /// Remove process-backed declarative materializations before a fresh tick.
    pub fn clear_materialized_declaratives(&mut self) {
        self.permanent_modifiers.retain(|_, entries| {
            entries.retain(|entry| !entry.materialized_declarative);
            !entries.is_empty()
        });
        self.permanent_keywords.retain(|_, entries| {
            entries.retain(|entry| !entry.materialized_declarative);
            !entries.is_empty()
        });
        self.player_modifiers.retain(|_, entries| {
            entries.retain(|entry| !entry.materialized_declarative);
            !entries.is_empty()
        });
    }

    /// Remove all modifiers attached to a permanent (e.g. when it leaves the field).
    pub fn clear_permanent(&mut self, target: PermanentHandle) {
        self.permanent_modifiers.remove(&target);
        self.permanent_keywords.remove(&target);
    }

    /// Expire modifiers at the end of a player's turn.
    ///
    /// - `Expiry::EndOfTurn` — removed unconditionally.
    /// - `Expiry::EndOfOpponentsTurn` — removed when `source_player != ending_player`.
    /// - `Expiry::EndOfYourTurn` — removed when `source_player == ending_player`
    ///   (mirror of `EndOfOpponentsTurn`).
    pub fn expire_end_of_turn(&mut self, ending_player: u8) {
        let is_dead = |expiry: Expiry, source_player: u8| -> bool {
            match expiry {
                Expiry::EndOfTurn => true,
                Expiry::EndOfOpponentsTurn => source_player != ending_player,
                Expiry::EndOfYourTurn => source_player == ending_player,
                _ => false,
            }
        };
        for entries in self.permanent_modifiers.values_mut() {
            entries.retain(|e| !is_dead(e.expiry, e.source_player));
        }
        for kws in self.permanent_keywords.values_mut() {
            kws.retain(|entry| !is_dead(entry.expiry, entry.source_player));
        }
    }

    /// Expire modifiers at the end of an attack.
    pub fn expire_end_of_attack(&mut self) {
        for entries in self.permanent_modifiers.values_mut() {
            entries.retain(|e| !matches!(e.expiry, Expiry::EndOfAttack | Expiry::EndOfBattle));
        }
        for kws in self.permanent_keywords.values_mut() {
            kws.retain(|entry| !matches!(entry.expiry, Expiry::EndOfAttack | Expiry::EndOfBattle));
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
    pub fn player_modifiers_iter(
        &self,
        target_player: PlayerId,
    ) -> impl Iterator<Item = &PlayerModifierEntry> {
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
    /// - `Expiry::EndOfYourTurn` — removed when `ending_player == entry.source_player`
    ///   (mirror of `EndOfOpponentsTurn`).
    pub fn expire_player_end_of_turn(&mut self, ending_player: PlayerId) {
        for entries in self.player_modifiers.values_mut() {
            entries.retain(|e| match e.expiry {
                Expiry::EndOfTurn => false,
                Expiry::EndOfOpponentsTurn => e.source_player == ending_player,
                Expiry::EndOfYourTurn => e.source_player != ending_player,
                _ => true,
            });
        }
    }

    /// Expire player-scoped modifiers whose `source_permanent` matches `handle`.
    /// Called whenever a permanent leaves the battle area.
    pub fn expire_player_on_permanent_leave(&mut self, handle: PermanentHandle) {
        for entries in self.player_modifiers.values_mut() {
            entries.retain(|e| {
                !(matches!(e.expiry, Expiry::UntilLeaveField) && e.source_permanent == Some(handle))
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
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::EndOfTurn, 0),
        );
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
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::EndOfTurn, 0),
        );
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 500, Expiry::Permanent, 0),
        );
        reg.expire_end_of_turn(0);
        assert_eq!(reg.sum(target, ModifierType::ChangeDp), 500);
    }

    #[test]
    fn end_of_your_turn_expiry_mirrors_end_of_opponents_turn() {
        // P0 installs both an `EndOfYourTurn` and an `EndOfOpponentsTurn`
        // modifier. At end of P0's turn, only `EndOfYourTurn` should expire;
        // at end of P1's turn, only `EndOfOpponentsTurn` should expire.
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 100, Expiry::EndOfYourTurn, 0),
        );
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 200, Expiry::EndOfOpponentsTurn, 0),
        );

        // End of P1's turn first: EndOfOpponentsTurn expires, EndOfYourTurn stays.
        reg.expire_end_of_turn(1);
        assert_eq!(
            reg.sum(target, ModifierType::ChangeDp),
            100,
            "after opponent's turn end, only EndOfYourTurn entry should remain"
        );

        // End of P0's turn next: EndOfYourTurn expires.
        reg.expire_end_of_turn(0);
        assert_eq!(
            reg.sum(target, ModifierType::ChangeDp),
            0,
            "after own turn end, EndOfYourTurn entry should expire"
        );
    }

    #[test]
    fn until_condition_and_once_used_persist_through_turn_ends() {
        // Until the continuous controller and consumption tracker land,
        // entries with these expiries are stored but never auto-removed
        // by `expire_end_of_turn` / `expire_end_of_attack`. This test
        // pins that contract so consuming tracks know the storage shape.
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 100, Expiry::UntilCondition, 0),
        );
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 50, Expiry::OnceUsed(1), 0),
        );
        reg.expire_end_of_turn(0);
        reg.expire_end_of_turn(1);
        reg.expire_end_of_attack();
        assert_eq!(
            reg.sum(target, ModifierType::ChangeDp),
            150,
            "UntilCondition + OnceUsed entries must persist through turn-end cycles"
        );
    }

    #[test]
    fn disable_effect_timing_query_is_per_timing() {
        // A `DisableEffect{timing: WhenAttacking}` modifier on a permanent
        // suppresses `WhenAttacking` only — `OnAttack` and other timings
        // are not affected. Pins Track A's dispatch contract.
        use crate::enums::EffectTiming;
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(
            target,
            ModifierEntry::disable_effect(EffectTiming::WhenAttacking, Expiry::EndOfTurn, 0),
        );
        assert!(reg.is_timing_disabled(target, EffectTiming::WhenAttacking));
        assert!(!reg.is_timing_disabled(target, EffectTiming::OnAttack));
        assert!(!reg.is_timing_disabled(target, EffectTiming::OnPlay));
        // Untouched permanents see no suppression.
        let other = h(1, 0);
        assert!(!reg.is_timing_disabled(other, EffectTiming::WhenAttacking));
    }

    #[test]
    fn disable_effect_query_ignores_other_modifier_types() {
        // ChangeDp on a permanent must not be reported as timing-disabling.
        use crate::enums::EffectTiming;
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::EndOfTurn, 0),
        );
        assert!(!reg.is_timing_disabled(target, EffectTiming::WhenAttacking));
    }

    #[test]
    fn end_of_your_turn_player_scoped() {
        let mut reg = ModifierRegistry::new();
        // Source player 1 installs a player-scoped modifier on player 0
        // that expires at the end of P1's own turn.
        reg.add_player_modifier(
            0,
            PlayerModifierEntry::simple(
                ModifierType::CannotPlayDigimonByEffect,
                0,
                Expiry::EndOfYourTurn,
                None,
                1,
            ),
        );
        // Player 0's turn ends — the modifier should still be active
        // (P1 is the source, P1's turn hasn't ended yet).
        reg.expire_player_end_of_turn(0);
        assert!(reg.player_has(0, ModifierType::CannotPlayDigimonByEffect));
        // Player 1's turn ends — the modifier should expire.
        reg.expire_player_end_of_turn(1);
        assert!(!reg.player_has(0, ModifierType::CannotPlayDigimonByEffect));
    }

    #[test]
    fn clear_on_leave_field() {
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::Permanent, 0),
        );
        reg.grant_keyword(target, Keyword::Rush, Expiry::Permanent, 0);
        reg.clear_permanent(target);
        assert_eq!(reg.sum(target, ModifierType::ChangeDp), 0);
        assert!(!reg.has_keyword(target, Keyword::Rush));
    }
}
