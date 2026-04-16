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
}

/// Card color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardColor {
    Red,
    Blue,
    Yellow,
    Green,
    Black,
    Purple,
    White,
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
    SecurityEffect,
    CounterEffect,

    // Turn-based
    StartOfYourTurn,
    StartOfOpponentsTurn,
    EndOfYourTurn,
    EndOfOpponentsTurn,
    EndOfAttack,

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

    // Entry/exit
    OnEnterField,
    OnEnterFieldAnyone,
    OnLeaveField,

    // Cost/play modification
    BeforePayCost,
    WhenPlayedFromHand,

    // Digivolution
    OnDigivolve,
    OnDnaDigivolve,
    OnDigiXros,

    // Continuous / always active
    AlwaysActive,
    Declarative,

    // Option card timings
    OptionMain,
    OptionSecurity,

    // Special
    None,
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

    // Attack
    CannotAttack,
    CannotAttackPlayer,
    CanAttackUnsuspended,
    CanAttackActivePlayer,

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
