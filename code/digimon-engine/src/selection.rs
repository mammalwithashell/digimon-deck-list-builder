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
use crate::enums::{EffectSourceKind, EffectTiming, GamePhase, PlayerId};
use crate::permanent::PermanentHandle;
use crate::trigger_context::TriggerContext;

/// Bitset of zones for `SelectionKind::UnionZone`. Designed to be extended
/// with additional zone bits in later Phase 4 tasks without breaking callers.
///
/// The `|` operator is implemented so callers can write
/// `UnionZoneSet::HAND | UnionZoneSet::TRASH` in the same style as `bitflags`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnionZoneSet(pub u8);

impl UnionZoneSet {
    /// The player's hand.
    pub const HAND: UnionZoneSet = UnionZoneSet(0b0001);
    /// The player's trash.
    pub const TRASH: UnionZoneSet = UnionZoneSet(0b0010);

    /// Returns `true` if the given zone bit is set.
    pub fn contains(self, other: UnionZoneSet) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns `true` if no zone bits are set.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for UnionZoneSet {
    type Output = UnionZoneSet;
    fn bitor(self, rhs: UnionZoneSet) -> UnionZoneSet {
        UnionZoneSet(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for UnionZoneSet {
    fn bitor_assign(&mut self, rhs: UnionZoneSet) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSelectionRef {
    pub permanent: PermanentHandle,
    pub field_index: u8,
    pub source_index: u8,
    pub card: CardHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BreedingPermanentSelectionRef {
    pub player: PlayerId,
    pub card: CardHandle,
}

/// Called when a selection resolves with a concrete action ID.
pub type SelectionCallback = Box<dyn FnOnce(&mut crate::game::Game, u16) + Send + Sync + 'static>;

/// Called when an optional selection is declined via PASS.
pub type DeclineCallback = Box<dyn FnOnce(&mut crate::game::Game) + Send + Sync + 'static>;

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
    /// Pick cards from one named reveal bucket. Reuses the reveal action range;
    /// `bucket_index` identifies which bucket is currently being resolved.
    RevealBucket {
        bucket_index: u8,
        min: u8,
        max: u8,
        picked: u8,
    },
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

    // ── Phase 4 kinds ─────────────────────────────────────────────────────
    // Full mask + decoder + helper dispatch lands in Tasks 2-5. Defined
    // here so all downstream match arms compile from day one.
    /// Pick one card from a union of two or more zones (e.g. hand OR trash).
    /// `zones` is a `UnionZoneSet` bitfield indicating which zones are in
    /// scope. The action-space encoding for each zone is resolved in Task 2.
    UnionZone { zones: UnionZoneSet },

    /// Pick cards in an ordered sequence (permutation). The caller re-installs
    /// the prompt after each pick, decrementing `remaining`; at 0 the callback
    /// fires with the final pick. Full decoder lands in Task 3.
    OrderedPermutation { remaining: u8 },

    /// Pick up to `max` cards from a zone; `picked` tracks how many have been
    /// chosen so far. The callback fires on each pick; the effect decides when
    /// to stop (or the player passes once `picked >= 1`). Full decoder in Task 4.
    CountCappedMultiSelect { max: u8, picked: u8 },

    /// Player may accept or decline an optional replacement effect. Backed by
    /// EffectChoice action range (accept) + PASS (decline). `valid_action_ids`
    /// holds exactly one ACCEPT entry; `is_optional = true` admits PASS.
    Replacement,
    /// Pick source cards across one or more own battle-area permanents.
    SourceMulti { min: u8, max: u8, picked: u8 },
    /// Pick opponent permanents whose total DP is capped by a remaining budget.
    DpBudget { remaining_dp: i32, picked: u8 },
    /// Pick the selecting player's breeding-area permanent.
    BreedingPermanent,
}

/// One branch of a `SelectionKind::EffectChoice` prompt.
#[derive(Debug, Clone)]
pub struct EffectChoiceEntry {
    pub label: String,
    pub action_id: u16,
    pub source_card: Option<CardHandle>,
    pub source_kind: Option<EffectSourceKind>,
    pub timing: Option<EffectTiming>,
    pub is_optional: bool,
    pub observation_metadata: crate::effect::EffectObservationMetadata,
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
    /// Source-kind provenance to preserve across parked callbacks.
    pub source_kind: EffectSourceKind,
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
            .field("source_kind", &self.source_kind)
            .finish_non_exhaustive()
    }
}

/// Pure-data subset of `PendingSelection` — excludes the non-serializable
/// `callback` / `on_decline` closures so this type is `Clone` + `Send` and
/// can cross the FFI boundary. Matches the Python `pendingSelection` dict
/// shape emitted by `digimon_gym/engine/game/serialization.py:338`.
#[derive(Debug, Clone)]
pub struct PendingSelectionView {
    pub kind: SelectionKind,
    pub selecting_player: PlayerId,
    pub previous_phase: GamePhase,
    pub valid_action_ids: Vec<u16>,
    pub is_optional: bool,
    pub prompt: String,
    pub effect_choices: Option<Vec<EffectChoiceEntry>>,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub source_kind: EffectSourceKind,
}

impl PendingSelectionView {
    /// Stable string form of `kind`, used for JSON/PyDict output. Debug
    /// formatting is deliberate — every variant stringifies to its Rust
    /// identifier so Python consumers can pattern-match without a shared
    /// schema.
    pub fn kind_str(&self) -> String {
        format!("{:?}", self.kind)
    }

    /// Stable string form of `previous_phase`.
    pub fn previous_phase_str(&self) -> String {
        format!("{:?}", self.previous_phase)
    }
}

impl PendingSelection {
    /// Snapshot of the serializable fields for FFI / UI consumers.
    pub fn view(&self) -> PendingSelectionView {
        PendingSelectionView {
            kind: self.kind,
            selecting_player: self.selecting_player,
            previous_phase: self.previous_phase,
            valid_action_ids: self.valid_action_ids.clone(),
            is_optional: self.is_optional,
            prompt: self.prompt.clone(),
            effect_choices: self.effect_choices.clone(),
            source_card: self.source_card,
            source_permanent: self.source_permanent,
            source_kind: self.source_kind,
        }
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
    pub source_kind: EffectSourceKind,
    pub controller: PlayerId,
    pub timing: EffectTiming,
    pub trigger_context: Option<TriggerContext>,
    pub effect_slot: u8,
    pub is_optional: bool,
    pub is_turn_player: bool,
    /// Card ID string, carried so the drainer can re-look-up the effect
    /// from the registry without scanning zones for a matching `CardHandle`.
    pub card_id: String,
    /// True only for queue entries originally collected from a below-top
    /// digivolution source scan. Prevents top-card effects from gaining
    /// source-position liveness after being covered by a later digivolution.
    pub allow_below_top_liveness: bool,
}

/// Queued triggered effect parked after its `pay_cost_fn` installed a
/// mandatory/optional selection. The cost callback has already returned
/// `true`; once the selection chain resolves, the effect resumes at the
/// max-per-turn record + process stage. If the callback explicitly declines
/// the pay cost, this parked entry is discarded.
#[derive(Debug, Clone)]
pub struct PendingPayCostEffect {
    pub queued_effect: QueuedEffect,
    pub declined: bool,
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
    /// Observer timing fired after a breeding-area permanent moves to the
    /// battle area. Scans the moving player's battle area while carrying the
    /// moved permanent/card as event context.
    MovedFromBreeding {
        player: PlayerId,
        permanent: PermanentHandle,
        card: CardHandle,
    },
    /// Observer timing fired after a battle-area permanent digivolves. Scans
    /// battle areas while carrying the just-digivolved permanent/card as
    /// event context.
    Digivolved {
        player: PlayerId,
        permanent: PermanentHandle,
        card: CardHandle,
    },
    /// Observer timing fired after a permanent enters the battle area. Scans
    /// all players' battle areas while carrying the entering permanent/card
    /// as event context.
    EnteredField {
        player: PlayerId,
        permanent: PermanentHandle,
        card: CardHandle,
    },
    /// Observer timing fired after a persistent Option is placed. Delay and
    /// Training Options have a standalone permanent; Link Options instead
    /// identify their host and linked card without pretending the linked card
    /// is a permanent.
    OptionPlaced {
        player: PlayerId,
        permanent: Option<PermanentHandle>,
        linked_host: Option<PermanentHandle>,
        card: CardHandle,
    },
    /// Generic observer timing carrying the permanent/card that caused the
    /// event. Used by event-gated delayed Options such as suspend watchers.
    EventObserved {
        player: PlayerId,
        permanent: PermanentHandle,
        card: CardHandle,
    },
    /// Observer timing fired after a card under a permanent's top card is
    /// trashed from that digivolution stack. Scans all players' battle areas
    /// while carrying the former host and trashed source card as event context.
    SourceTrashedFromStack {
        player: PlayerId,
        host: PermanentHandle,
        host_card: CardHandle,
        card: CardHandle,
        cause: crate::trigger_context::EventCause,
    },
    /// Observer timing fired after a card is removed from a player's security
    /// stack. `affected_player` owns the security stack; `observer_player`
    /// is the zone being scanned for either own-side or opponent-side
    /// observers; `source_player` is the attacker/effect controller.
    SecurityRemoved {
        affected_player: PlayerId,
        observer_player: PlayerId,
        source_player: PlayerId,
        card: CardHandle,
        cause: crate::trigger_context::EventCause,
    },
}

/// Transient per-security-check state. Lives on `Game` from the moment the
/// defender's security card is popped until the check finishes (either the
/// card is trashed or an effect plays it from security).
///
/// The `played` bit is raised by `EffectContext::play_pending_security` — it
/// signals the security-resolution loop that the card is now a Permanent on
/// the field, and must NOT be trashed at the end of the check.
#[derive(Debug, Clone)]
pub struct PendingSecurity {
    pub defender: PlayerId,
    pub card: CardSource,
    pub played: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityRemovalDestination {
    Trash,
    Hand(PlayerId),
    BottomSource(PermanentHandle),
    Digivolve {
        player: PlayerId,
        target: PermanentHandle,
        turn: u16,
    },
    Security {
        player: PlayerId,
        position: crate::enums::StackPosition,
        face_up: bool,
    },
}

#[derive(Debug, Clone)]
pub struct PendingEffectSecurityRemoval {
    pub defender: PlayerId,
    pub observer_player: PlayerId,
    pub source_player: PlayerId,
    pub cause: crate::trigger_context::EventCause,
    pub destination: SecurityRemovalDestination,
    pub previous_pending_security: Option<PendingSecurity>,
}

/// Transient state for an Option card mid-resolution. Mirrors
/// PendingSecurity / PendingAttack. Carries the card between pay-cost
/// and dispose so effect scripts can reference it via ctx.source_card.
#[derive(Debug, Clone)]
pub struct PendingOption {
    pub owner: PlayerId,
    pub card: CardSource,
    pub source_kind: OptionUseSource,
    pub resolution_phase: OptionResolutionPhase,
}

/// Source zone for an in-flight Option use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionUseSource {
    Hand,
    Trash,
}

impl OptionUseSource {
    pub fn zone(self) -> crate::enums::Zone {
        match self {
            OptionUseSource::Hand => crate::enums::Zone::Hand,
            OptionUseSource::Trash => crate::enums::Zone::Trash,
        }
    }
}

/// Outcome of a `play_option_from_hand` / `play_option_from_trash` call.
///
/// `Trashed` — Standard Option fully resolved and went to owner's trash.
/// `Pending` — an `OptionMain` effect installed a `PendingSelection`;
///   caller must drive the selection to completion (dispose re-enters via
///   the post-resolution path).
/// `Invalid` — validation failed; no state was mutated.
/// `Delayed` / `Linked` / `Training` — populated in Tasks 3/4/5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum OptionPlayResult {
    Trashed,
    Delayed(PermanentHandle),
    Linked { source: PermanentHandle },
    Training(PermanentHandle),
    Pending,
    Invalid,
}

/// Where we are in the resolve-and-dispose sequence for an Option card.
///
/// Variants appear in typical execution order: Link Options begin with
/// `LinkSelectHost` (pay cost → select host → fire `OptionMain` → attach →
/// fire `OnLink` → `Done`); Standard / Delay / Training Options begin with
/// `MainEffectDrain`; all Options terminate at `Done`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionResolutionPhase {
    LinkSelectHost,
    MainEffectDrain,
    ArtsSelectTarget,
    Disposing,
    Done,
}

/// Phase of an in-flight security-card resolution. Drives the
/// `drive_security_resolution` state machine in `combat.rs` so that a
/// `pending_selection` installed inside a `SecuritySkill` process can pause
/// the flow and resume from the same phase after the selection resolves.
///
/// Order matches Python's `Player.security_attack` / `_execute_security_checks`
/// sequence: SecuritySkill → battle → OnSecurityCheck → OnLoseSecurity → dispose.
/// See RUST_PYTHON_PARITY §2.5b / §2.5j.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityPhase {
    /// Enqueue + drain `SecuritySkill` effects on the revealed card.
    SecuritySkillDrain,
    /// Digimon-vs-security DP battle (skipped for non-Digimon security).
    BattleResolved,
    /// Enqueue + drain `OnSecurityCheck` observer effects over the defender's
    /// battle area.
    OnSecurityCheckDrain,
    /// Enqueue + drain `OnLoseSecurity` on the revealed card (observer timing).
    OnLoseSecurityDrain,
    /// Trash the card (unless `pending_security.played` is set), then fire
    /// `OnOpponentSecurityRemoved` observers in the attacker's battle area.
    /// If that observer drain installs a pending selection, the state
    /// machine parks with phase advanced to `DisposeFinalize` so the resume
    /// path doesn't re-enqueue the observer on re-entry (§2.5j residual).
    Dispose,
    /// Post-observer finalization: extract `SecurityResolutionState`,
    /// decide terminal outcome vs. loop to next security card.
    DisposeFinalize,
}

/// Mid-security-check state. Installed by `Game::resolve_security_card` at
/// the start of each check, mutated by `drive_security_resolution` as phases
/// advance, and cleared at `Dispose`.
///
/// When a `SecuritySkill` process installs a `pending_selection`, the
/// outer combat flow unwinds; `Game::advance_security_resolution` re-enters
/// the state machine after `resolve_generic_selection` invokes the
/// callback. See RUST_PYTHON_PARITY §2.5j.
#[derive(Debug, Clone)]
pub struct SecurityResolutionState {
    /// The attacking permanent. `None` iff the reveal was not triggered by
    /// a combat check (reserved for future non-combat reveal-from-security
    /// effects).
    pub attacker: Option<PermanentHandle>,
    pub defender: PlayerId,
    pub turn_player: PlayerId,
    /// Handle of the revealed card, repeated here so the context enrichers
    /// don't need to re-read `pending_security` after it's cleared.
    pub revealed_card: CardHandle,
    pub card_kind: crate::enums::CardKind,
    /// True iff the card was in `face_up_security` when popped — exposed to
    /// `OnSecurityCheck` observers via `SecurityRevealSnapshot`.
    pub was_face_up: bool,
    pub phase: SecurityPhase,
    /// Remaining security-check iterations for the owning `Player` attack.
    /// Absorbed from the outer loop counter so a pause inside phase 1
    /// doesn't drop the remaining checks.
    pub checks_remaining: u8,
    /// Running outcome for the current card's resolution. Updated when the
    /// DP battle concludes; returned at `Dispose`.
    pub outcome_so_far: crate::combat::AttackResult,
}

/// Snapshot of the most recently revealed security card on a player, used
/// to enrich `OnSecurityCheck` observer effect contexts. Lives on `Player`
/// alongside `face_up_security`. Mirrors Python's `_last_security_card` /
/// `_last_security_was_face_up` pair (RUST_PYTHON_PARITY §2.5l).
#[derive(Debug, Clone, Copy)]
pub struct SecurityRevealSnapshot {
    pub card: CardHandle,
    pub was_face_up: bool,
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
    /// Phase 9 Task 4 — post-Block / pre-Battle checkpoint. The state
    /// machine lands here unconditionally once the Block window closes
    /// (whether a block was declared, declined, or no candidate existed).
    /// The PostBlock arm tests whether `effective_target` is still valid;
    /// if not, it considers Raid-driven retarget before either transitioning
    /// to Battle or fizzling to Cleanup. Spec §4.2.
    PostBlock,
    /// No further interrupts pending. `resolve_pending_battle` will fire.
    Battle,
    /// Phase 9 Task 6 — post-Battle checkpoint. The Battle arm transitions
    /// the attack here immediately before firing a `<Piercing>`-driven
    /// follow-up security check against the defending player. The state
    /// is primarily a provenance marker: `resolve_pending_battle` has
    /// already run (DP compared + defender wiped), and the security
    /// pipeline takes over via `enter_piercing_security_check`. If that
    /// pipeline parks on a `PendingSelection` (e.g. a `WhenWouldLoseSecurity`
    /// optional replacement), `advance_security_resolution` finalizes the
    /// attack via `cleanup_attack` when the selection chain clears.
    /// Spec §4.3.
    PostBattle,
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
    /// True once declaration has crossed the observable boundary: attacker was
    /// marked attacking, suspend/attack count was applied when appropriate, and
    /// `OnAttack` / declared-attack observer windows have begun. Optional
    /// pre-declaration replacements can park while this is still false.
    pub declaration_committed: bool,
    /// Phase 9: set by a `WhenWouldAttack` / `WhenWouldBeAttackTarget`
    /// replacement whose process calls `rctx.cancel()`. `advance_pending_attack`
    /// detects this and short-circuits directly to `Cleanup`. EndOfAttack
    /// still fires in cleanup; EndOfBattle does NOT (no battle occurred).
    pub cancelled: bool,
    /// Phase 9: set to `true` by `resolve_battle` once a Digimon-vs-Digimon
    /// DP comparison has run. Used by cleanup to decide whether EndOfBattle
    /// should fire — Phase 1 fires it unconditionally inside `resolve_battle`,
    /// so this flag is currently informational / test-visible only. Player
    /// security attacks leave this `false`, matching the Python semantics
    /// where EndOfBattle only fires for Digimon-vs-Digimon battles.
    pub battle_occurred: bool,
    /// Phase to return to once the attack finishes (`Main` for normal
    /// attacks, `EndOfTurnAction` for end-of-turn vortex attacks, etc.).
    pub return_phase: GamePhase,
    pub state: AttackState,
    /// Phase 9 Task 3 — how many Counter windows this attack has already
    /// entered. Incremented when `try_enter_counter` installs a Counter
    /// selection; compared against `MAX_COUNTER_DEPTH` (= 1) on
    /// subsequent entries to suppress nested Counter windows that would
    /// otherwise recurse from a counter body's side-effects. Spec §5.4.
    pub counter_depth: u8,
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
