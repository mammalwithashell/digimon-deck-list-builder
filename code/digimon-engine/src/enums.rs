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
    Dual,
}

/// Rules-facing source classification for a resolving effect.
///
/// This is intentionally distinct from activation zone/timing. A Digimon
/// security effect is still a Digimon effect, and a DUAL card is an Option
/// effect while being used as an Option but a Digimon effect once it is on a
/// Digimon stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectSourceKind {
    Digimon,
    Tamer,
    Option,
    Rule,
}

/// Card rarity.
///
/// Variant order mirrors Python's `Rarity` enum in
/// `digimon_gym/engine/data/enums.py` and the ingester's `RARITY_MAP`:
/// C=0, U=1, R=2, SR=3, SEC=4, P=5, NoRarity=6.
/// `u8::MAX` is reserved by deck-data deserializers as the sentinel for
/// "missing rarity"; it should be mapped to `NoRarity` before calling
/// `from_u8` so invalid numeric rarity values can fail fast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rarity {
    C = 0,
    U = 1,
    R = 2,
    SR = 3,
    SEC = 4,
    P = 5,
    NoRarity = 6,
}

impl Rarity {
    pub fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::C),
            1 => Some(Self::U),
            2 => Some(Self::R),
            3 => Some(Self::SR),
            4 => Some(Self::SEC),
            5 => Some(Self::P),
            6 => Some(Self::NoRarity),
            _ => None,
        }
    }

    pub const fn mask(self) -> u8 {
        1u8 << (self as u8)
    }

    pub const fn is_in_mask(self, mask: u8) -> bool {
        mask & self.mask() != 0
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::C => "C",
            Self::U => "U",
            Self::R => "R",
            Self::SR => "SR",
            Self::SEC => "SEC",
            Self::P => "P",
            Self::NoRarity => "NoRarity",
        }
    }
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
    SelectBreedingPermanent,
}

impl GamePhase {
    /// Python-parity phase name. Use this when serializing phase into a
    /// string field that will be compared against Python's `GamePhase.name`.
    pub fn py_name(&self) -> &'static str {
        match self {
            GamePhase::Mulligan => "Mulligan",
            GamePhase::Unsuspend => "Start", // Python enum name
            GamePhase::Draw => "Draw",
            GamePhase::Breeding => "Breeding",
            GamePhase::Main => "Main",
            GamePhase::EndTurn => "End", // Python enum name
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
            GamePhase::SelectBreedingPermanent => "SelectBreedingPermanent",
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
    /// Fires on a security card that is trashed from the security stack by
    /// an effect, distinct from normal attack security checks.
    OnDiscardSecurity,
    CounterEffect,

    // Turn-based
    StartOfYourTurn,
    StartOfOpponentsTurn,
    /// Fires at the start of the controller's Main phase (after Draw).
    /// Card scripts can use this for "at the start of your main phase" effects.
    StartOfYourMainPhase,
    EndOfYourTurn,
    EndOfOpponentsTurn,
    EndOfYourNextTurn,
    EndOfOpponentsNextTurn,
    UntilNextUnsuspend,
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
    /// Fires when a permanent is played by the observer's controller.
    /// Carries the played permanent/card as event context and scans only the
    /// playing player's observer zones.
    OnAllyPlayed,
    OnLeaveField,
    /// Fires when a Digimon is hatched from the breeding area into the
    /// battle area. Observer timing for the hatching player's permanents.
    OnHatch,
    /// Fires when a breeding-area Digimon moves into the battle area.
    /// Observer timing for the moving player's battle area.
    OnMove,

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
    /// Fires before a permanent digivolves into another card.
    WhenPermanentWouldDigivolve,
    /// Fires before a permanent enters play.
    WhenPermanentWouldPlay,
    /// Fires before a Plug-In/Link option attaches to a carrier.
    WhenWouldLink,
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

    /// Global observer: fires after a persistent Option card is placed into
    /// the battle area and has a stable permanent/card handle.
    OnOptionPlaced,

    /// Global observer: fires after a persistent Option card is trashed via
    /// the explicit Option lifecycle API.
    OnOptionTrashed,

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
    /// Fires when one of your security cards is removed from the stack
    /// (by security check or by effect). Mirrors cards such as BT4-097
    /// Kari Kamiya that watch their controller's own security.
    OnOwnSecurityRemoved,
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
    StartOfYourNextTurn,
    OnEvent(EffectTiming),
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
    Raid,
    Alliance,
    BlastDigivolve,
    Save,
    /// DCGO `MaterialSave N` — active skill that moves up to N digivolution
    /// sources under another permanent. Parsed from `<Material Save N>`.
    /// Auto-install wires up in Phase D; the variant exists now so parser
    /// and script authors can carry the parameter.
    MaterialSave(u8),
    /// Printed `<Digi-Burst N>` — an active effect cost marker. The keyword
    /// parser carries the parameter; card bodies still author the "effect
    /// below" through DSL `digi_burst` or equivalent scripts.
    DigiBurst(u8),
    Fortitude,
    Overclock,
    Barrier,
    /// Printed `<Decoy>` (RULES_CONTEXT 16-XX). The carrier may substitute
    /// itself for an ally Digimon's deletion. The `u8` payload encodes the
    /// printed color filter as a bitmask over `CardColor` (bit `n` set ⇒
    /// allies of color `n` are eligible). `0` means no color filter — every
    /// ally Digimon is eligible (matches the un-parameterised printing).
    /// Multi-color filters (e.g. `<Decoy (Red/Black)>`) OR the bits.
    ///
    /// **Trait filters NOT encoded here.** Printed forms such as
    /// `<Decoy ([Bagra Army] trait)>` parse to `Decoy(0)` (no color filter)
    /// and require a hand-rolled `CardEffect` override to apply the trait
    /// gate. See `RUST_ENGINE_GAPS.md` "Decoy color-filter parameterisation"
    /// for the trait-filter follow-up entry.
    Decoy(u8),
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

    /// Progress — while the attacker has Progress, the defender's
    /// `SecuritySkill` effects do not fire on reveal. Consumed by the
    /// `SecuritySkillDrain` arm of `drive_security_resolution` (§2.5c).
    Progress,

    /// DCGO `Retaliation` — when this Digimon is deleted in battle, delete
    /// the battled opponent Digimon. Mandatory trigger (RULES_CONTEXT 16-12).
    /// Wire-up Phase E §E1 — auto-installed `OnDeletion` trigger gated on
    /// `deletion_cause() == Some(Battle)` that calls
    /// `ctx.battle_opponent_of(self)` to find the winner.
    Retaliation,

    /// DCGO `Scapegoat` — when this Digimon would be deleted by anything
    /// other than its own controller's effect, optionally delete another of
    /// the controller's permanents to cancel the deletion. Wire-up Phase E
    /// §E2 — auto-installed `WhenWouldBeDeleted` substitute replacement.
    /// RULES_CONTEXT 16-31.
    Scapegoat,

    /// DCGO `Execute` — at end of your turn, this Digimon may attack
    /// (including unsuspended Digimon); when the attack ends, this
    /// Digimon is deleted. Trigger-type, Optional (RULES_CONTEXT 16-37).
    /// Wire-up Phase F §F1 — auto-installed `EndOfYourTurn` triggered
    /// effect that grants `MayAttack` + `CanAttackUnsuspended` for the
    /// upcoming attack window and queues an `EndOfAttack` self-deletion.
    Execute,

    /// DCGO `Iceclad` — passive (RULES_CONTEXT 16-34). When this Digimon
    /// is in a Digimon-vs-Digimon battle (NOT a security-Digimon battle),
    /// compare digivolution-card count instead of DP. Higher count wins;
    /// equal count = mutual destruction. No `keyword_to_auto_effect` arm —
    /// consumed directly in `combat::resolve_battle` via `has_keyword`.
    Iceclad,

    /// DCGO `MindLink` — active skill on Tamers (RULES_CONTEXT 16-27).
    /// `[Main]` activation: place this Tamer at the bottom of an own
    /// Digimon's digivolution stack. Target Digimon must have NO Tamer
    /// digivolution sources (DCGO `cardSource.IsTamer && !cardSource.IsFlipped`
    /// — face-down Tamer sources do NOT count, hence the `face_down` field
    /// on `CardSource`). Mandatory processing; optional timing under `[Main]`.
    /// Wire-up Phase F §F3.
    MindLink,

    /// DCGO `Training` — active skill (RULES_CONTEXT 16-40). `[Main]`
    /// activation usable from battle area OR breeding area: suspend self
    /// (cost) + place top deck card under self at stack bottom, face-down.
    /// Cost requires `is_suspended == false`. Wire-up Phase F §F4.
    Training,

    /// Arts Digivolve — DUAL Option-use keyword that offers an optional
    /// digivolution of the used Option card onto a legal Digimon target.
    ArtsDigivolve,
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
    VortexCanAttackPlayer,
    CanAttackUnsuspended,
    CanAttackActivePlayer,
    CannotAttackTarget,
    CannotBeRedirectedAsAttackTarget,
    CanNotSwitchAttackTarget,

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
    /// Modifier-granted form of `Keyword::Progress` — used when the
    /// immunity comes from a temporary effect rather than printed text.
    /// Checked alongside native `Keyword::Progress` by the SecuritySkill
    /// drain gate (§2.5c).
    ImmunityToOpponentEffects,
    /// Attacker skips the Digimon-vs-security DP battle entirely. Consumed
    /// by the `BattleResolved` arm of `drive_security_resolution` (§2.5d).
    DontBattleSecurityDigimon,

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
    /// Prevents the affected player from playing Tamer cards via effects
    /// (regardless of zone: hand or trash). Companion to `CannotPlayDigimonByEffect`.
    /// Checked in `play_from_hand_with_cost` and `play_from_trash_with_cost` when
    /// `source == PlaySource::ByEffect && card_kind == CardKind::Tamer`.
    CannotPlayTamerByEffect,
    CannotGainMemoryByEffect,
    CannotGainMemoryExceptFromTamers,
    CannotPlayFromTrash,
    CannotReducePlayCost,
    CannotReduceDigivolveCost,
    OpponentCannotReduceDigivolveCost,
    CannotActivateMainEffects,
    CannotActivateWhenDigivolvingEffects,
    CannotActivateWhenAttackingEffects,
    CannotActivateSecurityEffects,
    CannotDigivolveDigimonByEffect,
    CannotDrawByEffect,
    CannotAddSecurityByEffect,
    CannotTrashOpponentSecurity,
    CannotReduceOpponentSecurity,
    IgnoreColorRequirement,

    // ── Track C taxonomy completion (2026-05-06) ─────────────────────────
    // Cross-track foundation publishes these symbols early so Tracks B / D /
    // G / H / I / J can read them as they wire enforcement at their consult
    // sites. See `docs/RUST_ENGINE_API.md` §"Modifier consult-site checklist"
    // for the per-variant mutation paths.

    // Combat / attack-flow modifiers
    /// Player-scoped: this player's Digimon may attack the opposing player
    /// but not opposing Digimon. Companion to `CannotAttackPlayer`.
    /// Consult site: `combat::begin_attack` target legality (Track D).
    MayAttackPlayerOnly,
    /// Permanent-scoped: blocks the breeding-area → battle move and any
    /// effect-driven inter-zone move on this Digimon. Distinct from
    /// `CannotSuspend` (which only blocks orientation flips).
    /// Consult site: `move_from_breeding`, effect-driven move helpers.
    CannotMove,
    /// Permanent-scoped: this attacker cannot have its declared target
    /// switched mid-attack (Block / Raid / Counter retarget all skip).
    /// Consult site: `combat::redirect_attack_target` (Track D).
    CannotSwitchAttackTarget,
    /// Permanent-scoped affirmative inverse of `CannotAttackTarget` —
    /// "this Digimon CAN attack the otherwise-illegal defending permanent
    /// (e.g. an unsuspended Digimon)". Stack-overrides the negative.
    /// Consult site: `combat::is_legal_attack_target` (Track D).
    CanAttackTargetDefendingPermanent,

    // Memory / security gates
    /// Permanent-scoped: while this permanent is on the field, the
    /// controller's effects can't add memory (printed-text "your effects
    /// can't add memory" anchored to a specific Digimon). Distinct from
    /// player-scoped `CannotGainMemoryByEffect`.
    /// Consult site: `Game::adjust_memory_by_effect`.
    CannotAddMemory,
    /// Permanent-scoped: while this permanent is on the field, the
    /// controller's effects can't add security cards. Distinct from
    /// player-scoped `CannotAddSecurityByEffect`.
    /// Consult site: `Game::add_security_by_effect`.
    CannotAddSecurity,
    /// Player-scoped or permanent-scoped: replaces the end-of-turn
    /// "memory clamps to zero" rule with a different floor (e.g.
    /// "at end of turn, set memory to 1 if it would be 0"). The
    /// `value` field carries the new floor as a signed delta from
    /// the standard rule (default rule: clamp to 0).
    /// Consult site: `Game::end_turn` memory adjustment (Track D / E).
    ChangeEndTurnMinMemory,

    // Narrow protection (printed-text exact)
    /// Permanent-scoped: this Digimon's DP cannot be reduced by negative
    /// `ChangeDp` modifiers from any (or filtered) cause. Narrower than
    /// `CannotBeAffected` — only DP-minus is blocked. Honors
    /// `effect_immunity_filter` for opponent-only scope.
    /// Consult site: `Permanent::dp_modifier_apply` / DP calculation.
    ImmuneFromDPMinus,
    /// Permanent-scoped: this Digimon's digivolution-stack sources cannot
    /// be trashed (peeled) by effects. Distinct from `CannotBeDestroyed`,
    /// which protects the top card. Consult site: source-trash mutation
    /// (the inherited stack-peel path).
    ImmuneFromStackTrashing,

    // Effect-suppression
    /// Permanent-scoped: suppresses dispatch of one specific timing's
    /// observers on this permanent. The suppressed timing parameter is
    /// not encoded in the variant (which would explode the enum); it is
    /// stored on `ModifierEntry::disable_effect_timing` instead. Use
    /// `ModifierEntry::disable_effect(...)` constructor and read the
    /// timing via the entry. Consult site: Track A's observer dispatch
    /// before firing per-permanent observers.
    DisableEffect,

    // Identity / treat-as
    /// Permanent-scoped: this Tamer permanent is treated as a Digimon
    /// for predicates that gate on "Digimon" (e.g. attack-target legality,
    /// archetype counts). Consult site: `Permanent::is_digimon_for_rules`
    /// helper used by attack legality and archetype predicates.
    TreatAsDigimon,

    // ── DP / cost / metadata scaling (Track H aura territory) ────────────
    // Variants published here so the aura system can deliver them; many
    // overlap with existing `ChangeDp` / `ChangeBaseDp` / `ChangePlayCost`
    // but have narrower printed-text semantics that auras need to express.
    /// Like `ChangeDp` but applies to the topmost card's printed DP
    /// rather than the live permanent's DP — used by "this card's DP
    /// becomes X" effects that ignore stack DP.
    ChangeCardDP,
    /// Like `ChangeBaseDp` but applies to the bottom-of-stack origin
    /// (the digi-egg) — used by hatching-DP modifiers.
    ChangeOriginDP,
    /// Modifies the printed Security Attack value (`<Security Attack +N>`
    /// keyword count adjustment from external sources).
    ChangeSAttack,
    /// Modifies the link cost when this card is the host. Track Link's
    /// territory; published here for the aura system.
    ChangeLinkCost,
    /// Modifies the maximum number of linked cards on this host.
    ChangeLinkMax,
    /// Overrides the live permanent's level for predicates and digivolution
    /// requirements. Distinct from `ChangeLevel` which adjusts level by
    /// a delta on the printed value.
    ChangePermanentLevel,
    /// Adds or removes printed traits on this permanent — printed-text
    /// "this Digimon also has the [TRAIT] trait" effects.
    ChangeTraits,
    /// Treats this permanent as if its top card had a different printed
    /// name — used by "this Digimon is also named X" effects.
    ChangeBaseCardName,
    /// Treats this permanent as if its top card had a different printed
    /// color — distinct from `AddColor` which appends rather than replaces.
    ChangeBaseCardColor,
    /// Adjusts the level used when this card is being assembled (DigiXros
    /// material-level computation). Track Xros's territory.
    ChangeCardLevelForAssembly,
    /// Adds names this card is treated as for DigiXros material matching
    /// (printed-text "treat this card as named X for DigiXros").
    ChangeCardNamesForDigiXros,
}

/// When a modifier expires.
///
/// Lifecycle taxonomy used by the modifier registry. The expiry decides
/// when `ModifierRegistry::expire_*` removes the entry, and (for
/// `UntilCondition`) when the continuous controller evicts it after a
/// condition transition.
///
/// Variants:
/// - `Permanent` — never expires until the source is removed.
/// - `EndOfTurn` — removed at every turn-end regardless of whose turn.
/// - `EndOfOpponentsTurn` — removed at the end of an opponent's turn
///   (i.e. `ending_player != source_player`).
/// - `EndOfYourTurn` — removed at the end of the source player's own
///   turn (i.e. `ending_player == source_player`). Mirror of
///   `EndOfOpponentsTurn`. Use this for "until the end of your next turn"
///   effects scripted from the source player's POV.
/// - `EndOfAttack` — removed at the end of the current attack flow.
/// - `EndOfBattle` — removed at the end of the current battle resolution.
/// - `UntilLeaveField` — removed when the source permanent leaves the
///   battle area (consumes the leave-field event).
/// - `UntilCondition` — re-evaluated by the continuous controller; the
///   modifier is active while a per-entry boolean predicate holds. The
///   predicate itself lives on `ModifierEntry::until_condition` /
///   `PlayerModifierEntry::until_condition` so the `Expiry` enum stays
///   `Copy + Eq + Hash`. When the predicate transitions false, the
///   installed entry is removed and is not restored if the predicate later
///   becomes true again.
/// - `OnceUsed(u32)` — the value is the limit; the entry tracks
///   `uses_consumed` and is removed once `uses_consumed >= limit`. Use
///   `ModifierRegistry::consume_use` from the consult site to advance
///   the counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Expiry {
    Permanent,
    EndOfTurn,
    EndOfOpponentsTurn,
    EndOfYourTurn,
    EndOfAttack,
    EndOfBattle,
    UntilLeaveField,
    UntilCondition,
    OnceUsed(u32),
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
    Pauper,
    NoRestriction,
    Eden,
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
    Security(PlayerId, usize),
    Material(crate::permanent::PermanentHandle, usize),
    Reveal(CardHandle),
}

#[cfg(test)]
mod rarity_tests {
    use super::*;

    #[test]
    fn rarity_from_u8_rejects_unknown_values() {
        assert_eq!(Rarity::from_u8(0), Some(Rarity::C));
        assert_eq!(Rarity::from_u8(1), Some(Rarity::U));
        assert_eq!(Rarity::from_u8(6), Some(Rarity::NoRarity));
        assert_eq!(Rarity::from_u8(u8::MAX), None);
        assert_eq!(Rarity::from_u8(42), None);
    }

    #[test]
    fn rarity_masks_are_distinct() {
        let pauper_mask = Rarity::C.mask() | Rarity::U.mask();
        assert!(Rarity::C.is_in_mask(pauper_mask));
        assert!(Rarity::U.is_in_mask(pauper_mask));
        assert!(!Rarity::R.is_in_mask(pauper_mask));
        assert!(!Rarity::NoRarity.is_in_mask(pauper_mask));
    }
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
