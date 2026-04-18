//! Pending selection + effect queue types.
//!
//! When a card effect needs a player choice (target a Digimon, pick a hand
//! card, declare a blocker, choose resolution order for simultaneous triggers),
//! it parks a `PendingSelection` on `Game`. The engine flips `current_phase`
//! to the matching selection phase, the action mask renders only
//! `valid_action_ids`, and the next `step()` call resolves the selection by
//! invoking the stored callback.
//!
//! Callbacks capture the `Copy` handles they need (`CardHandle`,
//! `PermanentHandle`, `PlayerId`). No lifetimes bleed into the struct.
//!
//! See `docs/RUST_PYTHON_PARITY.md` §2.3 / §3.5 / §4.6 for motivation and
//! the accompanying plan file for the drainer design.

use std::collections::VecDeque;

use crate::card_source::{CardHandle, CardSource};
use crate::enums::{EffectTiming, GamePhase, PlayerId};
use crate::permanent::PermanentHandle;

/// Called when a selection resolves with a concrete action ID.
pub type SelectionCallback =
    Box<dyn FnOnce(&mut crate::game::Game, u16) + Send + Sync + 'static>;

/// Called when an optional selection is declined via PASS.
pub type DeclineCallback =
    Box<dyn FnOnce(&mut crate::game::Game) + Send + Sync + 'static>;

/// Taxonomy of selection prompts. Mirrors the Python `PendingSelection.kind`
/// tag — decoders use this plus `previous_phase` to route action IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectionKind {
    /// Pick an enemy (or unspecified-side) Digimon on the field.
    Target,
    /// Pick a Digimon/Tamer on the caller's own field.
    OwnField,
    /// Pick a Digimon on the opponent's field.
    OppField,
    /// Pick a card from a player's hand.
    Hand,
    /// Pick a card from a player's trash.
    Trash,
    /// Pick a material (digivolution source) for DNA / material effects.
    Material,
    /// Pick one of the cards currently exposed in `Game.revealed_cards`.
    Reveal,
    /// Pick a security stack slot.
    Security,
    /// Pick one of several labeled effect-text branches ("choose one").
    EffectChoice,
    /// Pick a specific source card in a permanent's digivolution stack.
    Source,
    /// Pick which of several pending triggered effects to resolve next.
    /// Installed by the effect queue drainer when a single controller has
    /// two or more triggers queued at the same timing (Digimon rules let
    /// the controller order their own simultaneous triggers).
    TriggerOrder,
}

/// One branch of a `SelectionKind::EffectChoice` prompt.
#[derive(Debug, Clone)]
pub struct EffectChoiceEntry {
    pub label: String,
    pub action_id: u16,
}

/// A parked selection. Installed by `EffectContext::select_*` helpers and the
/// queue drainer. Resolved by `Game::resolve_selection(player, action_id)`.
pub struct PendingSelection {
    pub kind: SelectionKind,
    /// The player whose turn it is to make this choice. The decoder rejects
    /// actions from any other player; the mask only emits bits when the
    /// observing player matches.
    pub selecting_player: PlayerId,
    /// Phase to restore after the callback runs (and installs no new
    /// selection). For `TriggerOrder`, this is the phase the drainer was
    /// running in — typically whichever phase triggered the effect batch.
    pub previous_phase: GamePhase,
    /// Every action ID the decoder will accept for this selection, in the
    /// order the effect wants them presented. The action mask uses this
    /// directly; no re-computation at resolution time.
    pub valid_action_ids: Vec<u16>,
    /// If true, `PASS` (action 62) is also legal. For `TriggerOrder` the
    /// semantics are "decline all remaining optional triggers controlled by
    /// `selecting_player`"; for other kinds it's a single-prompt decline.
    pub is_optional: bool,
    /// Human-readable prompt. Trace/debug only — not consumed by the engine.
    pub prompt: String,
    /// When `kind == EffectChoice`, the branches presented to the player.
    /// Each entry's `action_id` must also appear in `valid_action_ids`.
    pub effect_choices: Option<Vec<EffectChoiceEntry>>,
    /// Card that installed this selection (for provenance / debugging).
    pub source_card: CardHandle,
    /// Permanent that installed this selection, if any.
    pub source_permanent: Option<PermanentHandle>,
    /// Fired with the chosen action ID. Runs exactly once.
    pub callback: SelectionCallback,
    /// Fired instead of `callback` when the player passes on an optional
    /// selection. Runs exactly once.
    pub on_decline: Option<DeclineCallback>,
}

impl std::fmt::Debug for PendingSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingSelection")
            .field("kind", &self.kind)
            .field("selecting_player", &self.selecting_player)
            .field("previous_phase", &self.previous_phase)
            .field("valid_action_ids", &self.valid_action_ids)
            .field("is_optional", &self.is_optional)
            .field("prompt", &self.prompt)
            .field("effect_choices", &self.effect_choices)
            .field("source_card", &self.source_card)
            .field("source_permanent", &self.source_permanent)
            .finish_non_exhaustive()
    }
}

/// A triggered effect waiting to resolve. The queue holds these; the drainer
/// picks one at a time, respecting turn-player-bundle-first ordering and
/// giving the controller a `TriggerOrder` prompt whenever their bundle
/// contains more than one entry.
///
/// `effect_slot` is an index into the ordered `Vec<Effect>` returned by the
/// source card's `CardEffect::effects(handle)` — the drainer re-looks-up
/// the effect at run time rather than storing a `ProcessFn`, which would
/// complicate `Send`/`Sync` reasoning around the queue. `card_id` is carried
/// alongside so re-lookup doesn't have to scan zones for a handle.
#[derive(Debug, Clone)]
pub struct QueuedEffect {
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub controller: PlayerId,
    pub timing: EffectTiming,
    pub effect_slot: u8,
    pub is_optional: bool,
    pub is_turn_player: bool,
    /// Card ID string, carried so the drainer can re-look-up the effect
    /// from the registry without scanning zones for a matching `CardHandle`.
    pub card_id: String,
}

/// Describes where a trigger is firing from. Consumed by
/// `Game::enqueue_triggered` to decide which zones/permanents to scan.
#[derive(Debug, Clone, Copy)]
pub enum TriggerSource {
    /// A single permanent fires the trigger (OnPlay, OnAttack, OnDeletion).
    /// Only that permanent's own effects at the given timing are collected.
    Permanent(PermanentHandle),
    /// Every permanent in the player's battle area fires the trigger
    /// (EndOfYourTurn, StartOfYourTurn, etc.). Effects are collected in
    /// battle-area order; the player is the controller of every entry.
    PlayerBattleArea(PlayerId),
    /// A card revealed from security is firing `SecuritySkill` effects. The
    /// defender is the controller of the triggers; the card itself lives in
    /// `Game.pending_security` during resolution (it has been popped off the
    /// security stack but not yet trashed). Only effects whose `timing` and
    /// `security` flag match `SecuritySkill + security=true` are collected —
    /// mirroring Python's `is_security_effect` filter.
    SecurityRevealed {
        defender: PlayerId,
        card: CardHandle,
    },
    /// Global observer timing fired after a security card's own
    /// `SecuritySkill` effects resolve and the Digimon battle (if any) has
    /// been decided. Scans every permanent in the defender's battle area for
    /// `OnSecurityCheck`-timed effects. Mirrors Python's
    /// `EffectTiming.OnSecurityCheck` fire site in `combat.py:206-214`
    /// (RUST_PYTHON_PARITY §2.5b).
    OnSecurityCheck {
        attacker: PermanentHandle,
        defender: PlayerId,
        revealed_card: CardHandle,
        was_face_up: bool,
    },
}

/// Phase of an in-flight security-card resolution. Drives the
/// `drive_security_resolution` state machine in `combat.rs`. Order matches
/// Python's `_execute_security_checks` sequence. See RUST_PYTHON_PARITY
/// §2.5b / §2.5j.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityPhase {
    SecuritySkillDrain,
    BattleResolved,
    OnSecurityCheckDrain,
    OnLoseSecurityDrain,
    Dispose,
}

/// Mid-security-check state. Parks `drive_security_resolution` across
/// `pending_selection` pauses so `Game::advance_security_resolution` can
/// resume cleanly. See RUST_PYTHON_PARITY §2.5j.
#[derive(Debug, Clone)]
pub struct SecurityResolutionState {
    pub attacker: Option<PermanentHandle>,
    pub defender: PlayerId,
    pub turn_player: PlayerId,
    pub revealed_card: CardHandle,
    pub card_kind: crate::enums::CardKind,
    pub was_face_up: bool,
    pub phase: SecurityPhase,
    pub checks_remaining: u8,
    pub outcome_so_far: crate::combat::AttackResult,
}

/// Snapshot of the most recently revealed security card. Consumed by
/// `OnSecurityCheck` observer effects. RUST_PYTHON_PARITY §2.5l.
#[derive(Debug, Clone, Copy)]
pub struct SecurityRevealSnapshot {
    pub card: CardHandle,
    pub was_face_up: bool,
}

/// Transient per-security-check state. Lives on `Game` from the moment the
/// defender's security card is popped until the check finishes (either the
/// card is trashed or an effect plays it from security).
///
/// The `played` bit is raised by `EffectContext::play_from_security` — it
/// signals the security-resolution loop that the card is now a Permanent on
/// the field, and must NOT be trashed at the end of the check.
#[derive(Debug, Clone)]
pub struct PendingSecurity {
    pub defender: PlayerId,
    pub card: CardSource,
    pub played: bool,
}

/// Mid-attack state. Held separately from the effect queue because interrupt
/// windows (Alliance / Counter / Block) prompt *choices*, not *effects* — the
/// state machine advances through them until `Battle` resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackState {
    /// Just declared; `OnAttack` effects about to resolve, then Alliance.
    Declared,
    /// Waiting for the attacker's controller to pick an ally to suspend
    /// (or decline).
    AllianceOpen,
    /// Waiting for the defender to counter via blast-digivolve / Counter
    /// effect (or decline).
    CounterOpen,
    /// Waiting for the defender to declare a blocker (or decline).
    BlockOpen,
    /// No further interrupts pending. `resolve_pending_battle` will fire.
    Battle,
    /// Post-battle: clear EndOfAttack modifiers, fire EndOfAttack triggers.
    Cleanup,
}

/// The target of an attack. Counter effects can redirect by rewriting the
/// `effective_target` while leaving `original_target` intact for audits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackTarget {
    Digimon(PermanentHandle),
    Player(PlayerId),
}

/// In-flight attack. Set by `begin_attack`, advanced by
/// `advance_pending_attack`, cleared by `cleanup_attack`.
#[derive(Debug, Clone)]
pub struct PendingAttack {
    pub attacker: PermanentHandle,
    pub original_target: AttackTarget,
    pub effective_target: AttackTarget,
    pub is_blocked: bool,
    pub blocker: Option<PermanentHandle>,
    pub is_vortex: bool,
    /// Set when this attack was initiated by `<Overclock>` — a sacrifice has
    /// already been paid, and the attacker does NOT suspend on declaration.
    /// Interrupts (Alliance / Counter / Block) still fire normally; only the
    /// suspend-on-declare step is suppressed. Matches Python
    /// `resolve_attack(..., without_suspend=True)`.
    pub is_overclock: bool,
    /// Phase to return to once the attack finishes (`Main` for normal
    /// attacks, `EndOfTurnAction` for end-of-turn vortex attacks, etc.).
    pub return_phase: GamePhase,
    pub state: AttackState,
}

/// Helper type alias so callers can spell `Game.effect_queue` without
/// importing `std::collections::VecDeque` every time.
pub type EffectQueue = VecDeque<QueuedEffect>;

/// Reasons `Game::resolve_selection` can fail. Exposed so callers (Tauri
/// commands, RL step wrappers, unit tests) can distinguish "invalid action"
/// from "no prompt was pending" from "wrong player answered".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionError {
    /// No `pending_selection` was installed.
    NoPendingSelection,
    /// A different player is expected to answer this prompt.
    WrongPlayer,
    /// `action_id` is not in `valid_action_ids` and is not a valid PASS.
    InvalidAction,
    /// Selection plumbing exists but the resolver has not landed yet.
    /// Temporary — removed when PR3 lands the full decoder.
    NotYetImplemented,
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPendingSelection => write!(f, "no pending selection to resolve"),
            Self::WrongPlayer => write!(f, "wrong player for this selection"),
            Self::InvalidAction => write!(f, "action is not in valid_action_ids"),
            Self::NotYetImplemented => {
                write!(f, "selection resolution is not yet implemented")
            }
        }
    }
}

impl std::error::Error for SelectionError {}
