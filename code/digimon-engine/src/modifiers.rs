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
    /// Only these stable source identities are protected; never the whole host.
    ProtectedSourceCards(Vec<crate::card_source::CardHandle>),
    None,
    Traits {
        add: Vec<String>,
        replace: bool,
    },
    Name {
        value: String,
        base: bool,
    },
    SourceNameAliases {
        level_lte: Option<u8>,
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
    AdditiveSynthIdentity {
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
    /// `ModifierType::DnaMaterialIdentity` — while evaluating a DNA (or
    /// Blast-DNA) material requirement for a result card named
    /// `result_name`, the carrier is additionally treated as `level` and/or
    /// as named `name`. G-DNA-MATERIAL-TREATED-AS-FOR-TARGET.
    DnaMaterialIdentity {
        result_name: String,
        level: Option<u8>,
        name: Option<String>,
        result_scope: Option<digimon_dsl::compiled::CompiledDnaMaterialResultScope>,
    },
}

impl Default for ModifierPayload {
    fn default() -> Self {
        Self::None
    }
}

/// A single modifier entry.
///
/// `Clone` since 2026-06-18 (make-engine-cloneable task 1.2): the only
/// closure-valued fields — `replacement_condition` and `until_condition` —
/// are `Arc<dyn Fn>` shareable handles (Category-A behavior), so the entry
/// clones cheaply (shared behavior, copied data).
#[derive(Clone)]
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
    /// §15-15-5-1 / §15-15-5-2 record-and-suppress: the category of the
    /// effect that installed this entry (`None` for rule-installed and legacy
    /// entries, which no immunity filters). While a `CannotBeAffected` entry
    /// on the same permanent covers `(source_player, affecting_source_kind)`,
    /// [`ModifierRegistry::get`] hides the entry; it applies again the moment
    /// the immunity expires or its condition lapses, for whatever remains of
    /// its own duration (DCGO `ChangeDP.cs` `CanUseCondition`, re-evaluated at
    /// read time).
    pub affecting_source_kind: Option<EffectSourceKind>,
    pub affecting_source_identity: Option<(PlayerId, u64)>,
    pub immunity_excluded_source: Option<(PlayerId, u64)>,
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

/// Whether a `CannotBeAffected` entry's `filter` makes a permanent controlled
/// by `target_player` unaffected by an effect of category `source_kind`
/// controlled by `effect_controller`. `None` (an unfiltered entry) covers
/// every effect. The single predicate behind
/// [`ModifierRegistry::is_unaffected_by`] (the apply-time gate for one-shot
/// processing) and the read-time suppression in [`ModifierRegistry::get`].
pub fn immunity_filter_covers(
    filter: Option<&EffectImmunityFilter>,
    target_player: PlayerId,
    effect_controller: PlayerId,
    source_kind: EffectSourceKind,
) -> bool {
    let Some(filter) = filter else {
        return true;
    };
    if !filter
        .source_kind
        .map(|expected| source_kind.includes(expected))
        .unwrap_or(true)
    {
        return false;
    }
    match filter.controller {
        EffectControllerFilter::Any => true,
        EffectControllerFilter::OpponentOnly => effect_controller != target_player,
        EffectControllerFilter::OwnOnly => effect_controller == target_player,
    }
}

impl std::fmt::Debug for ModifierEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ModifierEntry");
        debug
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
            .field("affecting_source_kind", &self.affecting_source_kind)
            .field("disable_effect_timing", &self.disable_effect_timing)
            .field("install_order", &self.install_order)
            ;
        if let Some(identity) = self.affecting_source_identity { debug.field("affecting_source_identity", &identity); }
        if let Some(identity) = self.immunity_excluded_source { debug.field("immunity_excluded_source", &identity); }
        debug.finish_non_exhaustive()
    }
}

/// Compute the `pending_skips` value for a modifier being installed *now*.
///
/// `*NextTurn` expiries ([`Expiry::EndOfOpponentsNextTurn`] /
/// [`Expiry::EndOfYourNextTurn`]) must skip the turn-end that would
/// otherwise expire the entry one turn early. That only happens when the
/// install lands *during* that very turn:
/// - `EndOfOpponentsNextTurn` installed during an opponent's turn — the
///   current opponent turn-end must be skipped (returns 1).
/// - `EndOfYourNextTurn` installed during the source player's own turn —
///   the current own turn-end must be skipped (returns 1).
///
/// Every other case (all non-`*NextTurn` expiries, and `*NextTurn`
/// installs on a turn that is not the about-to-expire one) returns 0 —
/// the first matching turn-end is genuinely the *next* turn.
pub fn pending_skips_for_install(expiry: Expiry, source_player: u8, turn_player: u8) -> u8 {
    match expiry {
        Expiry::EndOfOpponentsNextTurn if turn_player != source_player => 1,
        Expiry::EndOfYourNextTurn if turn_player == source_player => 1,
        _ => 0,
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
            affecting_source_kind: None,
            affecting_source_identity: None,
            immunity_excluded_source: None,
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
            affecting_source_kind: None,
            affecting_source_identity: None,
            immunity_excluded_source: None,
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
            affecting_source_kind: None,
            affecting_source_identity: None,
            immunity_excluded_source: None,
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
            affecting_source_kind: None,
            affecting_source_identity: None,
            immunity_excluded_source: None,
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

    /// Tag the entry with the category of the effect installing it, so an
    /// immunity on the target suppresses it at read time (§15-15-5-2).
    pub fn with_affecting_source_identity(mut self, identity: Option<(PlayerId, u64)>) -> Self {
        self.affecting_source_identity = identity;
        self
    }

    pub fn with_affecting_source_kind(mut self, kind: EffectSourceKind) -> Self {
        self.affecting_source_kind = Some(kind);
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
#[derive(Clone)]
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
    /// Relevant turn ends to skip for next-turn expiry.
    pub pending_skips: u8,
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
            .field("pending_skips", &self.pending_skips)
            .field("install_order", &self.install_order)
            .finish_non_exhaustive()
    }
}

impl PlayerModifierEntry {
    /// Set install-time skips for the existing next-turn expiry variants.
    pub fn with_pending_skips(mut self, skips: u8) -> Self {
        self.pending_skips = skips;
        self
    }

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
            pending_skips: 0,
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
            pending_skips: 0,
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
            pending_skips: 0,
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
            | (ModifierType::CannotTrash, ModifierPayload::ProtectedSourceCards(_))
            | (ModifierType::ChangeTraits, ModifierPayload::Traits { .. })
            | (
                ModifierType::ChangeBaseCardName,
                ModifierPayload::Name { .. }
            )
            | (
                ModifierType::CanOnlyDigivolveInto,
                ModifierPayload::Name { .. }
            )
            | (
                ModifierType::SourceNameAliases,
                ModifierPayload::SourceNameAliases { .. }
            )
            | (
                ModifierType::ChangeBaseCardColor,
                ModifierPayload::Colors { .. }
            )
            | (ModifierType::AddColor, ModifierPayload::Colors { .. })
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
                ModifierPayload::SynthIdentity { .. } | ModifierPayload::AdditiveSynthIdentity { .. }
            )
            | (
                ModifierType::ChangePermanentLevel,
                ModifierPayload::LevelOverride { .. }
            )
            | (
                ModifierType::DnaMaterialIdentity,
                ModifierPayload::DnaMaterialIdentity { .. }
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
    /// §15-15-5-2 / §15-15-5-4 record-and-suppress for keyword grants: the
    /// category of the effect that granted the keyword (`None` for
    /// rule-installed and legacy entries, which no immunity filters). While a
    /// `CannotBeAffected` entry on the same permanent covers
    /// `(source_player, affecting_source_kind)` the keyword is not had, and
    /// it is had again the moment the immunity ends, for whatever remains of
    /// the grant's own duration (`ModifierRegistry::keyword_entry_applies`;
    /// G-ENGINE-UNAFFECTED-KEYWORD-GRANT-INSTALL-SKIP).
    affecting_source_kind: Option<EffectSourceKind>,
    affecting_source_identity: Option<(PlayerId, u64)>,
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
        let mut debug = f.debug_struct("KeywordEntry");
        debug
            .field("keyword", &self.keyword)
            .field("expiry", &self.expiry)
            .field("source_player", &self.source_player)
            .field("source_permanent", &self.source_permanent)
            .field("materialized_declarative", &self.materialized_declarative)
            .field("has_until_condition", &self.until_condition.is_some())
            .field("affecting_source_kind", &self.affecting_source_kind)
            .field("install_order", &self.install_order)
            ;
        if let Some(identity) = self.affecting_source_identity { debug.field("affecting_source_identity", &identity); }
        debug.finish()
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
            affecting_source_kind: None,
            affecting_source_identity: None,
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
            affecting_source_kind: None,
            affecting_source_identity: None,
            install_order: 0,
        }
    }
}

/// Closure body for a granted triggered effect — runs at the carrier's
/// matching timing with `EffectContext` set up so the carrier is the
/// `source_permanent` and the grantor's card is `source_card`. Track H §3.
pub type GrantedEffectBody =
    std::sync::Arc<dyn Fn(&mut crate::effect_context::EffectContext<'_>) + Send + Sync + 'static>;

/// Trigger qualifier of a granted triggered effect (G-DSL-GRANTED-TRIGGER-
/// QUALIFIER): decided when the event triggers the granted effect
/// (§15-8-3-1 / §15-8-3-4), read with the carrier as `source_permanent` and
/// the event's trigger context installed. An observer timing (`OnSuspend`)
/// enqueues a granted entry on every carrier for every matching event; the
/// qualifier keeps "When THIS Digimon becomes suspended" to its own event.
pub type GrantedTriggerQualifier = std::sync::Arc<
    dyn Fn(&crate::effect_context::EffectReadContext<'_>) -> bool + Send + Sync + 'static,
>;

/// Phase 4i — `Debug`-friendly wrapper around the granted-effect body
/// registry. The body closures are `Arc<dyn Fn ...>` which don't
/// implement `Debug`; the wrapper hides them behind a count.
#[derive(Default, Clone)]
pub struct GrantedEffectBodyRegistry {
    pub bodies: std::collections::HashMap<u64, GrantedEffectBody>,
    /// Trigger qualifiers keyed by the same body id (absent = none).
    pub qualifiers: std::collections::HashMap<u64, GrantedTriggerQualifier>,
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
        self.qualifiers.remove(&id);
        self.bodies.remove(&id)
    }
    pub fn insert_qualifier(&mut self, id: u64, qualifier: GrantedTriggerQualifier) {
        self.qualifiers.insert(id, qualifier);
    }
    pub fn qualifier(&self, id: u64) -> Option<&GrantedTriggerQualifier> {
        self.qualifiers.get(&id)
    }
}

/// A triggered effect granted onto a carrier permanent by some grantor
/// (Track H §3 — DCGO `AddSkillClass.cs` analog). The body fires when the
/// carrier's matching `timing` event drains. Cleanup happens via the
/// existing `clear_permanent` path when the carrier leaves the field, or
/// via `expire_end_of_turn` when the entry's `expiry` ticks out.
#[derive(Clone)]
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
    pub source_kind: EffectSourceKind,
    pub source_identity: Option<(PlayerId,u64)>,
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
        let mut debug = f.debug_struct("GrantedTriggeredEffect");
        debug
            .field("timing", &self.timing)
            .field("source_card", &self.source_card)
            .field("source_player", &self.source_player)
            .field("expiry", &self.expiry)
            .field("body_id", &self.body_id)
            ;
        if self.source_identity.is_some() || self.source_kind != EffectSourceKind::Digimon {
            debug.field("source_kind", &self.source_kind);
            debug.field("source_identity", &self.source_identity);
        }
        debug.finish_non_exhaustive()
    }
}

/// Source-independent, turn-bound grant, evaluated against each current carrier.
/// Predicate/body closures capture immutable DSL data and clone through Arc.
#[derive(Clone)]
pub struct DynamicTriggeredGrant {
    pub grant: GrantedTriggeredEffect,
    pub source_kind: crate::enums::EffectSourceKind,
    pub pending_skips: u8,
    pub predicate: std::sync::Arc<dyn Fn(&crate::effect_context::EffectReadContext<'_>, PermanentHandle) -> bool + Send + Sync>,
}

impl std::fmt::Debug for DynamicTriggeredGrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicTriggeredGrant")
            .field("grant", &self.grant)
            .field("pending_skips", &self.pending_skips)
            .finish_non_exhaustive()
    }
}

/// Tracks all active modifiers in the game.
#[derive(Debug, Default, Clone)]
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
    pub(crate) dynamic_triggered: Vec<DynamicTriggeredGrant>,
    next_install_order: u64,
    /// Memo for the incremental declarative-materialization skip
    /// (`Game::tick_declarative_effects`). The tick computes a fingerprint
    /// of the declarative-relevant game state; when it equals the stored
    /// fingerprint, the materialized state is already current and the full
    /// clear-and-rebuild is skipped. `decl_memo_valid == false` (derive
    /// `Default`) forces a rebuild on the first tick.
    decl_memo_valid: bool,
    decl_last_fingerprint: u64,
    /// §16-39-1 / §15-15-5-1 / §15-15-5-2: the `<Progress>` attacker (or an
    /// attacker with `ImmunityToOpponentEffects`) while it attacks -- the
    /// one piece of attack state the read path needs and cannot compute
    /// (`Game::progress_excludes` reads `pending_attack` and the printed /
    /// inherited keywords, which live on `Game`). [`ModifierRegistry::get`]
    /// hides every opponent-controlled, effect-installed entry on it, whenever
    /// that entry was installed (DCGO `Progress.cs` is a `CanNotAffectedClass`
    /// re-checked on every read, keyed to `attackProcess.AttackingPermanent`).
    /// `Game::refresh_progress_shield` owns it; the registry only follows the
    /// permanent through battle-area compaction
    /// ([`ModifierRegistry::shift_after_battle_area_remove`]), so a Digimon
    /// that slides into the attacker's old slot is never shielded.
    progress_shield: Option<PermanentHandle>,
}

impl ModifierRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Incremental declarative-materialization memo ─────────────────────
    //
    // `Game::tick_declarative_effects` clears and rebuilds the whole board's
    // materialized declarative state on every call. Most calls don't change
    // which declarative sources exist (or any state their conditions read),
    // so the rebuild is wasted. The tick fingerprints the declarative-
    // relevant inputs and skips the rebuild when the fingerprint is
    // unchanged. These accessors hold the last-seen fingerprint.

    /// The fingerprint stored after the last full rebuild, or `None` if the
    /// memo is invalid (forces a rebuild).
    pub fn declarative_memo(&self) -> Option<u64> {
        self.decl_memo_valid.then_some(self.decl_last_fingerprint)
    }

    /// Record the fingerprint that the just-completed full rebuild reflects.
    pub fn set_declarative_memo(&mut self, fingerprint: u64) {
        self.decl_last_fingerprint = fingerprint;
        self.decl_memo_valid = true;
    }

    /// Force the next tick to rebuild regardless of fingerprint (fallback /
    /// `force_full_rebuild` path).
    pub fn invalidate_declarative_memo(&mut self) {
        self.decl_memo_valid = false;
    }

    /// Order-independent digest of all NON-materialized modifier / keyword /
    /// player-scoped entries — the *inputs* a declarative condition can read
    /// (e.g. a DP buff a "while this has 5000+ DP" condition checks). Used by
    /// the tick fingerprint to notice modifier-state changes without
    /// per-mutator bookkeeping. A wrapping-sum of per-entry hashes is
    /// commutative (so the `HashMap` iteration order is irrelevant) and,
    /// unlike XOR, does not cancel field-identical duplicate entries.
    /// Materialized declaratives are the tick's *output* and are excluded.
    pub fn nonmaterialized_digest(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut sum: u64 = 0;
        for (handle, entries) in &self.permanent_modifiers {
            for e in entries {
                if e.materialized_declarative {
                    continue;
                }
                let mut h = std::collections::hash_map::DefaultHasher::new();
                handle.hash(&mut h);
                e.modifier.hash(&mut h);
                e.value.hash(&mut h);
                format!("{:?}", e.payload).hash(&mut h);
                e.expiry.hash(&mut h);
                e.source_permanent.hash(&mut h);
                e.source_player.hash(&mut h);
                format!("{:?}", e.effect_immunity_filter).hash(&mut h);
                format!("{:?}", e.disable_effect_timing).hash(&mut h);
                if e.affecting_source_identity.is_some() { e.affecting_source_identity.hash(&mut h); }
                if e.immunity_excluded_source.is_some() { e.immunity_excluded_source.hash(&mut h); }
                e.install_order.hash(&mut h);
                sum = sum.wrapping_add(h.finish());
            }
        }
        for (handle, entries) in &self.permanent_keywords {
            for e in entries {
                if e.materialized_declarative {
                    continue;
                }
                let mut h = std::collections::hash_map::DefaultHasher::new();
                handle.hash(&mut h);
                e.keyword.hash(&mut h);
                e.expiry.hash(&mut h);
                e.source_permanent.hash(&mut h);
                e.source_player.hash(&mut h);
                format!("{:?}", e.affecting_source_kind).hash(&mut h);
                if e.affecting_source_identity.is_some() { e.affecting_source_identity.hash(&mut h); }
                e.install_order.hash(&mut h);
                sum = sum.wrapping_add(h.finish());
            }
        }
        for (player, entries) in &self.player_modifiers {
            for e in entries {
                if e.materialized_declarative {
                    continue;
                }
                let mut h = std::collections::hash_map::DefaultHasher::new();
                player.hash(&mut h);
                e.modifier.hash(&mut h);
                e.value.hash(&mut h);
                format!("{:?}", e.payload).hash(&mut h);
                e.expiry.hash(&mut h);
                e.source_permanent.hash(&mut h);
                e.source_player.hash(&mut h);
                format!("{:?}", e.effect_immunity_filter).hash(&mut h);
                e.install_order.hash(&mut h);
                sum = sum.wrapping_add(h.finish());
            }
        }
        // The `<Progress>` shield changes what `get` returns for the attacker,
        // so a declarative condition reading its DP must see a rebuild.
        {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            "progress_shield".hash(&mut h);
            self.progress_shield.hash(&mut h);
            sum = sum.wrapping_add(h.finish());
        }
        sum
    }

    /// Canonical, order-independent snapshot of all MATERIALIZED declarative
    /// entries — the *output* of `tick_declarative_effects`. The debug-only
    /// differential oracle uses it to assert the fingerprint-skip path yields
    /// byte-identical materialized state to a full clear-and-rebuild. Sorted
    /// strings make the comparison order-independent and deliberately exclude
    /// `install_order` (an incidental monotonic counter that differs between
    /// two otherwise-identical rebuilds).
    #[cfg(debug_assertions)]
    pub fn materialized_snapshot(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (handle, entries) in &self.permanent_modifiers {
            for e in entries {
                if !e.materialized_declarative {
                    continue;
                }
                out.push(format!(
                    "M|{:?}|{:?}|{}|{:?}|{:?}|{}|{:?}|{:?}|{:?}",
                    handle,
                    e.modifier,
                    e.value,
                    e.payload,
                    e.expiry,
                    e.source_player,
                    e.source_permanent,
                    e.effect_immunity_filter,
                    e.disable_effect_timing,
                ));
            }
        }
        for (handle, entries) in &self.permanent_keywords {
            for e in entries {
                if !e.materialized_declarative {
                    continue;
                }
                out.push(format!(
                    "K|{:?}|{:?}|{:?}|{}|{:?}",
                    handle, e.keyword, e.expiry, e.source_player, e.source_permanent,
                ));
            }
        }
        for (player, entries) in &self.player_modifiers {
            for e in entries {
                if !e.materialized_declarative {
                    continue;
                }
                out.push(format!(
                    "P|{:?}|{:?}|{}|{:?}|{:?}|{}|{:?}|{:?}",
                    player,
                    e.modifier,
                    e.value,
                    e.payload,
                    e.expiry,
                    e.source_player,
                    e.source_permanent,
                    e.effect_immunity_filter,
                ));
            }
        }
        out.sort();
        out
    }

    /// Canonical modifier-state summary for replay verification digests.
    ///
    /// The registry is backed by `HashMap`s, so callers must not hash by raw
    /// iteration order. This intentionally excludes memo fields and monotonic
    /// install counters; it captures the active gameplay modifiers/keywords in
    /// a sorted, deterministic representation.
    pub(crate) fn verification_summary(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (handle, entries) in &self.permanent_modifiers {
            for entry in entries {
                out.push(format!(
                    "PM|{}|{}|{:?}|{}|{:?}|{:?}|{}|{}|{:?}|{}|{:?}|{:?}|{}",
                    handle.player,
                    handle.index,
                    entry.modifier,
                    entry.value,
                    entry.payload,
                    entry.expiry,
                    entry.source_player,
                    entry.materialized_declarative,
                    entry.source_permanent,
                    entry.until_condition.is_some(),
                    entry.cause_filter,
                    entry.effect_immunity_filter,
                    entry.pending_skips,
                ));
            }
        }
        for (handle, entries) in &self.permanent_keywords {
            for entry in entries {
                out.push(format!(
                    "KW|{}|{}|{:?}|{:?}|{}|{:?}|{}|{}",
                    handle.player,
                    handle.index,
                    entry.keyword,
                    entry.expiry,
                    entry.source_player,
                    entry.source_permanent,
                    entry.materialized_declarative,
                    entry.until_condition.is_some(),
                ));
            }
        }
        for (player, entries) in &self.player_modifiers {
            for entry in entries {
                let mut row = format!(
                    "PL|{}|{:?}|{}|{:?}|{:?}|{}|{}|{:?}|{}|{:?}|{:?}",
                    player,
                    entry.modifier,
                    entry.value,
                    entry.payload,
                    entry.expiry,
                    entry.source_player,
                    entry.materialized_declarative,
                    entry.source_permanent,
                    entry.until_condition.is_some(),
                    entry.cause_filter,
                    entry.effect_immunity_filter,
                );
                if entry.pending_skips > 0 {
                    row.push_str(&format!("|skip:{}", entry.pending_skips));
                }
                out.push(row);
            }
        }
        for (handle, entries) in &self.granted_triggered {
            for entry in entries {
                out.push(format!(
                    "GT|{}|{}|{:?}|{}|{}|{:?}",
                    handle.player,
                    handle.index,
                    entry.timing,
                    entry.source_card.0,
                    entry.source_player,
                    entry.expiry,
                ));
            }
        }
        for entry in &self.dynamic_triggered {
            let grant = &entry.grant;
            out.push(format!(
                "DGT|{}|{:?}|{}|{}|{:?}|{:?}|{}",
                grant.body_id,
                grant.timing,
                grant.source_card.0,
                grant.source_player,
                grant.expiry,
                entry.source_kind,
                entry.pending_skips,
            ));
        }
        out.sort();
        out
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

    /// Grant a keyword to a permanent with the given expiry (untagged: an
    /// engine / rule grant that no immunity or `<Progress>` shield hides).
    pub fn grant_keyword(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        expiry: Expiry,
        source_player: u8,
    ) {
        self.grant_keyword_entry(target, KeywordEntry::simple(keyword, expiry, source_player));
    }

    /// Grant a keyword as an EFFECT of category `source_kind` controlled by
    /// `source_player` (§15-15-5-2 / §15-15-5-4 record-and-suppress). The
    /// entry is always recorded; the readers hide it while a covering
    /// immunity or the `<Progress>` shield is on `target`.
    pub fn grant_keyword_from_effect(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        expiry: Expiry,
        source_player: u8,
        source_kind: EffectSourceKind,
    ) {
        self.grant_keyword_from_effect_identity(target, keyword, expiry, source_player, source_kind, None);
    }

    pub fn grant_keyword_from_effect_identity(
        &mut self, target: PermanentHandle, keyword: Keyword, expiry: Expiry,
        source_player: u8, source_kind: EffectSourceKind,
        source_identity: Option<(PlayerId, u64)>,
    ) {
        let mut entry = KeywordEntry::simple(keyword, expiry, source_player);
        entry.affecting_source_kind = Some(source_kind);
        entry.affecting_source_identity = source_identity;
        self.grant_keyword_entry(target, entry);
    }

    fn grant_keyword_entry(&mut self, target: PermanentHandle, mut entry: KeywordEntry) {
        self.next_install_order = self.next_install_order.saturating_add(1);
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
        let mut entry = KeywordEntry::simple(keyword, Expiry::UntilCondition, source_player);
        entry.until_condition = Some(predicate);
        self.grant_keyword_entry(target, entry);
    }

    /// [`Self::grant_keyword_with_until_condition`] for an EFFECT's grant
    /// (tagged with its `source_kind`; see [`Self::grant_keyword_from_effect`]).
    pub fn grant_keyword_with_until_condition_from_effect(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        predicate: UntilConditionFn,
        source_player: u8,
        source_kind: EffectSourceKind,
    ) {
        self.grant_keyword_with_until_condition_from_effect_identity(target,keyword,predicate,source_player,source_kind,None);
    }

    pub fn grant_keyword_with_until_condition_from_effect_identity(
        &mut self,target: PermanentHandle,keyword: Keyword,predicate: UntilConditionFn,
        source_player: u8,source_kind: EffectSourceKind,source_identity: Option<(PlayerId,u64)>,
    ) {
        let mut entry = KeywordEntry::simple(keyword, Expiry::UntilCondition, source_player);
        entry.until_condition = Some(predicate);
        entry.affecting_source_kind = Some(source_kind);
        entry.affecting_source_identity = source_identity;
        self.grant_keyword_entry(target, entry);
    }

    pub fn grant_declarative_keyword(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        expiry: Expiry,
        source_permanent: Option<PermanentHandle>,
        source_player: u8,
    ) {
        self.grant_declarative_keyword_from_effect_identity(target,keyword,expiry,source_permanent,source_player,None,None);
    }

    pub fn grant_declarative_keyword_from_effect_identity(
        &mut self,target: PermanentHandle,keyword: Keyword,expiry: Expiry,source_permanent: Option<PermanentHandle>,
        source_player: u8,source_kind: Option<EffectSourceKind>,source_identity: Option<(PlayerId,u64)>,
    ) {
        let mut entry = KeywordEntry::materialized_declarative(keyword,expiry,source_permanent,source_player);
        entry.affecting_source_kind = source_kind;
        entry.affecting_source_identity = source_identity;
        self.permanent_keywords.entry(target).or_default().push(entry);
    }

    /// Get all modifiers of a given type on a permanent that currently
    /// affect it.
    ///
    /// §15-15-5-1 / §15-15-5-2: an entry installed by an effect
    /// (`affecting_source_kind: Some`) is hidden while a `CannotBeAffected`
    /// entry on the same permanent covers that effect, and shows again as
    /// soon as the immunity ends -- the entry itself keeps its own duration.
    /// `CannotBeAffected` entries are never hidden. While `target` is the
    /// `<Progress>` shield (§16-39-1, [`ModifierRegistry::progress_shield`])
    /// every opponent-controlled, effect-installed entry on it is hidden,
    /// whether it was installed before or during the attack (§15-15-5-1,
    /// official Q&A Q6178), and shows again the moment the shield drops --
    /// the attack ends or the attacker loses `<Progress>` (§15-15-5-2).
    pub fn get(&self, target: PermanentHandle, modifier: ModifierType) -> Vec<&ModifierEntry> {
        self.applying_modifiers(target)
            .filter(|e| e.modifier == modifier)
            .collect()
    }

    /// Every modifier entry on `target` that currently affects it: the
    /// read-time gate of [`Self::get`] (the `<Progress>` shield and the
    /// immunity suppression), without the type filter. The ONE place the
    /// gate is evaluated: `get`, [`Self::permanent_modifiers_iter`],
    /// [`Self::any_with_type`] and [`Self::is_timing_disabled`] all read
    /// through it, so no reader sees an entry the rules say is not affecting
    /// the permanent (G-ENGINE-PROGRESS-SHIELD-BYPASSED-BY-TIMING-DISABLE-
    /// AND-FORMULA-AURAS).
    fn applying_modifiers(
        &self,
        target: PermanentHandle,
    ) -> impl Iterator<Item = &ModifierEntry> {
        let entries = self
            .permanent_modifiers
            .get(&target)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        // Hot path (`effective_dp`, search): only scan for suppression when
        // the permanent carries an immunity at all.
        let immune = entries
            .iter()
            .any(|e| e.modifier == ModifierType::CannotBeAffected);
        let shielded = self.progress_shield == Some(target);
        entries.iter().filter(move |e| {
            !(shielded && Self::hidden_by_progress_shield(target, e))
                && !(immune && Self::suppressed_by_immunity(entries, target, e))
        })
    }

    /// Whether `<Progress>` hides `entry` on the shielded `target` (§16-39-1
    /// "isn't affected by your opponent's effects"): the entry was installed
    /// by an effect (an `affecting_source_kind` tag, or a materialized
    /// declarative aura) controlled by `target`'s opponent. Untagged
    /// (engine-installed) entries and the attacker's own effects are not
    /// hidden; an opponent effect of any `EffectSourceKind`, `Rule` included,
    /// is (DCGO
    /// `Progress.cs` `SkillCondition`: an opponent effect with a source card).
    fn hidden_by_progress_shield(target: PermanentHandle, entry: &ModifierEntry) -> bool {
        entry.source_player != target.player
            && (entry.affecting_source_kind.is_some() || entry.materialized_declarative)
    }

    /// The permanent `<Progress>` currently shields (§16-39-1), if any.
    pub fn progress_shield(&self) -> Option<PermanentHandle> {
        self.progress_shield
    }

    /// Set the `<Progress>` shield. Only `Game::refresh_progress_shield`
    /// calls this: it is the one place that can evaluate `<Progress>`.
    pub(crate) fn set_progress_shield(&mut self, shield: Option<PermanentHandle>) {
        self.progress_shield = shield;
    }

    /// Whether `entry` (on `target`, whose entries are `entries`) is hidden by
    /// an immunity on `target` (§15-15-5-1).
    fn suppressed_by_immunity(
        entries: &[ModifierEntry],
        target: PermanentHandle,
        entry: &ModifierEntry,
    ) -> bool {
        let Some(kind) = entry.affecting_source_kind else {
            return false;
        };
        entry.modifier != ModifierType::CannotBeAffected
            && entries.iter().any(|imm| {
                imm.modifier == ModifierType::CannotBeAffected
                    && !(imm.immunity_excluded_source.is_some() && imm.immunity_excluded_source == entry.affecting_source_identity)
                    && immunity_filter_covers(
                        imm.effect_immunity_filter.as_ref(),
                        target.player,
                        entry.source_player,
                        kind,
                    )
            })
    }

    /// Whether `target` is currently unaffected by an effect of category
    /// `source_kind` controlled by `effect_controller` (§15-15-5-1): any of
    /// its `CannotBeAffected` entries covers it.
    pub fn is_unaffected_by(
        &self,
        target: PermanentHandle,
        effect_controller: PlayerId,
        source_kind: EffectSourceKind,
    ) -> bool {
        self.is_unaffected_by_from(target, effect_controller, source_kind, None)
    }

    pub fn is_unaffected_by_from(
        &self, target: PermanentHandle, effect_controller: PlayerId,
        source_kind: EffectSourceKind, source_identity: Option<(PlayerId, u64)>,
    ) -> bool {
        self.permanent_modifiers.get(&target).is_some_and(|entries| {
            entries.iter().any(|imm| {
                imm.modifier == ModifierType::CannotBeAffected
                    && !(imm.immunity_excluded_source.is_some() && imm.immunity_excluded_source == source_identity)
                    && immunity_filter_covers(
                        imm.effect_immunity_filter.as_ref(),
                        target.player,
                        effect_controller,
                        source_kind,
                    )
            })
        })
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

    /// Whether any `modifier` entry on `target` blocks an effect controlled
    /// by `effect_controller`, honoring each entry's
    /// `effect_immunity_filter.controller` scope:
    ///   - `OpponentOnly` — blocks only effects controlled by `target`'s
    ///     opponent ("your opponent's effects can't …", EX13-023);
    ///   - `OwnOnly` — blocks only the owner's own effects;
    ///   - `Any` / no filter — blocks every effect (back-compat for entries
    ///     installed without a filter).
    /// Used by the stacked-card protections (`ImmuneFromStackTrashing`,
    /// `ImmuneFromStackReturn`). G-ENGINE-STACKED-CARD-RETURN-PROTECTION.
    pub fn blocks_effect_from(
        &self,
        target: PermanentHandle,
        modifier: ModifierType,
        effect_controller: PlayerId,
    ) -> bool {
        let from_opponent = effect_controller != target.player;
        self.get(target, modifier).into_iter().any(|entry| {
            match entry
                .effect_immunity_filter
                .map(|f| f.controller)
                .unwrap_or(EffectControllerFilter::Any)
            {
                EffectControllerFilter::Any => true,
                EffectControllerFilter::OpponentOnly => from_opponent,
                EffectControllerFilter::OwnOnly => !from_opponent,
            }
        })
    }

    /// Iterate over every `ModifierEntry` attached to `target` that
    /// currently AFFECTS it (regardless of `ModifierType`): the same
    /// read-time gate as [`Self::get`] (§15-15-5-1 / §16-39-1), so the
    /// Phase 7 replacement dispatcher's `CannotBe*` scan, the views and the
    /// serializer never see an entry an immunity or the `<Progress>` shield
    /// hides. Entries stay installed and show again when the gate lifts.
    pub fn permanent_modifiers_iter(
        &self,
        target: PermanentHandle,
    ) -> impl Iterator<Item = &ModifierEntry> {
        self.applying_modifiers(target)
    }

    /// Whether ANY permanent in the registry has a modifier of the given
    /// type that currently affects it. Mirrors Python's "global modifier
    /// query" pattern — e.g. `_is_play_blocked_by_modifier` iterates every
    /// active modifier of type `CannotPlayFromHand` without keying by
    /// target. Reads through the [`Self::get`] gate per permanent.
    pub fn any_with_type(&self, modifier: ModifierType) -> bool {
        self.permanent_modifiers
            .keys()
            .any(|target| self.applying_modifiers(*target).any(|e| e.modifier == modifier))
    }

    /// Whether a granted keyword `entry` on `target` is currently HAD
    /// (§15-15-5-4 "won't be considered to have that effect"): the same
    /// read-time gate as [`Self::get`] applies to modifiers --
    /// - the `<Progress>` shield (§16-39-1) hides every keyword an
    ///   opponent's effect granted (a tagged entry or a materialized
    ///   declarative aura), except `<Progress>` itself: hiding a granted
    ///   `<Progress>` would drop the shield that hides it (a fixpoint the
    ///   rules give no answer to; no authored card grants it to an opponent);
    /// - a `CannotBeAffected` entry on `target` covering the granting
    ///   effect's `(source_player, affecting_source_kind)` hides the entry
    ///   (§15-15-5-1 / §15-15-5-2, Q6178 / Q6179), whichever was installed
    ///   first.
    /// Untagged entries (engine / rule grants) are always had.
    fn keyword_entry_applies(&self, target: PermanentHandle, entry: &KeywordEntry) -> bool {
        if self.progress_shield == Some(target)
            && entry.keyword != Keyword::Progress
            && entry.source_player != target.player
            && (entry.affecting_source_kind.is_some() || entry.materialized_declarative)
        {
            return false;
        }
        let Some(kind) = entry.affecting_source_kind else {
            return true;
        };
        !self.is_unaffected_by_from(target, entry.source_player, kind, entry.affecting_source_identity)
    }

    /// The granted keyword entries on `target` that are currently had
    /// ([`Self::keyword_entry_applies`]).
    fn applying_keywords(&self, target: PermanentHandle) -> impl Iterator<Item = &KeywordEntry> {
        self.permanent_keywords
            .get(&target)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .iter()
            .filter(move |entry| self.keyword_entry_applies(target, entry))
    }

    /// Whether the permanent has the given granted keyword.
    pub fn has_keyword(&self, target: PermanentHandle, keyword: Keyword) -> bool {
        self.applying_keywords(target)
            .any(|entry| entry.keyword == keyword)
    }

    /// Whether the permanent has a granted keyword of the same TYPE as
    /// `keyword`, whatever its value (Comprehensive Rules 4.3 §16-2).
    pub fn has_keyword_type(&self, target: PermanentHandle, keyword: &Keyword) -> bool {
        self.applying_keywords(target)
            .any(|entry| entry.keyword.same_type(keyword))
    }

    /// Active keywords granted to a permanent by modifiers/declarative auras.
    pub fn granted_keywords(&self, target: PermanentHandle) -> Vec<Keyword> {
        let mut out = Vec::new();
        for entry in self.applying_keywords(target) {
            if !out.contains(&entry.keyword) {
                out.push(entry.keyword);
            }
        }
        out
    }

    /// Keywords granted to `target` by a MATERIALIZED DECLARATIVE aura and by
    /// nothing else — the `G-ENGINE-AURA-GRANT-NO-TRIGGER` scan set.
    ///
    /// A filtered-target aura ("all of your [X] gain `<KW>`") installs its
    /// grant here through `grant_declarative_keyword`, and deliberately does
    /// NOT set the grantor effect's `granted_keyword` marker (that marker keys
    /// the synthesized auto-effect to the GRANTOR, which would be wrong for a
    /// grant landing on someone else). So for these entries — and only these —
    /// nothing else installs the keyword's trigger, and the trigger-dispatch
    /// scan must synthesize it on the RECIPIENT.
    ///
    /// A keyword that ALSO has a non-materialized entry on the same target is
    /// excluded: a runtime grant (`EffectContext::grant_keyword`) already
    /// registered a granted-triggered body for it through
    /// `grant_keyword_triggered_auto_effects`, so re-synthesizing would
    /// double-fire. (Printed / self-aura-marked origins are filtered out by the
    /// caller via `Game::has_keyword_from_card_sources`.)
    pub fn aura_granted_keywords(&self, target: PermanentHandle) -> Vec<Keyword> {
        let entries: Vec<&KeywordEntry> = self.applying_keywords(target).collect();
        let mut out: Vec<Keyword> = Vec::new();
        for entry in &entries {
            if !entry.materialized_declarative {
                continue;
            }
            if out.contains(&entry.keyword) {
                continue;
            }
            if entries
                .iter()
                .any(|other| !other.materialized_declarative && other.keyword == entry.keyword)
            {
                continue;
            }
            out.push(entry.keyword);
        }
        out
    }

    /// Net `<Security A. +N>` / `<Security A. -N>` contributed by *granted*
    /// keyword entries on `target` (e.g. an aura's
    /// `grant_keyword: SecurityAttackPlus`). Printed face/inherited keywords
    /// on the permanent's `card_sources` are summed separately by
    /// `Game::security_attack_keyword_bonus`; this folds in the registry side
    /// so aura-granted Security Attack keywords are actually consumed at the
    /// security loop.
    pub fn granted_security_attack_keyword_bonus(&self, target: PermanentHandle) -> i32 {
        self.applying_keywords(target)
            .map(|entry| match entry.keyword {
                Keyword::SecurityAttackPlus(n) => n as i32,
                Keyword::SecurityAttackMinus(n) => -(n as i32),
                _ => 0,
            })
            .sum()
    }

    /// Whether dispatch of `timing` should be suppressed for `target`.
    ///
    /// True iff there is at least one `ModifierType::DisableEffect` entry on
    /// `target` whose `disable_effect_timing == Some(timing)`. Read by
    /// Track A's observer dispatch hook before firing per-permanent
    /// observers — when this returns `true`, the dispatcher skips the
    /// permanent's effects at that specific timing without disabling the
    /// permanent's other effects.
    ///
    /// Reads through the [`Self::get`] gate: an opponent's `DisableEffect`
    /// on a `<Progress>` attacker (§16-39-1), or one covered by an immunity
    /// on `target` (§15-15-5-1), does not disable anything while it is
    /// hidden.
    pub fn is_timing_disabled(&self, target: PermanentHandle, timing: EffectTiming) -> bool {
        self.get(target, ModifierType::DisableEffect)
            .into_iter()
            .any(|e| e.disable_effect_timing == Some(timing))
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
        // These operations can remove/rekey materialized outputs while the
        // declarative input fingerprint remains unchanged.
        self.invalidate_declarative_memo();
        self.permanent_modifiers.remove(&target);
        self.permanent_keywords.remove(&target);
        self.granted_triggered.remove(&target);
    }

    /// Re-key permanent-scoped modifier state after removing one battle-area
    /// permanent. Battle-area handles are index based, so every later
    /// permanent for the same player shifts down by one.
    pub fn shift_after_battle_area_remove(&mut self, player: PlayerId, removed_index: u8) {
        // These operations can remove/rekey materialized outputs while the
        // declarative input fingerprint remains unchanged.
        self.invalidate_declarative_memo();
        fn shifted(
            handle: PermanentHandle,
            player: PlayerId,
            removed_index: u8,
        ) -> Option<PermanentHandle> {
            // Breeding uses a sentinel, not a battle-area vector index. Its
            // modifiers and source attribution survive battle compaction.
            if handle.player != player
                || handle.index == crate::action::space::BREEDING_TARGET as u8
            {
                return Some(handle);
            }
            if handle.index == removed_index {
                return None;
            }
            if handle.index > removed_index {
                return Some(PermanentHandle {
                    player: handle.player,
                    index: handle.index.saturating_sub(1),
                });
            }
            Some(handle)
        }

        fn remap_entries<T>(
            map: &mut HashMap<PermanentHandle, Vec<T>>,
            player: PlayerId,
            removed_index: u8,
        ) {
            let old = std::mem::take(map);
            for (handle, entries) in old {
                if let Some(new_handle) = shifted(handle, player, removed_index) {
                    map.entry(new_handle).or_default().extend(entries);
                }
            }
        }

        remap_entries(&mut self.permanent_modifiers, player, removed_index);
        remap_entries(&mut self.permanent_keywords, player, removed_index);
        remap_entries(&mut self.granted_triggered, player, removed_index);
        // The shield follows the attacker it was set on (and drops when that
        // permanent is the one removed); it never stays on the slot.
        self.progress_shield = self
            .progress_shield
            .and_then(|h| shifted(h, player, removed_index));

        for entries in self.permanent_modifiers.values_mut() {
            for entry in entries {
                entry.source_permanent = entry
                    .source_permanent
                    .and_then(|h| shifted(h, player, removed_index));
            }
        }
        for entries in self.permanent_keywords.values_mut() {
            for entry in entries {
                entry.source_permanent = entry
                    .source_permanent
                    .and_then(|h| shifted(h, player, removed_index));
            }
        }
        for entries in self.player_modifiers.values_mut() {
            for entry in entries {
                entry.source_permanent = entry
                    .source_permanent
                    .and_then(|h| shifted(h, player, removed_index));
            }
        }
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
    pub fn granted_triggered_entries_for_timing(
        &self, carrier: PermanentHandle, timing: crate::enums::EffectTiming,
    ) -> Vec<GrantedTriggeredEffect> {
        self.granted_triggered.get(&carrier).map(|entries|
            entries.iter().filter(|entry|entry.timing==timing).cloned().collect())
            .unwrap_or_default()
    }

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

    pub fn contains_granted_triggered_body_id(&self, body_id: u64) -> bool {
        self.dynamic_triggered.iter().any(|e| e.grant.body_id == body_id)
            || self.granted_triggered
                .values()
                .any(|entries| entries.iter().any(|e| e.body_id == body_id))
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
        self.dynamic_triggered.retain_mut(|entry| {
            if !relevant(entry.grant.expiry, entry.grant.source_player) {
                return true;
            }
            if entry.pending_skips > 0 {
                entry.pending_skips -= 1;
                return true;
            }
            false
        });
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
        ids.extend(self.dynamic_triggered.iter().filter_map(|entry| {
            (entry.pending_skips == 0 && relevant(entry.grant.expiry, entry.grant.source_player))
                .then_some(entry.grant.body_id)
        }));
        ids
    }

    /// Expire modifiers at the end of an attack.
    pub fn expire_end_of_attack(&mut self) {
        for entries in self.permanent_modifiers.values_mut() {
            entries.retain(|e| !matches!(e.expiry, Expiry::EndOfAttack | Expiry::EndOfBattle));
        }
        // §16-39-1 / §15-15-5-2: the attack is over, so `<Progress>` no longer
        // applies; the opponent's entries it hid start affecting the
        // attacker now, for whatever remains of their own duration.
        self.progress_shield = None;
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

    /// Strongest matching threshold; absence differs from an active zero.
    /// Threshold prohibitions overlap by union, never by summing values.
    pub fn player_max_modifier_value(&self, player: PlayerId, modifier: ModifierType) -> Option<i32> {
        self.player_modifiers.get(&player).into_iter().flatten()
            .filter(|entry| entry.modifier == modifier).map(|entry| entry.value).max()
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

    /// Summed "add N to this Digimon's DP deletion effects' maximums" delta on
    /// `host` (`ModifierType::ChangeDPDeleteEffectMaxDP`). Added to the DP cap
    /// of every deletion effect whose source permanent is `host`.
    /// G-ENGINE-DP-DELETION-MAX-MODIFIER.
    pub fn dp_delete_effect_max_delta(&self, host: PermanentHandle) -> i32 {
        self.get(host, ModifierType::ChangeDPDeleteEffectMaxDP)
            .into_iter()
            .map(|entry| match &entry.payload {
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
            entries.retain_mut(|e| {
                let relevant = match e.expiry {
                    Expiry::EndOfTurn => true,
                    Expiry::EndOfOpponentsTurn | Expiry::EndOfOpponentsNextTurn => {
                        e.source_player != ending_player
                    }
                    Expiry::EndOfYourTurn | Expiry::EndOfYourNextTurn => {
                        e.source_player == ending_player
                    }
                    _ => false,
                };
                if !relevant {
                    return true;
                }
                if matches!(e.expiry, Expiry::EndOfOpponentsNextTurn | Expiry::EndOfYourNextTurn)
                    && e.pending_skips > 0
                {
                    e.pending_skips -= 1;
                    return true;
                }
                false
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

    /// §16-39-1: every reader goes through the `<Progress>` shield gate, not
    /// only `get`: an opponent's effect-installed `DisableEffect` on the
    /// shielded attacker disables nothing, is not scanned by
    /// `permanent_modifiers_iter`, and is not found by `any_with_type` while
    /// the shield is up; it shows again the moment the shield drops
    /// (G-ENGINE-PROGRESS-SHIELD-BYPASSED-BY-TIMING-DISABLE-AND-FORMULA-AURAS).
    #[test]
    fn progress_shield_gates_timing_disable_raw_scan_and_any_with_type() {
        let mut reg = ModifierRegistry::new();
        let attacker = h(0, 0);
        let mut entry = ModifierEntry::simple(ModifierType::DisableEffect, 0, Expiry::EndOfTurn, 1)
            .with_affecting_source_kind(EffectSourceKind::Digimon);
        entry.disable_effect_timing = Some(EffectTiming::WhenAttacking);
        reg.add(attacker, entry);
        assert!(reg.is_timing_disabled(attacker, EffectTiming::WhenAttacking));
        assert_eq!(reg.permanent_modifiers_iter(attacker).count(), 1);
        assert!(reg.any_with_type(ModifierType::DisableEffect));

        reg.set_progress_shield(Some(attacker));
        assert!(
            !reg.is_timing_disabled(attacker, EffectTiming::WhenAttacking),
            "shielded: the opponent's timing disable is not affecting it"
        );
        assert_eq!(reg.permanent_modifiers_iter(attacker).count(), 0);
        assert!(!reg.any_with_type(ModifierType::DisableEffect));
        // The attacker's OWN effect is never hidden by `<Progress>`.
        let mut own = ModifierEntry::simple(ModifierType::DisableEffect, 0, Expiry::EndOfTurn, 0)
            .with_affecting_source_kind(EffectSourceKind::Digimon);
        own.disable_effect_timing = Some(EffectTiming::OnDeletion);
        reg.add(attacker, own);
        assert!(reg.is_timing_disabled(attacker, EffectTiming::OnDeletion));
        assert_eq!(reg.permanent_modifiers_iter(attacker).count(), 1);

        reg.set_progress_shield(None);
        assert!(reg.is_timing_disabled(attacker, EffectTiming::WhenAttacking));
        assert_eq!(reg.permanent_modifiers_iter(attacker).count(), 2);
    }

    /// §15-15-5-1 / §15-15-5-2: the immunity suppression gates the same
    /// readers; the entry outlives the immunity and applies again after it.
    #[test]
    fn immunity_gates_timing_disable_raw_scan_and_any_with_type() {
        let mut reg = ModifierRegistry::new();
        let target = h(1, 0);
        // P0's disable lasts until the end of P1's turn; P1's immunity ends
        // with the current (P0's) turn.
        let mut entry =
            ModifierEntry::simple(ModifierType::DisableEffect, 0, Expiry::EndOfOpponentsTurn, 0)
                .with_affecting_source_kind(EffectSourceKind::Digimon);
        entry.disable_effect_timing = Some(EffectTiming::OnDeletion);
        reg.add(target, entry);
        assert!(reg.is_timing_disabled(target, EffectTiming::OnDeletion));

        reg.add(
            target,
            ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::EndOfTurn, 1)
                .with_effect_immunity_filter(EffectImmunityFilter {
                    source_kind: Some(EffectSourceKind::Digimon),
                    controller: EffectControllerFilter::OpponentOnly,
                }),
        );
        assert!(
            !reg.is_timing_disabled(target, EffectTiming::OnDeletion),
            "immune to the opponent's Digimon effects: the disable is not had"
        );
        // The immunity entry itself is the only applying entry.
        assert_eq!(reg.permanent_modifiers_iter(target).count(), 1);
        assert!(!reg.any_with_type(ModifierType::DisableEffect));
        assert!(reg.any_with_type(ModifierType::CannotBeAffected));

        reg.expire_end_of_turn(0);
        assert!(
            reg.is_timing_disabled(target, EffectTiming::OnDeletion),
            "the immunity ended: the disable applies again"
        );
    }

    /// §15-15-5-4 / §16-39-1 for KEYWORD grants: an effect's grant is
    /// recorded and gated at read time like a modifier; an untagged (engine)
    /// grant is always had (G-ENGINE-UNAFFECTED-KEYWORD-GRANT-INSTALL-SKIP).
    #[test]
    fn keyword_grants_are_gated_by_shield_and_immunity_at_read_time() {
        let mut reg = ModifierRegistry::new();
        let target = h(0, 0);
        reg.grant_keyword_from_effect(
            target,
            Keyword::SecurityAttackMinus(1),
            Expiry::EndOfOpponentsTurn,
            1,
            EffectSourceKind::Digimon,
        );
        reg.grant_keyword(target, Keyword::Blocker, Expiry::EndOfTurn, 1);
        assert!(reg.has_keyword(target, Keyword::SecurityAttackMinus(1)));
        assert_eq!(reg.granted_security_attack_keyword_bonus(target), -1);

        reg.set_progress_shield(Some(target));
        assert!(!reg.has_keyword(target, Keyword::SecurityAttackMinus(1)));
        assert!(!reg.has_keyword_type(target, &Keyword::SecurityAttackMinus(9)));
        assert_eq!(reg.granted_security_attack_keyword_bonus(target), 0);
        assert_eq!(reg.granted_keywords(target), vec![Keyword::Blocker], "untagged grant stays");
        reg.set_progress_shield(None);
        assert!(reg.has_keyword(target, Keyword::SecurityAttackMinus(1)));

        reg.add(
            target,
            ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::EndOfTurn, 0)
                .with_effect_immunity_filter(EffectImmunityFilter {
                    source_kind: Some(EffectSourceKind::Digimon),
                    controller: EffectControllerFilter::OpponentOnly,
                }),
        );
        assert!(!reg.has_keyword(target, Keyword::SecurityAttackMinus(1)));
        assert!(reg.has_keyword(target, Keyword::Blocker));
    }

    #[test]
    fn pending_skips_for_install_next_turn_variants() {
        // EndOfOpponentsNextTurn: the relevant turn-end is an opponent's
        // turn-end. Installed on the source player's OWN turn, the next
        // opponent turn-end IS "the opponent's next turn" — skip 0.
        assert_eq!(
            pending_skips_for_install(Expiry::EndOfOpponentsNextTurn, 0, 0),
            0,
            "own-turn install of EndOfOpponentsNextTurn needs no skip"
        );
        // Installed DURING the opponent's turn, the current opponent
        // turn-end would expire it one turn early — skip 1.
        assert_eq!(
            pending_skips_for_install(Expiry::EndOfOpponentsNextTurn, 0, 1),
            1,
            "mid-opponent-turn install of EndOfOpponentsNextTurn must skip the current opp turn-end"
        );
        // EndOfYourNextTurn is the mirror: relevant turn-end is the
        // source player's own turn-end.
        assert_eq!(
            pending_skips_for_install(Expiry::EndOfYourNextTurn, 0, 0),
            1,
            "own-turn install of EndOfYourNextTurn must skip the current own turn-end"
        );
        assert_eq!(
            pending_skips_for_install(Expiry::EndOfYourNextTurn, 0, 1),
            0,
            "opponent-turn install of EndOfYourNextTurn needs no skip"
        );
    }

    #[test]
    fn pending_skips_for_install_zero_for_non_next_turn_expiries() {
        for expiry in [
            Expiry::EndOfTurn,
            Expiry::EndOfOpponentsTurn,
            Expiry::EndOfYourTurn,
            Expiry::Permanent,
        ] {
            assert_eq!(
                pending_skips_for_install(expiry, 0, 0),
                0,
                "non-*NextTurn expiry {expiry:?} never carries pending_skips"
            );
            assert_eq!(pending_skips_for_install(expiry, 0, 1), 0);
        }
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
    fn player_next_turn_summary_keeps_skip_bound_to_entry() {
        let make = |swap: bool| {
            let mut reg = ModifierRegistry::new();
            for (value, skips) in [(1, u8::from(swap)), (2, u8::from(!swap))] {
                reg.add_player_modifier(1, PlayerModifierEntry::simple(
                    ModifierType::ChangeDp, value, Expiry::EndOfOpponentsNextTurn, None, 0,
                ).with_pending_skips(skips));
            }
            reg
        };
        let mut a = make(false);
        let mut b = make(true);
        assert_ne!(a.verification_summary(), b.verification_summary());
        a.expire_player_end_of_turn(1);
        b.expire_player_end_of_turn(1);
        assert_ne!(a.verification_summary(), b.verification_summary());
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
