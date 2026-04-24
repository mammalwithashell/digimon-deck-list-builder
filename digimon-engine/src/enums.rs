use crate::card_source::CardHandle;
use serde::{Deserialize, Serialize};

/// Identifies a player in the game. 0-indexed internally.
pub type PlayerId = u8;

/// Card type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
}

/// Card color.
///
/// Variant order mirrors Python's `CardColor` enum in
/// `digimon_gym/engine/data/enums.py` so that `CardColor as u8` yields the
/// same integer used in `cards.json::card_colors` and evo-cost entries.
/// This lets callers cross-compare an enum value against the raw JSON int
/// (see `policies/greedy.rs`). If you add, remove, or reorder variants
/// here, also update `card_data.rs::parse_card_color`,
/// `action/mask.rs::evo_color`, and `serialization.rs::color_to_python_int`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardColor {
    Red = 0,
    Blue = 1,
    Yellow = 2,
    Green = 3,
    White = 4,
    Black = 5,
    Purple = 6,
}

/// Game phase — drives the state machine and action mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamePhase {
    // Standard turn phases
    Mulligan,
    Unsuspend,
    Draw,
    Breeding,
    Main,
    EndTurn,

    // Interrupt / selection phases
    SelectTarget,
    SelectMaterial,
    SelectTrash,
    SelectSource,
    SelectHand,
    SelectReveal,
    SelectSecurity,
    EffectChoice,

    // Combat sub-phases
    BlockTiming,
    CounterTiming,
    AllianceTiming,

    // Special
    EndOfTurnAction,
    GameOver,

    // Phase 4 selection kinds — richer prompt types (Tasks 2-5 wire dispatch)
    SelectUnion,
    SelectPermutation,
    SelectBudgeted,
}

impl GamePhase {
    /// Python-parity phase name. Use this when serializing phase into a
    /// string field that will be compared against Python's `GamePhase.name`.
    pub fn py_name(&self) -> &'static str {
        match self {
            GamePhase::Mulligan => "Mulligan",
            GamePhase::Unsuspend => "Start",            // Python enum name
            GamePhase::Draw => "Draw",
            GamePhase::Breeding => "Breeding",
            GamePhase::Main => "Main",
            GamePhase::EndTurn => "End",                // Python enum name
            GamePhase::SelectTarget => "SelectTarget",
            GamePhase::SelectMaterial => "SelectMaterial",
            GamePhase::SelectTrash => "SelectTrash",
            GamePhase::SelectSource => "SelectSource",
            GamePhase::SelectHand => "SelectHand",
            GamePhase::SelectReveal => "SelectReveal",
            GamePhase::SelectSecurity => "SelectSecurity",
            GamePhase::EffectChoice => "SelectEffectChoice", // Python name
            GamePhase::BlockTiming => "BlockTiming",
            GamePhase::CounterTiming => "CounterTiming",
            GamePhase::AllianceTiming => "AllianceTiming",
            GamePhase::EndOfTurnAction => "EndOfTurnAction",
            GamePhase::GameOver => "GameOver",
            // Phase 4 variants — no Python equivalent yet; use identifier as name
            GamePhase::SelectUnion => "SelectUnion",
            GamePhase::SelectPermutation => "SelectPermutation",
            GamePhase::SelectBudgeted => "SelectBudgeted",
        }
    }
}

/// When a card effect triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectTiming {
    // Standard effect timings
    OnPlay,
    WhenDigivolving,
    OnAttack,
    OnDeletion,
    WhenAttacking,
    OnBlock,
    /// Primary security-trigger timing — fires for each effect on the card
    /// revealed during a security check. Mirrors Python's `SecuritySkill`
    /// (`enums.py`). Effects that should run here must also set the
    /// `security` flag on the `Effect` struct (builder: `.security_flag()`).
    SecuritySkill,
    /// Observer timing fired once per security reveal **after** the revealed
    /// card's own effects resolve. Consumed by other-zone effects that watch
    /// for security checks (e.g. "When security is checked, gain 1 memory").
    /// Mirrors Python's `OnSecurityCheck`.
    OnSecurityCheck,
    /// Fires when a card leaves the security stack — whether it's trashed
    /// after resolution or played from security by an effect. Mirrors
    /// Python's `OnLoseSecurity`.
    OnLoseSecurity,
    CounterEffect,

    // Turn-based
    StartOfYourTurn,
    StartOfOpponentsTurn,
    /// Fires at the start of the controller's Main phase (after Draw).
    /// Card scripts can use this for "at the start of your main phase" effects.
    StartOfYourMainPhase,
    EndOfYourTurn,
    EndOfOpponentsTurn,
    EndOfAttack,
    /// Fires when a battle resolves (DP comparison complete) but before
    /// `EndOfAttack`. Used for "if this Digimon wins/loses a battle" effects.
    EndOfBattle,

    // Triggered by game events
    OnAllyAttack,
    OnOpponentAttack,
    OnDrawCard,
    OnTrash,
    OnReturn,
    OnSuspend,
    OnUnsuspend,
    OnAddToHand,
    OnReveal,
    OnPlaceSecurity,
    /// Fires when an attack declaration's target changes mid-combat (e.g.
    /// Blocker redirect). Observer timing for effects that react to the
    /// new target.
    OnAttackTargetChange,

    // Entry/exit
    OnEnterField,
    OnEnterFieldAnyone,
    OnLeaveField,
    /// Fires when a Digimon is hatched from the breeding area into the
    /// battle area. Observer timing for the hatching player's permanents.
    OnHatch,

    // Cost/play modification
    BeforePayCost,
    WhenPlayedFromHand,

    // Digivolution
    OnDigivolve,
    OnDnaDigivolve,
    OnDigiXros,

    // ── Phase 7 "Would*" replacement timings ──────────────────────────────
    // Dispatched via Game::try_replace before the state change commits. See
    // replacement.rs and docs/superpowers/specs/2026-04-21-would-replacement-timings-design.md.
    WhenWouldBeDeleted,
    WhenWouldLeaveBattleArea,
    WhenWouldBeReturnedToHand,
    WhenWouldBeReturnedToDeck,
    WhenWouldBeTrashed,
    WhenWouldBeDeDigivolved,
    WhenWouldLoseSecurity,
    WhenWouldDraw,
    WhenWouldPlaceInSecurity,
    // Reserved — Phase 9 wires dispatch.
    WhenWouldAttack,
    WhenWouldBeAttackTarget,

    // Deletion observers
    /// Fires when any permanent is deleted for either player — covers
    /// battle DP-loss, effect-driven deletion, and security-check deletion.
    /// Global observer; card scripts filter by owner/opponent as needed.
    OnAnyDeletion,

    // Continuous / always active
    AlwaysActive,
    Declarative,

    // Option card timings
    OptionMain,
    OptionSecurity,

    // Phase 8 Option timings
    /// Global observer: fires when any Option card is played by any player.
    OnUseOption,

    /// Fires when an Option's delayed body resolves. Most printed Delays
    /// fire at end of owner's next turn; see DelayTrigger for triggers.
    DelayEffect,

    /// Global observer: fires AFTER a card is linked to a host Digimon.
    /// Mirrors DCGO `WhenLinked` (ICardEffect.cs:992). Required by
    /// Appmon-trait cards — BT21-053 (Syakomon), BT21-054, BT21-059,
    /// BT21-073, AD1-005 all listen on this timing for "when this Digimon
    /// gains a linked card" effects. The `OptionMain` body of the link
    /// card fires BEFORE `OnLink`; the observer runs after attach.
    OnLink,

    /// Observer: fires when a linked card is trashed from its host.
    /// Mirrors DCGO `OnLinkCardDiscarded` (ICardEffect.cs:996).
    OnLinkedCardTrashed,

    /// Observer: fires when a linked card is cleanly unlinked (removed from
    /// its host without being trashed — e.g. an effect that returns the
    /// linked card to hand or deck). Rust-engine-specific counterpart to
    /// `OnLinkedCardTrashed`; DCGO folds both paths into
    /// `OnLinkCardDiscarded` + explicit zone checks.
    OnUnlink,

    /// Observer: fires when a Training Option is trashed from the field.
    /// Rust-engine-specific timing — DCGO expresses the same hook via a
    /// generic on-trash predicate gated on `Training` state rather than a
    /// dedicated `EffectTiming` variant.
    OnTrainingTrash,

    // [Main] activated effects — zone-scoped variants. DCGO gates these via
    // `EffectTiming.OnDeclaration` + `CanUseCondition` zone checks; Python
    // reduces that to `_is_{hand,field,trash}_main` bool flags on the effect.
    // Rust promotes the zone distinction into the timing enum itself so
    // `effect.timing == MainFromHand` is the sole dispatch key for the mask
    // and decoder. See RUST_PYTHON_PARITY.md §4.5c.
    MainFromHand,
    MainOnField,
    MainFromTrash,

    // Archetype-specific observers
    /// Fires when an opponent's security card is removed from the stack
    /// (by security check or by effect). Medusamon core archetype observer.
    OnOpponentSecurityRemoved,
    /// Fires when a card is trashed from a permanent's digivolution stack
    /// (cost payment, source-displacement effects, etc.). Rocks core
    /// archetype observer.
    OnDigivolutionCardTrashed,

    // Special
    None,
}

/// When a Delay Option's body fires relative to the play. Most printed
/// cards use `EndOfYourNextTurn`; `EndOfThisTurn` is rare but present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DelayTrigger {
    EndOfYourNextTurn,
    EndOfThisTurn,
}

/// Keywords that can be granted or checked on permanents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Keyword {
    Blocker,
    SecurityAttackPlus(i8),
    SecurityAttackMinus(i8),
    Rush,
    Jamming,
    Piercing,
    Reboot,
    DeDigivolve(u8),
    DrawX(u8),
    Blitz,
    Armor,
    Raid,
    Alliance,
    Blast,
    Save,
    Fortitude,
    Overclock,
    Barrier,
    Decoy,
    Material,
    Partition,
    Vortex,
    /// Attacker keyword — while this Digimon is attacking, every opponent
    /// Digimon is treated as having Blocker. Consumed by
    /// `combat::try_enter_block`. Mirrors Python's `_is_collision`.
    Collision,

    // Phase 7 — replacement-backed keywords. Printed parsing lands Task 6.
    Evade,
    Fragment(u8),
    Decode,
    ArmorPurge,
}

/// Zone where a card can exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Zone {
    Hand,
    Deck,
    DigitamaDeck,
    Security,
    Trash,
    BattleArea,
    BreedingArea,
    CommanderZone,
    Reveal,
}

/// Modifier types for the modifier registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierType {
    // DP
    ChangeDp,
    ChangeBaseDp,
    DpFloor,
    DontHaveDp,

    // Cost
    ChangePlayCost,
    ChangeDigivolveCost,
    CannotReduceCost,

    // Destruction protection
    CannotBeDestroyed,
    CannotBeDestroyedByBattle,
    CannotBeDestroyedByEffect,
    CannotBeRemoved,

    // Movement protection (Phase 7 Task 5 — auto-install as replacements)
    CannotBeReturnedToDeck,
    CannotBeReturnedToHand,
    CannotBeTrashedByEffect,
    CannotBeDeDigivolved,

    // Attack
    CannotAttack,
    CannotAttackPlayer,
    CanAttackUnsuspended,
    CanAttackActivePlayer,
    CannotAttackTarget,

    // Suspend
    CannotSuspend,
    CannotUnsuspend,

    // Selection/targeting
    CannotBeSelectedByEffect,
    CannotBeAffected,

    // Keywords (granted via modifiers)
    GrantBlocker,
    GrantRush,
    GrantJamming,
    GrantPiercing,
    GrantReboot,
    GrantBlitz,
    GrantAlliance,
    GrantRaid,
    GrantBarrier,
    GrantArmor,
    GrantDecoy,
    GrantVortex,
    GrantOverclock,

    // End-of-turn attack grants. MayAttack permits one optional attack
    // during the EndOfTurnAction phase. ForceAttack mandates an attack
    // during the turn: it drives a global mask replacement in Main
    // (§4.7d) and also emits attack bits in EndOfTurnAction (§4.6c).
    MayAttack,
    ForceAttack,

    // Security
    SecurityAttackChange,

    // Digivolution
    CannotDigivolve,

    // Color
    ChangeColor,
    AddColor,

    // Level
    ChangeLevel,

    // Miscellaneous
    CannotReturnToHand,
    CannotTrash,
    CannotBlock,
    CannotCounter,
    DrawBlock,
    MemoryBlock,
    CannotPlayFromHand,

    // ── Phase 6 flood gates (player-scoped) ──────────────────────────────
    // Enforcement wires up in Tasks 3-4; for now these are pure data.
    CannotPlayDigimonByEffect,
    CannotGainMemoryByEffect,
    CannotGainMemoryExceptFromTamers,
    CannotReducePlayCost,
    CannotActivateMainEffects,
    CannotActivateWhenDigivolvingEffects,
    CannotActivateSecurityEffects,
    CannotDigivolveDigimonByEffect,
    CannotDrawByEffect,
    CannotAddSecurityByEffect,
    CannotTrashOpponentSecurity,
    CannotReduceOpponentSecurity,
    #[allow(dead_code)]
    IgnoreColorRequirement,
}

/// When a modifier expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Expiry {
    Permanent,
    EndOfTurn,
    EndOfOpponentsTurn,
    EndOfAttack,
    EndOfBattle,
    UntilLeaveField,
}

/// How first-turn draw skip works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkipDraw {
    /// Standard: the first player (turn_count == 1, turn_player == 0) skips
    /// their draw. All subsequent turns, including the second player's first
    /// turn (turn_count == 2), draw normally. Matches Python's
    /// `if self.turn_count == 1: pass` rule.
    FirstPlayerOnly,
    /// EDH: every player skips draw on their first turn (turn_count <= player_count).
    AllRound1,
    /// No skip
    None,
}

/// Format / game-mode selector. Mirrors Python `digimon_gym.engine.data.enums.GameMode`.
/// Maps to a `Rules` preset via `Rules::for_mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameMode {
    Standard,
    NoRestriction,
    EdhCommander,
    Titan,
}

/// Role-within-format for asymmetric formats. Currently only Titan uses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TitanRole {
    Boss,
    Team,
}

/// Why a card is being played — distinguishes player-driven hand plays from
/// effect-initiated plays. Threaded through play and digivolve helpers so
/// flood-gate modifiers (e.g. `CannotPlayDigimonByEffect`) can inspect the
/// source at enforcement time (Task 4).
///
/// - `ByHand` — the turn player spent memory for the printed cost.
/// - `ByEffect` — another effect triggered this play (free or effect-paid).
/// - `ByDigivolve` — digivolving onto a pre-digi (not strictly "play", but
///   relevant for some flood gates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlaySource {
    ByHand,
    ByEffect,
    ByDigivolve,
}

/// How a play-from-zone helper should compute the memory cost deducted.
///
/// - `Free` — pay 0 memory regardless of printed cost. Used by "play without
///   paying its cost" effects.
/// - `Reduce(n)` — pay max(0, printed_cost - n). Used by "play with cost
///   reduced by n" effects. Negative reductions (cost increases) are allowed.
/// - `Fixed(n)` — pay exactly n regardless of printed cost. Used by the rare
///   "play for exactly n memory" effects. Negative values clamp to 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CostDelta {
    Free,
    Reduce(i16),
    Fixed(i16),
}

impl CostDelta {
    /// Resolve the concrete memory cost to deduct given a printed cost.
    pub fn resolve(self, printed_cost: u16) -> u16 {
        match self {
            CostDelta::Free => 0,
            CostDelta::Reduce(n) => {
                let reduced = printed_cost as i32 - n as i32;
                reduced.max(0) as u16
            }
            CostDelta::Fixed(n) => n.max(0) as u16,
        }
    }
}

/// Placement position when moving a card to the deck, security stack, or
/// digivolution source stack. `Random` shuffles the single card into a
/// random index — used by "shuffle into the deck" effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StackPosition {
    Top,
    Bottom,
    Random,
}

/// Where a card originates from for `place_as_bottom_source` and similar
/// cross-zone moves. Named `Ref` because it indexes a live zone; the
/// caller must ensure the index/handle is valid at call time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardSourceRef {
    Hand(PlayerId, usize),
    Trash(PlayerId, usize),
    DeckTop(PlayerId),
    Reveal(CardHandle),
}

#[cfg(test)]
mod cost_delta_tests {
    use super::*;

    #[test]
    fn free_is_always_zero() {
        assert_eq!(CostDelta::Free.resolve(0), 0);
        assert_eq!(CostDelta::Free.resolve(12), 0);
    }

    #[test]
    fn reduce_subtracts() {
        assert_eq!(CostDelta::Reduce(3).resolve(10), 7);
        assert_eq!(CostDelta::Reduce(0).resolve(10), 10);
    }

    #[test]
    fn reduce_clamps_at_zero() {
        assert_eq!(CostDelta::Reduce(100).resolve(10), 0);
    }

    #[test]
    fn reduce_negative_increases_cost() {
        assert_eq!(CostDelta::Reduce(-2).resolve(10), 12);
    }

    #[test]
    fn fixed_replaces_cost() {
        assert_eq!(CostDelta::Fixed(4).resolve(10), 4);
        assert_eq!(CostDelta::Fixed(0).resolve(10), 0);
    }

    #[test]
    fn fixed_clamps_at_zero() {
        assert_eq!(CostDelta::Fixed(-3).resolve(10), 0);
    }
}
