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

    // [Main] activated effects — zone-scoped variants. DCGO gates these via
    // `EffectTiming.OnDeclaration` + `CanUseCondition` zone checks; Python
    // reduces that to `_is_{hand,field,trash}_main` bool flags on the effect.
    // Rust promotes the zone distinction into the timing enum itself so
    // `effect.timing == MainFromHand` is the sole dispatch key for the mask
    // and decoder. See RUST_PYTHON_PARITY.md §4.5c.
    MainFromHand,
    MainOnField,
    MainFromTrash,

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
    /// Attacker keyword — while this Digimon is attacking, every opponent
    /// Digimon is treated as having Blocker. Consumed by
    /// `combat::try_enter_block`. Mirrors Python's `_is_collision`.
    Collision,
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
