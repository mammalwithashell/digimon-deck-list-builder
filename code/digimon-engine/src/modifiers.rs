//! Modifier registry — typed effect modifiers with expiry.
//!
//! Modifiers are temporary or permanent effects applied to permanents.
//! Examples: +1000 DP this turn, can't be deleted by effects, granted blocker, etc.

use std::collections::HashMap;
use std::sync::Arc;

use crate::enums::{
    CardColor, CardKind, EffectSourceKind, EffectTiming, Expiry, Keyword, ModifierType, PlayerId,
};
use crate::permanent::PermanentHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierSubject {
    Permanent(PermanentHandle),
    Player(PlayerId),
}

pub type UntilConditionFn =
    Arc<dyn Fn(&crate::game::Game, ModifierSubject) -> bool + Send + Sync + 'static>;

/// Typed payload carried by parametric modifier variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModifierPayload {
    None,
    Traits {
        add: Vec<String>,
        replace: bool,
    },
    Name {
        value: String,
        base: bool,
    },
    Colors {
        value: Vec<CardColor>,
        base: bool,
    },
    DigiXrosNames {
        aliases: Vec<String>,
    },
    Dp {
        value: i32,
        base: bool,
        origin: bool,
    },
    SecurityAttack {
        delta: i32,
        invert: bool,
    },
    EndTurnMinMemory {
        value: i32,
    },
    LinkCost {
        delta: i32,
    },
    LinkMax {
        delta: i32,
    },
    LevelForAssembly {
        value: u8,
    },
    SynthIdentity {
        kind: CardKind,
        level: u8,
        colors: Vec<CardColor>,
        traits: Vec<String>,
        dp: i32,
    },
    LevelOverride {
        value: i32,
        delta: bool,
    },
}

impl Default for ModifierPayload {
    fn default() -> Self {
        Self::None
    }
}

/// A single modifier entry.
///
/// Not `Clone` — `replacement_condition` is a `Box<dyn Fn + Send + Sync>`,
/// which cannot be cloned in general. Consumers should share via reference
/// or rebuild via `ModifierEntry::simple(...)` / literal construction.
pub struct ModifierEntry {
    pub modifier: ModifierType,
    pub value: i32,
    pub payload: ModifierPayload,
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
    /// Runtime predicate for `Expiry::UntilCondition`. The controller
    /// evaluates it against the installed subject after mutation events.
    pub until_condition: Option<UntilConditionFn>,
    /// Optional source-kind/controller filter for CannotBeAffected-style gates.
    pub effect_immunity_filter: Option<EffectImmunityFilter>,
    /// For `ModifierType::DisableEffect` entries — the specific
    /// `EffectTiming` that is suppressed on this permanent. None for
    /// every other variant. Read by the observer dispatch hook before
    /// firing per-permanent observers (Track A's responsibility).
    pub disable_effect_timing: Option<EffectTiming>,
    /// Track H Phase 4 (mid-opp-turn `*NextTurn` semantics) —
    /// number of relevant turn-end firings to skip before the entry
    /// becomes eligible for `EndOfOpponentsNextTurn`/`EndOfYourNextTurn`
    /// expiry. Default 0 (immediate). Install with 1 when the install
    /// happens DURING the target-turn-player's turn — the current
    /// turn-end then counts as the "skip", and the NEXT firing
    /// expires.
    pub pending_skips: u8,
    #[doc(hidden)]
    pub install_order: u64,
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
            .field("payload", &self.payload)
            .field("expiry", &self.expiry)
            .field("source_permanent", &self.source_permanent)
            .field("source_player", &self.source_player)
            .field("materialized_declarative", &self.materialized_declarative)
            .field("cause_filter", &self.cause_filter)
            .field("has_until_condition", &self.until_condition.is_some())
            .field("effect_immunity_filter", &self.effect_immunity_filter)
            .field("disable_effect_timing", &self.disable_effect_timing)
            .field("install_order", &self.install_order)
            .finish_non_exhaustive()
    }
}

impl ModifierEntry {
    /// Back-compat constructor: no cause filter, no replacement condition.
    pub fn simple(modifier: ModifierType, value: i32, expiry: Expiry, source_player: u8) -> Self {
        Self {
            modifier,
            value,
            payload: ModifierPayload::None,
            expiry,
            source_permanent: None,
            source_player,
            materialized_declarative: false,
            cause_filter: None,
            replacement_condition: None,
            until_condition: None,
            effect_immunity_filter: None,
            disable_effect_timing: None,
            pending_skips: 0,
            install_order: 0,
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
            payload: ModifierPayload::None,
            expiry,
            source_permanent,
            source_player,
            materialized_declarative: true,
            cause_filter: None,
            replacement_condition: None,
            until_condition: None,
            effect_immunity_filter: None,
            disable_effect_timing: None,
            pending_skips: 0,
            install_order: 0,
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
            payload: ModifierPayload::None,
            expiry,
            source_permanent: None,
            source_player,
            materialized_declarative: false,
            cause_filter: None,
            replacement_condition: None,
            until_condition: None,
            effect_immunity_filter: None,
            disable_effect_timing: Some(timing),
            pending_skips: 0,
            install_order: 0,
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
            payload: ModifierPayload::None,
            expiry,
            source_permanent: None,
            source_player,
            materialized_declarative: false,
            cause_filter: default_passive_cause_filter(modifier),
            replacement_condition: None,
            until_condition: None,
            effect_immunity_filter: None,
            disable_effect_timing: None,
            pending_skips: 0,
            install_order: 0,
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

    pub fn with_until_condition(mut self, cond: UntilConditionFn) -> Self {
        self.until_condition = Some(cond);
        self
    }

    /// Phase 4 — set `pending_skips` for `*NextTurn` expiry semantics.
    /// When installing during the same player's turn whose end would
    /// otherwise immediately expire the entry, pass 1 to skip that
    /// firing and expire on the NEXT one. Defaults to 0 (immediate
    /// expiry on the first matching turn-end).
    pub fn with_pending_skips(mut self, skips: u8) -> Self {
        self.pending_skips = skips;
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

    pub fn with_payload(mut self, payload: ModifierPayload) -> Self {
        self.payload = payload;
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
    pub payload: ModifierPayload,
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
    pub until_condition: Option<UntilConditionFn>,
    pub effect_immunity_filter: Option<EffectImmunityFilter>,
    #[doc(hidden)]
    pub install_order: u64,
}

impl std::fmt::Debug for PlayerModifierEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerModifierEntry")
            .field("modifier", &self.modifier)
            .field("value", &self.value)
            .field("payload", &self.payload)
            .field("expiry", &self.expiry)
            .field("source_permanent", &self.source_permanent)
            .field("source_player", &self.source_player)
            .field("materialized_declarative", &self.materialized_declarative)
            .field("cause_filter", &self.cause_filter)
            .field("has_until_condition", &self.until_condition.is_some())
            .field("effect_immunity_filter", &self.effect_immunity_filter)
            .field("install_order", &self.install_order)
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
            payload: ModifierPayload::None,
            expiry,
            source_permanent,
            source_player,
            materialized_declarative: false,
            cause_filter: None,
            replacement_condition: None,
            until_condition: None,
            effect_immunity_filter: None,
            install_order: 0,
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
            payload: ModifierPayload::None,
            expiry,
            source_permanent,
            source_player,
            materialized_declarative: true,
            cause_filter: None,
            replacement_condition: None,
            until_condition: None,
            effect_immunity_filter: None,
            install_order: 0,
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
            payload: ModifierPayload::None,
            expiry,
            source_permanent,
            source_player,
            materialized_declarative: false,
            cause_filter: default_passive_cause_filter(modifier),
            replacement_condition: None,
            until_condition: None,
            effect_immunity_filter: None,
            install_order: 0,
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

    pub fn with_until_condition(mut self, cond: UntilConditionFn) -> Self {
        self.until_condition = Some(cond);
        self
    }

    pub fn with_payload(mut self, payload: ModifierPayload) -> Self {
        self.payload = payload;
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

fn payload_matches_modifier(modifier: ModifierType, payload: &ModifierPayload) -> bool {
    matches!(
        (modifier, payload),
        (_, ModifierPayload::None)
            | (ModifierType::ChangeTraits, ModifierPayload::Traits { .. })
            | (
                ModifierType::ChangeBaseCardName,
                ModifierPayload::Name { .. }
            )
            | (
                ModifierType::ChangeBaseCardColor,
                ModifierPayload::Colors { .. }
            )
            | (
                ModifierType::ChangeCardNamesForDigiXros,
                ModifierPayload::DigiXrosNames { .. }
            )
            | (ModifierType::ChangeCardDP, ModifierPayload::Dp { .. })
            | (ModifierType::ChangeOriginDP, ModifierPayload::Dp { .. })
            | (
                ModifierType::ChangeSAttack,
                ModifierPayload::SecurityAttack { .. }
            )
            | (
                ModifierType::SecurityAttackChange,
                ModifierPayload::SecurityAttack { .. }
            )
            | (
                ModifierType::ChangeEndTurnMinMemory,
                ModifierPayload::EndTurnMinMemory { .. }
            )
            | (
                ModifierType::ChangeLinkCost,
                ModifierPayload::LinkCost { .. }
            )
            | (ModifierType::ChangeLinkMax, ModifierPayload::LinkMax { .. })
            | (
                ModifierType::ChangeCardLevelForAssembly,
                ModifierPayload::LevelForAssembly { .. }
            )
            | (
                ModifierType::TreatAsDigimon,
                ModifierPayload::SynthIdentity { .. }
            )
            | (
                ModifierType::ChangePermanentLevel,
                ModifierPayload::LevelOverride { .. }
            )
    )
}

fn guard_install(
    modifier: ModifierType,
    payload: &ModifierPayload,
    expiry: Expiry,
    has_until_condition: bool,
) {
    debug_assert!(
        payload_matches_modifier(modifier, payload),
        "modifier payload mismatch: {:?} cannot carry {:?}",
        modifier,
        payload
    );
    debug_assert!(
        !matches!(expiry, Expiry::UntilCondition) || has_until_condition,
        "Expiry::UntilCondition entries must carry an until_condition predicate"
    );
}

#[derive(Clone)]
struct KeywordEntry {
    keyword: Keyword,
    expiry: Expiry,
    source_player: u8,
    source_permanent: Option<PermanentHandle>,
    materialized_declarative: bool,
    /// Track H §4 — install-once continuous gate for keyword grants.
    /// When `Expiry::UntilCondition`, the controller evaluates this
    /// predicate after mutation events and evicts the entry on
    /// false; printed-semantics rule (`false → true` does NOT
    /// re-install) holds. Mirrors `ModifierEntry.until_condition`.
    until_condition: Option<UntilConditionFn>,
    /// Globally-monotone install order shared with `ModifierEntry`
    /// and `PlayerModifierEntry` so the UntilCondition controller can
    /// FIFO-evict across all three stores via a single counter.
    install_order: u64,
}

impl std::fmt::Debug for KeywordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeywordEntry")
            .field("keyword", &self.keyword)
            .field("expiry", &self.expiry)
            .field("source_player", &self.source_player)
            .field("source_permanent", &self.source_permanent)
            .field("materialized_declarative", &self.materialized_declarative)
            .field("has_until_condition", &self.until_condition.is_some())
            .field("install_order", &self.install_order)
            .finish()
    }
}

impl KeywordEntry {
    fn simple(keyword: Keyword, expiry: Expiry, source_player: u8) -> Self {
        Self {
            keyword,
            expiry,
            source_player,
            source_permanent: None,
            materialized_declarative: false,
            until_condition: None,
            install_order: 0,
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
            until_condition: None,
            install_order: 0,
        }
    }
}

/// Closure body for a granted triggered effect — runs at the carrier's
/// matching timing with `EffectContext` set up so the carrier is the
/// `source_permanent` and the grantor's card is `source_card`. Track H §3.
pub type GrantedEffectBody =
    std::sync::Arc<dyn Fn(&mut crate::effect_context::EffectContext<'_>) + Send + Sync + 'static>;

/// Phase 4i — `Debug`-friendly wrapper around the granted-effect body
/// registry. The body closures are `Arc<dyn Fn ...>` which don't
/// implement `Debug`; the wrapper hides them behind a count.
#[derive(Default)]
pub struct GrantedEffectBodyRegistry {
    pub bodies: std::collections::HashMap<u64, GrantedEffectBody>,
}

impl std::fmt::Debug for GrantedEffectBodyRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrantedEffectBodyRegistry")
            .field("count", &self.bodies.len())
            .finish()
    }
}

impl GrantedEffectBodyRegistry {
    pub fn insert(&mut self, id: u64, body: GrantedEffectBody) {
        self.bodies.insert(id, body);
    }
    pub fn get(&self, id: u64) -> Option<&GrantedEffectBody> {
        self.bodies.get(&id)
    }
    pub fn remove(&mut self, id: u64) -> Option<GrantedEffectBody> {
        self.bodies.remove(&id)
    }
}

/// A triggered effect granted onto a carrier permanent by some grantor
/// (Track H §3 — DCGO `AddSkillClass.cs` analog). The body fires when the
/// carrier's matching `timing` event drains. Cleanup happens via the
/// existing `clear_permanent` path when the carrier leaves the field, or
/// via `expire_end_of_turn` when the entry's `expiry` ticks out.
pub struct GrantedTriggeredEffect {
    /// Timing on the CARRIER that triggers this granted body. v1 dispatch
    /// hook is wired through `enqueue_from_permanent` /
    /// `enqueue_from_breeding_permanent` (Phase 4b) — covers ALL
    /// EffectTiming variants automatically via `pending_granted_fires`.
    pub timing: crate::enums::EffectTiming,
    /// Grantor card identity — surfaces as `EffectContext::source_card`
    /// inside the body, mirroring DCGO `EffectSourceCard`.
    pub source_card: crate::card_source::CardHandle,
    /// Player who installed the grant — controls expiry behavior and
    /// shows up as `EffectContext::player`.
    pub source_player: PlayerId,
    pub expiry: crate::enums::Expiry,
    /// Phase 4i — body id into `Game::granted_effect_bodies`. v1 keeps
    /// the body `Arc` here as well (for direct inline-fire compat)
    /// AND stores it in the registry under this id (for queue-based
    /// dispatch through `QueuedEffect::granted_effect_id`). When the
    /// entry is evicted (clear_permanent / expire_end_of_turn) the
    /// registry entry must also be removed to avoid leaks.
    pub body_id: u64,
    pub body: GrantedEffectBody,
}

impl std::fmt::Debug for GrantedTriggeredEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrantedTriggeredEffect")
            .field("timing", &self.timing)
            .field("source_card", &self.source_card)
            .field("source_player", &self.source_player)
            .field("expiry", &self.expiry)
            .field("body_id", &self.body_id)
            .finish_non_exhaustive()
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
    /// Track H §3 — granted triggered effects keyed by carrier
    /// permanent. DCGO `AddSkillClass.cs` analog: the grantor publishes
    /// a closure body and a target carrier; the body fires when the
    /// carrier's matching `timing` drains. v1 dispatch covers
    /// `OnDeletion`; additional timings extend the hook in
    /// `Game::fire_granted_triggered_effects` without schema changes.
    granted_triggered: HashMap<PermanentHandle, Vec<GrantedTriggeredEffect>>,
    next_install_order: u64,
}

impl ModifierRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a modifier to a permanent.
    pub fn add(&mut self, target: PermanentHandle, mut entry: ModifierEntry) {
        guard_install(
            entry.modifier,
            &entry.payload,
            entry.expiry,
            entry.until_condition.is_some(),
        );
        self.next_install_order = self.next_install_order.saturating_add(1);
        entry.install_order = self.next_install_order;
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
        self.next_install_order = self.next_install_order.saturating_add(1);
        let mut entry = KeywordEntry::simple(keyword, expiry, source_player);
        entry.install_order = self.next_install_order;
        self.permanent_keywords
            .entry(target)
            .or_default()
            .push(entry);
    }

    /// Track H §4 — grant a keyword scoped to `Expiry::UntilCondition`
    /// with a runtime predicate. The UntilCondition controller (PR
    /// #458) evicts the entry on the first `predicate → false` after
    /// install; the printed-semantics rule holds (no re-install when
    /// the predicate flips back to true). Mirrors
    /// `add_modifier_with_until_condition` for keyword grants.
    pub fn grant_keyword_with_until_condition(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        predicate: UntilConditionFn,
        source_player: u8,
    ) {
        self.next_install_order = self.next_install_order.saturating_add(1);
        let mut entry = KeywordEntry::simple(keyword, Expiry::UntilCondition, source_player);
        entry.install_order = self.next_install_order;
        entry.until_condition = Some(predicate);
        self.permanent_keywords
            .entry(target)
            .or_default()
            .push(entry);
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

    /// Net `<Security A. +N>` / `<Security A. -N>` contributed by *granted*
    /// keyword entries on `target` (e.g. an aura's
    /// `grant_keyword: SecurityAttackPlus`). Printed face/inherited keywords
    /// on the permanent's `card_sources` are summed separately by
    /// `Game::security_attack_keyword_bonus`; this folds in the registry side
    /// so aura-granted Security Attack keywords are actually consumed at the
    /// security loop.
    pub fn granted_security_attack_keyword_bonus(&self, target: PermanentHandle) -> i32 {
        self.permanent_keywords
            .get(&target)
            .map(|kws| {
                kws.iter()
                    .map(|entry| match entry.keyword {
                        Keyword::SecurityAttackPlus(n) => n as i32,
                        Keyword::SecurityAttackMinus(n) => -(n as i32),
                        _ => 0,
                    })
                    .sum()
            })
            .unwrap_or(0)
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
        self.granted_triggered.remove(&target);
    }

    /// Track H §3 — install a granted triggered effect on `carrier`.
    /// The body fires when the carrier's matching `timing` drains.
    pub fn add_granted_triggered(
        &mut self,
        carrier: PermanentHandle,
        entry: GrantedTriggeredEffect,
    ) {
        self.granted_triggered
            .entry(carrier)
            .or_default()
            .push(entry);
    }

    /// Track H §3 — fetch granted triggered effects on `carrier` whose
    /// timing matches `timing`. Returns clones of the body closures
    /// alongside attribution so the dispatcher can build an
    /// `EffectContext` per fire without holding a registry borrow.
    pub fn granted_triggered_for_timing(
        &self,
        carrier: PermanentHandle,
        timing: crate::enums::EffectTiming,
    ) -> Vec<(crate::card_source::CardHandle, PlayerId, GrantedEffectBody)> {
        self.granted_triggered
            .get(&carrier)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|e| e.timing == timing)
                    .map(|e| (e.source_card, e.source_player, e.body.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Phase 4i variant — returns body_id alongside attribution. Used by
    /// the queue-based dispatcher to construct a `QueuedEffect` with
    /// `granted_effect_id` set so the drainer fetches the body from
    /// `Game::granted_effect_bodies` at fire time.
    pub fn granted_triggered_for_timing_with_ids(
        &self,
        carrier: PermanentHandle,
        timing: crate::enums::EffectTiming,
    ) -> Vec<(u64, crate::card_source::CardHandle, PlayerId)> {
        self.granted_triggered
            .get(&carrier)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|e| e.timing == timing)
                    .map(|e| (e.body_id, e.source_card, e.source_player))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Phase 4i — collect body ids that should be reaped from the
    /// game-level body registry when a permanent's granted entries are
    /// cleared (via `clear_permanent`). Returns the ids; the caller
    /// removes the entries from the registry.
    pub fn drain_granted_triggered_ids_on_carrier(&self, carrier: PermanentHandle) -> Vec<u64> {
        self.granted_triggered
            .get(&carrier)
            .map(|entries| entries.iter().map(|e| e.body_id).collect())
            .unwrap_or_default()
    }

    /// Expire modifiers at the end of a player's turn.
    ///
    /// - `Expiry::EndOfTurn` — removed unconditionally.
    /// - `Expiry::EndOfOpponentsTurn` — removed when `source_player != ending_player`.
    /// - `Expiry::EndOfYourTurn` — removed when `source_player == ending_player`
    ///   (mirror of `EndOfOpponentsTurn`).
    /// - `Expiry::EndOfOpponentsNextTurn` — Track H §3 (EX1-068 family).
    ///   v1 aliased to `EndOfOpponentsTurn` semantics: removed on the next
    ///   opp-turn-end after install. This matches printed-text behavior
    ///   for installs on source's own turn (the common case — `[Main]`
    ///   effects, `WhenDigivolving`, etc.). For installs DURING opp's
    ///   turn, the printed text "until end of their NEXT turn" should
    ///   skip the current opp-turn-end; v1 doesn't track that nuance —
    ///   would require a per-entry `pending_skips` counter set at install
    ///   time. Tracked as follow-up.
    /// - `Expiry::EndOfYourNextTurn` — symmetric mirror, aliased to
    ///   `EndOfYourTurn` semantics with the same v1 caveat.
    pub fn expire_end_of_turn(&mut self, ending_player: u8) {
        // Phase 4: split is_dead from skip-tracking. For `*NextTurn`
        // variants, the first relevant turn-end decrements pending_skips
        // (or expires immediately if 0); subsequent firings expire.
        // This matches printed "until end of their NEXT turn" — mid-
        // opp-turn installs with pending_skips=1 skip the current
        // opp-turn-end, expire on the next.
        let relevant = |expiry: Expiry, source_player: u8| -> bool {
            match expiry {
                Expiry::EndOfTurn => true,
                Expiry::EndOfOpponentsTurn | Expiry::EndOfOpponentsNextTurn => {
                    source_player != ending_player
                }
                Expiry::EndOfYourTurn | Expiry::EndOfYourNextTurn => source_player == ending_player,
                _ => false,
            }
        };
        let is_next_turn_variant = |expiry: Expiry| -> bool {
            matches!(
                expiry,
                Expiry::EndOfOpponentsNextTurn | Expiry::EndOfYourNextTurn
            )
        };
        for entries in self.permanent_modifiers.values_mut() {
            entries.retain_mut(|e| {
                if !relevant(e.expiry, e.source_player) {
                    return true;
                }
                if is_next_turn_variant(e.expiry) && e.pending_skips > 0 {
                    e.pending_skips -= 1;
                    return true;
                }
                false
            });
        }
        // KeywordEntry/GrantedTriggeredEffect don't (yet) carry
        // pending_skips; alias to standard EndOf*Turn semantics. This
        // is the same behavior they had pre-Phase-4 — printed-text
        // fidelity is preserved for source-turn installs (the common
        // case); the edge-case mid-opp-turn install is only nuanced
        // for ModifierEntry today.
        for kws in self.permanent_keywords.values_mut() {
            kws.retain(|entry| !relevant(entry.expiry, entry.source_player));
        }
        for entries in self.granted_triggered.values_mut() {
            entries.retain(|e| !relevant(e.expiry, e.source_player));
        }
    }

    /// Track H Phase 4k cleanup helper — collect body_ids of granted-
    /// triggered entries that WOULD be evicted at the upcoming
    /// `expire_end_of_turn(ending_player)` call. Caller (`Game`) prunes
    /// the body registry against this list to avoid leaking closure
    /// bodies.
    pub fn collect_expiring_granted_body_ids(&self, ending_player: u8) -> Vec<u64> {
        let relevant = |expiry: Expiry, source_player: u8| -> bool {
            match expiry {
                Expiry::EndOfTurn => true,
                Expiry::EndOfOpponentsTurn | Expiry::EndOfOpponentsNextTurn => {
                    source_player != ending_player
                }
                Expiry::EndOfYourTurn | Expiry::EndOfYourNextTurn => source_player == ending_player,
                _ => false,
            }
        };
        let mut ids = Vec::new();
        for entries in self.granted_triggered.values() {
            for e in entries {
                if relevant(e.expiry, e.source_player) {
                    ids.push(e.body_id);
                }
            }
        }
        ids
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
    pub fn add_player_modifier(&mut self, target_player: PlayerId, mut entry: PlayerModifierEntry) {
        guard_install(
            entry.modifier,
            &entry.payload,
            entry.expiry,
            entry.until_condition.is_some(),
        );
        self.next_install_order = self.next_install_order.saturating_add(1);
        entry.install_order = self.next_install_order;
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

    pub fn any_player_has(&self, modifier: ModifierType) -> bool {
        self.player_modifiers
            .values()
            .any(|entries| entries.iter().any(|e| e.modifier == modifier))
    }

    pub fn any_other_player_has(&self, acting_player: PlayerId, modifier: ModifierType) -> bool {
        self.player_modifiers.iter().any(|(player, entries)| {
            *player != acting_player && entries.iter().any(|e| e.modifier == modifier)
        })
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

    pub fn end_turn_min_memory_floor(&self, player: PlayerId) -> Option<i32> {
        let player_floors = self
            .player_modifiers
            .get(&player)
            .into_iter()
            .flatten()
            .filter(|entry| entry.modifier == ModifierType::ChangeEndTurnMinMemory)
            .map(|entry| match &entry.payload {
                ModifierPayload::EndTurnMinMemory { value } => *value,
                ModifierPayload::None => entry.value,
                _ => 0,
            });
        let permanent_floors = self
            .permanent_modifiers
            .iter()
            .filter(move |(handle, _)| handle.player == player)
            .flat_map(|(_, entries)| entries)
            .filter(|entry| entry.modifier == ModifierType::ChangeEndTurnMinMemory)
            .map(|entry| match &entry.payload {
                ModifierPayload::EndTurnMinMemory { value } => *value,
                ModifierPayload::None => entry.value,
                _ => 0,
            });
        player_floors.chain(permanent_floors).max()
    }

    pub fn link_cost_delta_for_player(&self, player: PlayerId) -> i32 {
        let player_delta = self
            .player_modifiers
            .get(&player)
            .into_iter()
            .flatten()
            .filter(|entry| entry.modifier == ModifierType::ChangeLinkCost)
            .map(|entry| match &entry.payload {
                ModifierPayload::LinkCost { delta } => *delta,
                ModifierPayload::None => entry.value,
                _ => 0,
            });
        let permanent_delta = self
            .permanent_modifiers
            .iter()
            .filter(move |(handle, _)| handle.player == player)
            .flat_map(|(_, entries)| entries)
            .filter(|entry| entry.modifier == ModifierType::ChangeLinkCost)
            .map(|entry| match &entry.payload {
                ModifierPayload::LinkCost { delta } => *delta,
                ModifierPayload::None => entry.value,
                _ => 0,
            });
        player_delta.chain(permanent_delta).sum()
    }

    pub fn link_max_delta(&self, host: PermanentHandle) -> i32 {
        self.get(host, ModifierType::ChangeLinkMax)
            .into_iter()
            .map(|entry| match &entry.payload {
                ModifierPayload::LinkMax { delta } => *delta,
                ModifierPayload::None => entry.value,
                _ => 0,
            })
            .sum()
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
                Expiry::EndOfOpponentsTurn | Expiry::EndOfOpponentsNextTurn => {
                    e.source_player == ending_player
                }
                Expiry::EndOfYourTurn | Expiry::EndOfYourNextTurn => {
                    e.source_player != ending_player
                }
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

    pub fn until_condition_candidates(&self) -> Vec<(u64, ModifierSubject)> {
        let mut out = Vec::new();
        for (target, entries) in &self.permanent_modifiers {
            for entry in entries {
                if matches!(entry.expiry, Expiry::UntilCondition) {
                    out.push((entry.install_order, ModifierSubject::Permanent(*target)));
                }
            }
        }
        for (player, entries) in &self.player_modifiers {
            for entry in entries {
                if matches!(entry.expiry, Expiry::UntilCondition) {
                    out.push((entry.install_order, ModifierSubject::Player(*player)));
                }
            }
        }
        // Track H §4 — keyword entries with UntilCondition expiry are
        // also driven by the controller. Subject is the target
        // permanent (keywords are permanent-scoped). Install-order is
        // shared with the modifier counter so FIFO ordering across
        // both stores is correct.
        for (target, entries) in &self.permanent_keywords {
            for entry in entries {
                if matches!(entry.expiry, Expiry::UntilCondition) {
                    out.push((entry.install_order, ModifierSubject::Permanent(*target)));
                }
            }
        }
        out.sort_by_key(|(order, _)| *order);
        out
    }

    pub fn evaluate_until_condition(
        &self,
        subject: ModifierSubject,
        install_order: u64,
        game: &crate::game::Game,
    ) -> Option<bool> {
        match subject {
            ModifierSubject::Permanent(target) => {
                if let Some(entries) = self.permanent_modifiers.get(&target) {
                    if let Some(entry) = entries.iter().find(|e| e.install_order == install_order) {
                        return entry
                            .until_condition
                            .as_ref()
                            .map(|pred| pred(game, subject));
                    }
                }
                // Track H §4 — keyword-grant fallback.
                if let Some(entries) = self.permanent_keywords.get(&target) {
                    if let Some(entry) = entries.iter().find(|e| e.install_order == install_order) {
                        return entry
                            .until_condition
                            .as_ref()
                            .map(|pred| pred(game, subject));
                    }
                }
                None
            }
            ModifierSubject::Player(player) => self
                .player_modifiers
                .get(&player)?
                .iter()
                .find(|entry| entry.install_order == install_order)
                .and_then(|entry| entry.until_condition.as_ref())
                .map(|pred| pred(game, subject)),
        }
    }

    pub fn remove_until_condition_by_order(
        &mut self,
        subject: ModifierSubject,
        install_order: u64,
    ) -> bool {
        match subject {
            ModifierSubject::Permanent(target) => {
                if let Some(entries) = self.permanent_modifiers.get_mut(&target) {
                    let before = entries.len();
                    entries.retain(|entry| entry.install_order != install_order);
                    let removed = entries.len() != before;
                    if entries.is_empty() {
                        self.permanent_modifiers.remove(&target);
                    }
                    if removed {
                        return true;
                    }
                }
                // Track H §4 — keyword-grant fallback.
                let Some(entries) = self.permanent_keywords.get_mut(&target) else {
                    return false;
                };
                let before = entries.len();
                entries.retain(|entry| entry.install_order != install_order);
                let removed = entries.len() != before;
                if entries.is_empty() {
                    self.permanent_keywords.remove(&target);
                }
                removed
            }
            ModifierSubject::Player(player) => {
                let Some(entries) = self.player_modifiers.get_mut(&player) else {
                    return false;
                };
                let before = entries.len();
                entries.retain(|entry| entry.install_order != install_order);
                let removed = entries.len() != before;
                if entries.is_empty() {
                    self.player_modifiers.remove(&player);
                }
                removed
            }
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
    #[should_panic(
        expected = "Expiry::UntilCondition entries must carry an until_condition predicate"
    )]
    fn until_condition_rejects_missing_predicate_in_debug() {
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 100, Expiry::UntilCondition, 0),
        );
    }

    #[test]
    fn until_condition_with_predicate_installs() {
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 100, Expiry::UntilCondition, 0)
                .with_until_condition(std::sync::Arc::new(|_, _| true)),
        );
        assert_eq!(reg.sum(target, ModifierType::ChangeDp), 100);
        assert_eq!(reg.until_condition_candidates().len(), 1);
    }

    #[test]
    fn once_used_persists_through_turn_ends_until_consumed() {
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 50, Expiry::OnceUsed(1), 0),
        );
        reg.expire_end_of_turn(0);
        reg.expire_end_of_turn(1);
        reg.expire_end_of_attack();
        assert_eq!(
            reg.sum(target, ModifierType::ChangeDp),
            50,
            "OnceUsed entries must persist through turn-end cycles until a consult site consumes them"
        );
    }

    #[test]
    #[should_panic(expected = "modifier payload mismatch")]
    fn debug_guard_rejects_mismatched_payload() {
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.add(
            target,
            ModifierEntry::simple(ModifierType::ChangeDp, 0, Expiry::Permanent, 0).with_payload(
                ModifierPayload::Traits {
                    add: vec!["Holy".to_string()],
                    replace: false,
                },
            ),
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
