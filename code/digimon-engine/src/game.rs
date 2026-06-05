use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::card_data::CardData;
use crate::card_source::{CardHandle, CardSource};
use crate::cards::{build_registry, CardEffectRegistry};
use crate::dsl_cards::formula_registry::FormulaExtensionRegistry;
use crate::enums::{Expiry, GamePhase, ModifierType, PlayerId};
use crate::logger::{GameLogger, SilentLogger};
use crate::modifiers::ModifierRegistry;
use crate::permanent::PermanentHandle;
use crate::player::{OriginalDeckCardCount, Player};
use crate::rules::Rules;
use crate::selection::{
    EffectQueue, PendingAttack, PendingEffectSecurityRemoval, PendingOption, PendingPayCostEffect,
    PendingSecurity, PendingSelection, SecurityResolutionState, SelectionError,
};
use crate::token_registry::TokenRegistry;
use crate::trigger_context::TriggerContext;

/// Reasons `Game::activate_overclock` can fail. Exposed so callers
/// (Tauri commands, tests, Python bindings) can distinguish between
/// phase-violation and state-violation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverclockError {
    /// Current phase is not `EndOfTurnAction`.
    WrongPhase,
    /// Another selection or attack is in flight.
    Busy,
    /// The indicated permanent does not have `<Overclock>` (either the
    /// keyword isn't granted, or the slot doesn't hold a Digimon).
    NotOverclock,
    /// No sacrificeable Digimon is available to pay the Overclock cost.
    NoSacrifice,
    /// `overclock_index` is out of range for the turn player's battle area.
    InvalidIndex,
}

impl std::fmt::Display for OverclockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongPhase => write!(f, "activate_overclock called outside EndOfTurnAction"),
            Self::Busy => write!(
                f,
                "activate_overclock called while a selection or attack is in flight"
            ),
            Self::NotOverclock => write!(f, "permanent does not have <Overclock>"),
            Self::NoSacrifice => write!(f, "no sacrificeable Digimon available"),
            Self::InvalidIndex => write!(f, "overclock_index out of range"),
        }
    }
}

impl std::error::Error for OverclockError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum TerminalOutcomeReason {
    SecurityAttack,
    DeckOut,
    EngineDeclared,
    UnknownWin,
    StepLimit,
    Crash,
    UnknownDraw,
    /// The losing player chose to concede the game (action `93` /
    /// `Game::concede`). Emitted with a winner equal to the conceder's
    /// opponent. Surfaces to Python as `win_reason = "concede"`.
    Concede,
}

impl TerminalOutcomeReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SecurityAttack => "security_attack",
            Self::DeckOut => "deck_out",
            Self::EngineDeclared => "engine_declared",
            Self::UnknownWin => "unknown_win",
            Self::StepLimit => "step_limit",
            Self::Crash => "crash",
            Self::UnknownDraw => "unknown_draw",
            Self::Concede => "concede",
        }
    }

    pub fn result(self) -> &'static str {
        match self {
            Self::Crash | Self::UnknownDraw => "draw",
            Self::SecurityAttack | Self::DeckOut | Self::EngineDeclared | Self::UnknownWin => "win",
            Self::StepLimit => "win",
            Self::Concede => "win",
        }
    }
}

fn face_keywords(card_data: &CardData) -> Vec<crate::enums::Keyword> {
    if card_data.effect_text.is_empty()
        && card_data.inherited_text.is_empty()
        && card_data.security_text.is_empty()
    {
        return card_data.keywords.clone();
    }
    crate::card_data::parse_printed_keywords(&card_data.effect_text, "", "")
}

fn inherited_keywords(card_data: &CardData) -> Vec<crate::enums::Keyword> {
    if card_data.inherited_text.is_empty() {
        return Vec::new();
    }
    crate::card_data::parse_printed_keywords("", &card_data.inherited_text, "")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedOptionLifecycleResumeKind {
    StartTurn,
    EndTurn {
        ending_player: PlayerId,
    },
    Event {
        timing: crate::enums::EffectTiming,
    },
    /// Standard `<Delay>` activated by a player `[Main]`-phase action
    /// (PUPPETS-G009). The Option's `DelayEffect` body installed a pending
    /// selection; once it resolves, the Option is trashed as the activation
    /// cost. No turn-keyed scan resumes — this kind only carries the deferred
    /// trash of the activated Option.
    MainPhaseActivation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedOptionLifecycleResume {
    pub(crate) turn: u16,
    pub(crate) kind: DelayedOptionLifecycleResumeKind,
    pub(crate) pending_delete_key: Option<(PlayerId, u16)>,
    pub(crate) skip_key: Option<(PlayerId, u16)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EndTurnResume {
    pub(crate) ending_player: PlayerId,
    pub(crate) memory_before_end_effects: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingWouldPlayResume {
    pub(crate) player: PlayerId,
    pub(crate) card: crate::card_source::CardHandle,
    pub(crate) effective_cost: u16,
    pub(crate) origin: PendingWouldPlayOrigin,
    pub(crate) effect_initiated: bool,
    /// PUPPETS-G030 — when `true`, the just-played permanent's own `[On Play]`
    /// effects are NOT enqueued for this play event. Used by BT5-106's
    /// [Security] clause ("Any [On Play] effects on Digimon played with this
    /// effect don't activate."). Scoped strictly to the played permanent and
    /// this single play event: other permanents' On Play, and every other
    /// timing (OnEnterFieldAnyone / OnAllyPlayed), are unaffected.
    pub(crate) suppress_on_play: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingWouldPlayOrigin {
    Hand,
    Trash {
        index: usize,
    },
    SecurityTop {
        was_face_up: bool,
    },
    Reveal {
        index: usize,
    },
    Source {
        permanent: PermanentHandle,
        source_index: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingWouldLinkResume {
    pub(crate) host: PermanentHandle,
    pub(crate) card: crate::card_source::CardHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingWouldDigivolveResume {
    pub(crate) player: PlayerId,
    pub(crate) permanent: PermanentHandle,
    pub(crate) card: crate::card_source::CardHandle,
    pub(crate) effective_cost: u16,
}

/// The core game state. Drives the turn state machine.
///
/// `impl Game` blocks for this struct are spread across three files for
/// readability; every method remains `Game::*` in the call surface:
/// - `game.rs` — struct, constructor, mulligan, state queries, memory
///   mgmt, tensor/DP/OPT helpers, elimination.
/// - `game_phases.rs` — turn lifecycle: `begin_turn`, `end_turn`,
///   `rotate_turn_player`, `pass_turn`, `activate_overclock`,
///   `fire_end_of_your_turn`.
/// - `game_actions.rs` — player mutators: `play_from_hand`,
///   `digivolve_from_hand`, `move_from_breeding`, `activate_*_main`,
///   `initiate_dna_digivolve`.
#[derive(Debug)]
pub struct Game {
    pub rules: Rules,
    pub players: Vec<Player>,
    pub turn_count: u16,
    /// Cumulative regular-digivolve count per player. Incremented on every
    /// successful regular digivolve (including via DNA, which also bumps
    /// `n_dna_digivolutions`). Indexed by Rust 0-based PlayerId. Monotonic
    /// per game — never reset, never decremented. Backs the digivolve
    /// reward-shaping signal in DigimonEnv. See
    /// `docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md`.
    pub n_digivolutions: [u32; 2],
    /// Cumulative DNA-digivolve count per player. Incremented on every
    /// successful DNA digivolve, on top of `n_digivolutions`.
    pub n_dna_digivolutions: [u32; 2],
    /// Cumulative count of digivolve-driven attacks that reached an
    /// opponent's security stack per attacker player. Incremented once
    /// per qualifying attack (NOT once per security card revealed) in
    /// `resolve_battle_attack_target` when the attack target is
    /// `AttackTarget::Player`, the attacker's effective level is ≥ 5,
    /// and the security loop will actually pop at least one card (i.e.
    /// not Jamming-zeroed). Blocked attacks never reach this site
    /// because blocks redirect the target to a Digimon. Piercing
    /// follow-ups also do NOT count because the originating attack
    /// target was a Digimon (only the `Player` arm increments).
    /// Indexed by Rust 0-based PlayerId. Monotonic — never reset.
    /// Backs the `digivolve_driven_attack` reward signal.
    /// See `openspec/changes/add-gameplay-reward-config/`.
    pub n_digivolve_driven_attacks: [u32; 2],
    /// Per-turn count of Digimon attacks declared by each player. Reset when
    /// that player begins a new turn, after prior end-of-turn observers have
    /// had a chance to inspect the ending turn's history.
    pub digimon_attacks_this_turn: [u32; 2],
    pub current_phase: GamePhase,
    /// Memory seesaw value. Positive = favor of memory_pair.0, negative = favor of memory_pair.1.
    pub memory: i16,
    /// The active pair for the memory seesaw: (active_player, next_player).
    pub memory_pair: (PlayerId, PlayerId),
    /// Turn rotation order. Eliminated players are removed.
    pub turn_order: Vec<PlayerId>,
    /// Index into turn_order for the current turn player.
    pub turn_player_idx: usize,
    pub game_over: bool,
    pub winner: Option<PlayerId>,
    pub terminal_outcome_reason: Option<TerminalOutcomeReason>,
    /// Shared card data store (all cards in the game reference into this).
    pub card_data: Vec<CardData>,
    /// Compiled DSL alternate digivolution paths keyed by result card id.
    #[cfg(feature = "dsl-yaml-loader")]
    pub(crate) alt_path_registry: HashMap<String, Vec<digimon_dsl::compiled::CompiledAltPath>>,
    /// Active modifiers (DP buffs, granted keywords, etc.) attached to permanents.
    pub modifiers: ModifierRegistry,
    /// Source-independent continuous mass modifiers (e.g. "all opponent Digimon
    /// get -5000DP until end of opponent's next turn"). Re-applied to a live
    /// candidate set every `tick_declarative_effects`; pruned at turn-end. See
    /// `crate::floating_modifier`.
    pub floating_mass_modifiers: Vec<crate::floating_modifier::FloatingMassModifier>,
    /// Card effect registry — maps card_id to effect implementations.
    pub effect_registry: CardEffectRegistry,
    /// Runtime callbacks for DSL `raw_rust` formulas.
    pub formula_extensions: FormulaExtensionRegistry,
    /// Token metadata registry — maps canonical token names (e.g.
    /// "petrification") to `TokenDef` rows. `Game::new` pre-populates
    /// this via `token_registry::build_registry` and pushes a synthetic
    /// `CardData` row into `card_data` for each registered token so
    /// materialized tokens can reuse the standard `CardSource::data_index`
    /// machinery. Consumed by `EffectContext::play_token`.
    pub token_registry: TokenRegistry,
    /// RNG for shuffling and random effects.
    pub rng: StdRng,
    /// Counter for assigning unique card instance indices.
    next_card_index: u16,
    /// Players still owing a mulligan decision, in order. Empty once mulligan
    /// is finalized. Driven by `accept_mulligan`; see §1.6 in RUST_PYTHON_PARITY.
    pub mulligan_pending: Vec<PlayerId>,
    /// Whether each player has already re-drawn during mulligan. Indexed by
    /// `PlayerId`. Used by the action mask to suppress the re-draw bit once
    /// a player has used their single mulligan.
    pub mulligan_used: Vec<bool>,
    /// Cards currently revealed to all players (e.g. top-of-deck reveals,
    /// search pools). Rendered into the observation tensor at `OFF_REVEALED`
    /// and cleared on turn rotation. Populated by reveal-from-deck / search
    /// effects. Matches Python's `Game.revealed_cards`.
    pub revealed_cards: Vec<CardSource>,

    /// Parked player-choice prompt, if any. Set by `EffectContext::select_*`
    /// helpers and the effect-queue drainer; resolved by
    /// `Game::resolve_selection`. See `selection.rs` for the design.
    /// Always `None` until the selection subsystem lands (PR2/PR3).
    pub pending_selection: Option<PendingSelection>,
    /// Triggered effects waiting to resolve at the current timing window.
    /// Populated by `enqueue_triggered` and drained by `drain_effect_queue`.
    /// Empty until the drainer lands (PR2).
    pub effect_queue: EffectQueue,
    /// Track H §3 — pending granted-triggered-effect fires queued by
    /// `enqueue_from_permanent` and `enqueue_from_breeding_permanent`.
    /// `drain_effect_queue` flushes this queue AFTER its main loop
    /// settles (printed observers fire first, granted bodies second —
    /// matches DCGO's "appended to effect list" semantic for granted
    /// effects). Each granted body runs inline; if it enqueues further
    /// triggered effects, the drain loop re-runs.
    pub pending_granted_fires: Vec<(
        crate::permanent::PermanentHandle,
        crate::enums::EffectTiming,
    )>,
    /// Track H §3 Phase 4i — registry of granted-triggered-effect
    /// bodies indexed by id. Bodies are allocated an id at install
    /// time (`grant_triggered_effect`) and referenced from
    /// `QueuedEffect::granted_effect_id` so the drainer can fetch and
    /// run them without a card_id+effect_slot lookup. Selection-
    /// driving bodies park on `pending_selection` like printed effects;
    /// the resume hook re-fetches the body via id.
    pub granted_effect_bodies: crate::modifiers::GrantedEffectBodyRegistry,
    /// Counter for allocating fresh granted-effect ids. Monotonic
    /// across the game's lifetime; never reused.
    pub next_granted_effect_id: u64,
    /// Queued triggered effect parked while its `pay_cost_fn` resolves a
    /// player selection. Resumed by `resolve_selection` before ordinary queue
    /// draining continues.
    pub pending_pay_cost_effect: Option<PendingPayCostEffect>,
    /// Outer pay-cost continuations parked beneath the current one. This is
    /// populated when a pay-cost selection callback drains another queued
    /// effect that itself parks for a pay-cost selection.
    pub pending_pay_cost_stack: Vec<PendingPayCostEffect>,
    /// In-flight attack, if any. Installed by `begin_attack`, advanced by
    /// the combat state machine, cleared by `cleanup_attack`.
    /// Always `None` until the combat state machine lands (PR4).
    pub pending_attack: Option<PendingAttack>,
    /// Transient state for an in-progress security check. Set by
    /// `resolve_security_card` before firing `SecuritySkill` effects and
    /// cleared afterward. `EffectContext::play_pending_security` inspects and
    /// mutates this slot to keep the revealed card from being trashed.
    pub pending_security: Option<PendingSecurity>,
    /// Continuations for effect-driven security removal helpers that park
    /// inside an `OnLoseSecurity` observer. Combat security checks use
    /// `security_resolution`; direct helpers use this smaller stack.
    pub pending_effect_security_removal: Vec<PendingEffectSecurityRemoval>,
    /// Phase 8: in-flight Option card resolution. Set when an Option is
    /// played and cleared after dispose. Dispatch lands in Tasks 2-6.
    pub pending_option: Option<PendingOption>,
    /// Inert context for a pending DigiXros play from hand. The play-cost path
    /// installs this before cost hooks run and clears it when the play commits
    /// or aborts. Later slices use it for material selection and source commit.
    pub(crate) pending_digixros_transaction: Option<crate::digixros::DigiXrosTransaction>,
    /// "Leaving / limbo" holding slot for battle-area DigiXros materials whose
    /// `WhenWouldLeaveBattleArea` replacement window parked a `pending_selection`
    /// (e.g. BT17-095's optional `<Delay>` accept). The departing material is
    /// popped OUT of `battle_area` (so it is no longer any permanent's top card —
    /// satisfying the "the material has left" precondition) but retained here so
    /// the parked observer's reward (a DNA-evo) can still EXTRACT it into a new
    /// permanent. On finalize, any material still in limbo (the observer declined
    /// / was ineligible) is restored to `battle_area` to be consumed under the
    /// DigiXros host as normal. Addressed by handles whose index is offset by
    /// `LIMBO_INDEX_BASE`. See rule G-DIGIXROS-REDIRECT-EXTRACTION (judge-quiz
    /// Q26/Q27).
    /// Each entry is `(owner, original_battle_handle, permanent)`. The original
    /// battle handle (captured BEFORE the `battle_area.remove` shift) lets a
    /// parked replacement whose subject was the leaving permanent re-resolve to
    /// the limbo-encoded handle (`remap_digixros_limbo_subject`), since the
    /// subject is addressed by index alone.
    pub(crate) digixros_leaving_limbo:
        Vec<(crate::enums::PlayerId, crate::permanent::PermanentHandle, crate::permanent::Permanent)>,
    /// Turn-scoped DigiXros wildcard material modifiers waiting to be copied
    /// into the next matching transaction.
    pub(crate) active_digixros_wildcards: Vec<crate::digixros::ActiveDigiXrosWildcardSubstitution>,
    /// Continuation marker for Delay/Training Option placement observers.
    /// Option play normally calls `check_turn_end` after disposal. If
    /// placement observers park a selection after `pending_option` has already
    /// been consumed, this marker lets `resolve_generic_selection` run the
    /// turn-end check after the observer queue settles.
    #[doc(hidden)]
    pub(crate) pending_option_placed_turn_check: bool,
    /// Link Option continuation parked when `OnOptionPlaced` observers install
    /// a selection after the linked card has attached but before `OnLink` has
    /// fired.
    #[doc(hidden)]
    pub(crate) pending_option_placed_link_resume: Option<PermanentHandle>,
    /// Mid-security-check resolution state. Set by `resolve_security_card`
    /// at phase entry, mutated by `drive_security_resolution` as phases
    /// advance, and cleared at `Dispose`. Non-`None` when the engine is
    /// paused inside a security check — a `pending_selection` installed by
    /// a `SecuritySkill` process pauses resolution here; resumption is
    /// driven by `Game::advance_security_resolution`, called from
    /// `resolve_generic_selection`. See RUST_PYTHON_PARITY §2.5j.
    pub security_resolution: Option<SecurityResolutionState>,
    /// Safety rail matching Python's `_resolve_effect_stack` max-iterations=50
    /// cap. Incremented per drain step; reset to 0 when the queue empties.
    /// Prevents a self-triggering chain from hanging the engine.
    /// Consumed by the drainer in PR2.
    #[allow(dead_code)]
    pub(crate) effect_chain_depth: u16,

    /// Re-entrancy depth for the state-based ≤0-DP rules-check
    /// (`run_state_based_rules_check`). `drain_effect_queue` is called
    /// re-entrantly from inside effect bodies (effect_context) and from the
    /// batched-deletion deferred drain. The rules-check runs ONLY at the
    /// OUTERMOST drain (`effect_drain_depth == 1`) — after a top-level
    /// effect / rule-action has fully finished — and is run BETWEEN top-level
    /// queued effects (after each `run_queued_effect`) so a Digimon driven to
    /// ≤0 DP by one effect is deleted before the next queued trigger resolves
    /// (judge Q24). It is never run between the sub-steps of one resolving
    /// effect (the judge rule: "rule checks don't happen until an ongoing
    /// effect or rule action finishes" — Q6/Q13/Q14). Incremented on entry to
    /// `drain_effect_queue`, decremented on exit.
    pub(crate) effect_drain_depth: u16,

    /// Game logger. Defaults to `SilentLogger` (zero-overhead for RL
    /// training). Callers that want human-readable traces install a
    /// `VerboseLogger` via `set_logger`. Parity with Python's
    /// `Game.logger` field.
    pub logger: Box<dyn GameLogger>,

    /// Event buffer drained per `step` by the runner. See
    /// `src/events.rs` for the event taxonomy.
    pub events: Vec<crate::events::GameEvent>,
    /// Monotonic counter for `GameEvent::seq`. Never decreases across the
    /// lifetime of a `Game`.
    pub event_seq: u64,

    /// Current nesting depth of `Game::try_replace`. Incremented on entry,
    /// decremented on exit. At `>= MAX_REPLACEMENT_DEPTH`, the dispatcher
    /// short-circuits to `ReplacementOutcome::None` — safety rail against
    /// self-referential replacement chains (e.g. two permanents each
    /// replacing the other's deletion with "cancel").
    #[doc(hidden)]
    pub replacement_depth: u8,

    /// Outcome slot written by a replacement-selection callback (optional
    /// replacement accept path) and read by the `try_replace` caller after
    /// the selection resolves. `None` outside a replacement window; `None`
    /// on decline. See `replacement::try_replace_impl`.
    #[doc(hidden)]
    pub replacement_pending_outcome: Option<crate::replacement::ReplacementOutcome>,
    /// Outcome slot written by `SelectionKind::PlayOrder` resolution. Read
    /// by the Python `MatchEnv` wrapper (BO3 match training) to determine
    /// which side plays first in the next game. `None` until a play-order
    /// selection has been resolved; the wrapper takes the value and resets
    /// the slot. The engine itself is BO3-agnostic.
    pub last_play_order_choice: Option<crate::selection::PlayOrder>,
    /// Fire-site continuation for optional `WhenPermanentWouldPlay`
    /// replacements whose subject is a card in hand.
    #[doc(hidden)]
    pub(crate) pending_would_play_resume: Option<PendingWouldPlayResume>,
    /// Transient hand-off for an `[Assembly]` play (G-ASSEMBLY-PLAY-EXECUTION):
    /// the played card's handle plus the trash material handles to place at the
    /// bottom of its digivolution stack. Consumed by
    /// `commit_play_from_hand_card_no_replace` AFTER the permanent is created
    /// but BEFORE its `[On Play]` / `[When Digivolving]` effects fire — so a
    /// card whose play effects read its own digivolution-card count (e.g.
    /// AD1-025 Omnimon's bounce) sees the assembled materials. Set immediately
    /// before the assembly play's `finish_play_from_hand_after_reductions` and
    /// cleared on consume. Same transient-slot pattern as
    /// `pending_would_play_resume`.
    #[doc(hidden)]
    pub(crate) pending_assembly_materials: Option<(crate::card_source::CardHandle, Vec<crate::card_source::CardHandle>)>,
    /// Fire-site continuation for optional `WhenWouldLink` replacements whose
    /// subject is the pending Link Option card.
    #[doc(hidden)]
    pub(crate) pending_would_link_resume: Option<PendingWouldLinkResume>,
    /// Fire-site continuation for optional `WhenPermanentWouldDigivolve`
    /// replacements whose subject is the permanent about to digivolve.
    #[doc(hidden)]
    pub(crate) pending_would_digivolve_resume: Option<PendingWouldDigivolveResume>,

    /// Player-scoped one-shot future-digivolve cost reducers
    /// (`G-COST-REDUCE-ALLY-DIGIVOLVE`). Installed by a `[Main]` effect with
    /// no field permanent to host it (BT3-103 Hidden Potential Discovered!).
    /// Consulted at the top of each digivolve-from-hand cost path BEFORE the
    /// synchronous field-hosted `BeforePayCost` scan. See
    /// `player_cost_reducer.rs` for the lifecycle.
    pub player_digivolve_cost_reducers: Vec<crate::player_cost_reducer::PlayerDigivolveCostReducer>,

    /// Reduction (in memory) granted by an already-resolved player-scoped
    /// digivolve cost reducer for the digivolution currently being
    /// re-entered. Set by the accept/decline callbacks of the player-scoped
    /// reducer prompt; read by the digivolve cost calculation and cleared
    /// once consumed. `0` means "no player-scoped reduction" (also the
    /// decline outcome).
    pub(crate) pending_player_digivolve_reduction: i32,

    /// Spec §7.5 once-per-event guard. Records `(timing, subject)` pairs that
    /// have already fired within the current `try_replace` call chain so a
    /// redirected route does not re-fire the same timing for the same subject
    /// (e.g. `WhenWouldLeaveBattleArea` super-timing double-fire when a
    /// `Redirected(Deck)` outcome on `WhenWouldBeDeleted` routes through
    /// `return_to_deck`, which would otherwise re-invoke
    /// `WhenWouldLeaveBattleArea` for the same permanent).
    ///
    /// Cleared at the outermost entry (when `replacement_depth == 0`) of
    /// `try_replace_impl` — unless `in_replacement_commit` is set, in which
    /// case we are continuing the original call chain across a callback
    /// resolution boundary and the set must be preserved. See
    /// `replacement::try_replace_impl`.
    #[doc(hidden)]
    pub replacement_fired: std::collections::HashSet<(
        crate::enums::EffectTiming,
        crate::replacement::ReplacementSubject,
    )>,

    /// Spec §7.5 continuation marker. Set by the optional-replacement callback
    /// (accept/decline) just before invoking `commit_deferred_outcome`, cleared
    /// after the commit returns. While true, `try_replace_impl` treats a
    /// depth==0 entry as a continuation of the original call chain and does
    /// NOT clear `replacement_fired` — so the fired-set from the original
    /// event survives the callback boundary and blocks double-fires during
    /// the commit's zone-mover calls.
    #[doc(hidden)]
    pub(crate) in_replacement_commit: bool,

    /// Controller of the effect whose `process` is currently running, if
    /// any. Set by `run_queued_effect` at dispatch time and cleared at the
    /// end of the call. Consumed by `infer_deletion_cause` (and Task 4's
    /// sibling route inference helpers) to distinguish Own-effect vs
    /// Opponent-effect deletions at the fire-site. `None` when no effect is
    /// currently executing (e.g. direct-from-test call, combat,
    /// security-check driver between drains).
    #[doc(hidden)]
    pub(crate) effect_source_player: Option<PlayerId>,
    #[doc(hidden)]
    pub(crate) effect_source_card: Option<crate::card_source::CardHandle>,
    #[doc(hidden)]
    pub(crate) effect_source_permanent: Option<crate::permanent::PermanentHandle>,

    /// Runtime metadata for the trigger whose effect is currently resolving.
    /// DSL event predicates and `event_target` / `event_card` bindings read
    /// this slot. It is set by the effect-queue dispatcher around a queued
    /// effect and restored afterward.
    #[doc(hidden)]
    pub current_trigger_context: Option<TriggerContext>,

    /// The cause of the deletion currently being observed by `OnDeletion`
    /// effects. Set by `commit_permanent_deletion` immediately before
    /// `enqueue_triggered(OnDeletion, ...)`; cleared after the drain via a
    /// panic-safe `catch_unwind` scope at the fire-site. Read by
    /// `EffectContext::deletion_cause()` / `was_deleted_by_effect()` /
    /// `was_deleted_by_opponent()`.
    ///
    /// `None` outside an OnDeletion observer body. Phase B §B5.
    #[doc(hidden)]
    pub(crate) current_deletion_cause: Option<crate::replacement::ReplacementCause>,

    /// Observer-facing cause override for the deletion currently being
    /// finalized. Replacement windows still read `current_deletion_cause`;
    /// this slot only refines the `TriggerContext` payload for timings such
    /// as `OnAnyDeletion` that distinguish a keyword route like Overclock.
    #[doc(hidden)]
    pub(crate) current_deletion_event_cause_override: Option<crate::trigger_context::EventCause>,

    /// Paused Overclock attack after the sacrifice deletion installed one or
    /// more observer selections. Once those selections finish, the source card
    /// is re-resolved by handle/token instead of trusting the old slot.
    #[doc(hidden)]
    pub(crate) pending_overclock_attack: Option<(PlayerId, CardHandle, PlayerId)>,
    /// Overclock source cards declined during the current EndOfTurnAction
    /// window. Decline is a real choice; without this guard the same optional
    /// cost prompt can be re-opened indefinitely without changing game state.
    #[doc(hidden)]
    pub(crate) declined_overclock_this_eot: HashSet<CardHandle>,

    /// True while effects are being evaluated from a DNA digivolution event.
    /// Consumed by the DSL `dna_origin` predicate for clauses like
    /// "[When Digivolving] If DNA digivolving, ...".
    #[doc(hidden)]
    pub(crate) current_dna_origin: Option<bool>,

    /// Parked replacement state when a `WhenWouldBe*` replacement-process
    /// closure installs a nested player selection. Set by the dispatcher's
    /// post-process hook in `replacement::run_candidate_inner`; drained by
    /// `effect_queue::resolve_generic_selection` after the user's callback
    /// runs. `None` outside a parked-replacement scope.
    ///
    /// **Single-outstanding invariant:** at most one slot occupied at a time;
    /// the dispatcher `debug_assert!`s on duplicate install.
    ///
    /// **Coexistence with `dsl_outer_tail`** (Phase 2d): independent slots for
    /// independent concerns. Phase C §4.1.
    #[doc(hidden)]
    pub(crate) parked_replacement: Option<crate::replacement::ParkedReplacement>,

    /// Temporary bridge used by DSL replacement-process outcome steps before
    /// the dispatcher has installed a `ParkedReplacement`. The replacement
    /// lowering drains this into the active `ReplacementContext`.
    #[doc(hidden)]
    pub(crate) dsl_replacement_outcome: Option<crate::replacement::ReplacementOutcome>,

    /// Phase 9 Task 3 — set to `true` while a hand Counter Option is
    /// resolving through `play_option_from_hand`. Consumed by
    /// `play_option_core` to fire CounterEffect timing on the played
    /// card's effects BEFORE `OptionMain`. Cleared when the Counter
    /// resolver finishes the Option play. Spec §5.2.
    #[doc(hidden)]
    pub(crate) in_counter_window: bool,

    /// Active multi-target deletion batch (DCGO `DestroyPermanentsClass`
    /// equivalent). `Some(batch)` while `delete_permanents_batch` is in
    /// flight at a top level; `None` otherwise. Carries the kill list,
    /// snapshots, and stage marker through the 10-step DCGO flow.
    ///
    /// **Single-outstanding (top-level):** at most one batch is open at a
    /// time. A nested `delete_permanents_batch` call from inside an
    /// OnDeletion handler is bounded by `DeletionBatch::depth` — see
    /// `deletion_batch.rs` for the rationale.
    ///
    /// See `openspec/specs/permanent-deletion-semantics/spec.md` for the
    /// requirement contract this state implements.
    #[doc(hidden)]
    pub(crate) active_deletion_batch: Option<crate::deletion_batch::DeletionBatch>,

    /// Phase 2d Task 7: when a control-flow or iteration step's body parks
    /// a selection, the steps that follow the control-flow step in the
    /// OUTER slice are captured here. Selection-install callbacks drain
    /// this after their own tail completes, resuming the outer slice.
    ///
    /// `None` outside of a parked control-flow continuation. Always cleared
    /// at the bottom of the selection callback that drained it.
    ///
    /// **Invariant: at most one outstanding outer continuation at a time.**
    /// `run_steps` MUST `debug_assert!(self.dsl_outer_tail.is_none())` before
    /// writing — overwriting a `Some` value would silently drop a parked
    /// outer slice and abort the user's still-pending sequence. Today the
    /// dispatcher guarantees this by never re-entering `run_steps` from
    /// within a selection callback before that callback's drain runs (the
    /// callback drains and then the outer slice is gone), but a future
    /// change that allows nested parks (e.g. an `Optional` body whose
    /// inner `If` body itself parks) will need to either (a) make this a
    /// `Vec<(_, _)>` stack, or (b) refuse the second park with a clear
    /// validation error. Don't silently overwrite.
    #[doc(hidden)]
    pub dsl_outer_tail: Option<(
        Vec<digimon_dsl::compiled::CompiledStep>,
        crate::dsl_cards::bindings::Bindings,
        crate::dsl_cards::step::StepRuntime,
    )>,

    /// Cost-pay abort flag — set when the player PASSes on a cost-pay
    /// selection (a `select_hand` / `select_trash` / `select_union_zone`
    /// with `cost: true`). The DSL step runner checks this at the top of
    /// every iteration and short-circuits the rest of the clause body, so a
    /// declined "By trashing X, Y" cost does NOT run Y (no trash, no draw,
    /// no zone-move). Non-cost optional selects (the "you may pick X; then
    /// always do Y" pattern) leave this flag false, so their tails still
    /// run on decline.
    ///
    /// Scope: per `on_decline` invocation in
    /// `effect_queue::resolve_generic_selection` (save+clear on entry,
    /// restore on exit). The save/restore prevents a parent clause's
    /// abort from leaking into an unrelated child clause that fires
    /// downstream of the resolved selection.
    #[doc(hidden)]
    pub(crate) dsl_clause_aborted: bool,

    /// Phase 2f4 Task 1 — one-shot delayed-effect queue. Entries are scheduled
    /// via `EffectContext::schedule_delayed` and drained by
    /// `scheduled_effects::fire_scheduled_for_timing` whose `when:` matches.
    /// Task 2 wires the drain into observer-fire boundaries (turn end, etc.).
    pub scheduled_effects: Vec<crate::scheduled_effects::ScheduledEffect>,
    /// Continuation for a scheduled-effect drain paused by a DSL selection.
    pub scheduled_drain_tail: Option<crate::scheduled_effects::ScheduledDrainTail>,

    /// PUPPETS-G003 — provenance-keyed deletions scheduled for this turn's
    /// end. An effect that plays a Digimon and must delete *that specific
    /// permanent* at turn end ("At turn end, delete the Digimon this effect
    /// played") pushes one entry here, keyed by a stable `ProvenanceToken`
    /// (the played card's identity) rather than a battle-area index that can
    /// shift. Drained by `scheduled_effects::fire_scheduled_provenance_deletions`
    /// from `fire_end_of_your_turn`; the queue is cleared each turn so a
    /// played permanent that already left is a silent no-op.
    pub scheduled_provenance_deletions: Vec<crate::scheduled_effects::ScheduledProvenanceDeletion>,
    /// PUPPETS-G016 — provenance-keyed deletions scheduled for the end of the
    /// **opponent's** turn. Mirror of `scheduled_provenance_deletions` but
    /// drained from `rotate_turn_player` (after `EndOfOpponentsTurn` observers
    /// and scheduled-effect drains) rather than from `fire_end_of_your_turn`.
    /// Used by P-165 ShoeShoemon ("At the end of your opponent's turn, delete
    /// that token").
    pub scheduled_provenance_deletions_opp:
        Vec<crate::scheduled_effects::ScheduledProvenanceDeletion>,
    /// Continuation for a delayed-option lifecycle paused by a DelayEffect or
    /// delete/replacement selection. Re-entered from `resolve_selection`.
    pub(crate) pending_delayed_option_lifecycle: Option<DelayedOptionLifecycleResume>,
    pub(crate) pending_delayed_option_lifecycle_stack: Vec<DelayedOptionLifecycleResume>,
    /// Continuation for the regular EndTurn state machine when an
    /// EndOfYourTurn effect parks a player selection.
    pub(crate) pending_end_turn_resume: Option<EndTurnResume>,

    /// Deferred-drain depth counter (post-2026-05-23 architectural change).
    ///
    /// When non-zero, `fire_on_*` observer helpers must NOT inline-drain the
    /// effect queue — they should only enqueue, letting whichever outer
    /// scope (the select callback or the outer-tail runner that incremented
    /// the counter) flush the queue when it exits.
    ///
    /// Mirrors DCGO's pattern (`DCGO/Assets/Scripts/Script/CardController.cs`
    /// `IAddSecurity.AddSecurity()`): trigger enqueue is separate from drain,
    /// and the drain happens at explicit checkpoints rather than at every
    /// observer-fire site. Prevents the nested-park collision documented at
    /// `qa/archetype-qa/engine-gaps.md` `G-DSL-OUTER-TAIL-NESTED-PARK`:
    /// `fire_on_place_security`'s inline `drain_effect_queue()` previously
    /// fired a second copy of the same triggered effect mid-callback while
    /// `dsl_outer_tail` was still occupied by the first.
    ///
    /// Incremented by `enter_deferred_drain()` and decremented by
    /// `exit_deferred_drain_and_flush()`. The two are intentionally paired —
    /// the latter flushes on the final exit (counter going 1 → 0) so any
    /// triggers queued during the deferred scope fire now, at a context
    /// where `dsl_outer_tail` is empty.
    #[doc(hidden)]
    pub(crate) draining_deferred: u32,

    until_condition_dirty: bool,
    until_condition_last_cycle_evaluations: usize,
    until_condition_total_evaluations: u64,
    until_condition_reevaluation_cycles: u64,

    /// Supplier of opaque-deck reveals. `Some` when at least one player
    /// has `opaque_deck_state == Some(_)` (set by
    /// `Game::new_with_opaque_opponent`). The engine calls
    /// `RevealSource::next_reveal` whenever it would draw, mill, or pop
    /// security from an opaque player's pile.
    ///
    /// See `crate::opaque_deck` for the trait contract. `Game` is not
    /// `Clone`, so the non-`Clone` `Box<dyn RevealSource>` doesn't impose
    /// any constraint on snapshotting.
    pub reveal_source: Option<Box<dyn crate::opaque_deck::RevealSource>>,

    /// Cached card-id → data-index lookup, materialized once at
    /// construction time. Used by opaque-mode draws to turn revealed
    /// card-id strings into `CardSource` instances. Empty for non-opaque
    /// games (the standard constructor uses an in-scope `data_index_map`
    /// directly without caching it on the Game).
    pub(crate) opaque_data_index_map: Option<std::collections::HashMap<String, usize>>,
}

impl Game {
    /// Create a new game with the given decks and rules.
    /// `deck_card_ids` is one deck per player, each a list of card_id strings.
    /// `all_card_data` is the full card database.
    pub fn new(
        deck_card_ids: &[Vec<String>],
        all_card_data: &std::collections::HashMap<String, CardData>,
        rules: Rules,
        seed: Option<u64>,
    ) -> Result<Self, String> {
        if deck_card_ids.len() != rules.player_count as usize {
            return Err(format!(
                "Expected {} decks, got {}",
                rules.player_count,
                deck_card_ids.len()
            ));
        }

        let mut rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };

        let mut effective_card_data = all_card_data.clone();
        #[cfg(feature = "dsl-yaml-loader")]
        let mut alt_path_registry: HashMap<
            String,
            Vec<digimon_dsl::compiled::CompiledAltPath>,
        > = HashMap::new();
        #[cfg(feature = "dsl-yaml-loader")]
        if let Ok(dsl_registry) = crate::dsl_registry::from_embedded() {
            crate::dsl_bridge::enrich_card_data_with_dsl_alt_paths(
                &mut effective_card_data,
                &dsl_registry,
            );
            alt_path_registry = dsl_registry
                .iter()
                .filter_map(|(card_id, compiled)| {
                    if compiled.alt_paths.is_empty() {
                        None
                    } else {
                        Some((card_id.clone(), compiled.alt_paths.clone()))
                    }
                })
                .collect();
        }

        // Build card data store (flat vec, indexed by position)
        let mut card_data_store: Vec<CardData> = Vec::new();
        let mut data_index_map: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for (card_id, data) in &effective_card_data {
            let idx = card_data_store.len();
            data_index_map.insert(card_id.clone(), idx);
            card_data_store.push(data.clone());
        }

        let mut next_card_index: u16 = 0;

        // Create players and populate decks
        let mut players = Vec::with_capacity(rules.player_count as usize);
        for (player_idx, deck_ids) in deck_card_ids.iter().enumerate() {
            let player_id = player_idx as PlayerId;
            let mut player = Player::new(player_id);
            let mut original_deck_counts: BTreeMap<String, (u16, bool)> = BTreeMap::new();

            for card_id in deck_ids {
                let data_idx = data_index_map
                    .get(card_id)
                    .ok_or_else(|| format!("Card {} not found in card database", card_id))?;

                let card_data = &card_data_store[*data_idx];
                let card = CardSource::new(*data_idx, player_id, next_card_index);
                next_card_index += 1;

                // Route to correct deck based on card kind
                if card_data.card_kind == crate::enums::CardKind::DigiEgg {
                    player.digitama_deck.push(card);
                    let entry = original_deck_counts
                        .entry(card_id.clone())
                        .or_insert((0, true));
                    entry.0 += 1;
                    entry.1 = true;
                } else {
                    player.deck.push(card);
                    let entry = original_deck_counts
                        .entry(card_id.clone())
                        .or_insert((0, false));
                    entry.0 += 1;
                }
            }

            player.original_deck = original_deck_counts
                .into_iter()
                .map(|(card_id, (count, is_digitama))| OriginalDeckCardCount {
                    card_id,
                    count,
                    is_digitama,
                })
                .collect();

            // Shuffle decks
            player.shuffle_deck(&mut rng);
            player.shuffle_digitama_deck(&mut rng);

            players.push(player);
        }

        // Initial turn order / coin flip. Seeded games use a backend-neutral
        // starting-player selection so Rust/Python parity does not depend on
        // matching RNG algorithms. Preserve the historical RNG consumption for
        // seeded games so deck/mulligan reshuffles remain stable.
        let mut turn_order: Vec<PlayerId> = (0..rules.player_count).collect();
        if let Some(s) = seed {
            let mut consumed_turn_order = turn_order.clone();
            consumed_turn_order.shuffle(&mut rng);
            if !turn_order.is_empty() {
                let start = (s as usize) % turn_order.len();
                turn_order.rotate_left(start);
            }
        } else {
            turn_order.shuffle(&mut rng);
        }
        let memory_pair = if turn_order.len() >= 2 {
            (turn_order[0], turn_order[1])
        } else {
            (turn_order[0], turn_order[0])
        };

        let player_count = rules.player_count as usize;
        let mulligan_pending = turn_order.clone();
        let mulligan_used = vec![false; player_count];

        // Build the token registry and absorb synthetic CardData rows
        // for each registered token. This extends `card_data_store` with
        // rows whose `card_id` matches `TokenDef::card_id`
        // (e.g. "TOKEN_PETRIFICATION") — `EffectContext::play_token`
        // uses those card_ids to look up the data_index when spawning a
        // token. Tokens never appear in a player's deck, so pushing here
        // does not affect the data_index_map used during deck seeding.
        let token_registry = crate::token_registry::build_registry();
        for def in token_registry.iter() {
            card_data_store.push(def.to_card_data());
        }

        let mut game = Self {
            rules,
            players,
            turn_count: 0,
            n_digivolutions: [0u32, 0u32],
            n_dna_digivolutions: [0u32, 0u32],
            n_digivolve_driven_attacks: [0u32, 0u32],
            digimon_attacks_this_turn: [0u32, 0u32],
            current_phase: GamePhase::Mulligan,
            memory: 0,
            memory_pair,
            turn_order,
            turn_player_idx: 0,
            game_over: false,
            winner: None,
            terminal_outcome_reason: None,
            card_data: card_data_store,
            #[cfg(feature = "dsl-yaml-loader")]
            alt_path_registry,
            modifiers: ModifierRegistry::new(),
            floating_mass_modifiers: Vec::new(),
            effect_registry: build_registry(),
            formula_extensions: FormulaExtensionRegistry::empty(),
            token_registry,
            rng,
            next_card_index,
            mulligan_pending,
            mulligan_used,
            revealed_cards: Vec::new(),
            pending_selection: None,
            effect_queue: EffectQueue::new(),
            pending_granted_fires: Vec::new(),
            granted_effect_bodies: crate::modifiers::GrantedEffectBodyRegistry::default(),
            next_granted_effect_id: 0,
            pending_pay_cost_effect: None,
            pending_pay_cost_stack: Vec::new(),
            pending_attack: None,
            pending_security: None,
            pending_effect_security_removal: Vec::new(),
            pending_option: None,
            pending_digixros_transaction: None,
            digixros_leaving_limbo: Vec::new(),
            active_digixros_wildcards: Vec::new(),
            pending_option_placed_turn_check: false,
            pending_option_placed_link_resume: None,
            security_resolution: None,
            effect_chain_depth: 0,
            effect_drain_depth: 0,
            logger: Box::new(SilentLogger),
            events: Vec::new(),
            event_seq: 0,
            replacement_depth: 0,
            replacement_pending_outcome: None,
            last_play_order_choice: None,
            pending_would_play_resume: None,
            pending_assembly_materials: None,
            pending_would_link_resume: None,
            pending_would_digivolve_resume: None,
            player_digivolve_cost_reducers: Vec::new(),
            pending_player_digivolve_reduction: 0,
            replacement_fired: std::collections::HashSet::new(),
            in_replacement_commit: false,
            effect_source_player: None,
            effect_source_card: None,
            effect_source_permanent: None,
            current_trigger_context: None,
            current_deletion_cause: None,
            current_deletion_event_cause_override: None,
            pending_overclock_attack: None,
            declined_overclock_this_eot: HashSet::new(),
            current_dna_origin: None,
            parked_replacement: None,
            dsl_replacement_outcome: None,
            in_counter_window: false,
            active_deletion_batch: None,
            dsl_outer_tail: None,
            dsl_clause_aborted: false,
            scheduled_effects: Vec::new(),
            scheduled_drain_tail: None,
            scheduled_provenance_deletions: Vec::new(),
            scheduled_provenance_deletions_opp: Vec::new(),
            pending_delayed_option_lifecycle: None,
            pending_delayed_option_lifecycle_stack: Vec::new(),
            pending_end_turn_resume: None,
            draining_deferred: 0,
            until_condition_dirty: false,
            until_condition_last_cycle_evaluations: 0,
            until_condition_total_evaluations: 0,
            until_condition_reevaluation_cycles: 0,
            reveal_source: None,
            opaque_data_index_map: None,
        };

        // Deal starting hands. Security is deliberately NOT laid here — it
        // waits until mulligan finalizes, so a player who mulligans has the
        // full deck to re-shuffle into (matches Python setup order).
        for i in 0..player_count {
            game.players[i].draw_many(game.rules.starting_hand);
        }

        Ok(game)
    }

    /// Reset this game's **mutable** state in place to a clean, pre-deal
    /// state — reusing the already-built immutable shared state
    /// (`card_data`, `effect_registry`, `formula_extensions`,
    /// `token_registry`, `alt_path_registry`), `rules`, and the installed
    /// `logger`. This is the cheap foundation for replay reset-and-replay
    /// (see `runners::replay`): backward seek resets in place and replays
    /// forward instead of reconstructing via `Game::new` (which would clone
    /// every `CardData` and rebuild every registry).
    ///
    /// A full-state snapshot of `Game` is not possible — the mutable graph
    /// is closure-bearing (`ModifierEntry` is non-`Clone`,
    /// `pending_selection` carries a boxed callback, several parked
    /// continuations hold closures). Reset-and-replay sidesteps that.
    ///
    /// **Maintenance invariant:** every mutable field added to `Game` MUST
    /// be reset here to its `Game::new` default. The
    /// `reset_for_replay_restores_defaults` guard test asserts this; keep
    /// the two in lockstep. Fields intentionally preserved: `rules`,
    /// `card_data`, `alt_path_registry`, `effect_registry`,
    /// `formula_extensions`, `token_registry`, `logger`.
    pub fn reset_for_replay(&mut self) {
        let player_count = self.rules.player_count as usize;
        // Fresh players — zones are re-laid by the replay relay step.
        self.players = (0..player_count)
            .map(|i| Player::new(i as PlayerId))
            .collect();
        self.turn_count = 0;
        self.n_digivolutions = [0u32, 0u32];
        self.n_dna_digivolutions = [0u32, 0u32];
        self.n_digivolve_driven_attacks = [0u32, 0u32];
        self.digimon_attacks_this_turn = [0u32, 0u32];
        self.current_phase = GamePhase::Mulligan;
        self.memory = 0;
        let turn_order: Vec<PlayerId> = (0..self.rules.player_count).collect();
        self.memory_pair = if turn_order.len() >= 2 {
            (turn_order[0], turn_order[1])
        } else {
            (turn_order[0], turn_order[0])
        };
        self.turn_order = turn_order;
        self.turn_player_idx = 0;
        self.game_over = false;
        self.winner = None;
        self.terminal_outcome_reason = None;
        // card_data / alt_path_registry / effect_registry / formula_extensions
        // / token_registry intentionally preserved (immutable shared state).
        self.modifiers = ModifierRegistry::new();
        self.floating_mass_modifiers = Vec::new();
        // Reseed deterministically — mirrors the historical backward-seek
        // rebuild path (`Game::new(.., Some(0))`).
        self.rng = StdRng::seed_from_u64(0);
        self.next_card_index = 0;
        self.mulligan_pending = Vec::new();
        self.mulligan_used = vec![false; player_count];
        self.revealed_cards = Vec::new();
        self.pending_selection = None;
        self.effect_queue = EffectQueue::new();
        self.pending_granted_fires = Vec::new();
        self.granted_effect_bodies = crate::modifiers::GrantedEffectBodyRegistry::default();
        self.next_granted_effect_id = 0;
        self.pending_pay_cost_effect = None;
        self.pending_pay_cost_stack = Vec::new();
        self.pending_attack = None;
        self.pending_security = None;
        self.pending_effect_security_removal = Vec::new();
        self.pending_option = None;
        self.pending_option_placed_turn_check = false;
        self.pending_option_placed_link_resume = None;
        self.security_resolution = None;
        self.effect_chain_depth = 0;
        // logger intentionally preserved (session owns the logger choice).
        self.events = Vec::new();
        self.event_seq = 0;
        self.replacement_depth = 0;
        self.replacement_pending_outcome = None;
        self.last_play_order_choice = None;
        self.pending_would_play_resume = None;
        self.pending_would_link_resume = None;
        self.pending_would_digivolve_resume = None;
        self.player_digivolve_cost_reducers = Vec::new();
        self.pending_player_digivolve_reduction = 0;
        self.replacement_fired = std::collections::HashSet::new();
        self.in_replacement_commit = false;
        self.effect_source_player = None;
        self.effect_source_card = None;
        self.effect_source_permanent = None;
        self.current_trigger_context = None;
        self.current_deletion_cause = None;
        self.current_deletion_event_cause_override = None;
        self.pending_overclock_attack = None;
        self.declined_overclock_this_eot = HashSet::new();
        self.current_dna_origin = None;
        self.parked_replacement = None;
        self.dsl_replacement_outcome = None;
        self.in_counter_window = false;
        self.active_deletion_batch = None;
        self.dsl_outer_tail = None;
        self.dsl_clause_aborted = false;
        self.scheduled_effects = Vec::new();
        self.scheduled_drain_tail = None;
        self.scheduled_provenance_deletions = Vec::new();
        self.scheduled_provenance_deletions_opp = Vec::new();
        self.pending_delayed_option_lifecycle = None;
        self.pending_delayed_option_lifecycle_stack = Vec::new();
        self.pending_end_turn_resume = None;
        self.draining_deferred = 0;
        self.until_condition_dirty = false;
        self.until_condition_last_cycle_evaluations = 0;
        self.until_condition_total_evaluations = 0;
        self.until_condition_reevaluation_cycles = 0;
        self.reveal_source = None;
        self.opaque_data_index_map = None;
    }

    /// Create a new game where one player's deck composition is known but
    /// its order is hidden — the engine consults `reveal_source` whenever
    /// it would draw from that player's pile. Used by the DCGO replay
    /// harness when replaying PvP recordings (where the local client only
    /// observes the opponent's reveals incrementally).
    ///
    /// ## Arguments
    /// - `my_player_id`: which side (0 or 1) gets the standard ordered
    ///   `my_deck`. The other player becomes opaque.
    /// - `my_deck`: ordered deck for the calling player (drawn from
    ///   index 0 first — standard `Game::new` semantics).
    /// - `opp_decklist`: unordered card-ID multiset for the opaque
    ///   opponent. Must equal the standard deck size for `rules` and
    ///   contain only card IDs known to `all_card_data`.
    /// - `reveal_source`: supplier the engine calls whenever it needs a
    ///   card from the opaque pile. The replay harness preloads a
    ///   `RevealQueue` with reveals observed from the recording.
    ///
    /// ## Integration scope (Phase 1)
    ///
    /// This constructor wires opaque draws into:
    /// - Initial-hand setup (the `draw_many` loop at end of `new`).
    /// - Mulligan redraws (via the same `draw_many` path post-redraw).
    /// - Security setup (`setup_security`).
    ///
    /// Mid-game draw paths (per-turn draw, effect-driven mill/draw/peek)
    /// are NOT yet integrated. Games that trigger those paths against the
    /// opaque player will panic with a clear "opaque mode + this draw
    /// path not yet supported" message rather than silently consume from
    /// the empty `deck` Vec. See task 6.6 follow-up.
    pub fn new_with_opaque_opponent(
        my_player_id: PlayerId,
        my_deck: Vec<String>,
        opp_decklist: Vec<String>,
        reveal_source: Box<dyn crate::opaque_deck::RevealSource>,
        all_card_data: &std::collections::HashMap<String, CardData>,
        rules: Rules,
        seed: Option<u64>,
    ) -> Result<Self, String> {
        // Standard player count is 2; opaque mode is undefined for >2 player
        // formats and explicitly rejected here.
        if rules.player_count != 2 {
            return Err(format!(
                "opaque-opponent-deck mode requires rules.player_count == 2, got {}",
                rules.player_count
            ));
        }
        if my_player_id >= 2 {
            return Err(format!("my_player_id must be 0 or 1, got {}", my_player_id));
        }

        // Validate the opponent decklist size matches what the rules expect.
        // (Game::new fails downstream if a deck is malformed; we surface
        // this earlier with a clearer message for the opaque path.)
        let expected_deck_size = my_deck.len();
        if opp_decklist.len() != expected_deck_size {
            return Err(format!(
                "opponent decklist size {} differs from calling-player deck size {}; \
                 both must equal the rules-mandated deck size",
                opp_decklist.len(),
                expected_deck_size
            ));
        }
        // Validate every card ID in the opponent decklist is known to the
        // card pool. (Game::new validates this for the in-order deck path
        // implicitly via the data_index_map lookup; we duplicate the check
        // here so opaque-mode failures surface at construction time rather
        // than at the first reveal.)
        for card_id in &opp_decklist {
            if !all_card_data.contains_key(card_id) {
                return Err(format!(
                    "opponent decklist contains unknown card ID `{}`",
                    card_id
                ));
            }
        }

        // For Game::new's standpoint, both decks must be provided. We
        // supply the opaque opponent a deterministic placeholder deck of
        // their declared composition — Game::new will populate
        // `player.deck` and shuffle it, then we immediately clear it
        // below and install the opaque state. This is the simplest
        // integration that reuses Game::new's effect-registry setup
        // and avoids forking the constructor.
        let placeholder_opp_deck = opp_decklist.clone();
        let decks_for_constructor = if my_player_id == 0 {
            vec![my_deck.clone(), placeholder_opp_deck]
        } else {
            vec![placeholder_opp_deck, my_deck.clone()]
        };

        let mut game = Self::new(&decks_for_constructor, all_card_data, rules, seed)?;

        // Replace the opponent's ordered deck with opaque state, and drop
        // the placeholder hand draws — opaque mode redraws via the
        // reveal source below.
        let opp_id = if my_player_id == 0 { 1u8 } else { 0u8 };
        let opp_idx = opp_id as usize;

        // Clear out the standard-path setup the opponent just received.
        // We're about to redraw their starting hand from the reveal source.
        // Also dump their digitama deck to be re-populated — wait, actually
        // digitama is dealt separately and is part of the decklist already
        // (4-5 DigiEggs interleaved). For Phase 1 we preserve the digitama
        // deck as-is (it's already shuffled and held in
        // `player.digitama_deck`) — opaque mode for digitama isn't in scope.
        let starting_hand = game.rules.starting_hand;
        game.players[opp_idx].hand.clear();
        game.players[opp_idx].deck.clear();

        // Cache the card-id → data_index lookup so opaque-mode reveals can
        // materialize CardSources.
        let mut data_index_map = std::collections::HashMap::new();
        for (i, card) in game.card_data.iter().enumerate() {
            data_index_map.insert(card.card_id.clone(), i);
        }
        game.opaque_data_index_map = Some(data_index_map);

        // Install the opaque deck state. Note: the digitama cards have
        // already been pulled out of the decklist by Game::new's per-card
        // routing (card_kind == DigiEgg goes to digitama_deck, others go
        // to deck). The opaque-deck multiset should reflect ONLY the
        // non-digitama portion to stay consistent.
        let non_digitama: Vec<String> = opp_decklist
            .iter()
            .filter(|id| {
                let data = match all_card_data.get(*id) {
                    Some(d) => d,
                    None => return false, // already validated above; unreachable
                };
                data.card_kind != crate::enums::CardKind::DigiEgg
            })
            .cloned()
            .collect();
        game.players[opp_idx].opaque_deck_state = Some(
            crate::opaque_deck::OpaqueDeckState::from_decklist(&non_digitama),
        );

        game.reveal_source = Some(reveal_source);

        // Redraw the opaque opponent's starting hand from the reveal
        // source. This is the first place real reveals get consumed.
        for _ in 0..starting_hand {
            game.draw_one_for_player(opp_id)?;
        }

        Ok(game)
    }

    /// Draw one card for `pid`. Branches on opaque mode: when
    /// `player.opaque_deck_state` is `Some`, consumes from the reveal
    /// source; otherwise pops from the ordered deck.
    ///
    /// Returns `Ok(true)` on successful draw, `Ok(false)` on deck-out
    /// (no cards remaining — either Vec is empty or opaque multiset is
    /// zero), or `Err(message)` for a reveal-source error.
    ///
    /// This is the single chokepoint setup-time draws use. Effect-driven
    /// draws still go through `Player::draw` directly — when one of those
    /// fires for an opaque player it will see an empty `deck` Vec and
    /// return false, which the engine treats as deck-out. That's the
    /// "panic with a clear message" deferred until task 6.6 follow-up;
    /// for now it manifests as the engine declaring deck-out, which is
    /// at least an obvious symptom.
    pub fn draw_one_for_player(&mut self, pid: PlayerId) -> Result<bool, String> {
        if (pid as usize) >= self.players.len() {
            return Err(format!("invalid player id {}", pid));
        }
        if self.players[pid as usize].opaque_deck_state.is_none() {
            // Standard path.
            return Ok(self.players[pid as usize].draw());
        }
        self.reveal_into_hand(pid, crate::opaque_deck::RevealKind::Draw)
    }

    /// Internal: request one reveal for `pid` and append the materialized
    /// CardSource to that player's hand. Used by `draw_one_for_player`
    /// in opaque mode.
    fn reveal_into_hand(
        &mut self,
        pid: PlayerId,
        kind: crate::opaque_deck::RevealKind,
    ) -> Result<bool, String> {
        let card = self.materialize_reveal(pid, kind)?;
        self.players[pid as usize].hand.push(card);
        Ok(true)
    }

    /// General-purpose "take one card from the top of `pid`'s deck" that
    /// branches on opaque mode. The caller decides which zone to push the
    /// returned CardSource into (trash for mill, revealed-list for peek,
    /// etc.) — this helper does NOT route to hand.
    ///
    /// Returns `Ok(Some(card))` on success, `Ok(None)` on deck-out (the
    /// standard-mode Vec is empty, or the opaque multiset is depleted —
    /// the latter currently surfaces as Err but may become Ok(None) once
    /// the engine has a typed deck-out vs reveal-error split).
    ///
    /// Used by effect-driven draws/mills/peeks that consume from the deck
    /// top but don't go through the hand. Each call site picks the right
    /// `kind` — `Mill` for trash-from-top, `Effect` for peek-and-reveal,
    /// `Draw` for "draw to hand" semantics (though `draw_one_for_player`
    /// is the convenience wrapper for that case).
    pub fn take_from_deck_top_for_player(
        &mut self,
        pid: PlayerId,
        kind: crate::opaque_deck::RevealKind,
    ) -> Result<Option<CardSource>, String> {
        if (pid as usize) >= self.players.len() {
            return Err(format!("invalid player id {}", pid));
        }
        if self.players[pid as usize].opaque_deck_state.is_none() {
            // Standard path — just pop from the ordered deck Vec.
            return Ok(self.players[pid as usize].deck.pop());
        }
        // Opaque path — materialize from reveal source.
        let card = self.materialize_reveal(pid, kind)?;
        Ok(Some(card))
    }

    /// Internal: request one reveal for `pid` and materialize a
    /// `CardSource`, without inserting it into any zone. The caller
    /// chooses which zone to push to (hand for draw, security for
    /// security-setup, trash for mill, etc.).
    fn materialize_reveal(
        &mut self,
        pid: PlayerId,
        kind: crate::opaque_deck::RevealKind,
    ) -> Result<CardSource, String> {
        let card_id = {
            let source = self
                .reveal_source
                .as_mut()
                .ok_or_else(|| "opaque mode but no reveal_source on Game".to_string())?;
            match source.next_reveal(kind) {
                Ok(c) => c,
                Err(e) => return Err(e.to_string()),
            }
        };

        // Consume from the player's multiset.
        let state = self.players[pid as usize]
            .opaque_deck_state
            .as_mut()
            .expect("caller verified opaque mode");
        if let Err(e) = state.consume(&card_id) {
            return Err(e.to_string());
        }

        // Materialize a CardSource from the card_id.
        let data_idx = self
            .opaque_data_index_map
            .as_ref()
            .and_then(|m| m.get(&card_id))
            .copied()
            .ok_or_else(|| {
                format!(
                    "opaque reveal returned card_id `{}` but it's not in the data index map",
                    card_id
                )
            })?;
        let card_index = self.next_card_index;
        self.next_card_index += 1;
        Ok(CardSource::new(data_idx, pid, card_index))
    }

    /// Set up `count` security cards for `pid`, branching on opaque mode.
    /// In standard mode, defers to `Player::setup_security`. In opaque
    /// mode, pushes `count` **placeholder** CardSources — their identities
    /// are materialized **lazily** when SecurityCheck flips them. This
    /// matches DCGO's PvP information model: the local client doesn't
    /// know the opponent's security cards' identities at setup time, only
    /// when they're flipped during gameplay.
    ///
    /// The opaque pile's multiset is also debited by `count` here, since
    /// these N cards are physically in the security stack (just hidden).
    /// When the placeholder is later materialized via
    /// [`Self::materialize_opaque_security_placeholder`], the multiset
    /// has already been debited and no further change happens — the
    /// materialization just resolves identity.
    pub fn setup_security_for_player(&mut self, pid: PlayerId, count: u8) -> Result<(), String> {
        if (pid as usize) >= self.players.len() {
            return Err(format!("invalid player id {}", pid));
        }
        if self.players[pid as usize].opaque_deck_state.is_none() {
            // Standard path.
            self.players[pid as usize].setup_security(count);
            return Ok(());
        }
        // Opaque path: push placeholders, debit the multiset for the
        // count (the cards exist in security; we just don't know which).
        // The multiset is debited by `count` total via repeated
        // pop-from-multiset-without-supplier — but we don't actually
        // know WHICH cards moved to security. The honest accounting:
        // decrement `total_remaining` by `count`, leave per-card counts
        // alone. When a placeholder is later materialized with a
        // specific card_id, that card's multiset count gets decremented
        // at that point. This double-counts slightly: total_remaining
        // is decremented at setup, then again at flip-materialization.
        // We compensate by NOT decrementing total in
        // materialize_opaque_security_placeholder.
        let state = self.players[pid as usize]
            .opaque_deck_state
            .as_mut()
            .expect("checked above");
        // Decrement total_remaining via a public helper to avoid
        // bypassing accounting. (OpaqueDeckState exposes restore()
        // but not direct count manipulation — add a "debit_without_id"
        // method or thread accordingly.) For now we use the existing
        // multiset structure: we can't decrement per-card without
        // knowing which cards, so instead we track a separate
        // "placeholder count" on the state that subtracts from the
        // effective total. See OpaqueDeckState::reserve_placeholders.
        state.reserve_placeholders(count as usize);

        for _ in 0..count {
            let card_index = self.next_card_index;
            self.next_card_index += 1;
            self.players[pid as usize]
                .security
                .push(CardSource::new_opaque_security_placeholder(pid, card_index));
        }
        Ok(())
    }

    /// Convenience wrapper around [`materialize_opaque_security_placeholder`]
    /// for effect-driven security access sites — the common pattern of
    /// "I'm about to remove security[idx]; if it's an opaque placeholder,
    /// resolve its identity first so subsequent reads of its fields
    /// (card_id, color, type, replacement-effect lookups, observer
    /// firings) see real data instead of garbage `data_index = 0`."
    ///
    /// Silently no-ops on:
    ///   - invalid pid / idx (caller is about to surface that anyway)
    ///   - non-opaque player (standard-mode security has no placeholders)
    ///   - already-materialized placeholder (idempotent inner helper)
    ///
    /// Errors from the underlying materialization are logged-and-ignored
    /// (the calling effect proceeds with garbage data, replay surfaces
    /// the divergence). Lifting these to typed errors would require
    /// fallible signatures on every effect-driven security path — out
    /// of scope for the Phase-1-lazy mechanism.
    pub fn ensure_security_materialized(&mut self, pid: PlayerId, security_idx: usize) {
        if (pid as usize) >= self.players.len() {
            return;
        }
        let needs = self.players[pid as usize]
            .security
            .get(security_idx)
            .map(|c| c.is_opaque_placeholder)
            .unwrap_or(false);
        if !needs {
            return;
        }
        if let Err(e) = self.materialize_opaque_security_placeholder(pid, security_idx) {
            eprintln!(
                "[opaque-deck] effect-driven security materialize error for player {} \
                 idx {}: {}",
                pid, security_idx, e
            );
        }
    }

    /// Materialize a placeholder security card at position `security_idx`
    /// by consuming a `RevealKind::Security` reveal from the source. The
    /// placeholder's `card_index` is preserved (face_up_security tracking
    /// stays consistent); all other fields are overwritten.
    ///
    /// No-op if the position doesn't hold a placeholder (already
    /// materialized, or in standard-mode game).
    ///
    /// Called by the engine's security-pop path BEFORE reading the
    /// security card's data: when about to flip security[0], if it's a
    /// placeholder, materialize first, then proceed with normal flip
    /// semantics.
    pub fn materialize_opaque_security_placeholder(
        &mut self,
        pid: PlayerId,
        security_idx: usize,
    ) -> Result<bool, String> {
        if (pid as usize) >= self.players.len() {
            return Err(format!("invalid player id {}", pid));
        }
        if self.players[pid as usize].opaque_deck_state.is_none() {
            return Ok(false);
        }
        let needs_materialization = self.players[pid as usize]
            .security
            .get(security_idx)
            .map(|c| c.is_opaque_placeholder)
            .unwrap_or(false);
        if !needs_materialization {
            return Ok(false);
        }

        // Pull the next Security reveal from the source.
        let card_id = {
            let source = self
                .reveal_source
                .as_mut()
                .ok_or_else(|| "opaque security flip but no reveal_source on Game".to_string())?;
            source
                .next_reveal(crate::opaque_deck::RevealKind::Security)
                .map_err(|e| e.to_string())?
        };
        // Consume from the per-card multiset. Note: total_remaining was
        // already debited at setup_security_for_player via
        // reserve_placeholders, so this consume call only decrements
        // the per-card slot, not total_remaining.
        {
            let state = self.players[pid as usize]
                .opaque_deck_state
                .as_mut()
                .expect("checked above");
            state
                .consume_per_card_only(&card_id)
                .map_err(|e| e.to_string())?;
        }
        // Look up data_index and overwrite the placeholder's fields,
        // preserving card_index for face_up_security continuity.
        let data_idx = self
            .opaque_data_index_map
            .as_ref()
            .and_then(|m| m.get(&card_id))
            .copied()
            .ok_or_else(|| {
                format!(
                    "security reveal returned card_id `{}` but it's not in the data index map",
                    card_id
                )
            })?;
        let slot = &mut self.players[pid as usize].security[security_idx];
        slot.data_index = data_idx;
        slot.is_opaque_placeholder = false;
        // face_down stays true (security is still face-down post-materialization
        // until it's actually flipped to face-up via face_up_security set
        // mutation; the engine's existing flip path handles that).
        Ok(true)
    }

    /// Mulligan-path helper: fold the opaque opponent's hand back into
    /// the opaque pile (restoring multiset counts), clear the hand, and
    /// redraw `starting_hand` cards via the reveal source.
    ///
    /// This is the opaque counterpart to the standard `redraw_hand`'s
    /// "drain hand into deck, shuffle, draw N" sequence — the conceptual
    /// equivalent without an ordered deck to shuffle. Reveals fold into
    /// hand in arrival order; consume errors bubble up. Used only when
    /// `players[pid].opaque_deck_state.is_some()`.
    fn redraw_hand_opaque(&mut self, pid: PlayerId) -> Result<(), String> {
        let starting_hand = self.rules.starting_hand;

        // Snapshot hand card IDs before mutating (the borrow checker
        // forbids holding `&card_data` + `&mut players` together).
        let cards_to_restore: Vec<String> = {
            let card_data = &self.card_data;
            self.players[pid as usize]
                .hand
                .iter()
                .map(|c| c.card_id(card_data).to_string())
                .collect()
        };

        // Restore each hand card back into the opaque multiset, then
        // clear the hand.
        {
            let state = self.players[pid as usize]
                .opaque_deck_state
                .as_mut()
                .expect("caller verified opaque mode");
            for card_id in &cards_to_restore {
                state.restore(card_id);
            }
        }
        self.players[pid as usize].hand.clear();

        // Re-draw the starting hand from the reveal source.
        for _ in 0..starting_hand {
            self.draw_one_for_player(pid)?;
        }
        Ok(())
    }

    /// Get the current turn player's ID.
    pub fn turn_player(&self) -> PlayerId {
        self.turn_order[self.turn_player_idx]
    }

    pub fn until_condition_last_cycle_evaluations(&self) -> usize {
        self.until_condition_last_cycle_evaluations
    }

    pub fn until_condition_reevaluation_cycles(&self) -> u64 {
        self.until_condition_reevaluation_cycles
    }

    pub fn mark_until_condition_dirty(&mut self) {
        self.until_condition_dirty = true;
    }

    pub fn reevaluate_until_condition_modifiers_if_dirty(&mut self) {
        if !self.until_condition_dirty {
            return;
        }
        if self.pending_selection.is_some()
            || !self.effect_queue.is_empty()
            || self.effect_chain_depth != 0
        {
            return;
        }
        self.until_condition_dirty = false;
        self.reevaluate_until_condition_modifiers();
    }

    pub fn reevaluate_until_condition_modifiers(&mut self) {
        let candidates = self.modifiers.until_condition_candidates();
        let mut evaluations = 0usize;
        for (install_order, subject) in candidates {
            let keep = self
                .modifiers
                .evaluate_until_condition(subject, install_order, self);
            let Some(keep) = keep else {
                continue;
            };
            evaluations += 1;
            if !keep {
                self.modifiers
                    .remove_until_condition_by_order(subject, install_order);
            }
        }
        self.until_condition_last_cycle_evaluations = evaluations;
        self.until_condition_total_evaluations = self
            .until_condition_total_evaluations
            .saturating_add(evaluations as u64);
        self.until_condition_reevaluation_cycles =
            self.until_condition_reevaluation_cycles.saturating_add(1);
    }

    /// Swap out the game logger (defaults to `SilentLogger`). Callers
    /// that want to capture trace/reject messages should install a
    /// `VerboseLogger` here.
    pub fn set_logger(&mut self, logger: Box<dyn GameLogger>) {
        self.logger = logger;
    }

    // ─── Mulligan ────────────────────────────────────────────────────

    /// The next player expected to make a mulligan decision, or `None` if
    /// mulligan is already complete.
    pub fn mulligan_current_player(&self) -> Option<PlayerId> {
        self.mulligan_pending.first().copied()
    }

    /// Record a mulligan decision for the current deciding player.
    ///
    /// - `keep = true` — keep the drawn hand as-is.
    /// - `keep = false` — shuffle the hand back into the deck, reshuffle,
    ///   draw a fresh `starting_hand`. `mulligan_used[player]` is set so the
    ///   action mask can suppress a second redraw.
    ///
    /// Returns `Err` if it's not this player's turn to decide or if mulligan
    /// is already complete.
    pub fn accept_mulligan(&mut self, player: PlayerId, keep: bool) -> Result<(), &'static str> {
        let Some(current) = self.mulligan_current_player() else {
            return Err("mulligan is already complete");
        };
        if current != player {
            return Err("it is a different player's turn to decide");
        }

        if !keep {
            self.redraw_hand(player);
            self.mulligan_used[player as usize] = true;
        }
        self.mulligan_pending.remove(0);

        if self.mulligan_pending.is_empty() {
            self.finalize_mulligan();
        }
        Ok(())
    }

    /// Shuffle the player's hand back into the deck and redraw `starting_hand`.
    ///
    /// In opaque mode (when `player.opaque_deck_state.is_some()`), the
    /// hand is restored into the opaque multiset and the redraw consumes
    /// `starting_hand` reveals from the supplier rather than popping from
    /// an ordered deck. See [`Self::redraw_hand_opaque`].
    fn redraw_hand(&mut self, player: PlayerId) {
        if self.players[player as usize].opaque_deck_state.is_some() {
            // Opaque mode: defer to the opaque-aware helper. Errors are
            // logged but not propagated — the original `redraw_hand`
            // signature is `-> ()` and lifting that to `Result` is a
            // broader API change. The error case here means the
            // reveal source is misaligned with the engine state, which
            // is a recording-corruption symptom the replay harness
            // will surface as a downstream parity failure.
            if let Err(e) = self.redraw_hand_opaque(player) {
                // Soft-fail: log + leave state best-effort. The hand
                // will be partially populated; subsequent draws will
                // continue to consume from the source.
                eprintln!(
                    "[opaque-deck] redraw_hand error for player {}: {}",
                    player, e
                );
            }
            return;
        }
        let starting_hand = self.rules.starting_hand;
        let p = self.player_mut(player);
        p.deck.extend(p.hand.drain(..));
        // Borrow the game's rng via a local reshuffle: move the cards into a
        // local vec, shuffle with game rng, put back.
        let mut deck = std::mem::take(&mut p.deck);
        deck.shuffle(&mut self.rng);
        self.player_mut(player).deck = deck;
        self.player_mut(player).draw_many(starting_hand);
    }

    /// Finalize mulligan: lay security for every player and begin turn 1.
    ///
    /// Security setup is opaque-aware: opaque players' security cards
    /// are pulled from the reveal source (tagged `RevealKind::Security`),
    /// not from an ordered `deck` Vec. Errors during opaque security
    /// reveal are logged but not propagated for the same reason
    /// `redraw_hand` does the same — the upstream API doesn't yet
    /// surface a fallible `finalize_mulligan` signature.
    fn finalize_mulligan(&mut self) {
        let security_count = self.rules.security_count;
        for i in 0..self.rules.player_count as usize {
            let pid = i as PlayerId;
            if let Err(e) = self.setup_security_for_player(pid, security_count) {
                eprintln!(
                    "[opaque-deck] security setup error for player {}: {}",
                    pid, e
                );
            }
        }
        self.turn_count = 1;
        self.memory = 0;
        self.begin_turn();
    }

    /// Get a reference to a player by ID.
    pub fn player(&self, id: PlayerId) -> &Player {
        &self.players[id as usize]
    }

    /// Get a mutable reference to a player by ID.
    pub fn player_mut(&mut self, id: PlayerId) -> &mut Player {
        &mut self.players[id as usize]
    }

    pub fn owner_of_card(&self, handle: crate::card_source::CardHandle) -> Option<PlayerId> {
        if let Some(owner) = self
            .players
            .iter()
            .find(|player| player.contains_card(handle))
            .map(|player| player.id)
        {
            return Some(owner);
        }
        if let Some(pending) = &self.pending_security {
            if pending.card.handle() == handle {
                return Some(pending.card.owner);
            }
        }
        if let Some(pending) = &self.pending_option {
            if pending.card.handle() == handle {
                return Some(pending.owner);
            }
        }
        self.revealed_cards
            .iter()
            .find(|card| card.handle() == handle)
            .map(|card| card.owner)
    }

    /// Get all non-eliminated opponents of a player.
    pub fn opponents(&self, id: PlayerId) -> Vec<PlayerId> {
        self.turn_order
            .iter()
            .copied()
            .filter(|&pid| pid != id)
            .collect()
    }

    // ─── Pending selection resolution ───────────────────────────────

    /// Resolve a pending selection with `action_id` submitted by `player`.
    ///
    /// Dispatches to `resolve_generic_selection` in `effect_queue.rs`,
    /// which validates, restores the pre-selection phase, invokes the
    /// callback (or `on_decline` for PASS on an optional prompt), and
    /// resumes the effect-queue drainer.
    ///
    /// Works uniformly for every `SelectionKind`: TriggerOrder, OppField,
    /// Hand, Trash, EffectChoice, etc. The callback stored on the
    /// selection does kind-specific decoding.
    pub fn resolve_selection(
        &mut self,
        player: PlayerId,
        action_id: u16,
    ) -> Result<(), SelectionError> {
        if self.pending_selection.is_none() {
            return Err(SelectionError::NoPendingSelection);
        }
        self.resolve_generic_selection(player, action_id)
    }

    /// Mark the currently parked pay-cost effect as declined. Selection
    /// callbacks installed by `pay_cost_fn` can call this after the player
    /// declines or the selected cost cannot be paid; the parked process tail
    /// will be discarded when the selection chain unwinds.
    pub fn decline_pending_pay_cost(&mut self) {
        if let Some(pending) = self.pending_pay_cost_effect.as_mut() {
            pending.declined = true;
        }
    }

    /// Trash a source selected through `EffectContext::select_own_sources`.
    /// Returns `true` when the stable source handle was found and moved.
    pub fn trash_source_ref(&mut self, source_ref: crate::selection::SourceSelectionRef) -> bool {
        let Some(permanent) = self
            .player_mut(source_ref.permanent.player)
            .battle_area
            .get_mut(source_ref.permanent.index as usize)
        else {
            return false;
        };
        let Some(pos) = permanent
            .card_sources
            .iter()
            .position(|source| source.handle() == source_ref.card)
        else {
            return false;
        };
        let removed = permanent.card_sources.remove(pos);
        self.apply_ace_overflow_for_sources(std::slice::from_ref(&removed));
        self.player_mut(source_ref.permanent.player)
            .trash
            .push(removed);
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        // Soft-remove the carrier slot if the trash emptied it. Sibling of
        // the digivolve-from-material fix landed in PR #533. This path is
        // hit by agent-selected "trash 1 of your digivolution sources"
        // effects (Rocks archetype). See
        // `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
        // `qa/archetype-qa/engine-gaps.md`.
        let _ = self.soft_remove_if_emptied(source_ref.permanent);
        true
    }

    pub(crate) fn apply_ace_overflow_for_sources(
        &mut self,
        sources: &[crate::card_source::CardSource],
    ) {
        let penalty: i16 = sources
            .iter()
            .filter(|source| !source.is_token)
            .filter_map(|source| self.card_data.get(source.data_index)?.ace_overflow)
            .map(|value| value as i16)
            .sum();
        if penalty != 0 {
            self.memory += penalty;
        }
    }

    pub fn remove_source_ref(
        &mut self,
        source_ref: crate::selection::SourceSelectionRef,
    ) -> Option<crate::card_source::CardHandle> {
        let permanent = self
            .player_mut(source_ref.permanent.player)
            .battle_area
            .get_mut(source_ref.permanent.index as usize)?;
        let pos = permanent
            .card_sources
            .iter()
            .position(|source| source.handle() == source_ref.card)?;
        if pos + 1 >= permanent.card_sources.len() {
            return None;
        }
        let removed = permanent.card_sources.remove(pos);
        let card = removed.handle();
        self.player_mut(source_ref.permanent.player)
            .hand
            .push(removed);
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        Some(card)
    }

    pub fn play_card_from_effect_without_cost(
        &mut self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
    ) -> Option<PermanentHandle> {
        let hand_index = self
            .player(player_id)
            .hand
            .iter()
            .position(|source| source.handle() == card)?;
        if !self.can_play_card_from_effect_without_cost(player_id, card, 1) {
            return None;
        }

        let turn = self.turn_count;
        let card_source = self.player_mut(player_id).hand.remove(hand_index);
        let perm = crate::permanent::Permanent::new(card_source, turn);
        self.player_mut(player_id).battle_area.push(perm);
        let field_index = self.player(player_id).battle_area.len() - 1;
        let entered = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        let top_card = self.players[player_id as usize].battle_area[field_index].top_card();
        let emitted_card_id = top_card.card_id(&self.card_data).to_string();
        let cost_printed = self.card_data[top_card.data_index].play_cost as i16;
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Play {
            seq,
            player: player_id,
            card_id: emitted_card_id,
            field_index: field_index as u8,
            // Effect-initiated free play — no memory paid.
            cost_paid: 0,
            cost_printed,
            // Generic effect-initiated free play is not a registered
            // alt-path; `via_alt_path` is reserved for
            // CompiledAltPathKind variants.
            via_alt_path: None,
        });
        // Effect-initiated play (`effect_initiated: true`) — helper wraps
        // OnPlay + OnEnterFieldAnyone + OnAllyPlayed in a deferred-drain
        // scope so simultaneous triggers share a TriggerOrder bundle.
        self.fire_play_event_triggers(player_id, field_index, true, false);
        Some(entered)
    }

    pub fn play_source_refs_from_effect_without_cost(
        &mut self,
        selected: Vec<crate::selection::SourceSelectionRef>,
    ) -> bool {
        let mut required_slots_by_player = vec![0usize; self.players.len()];
        for source_ref in &selected {
            let Some(permanent) = self
                .player(source_ref.permanent.player)
                .battle_area
                .get(source_ref.permanent.index as usize)
            else {
                return false;
            };
            let Some(pos) = permanent
                .card_sources
                .iter()
                .position(|source| source.handle() == source_ref.card)
            else {
                return false;
            };
            if pos + 1 >= permanent.card_sources.len() {
                return false;
            }
            let player_index = source_ref.permanent.player as usize;
            let Some(required_slots) = required_slots_by_player.get_mut(player_index) else {
                return false;
            };
            *required_slots += 1;
            if !self.can_play_card_from_effect_without_cost(
                source_ref.permanent.player,
                source_ref.card,
                *required_slots,
            ) {
                return false;
            }
        }

        let mut removed: Vec<(PlayerId, CardSource)> = Vec::with_capacity(selected.len());
        for source_ref in selected {
            let Some(permanent) = self
                .player_mut(source_ref.permanent.player)
                .battle_area
                .get_mut(source_ref.permanent.index as usize)
            else {
                return false;
            };
            let Some(pos) = permanent
                .card_sources
                .iter()
                .position(|source| source.handle() == source_ref.card)
            else {
                return false;
            };
            if pos + 1 >= permanent.card_sources.len() {
                return false;
            }
            removed.push((
                source_ref.permanent.player,
                permanent.card_sources.remove(pos),
            ));
        }

        let turn = self.turn_count;
        let mut entered = Vec::with_capacity(removed.len());
        for (player_id, card_source) in removed {
            let card = card_source.handle();
            let emitted_card_id = card_source.card_id(&self.card_data).to_string();
            let cost_printed = self.card_data[card_source.data_index].play_cost as i16;
            let player = self.player_mut(player_id);
            player
                .battle_area
                .push(crate::permanent::Permanent::new(card_source, turn));
            let field_index = player.battle_area.len() - 1;
            let permanent = PermanentHandle {
                player: player_id,
                index: field_index as u8,
            };
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::Play {
                seq,
                player: player_id,
                card_id: emitted_card_id,
                field_index: field_index as u8,
                // Effect-initiated multi-source free play — no memory paid.
                cost_paid: 0,
                cost_printed,
                via_alt_path: None,
            });
            entered.push((player_id, field_index, permanent, card));
        }

        // Multi-source effect-initiated play. Each entered card fires its
        // own play-event trigger bundle (OnPlay + OnEnterFieldAnyone +
        // OnAllyPlayed) through the helper. The helper's per-call
        // deferred-drain scope opens and closes per entered card, so each
        // card's triggers form their own TriggerOrder bundle — this
        // preserves the "one entry event = one bundle" semantic across
        // multi-source plays.
        //
        // Note: this is a behavior change vs. the previous inline pattern,
        // which drained ALL fire_on_play calls first (one per entered
        // card), then ALL OnEnterFieldAnyone/OnAllyPlayed enqueues for
        // all cards, then a single final drain. That batched-across-cards
        // shape coalesced multi-card-entry triggers into one giant drain.
        // The helper-per-card shape mirrors the single-card play site,
        // and matches DCGO which fires each card's enter-field broadcast
        // before the next card enters.
        for (player_id, field_index, _, _) in entered {
            self.fire_play_event_triggers(player_id, field_index, true, false);
        }
        true
    }

    pub fn can_play_card_from_effect_without_cost(
        &self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
        required_slots: usize,
    ) -> bool {
        let Some(player) = self.players.get(player_id as usize) else {
            return false;
        };
        if player.battle_area.len() + required_slots > self.rules.field_slots as usize {
            return false;
        }
        let Some(card_kind) = self.card_kind_for_handle(card) else {
            return false;
        };
        if card_kind == crate::enums::CardKind::Digimon
            && self.modifiers.player_has(
                player_id,
                crate::enums::ModifierType::CannotPlayDigimonByEffect,
            )
        {
            return false;
        }
        if card_kind == crate::enums::CardKind::Tamer
            && self.modifiers.player_has(
                player_id,
                crate::enums::ModifierType::CannotPlayTamerByEffect,
            )
        {
            return false;
        }
        true
    }

    pub fn card(&self, card: crate::card_source::CardHandle) -> &CardData {
        self.card_data_for_handle(card)
            .expect("card handle must resolve to card data")
    }

    /// Fire all applicable replacement effects for the given would-event.
    /// Returns the final `ReplacementOutcome` the caller must honor.
    ///
    /// **Invariant:** if this returns `ReplacementOutcome::None`, no side
    /// effects have been applied to `Game` state. If it returns any other
    /// variant, side effects from the chosen replacements have already
    /// committed and the caller must NOT re-apply the original event.
    ///
    /// **Optional replacements:** if an optional replacement is in scope,
    /// this function installs a `PendingSelection::Replacement` and returns
    /// `ReplacementOutcome::None`. The caller is expected to re-enter
    /// `try_replace` — or inspect `game.replacement_pending_outcome` —
    /// once `resolve_selection` has fired.
    ///
    /// Visibility note: `#[doc(hidden)] pub` rather than `pub(crate)` so the
    /// Phase 7 integration tests under `digimon-engine/tests/replacements/`
    /// can drive the dispatcher directly. Fire-sites inside the crate (Task
    /// 3+) will call this via normal crate-local dispatch.
    #[doc(hidden)]
    pub fn try_replace(
        &mut self,
        timing: crate::enums::EffectTiming,
        subject: crate::replacement::ReplacementSubject,
        cause: crate::replacement::ReplacementCause,
        original_destination: Option<crate::enums::Zone>,
    ) -> crate::replacement::ReplacementOutcome {
        crate::replacement::try_replace_impl(self, timing, subject, cause, original_destination)
    }

    /// Infer the `ReplacementCause` for a deletion of `target_handle` given
    /// the current game state. Priority:
    ///   1. `security_resolution.is_some()` → `SecurityCheck`
    ///   2. `pending_attack.is_some()` → `Battle`
    ///   3. `effect_source_player.is_some()` — an effect is currently
    ///      running; `Own` if its controller equals the target's
    ///      controller, otherwise `Opponent`.
    ///   4. Fallback → `OwnEffect`.
    ///
    /// Consumed by the deletion fire-site in `combat::delete_permanent_with_effects`.
    pub(crate) fn infer_deletion_cause(
        &self,
        target_handle: crate::permanent::PermanentHandle,
    ) -> crate::replacement::ReplacementCause {
        use crate::replacement::ReplacementCause;
        if self.security_resolution.is_some() {
            return ReplacementCause::SecurityCheck;
        }
        if self.pending_attack.is_some() {
            return ReplacementCause::Battle;
        }
        if let Some(acting) = self.effect_source_player {
            if acting == target_handle.player {
                return ReplacementCause::OwnEffect;
            }
            return ReplacementCause::OpponentEffect;
        }
        ReplacementCause::OwnEffect
    }

    /// The observer-facing `EventCause` for the deletion currently being
    /// finalized, applying override-first precedence:
    ///   1. `current_deletion_event_cause_override` (a keyword route like
    ///      Overclock refining the payload), else
    ///   2. `current_deletion_cause` converted to `EventCause`.
    ///
    /// `None` outside an OnDeletion / OnAnyDeletion drain. Every deletion-cause
    /// consumer that wants an `EventCause` (the `OnAnyDeletion`
    /// `DeletedObjectSnapshot` in `combat.rs`, the OnDeletion `TriggerContext`
    /// in `effect_queue.rs`) must route through this so they cannot drift.
    /// (`effect_context::observed_deletion_cause` keeps its own copy because it
    /// returns a `ReplacementCause`, not an `EventCause`.)
    #[doc(hidden)]
    pub(crate) fn observed_deletion_event_cause(
        &self,
    ) -> Option<crate::trigger_context::EventCause> {
        self.current_deletion_event_cause_override.or_else(|| {
            self.current_deletion_cause
                .map(crate::trigger_context::EventCause::from)
        })
    }

    /// Generalized cause inference for non-deletion Would-replacement fire-sites
    /// (return-to-hand/deck, trash-by-effect, draw, place-in-security,
    /// de-digivolve, etc.).
    ///
    /// Differs from `infer_deletion_cause` in that `Battle` is NOT a candidate:
    /// non-deletion routes are never reached via `resolve_battle`, so the only
    /// live signals are security-resolution and the effect-source player.
    ///
    /// Priority:
    ///   1. `effect_source_player.is_some()` — compare against `target_player`;
    ///      equal → `OwnEffect`, else `OpponentEffect`.
    ///   2. `security_resolution.is_some()` → `SecurityCheck`
    ///   3. Fallback → `OwnEffect`.
    ///
    /// Security card effects still run as card effects: `run_queued_effect`
    /// sets `effect_source_player` before the effect body mutates state. The
    /// ambient `security_resolution` state is only a `SecurityCheck` cause when
    /// no card effect is currently resolving (for example, rule cleanup or
    /// security battle resolution).
    ///
    /// Consumed by Task 4 fire-sites in `game_actions` / `effect_context`.
    pub(crate) fn infer_effect_cause(
        &self,
        target_player: PlayerId,
    ) -> crate::replacement::ReplacementCause {
        use crate::replacement::ReplacementCause;
        if let Some(acting) = self.effect_source_player {
            if acting == target_player {
                return ReplacementCause::OwnEffect;
            }
            return ReplacementCause::OpponentEffect;
        }
        if self.security_resolution.is_some() {
            return ReplacementCause::SecurityCheck;
        }
        ReplacementCause::OwnEffect
    }

    /// Test-only setter for `effect_source_player`. Production code MUST go
    /// through `run_queued_effect` (which sets/restores around the dispatch).
    /// Exposed `#[doc(hidden)] pub` so behavioral tests under
    /// `digimon-engine/tests/` can simulate "opponent effect currently
    /// resolving" without driving the queue.
    #[doc(hidden)]
    pub fn set_effect_source_player_for_test(&mut self, source: Option<crate::enums::PlayerId>) {
        self.effect_source_player = source;
    }

    /// Test-only setter for `parked_replacement`. Production code must go
    /// through the dispatcher's post-process hook in
    /// `replacement::run_candidate_inner`. Exposed so behavioral tests under
    /// `digimon-engine/tests/` can install a parked-replacement slot
    /// directly without driving an entire replacement-dispatch flow.
    #[doc(hidden)]
    pub fn install_parked_replacement_for_test(
        &mut self,
        parked: crate::replacement::ParkedReplacement,
    ) {
        self.parked_replacement = Some(parked);
    }

    /// Test-only getter for the parked-replacement outcome. The
    /// `parked_replacement` field is `pub(crate)`, so behavioral tests
    /// under `digimon-engine/tests/` cannot read it directly. Returns
    /// `None` when no replacement is parked.
    #[doc(hidden)]
    pub fn parked_replacement_outcome_for_test(
        &self,
    ) -> Option<crate::replacement::ReplacementOutcome> {
        self.parked_replacement.as_ref().map(|p| p.outcome)
    }

    /// Get the next player clockwise from the given player.
    pub fn next_clockwise(&self, id: PlayerId) -> PlayerId {
        let pos = self.turn_order.iter().position(|&p| p == id).unwrap_or(0);
        let next_pos = (pos + 1) % self.turn_order.len();
        self.turn_order[next_pos]
    }

    /// Start the game: auto-keep for every remaining mulligan-pending player
    /// and transition into turn 1. UIs / RL agents that want to make mulligan
    /// decisions explicitly should call `accept_mulligan` for each decider
    /// before invoking `start_game` (or instead of it — the last
    /// `accept_mulligan` call triggers `finalize_mulligan`, which begins turn 1).
    pub fn start_game(&mut self) {
        while let Some(p) = self.mulligan_current_player() {
            // Auto-keep; infallible because we just asked who's current.
            let _ = self.accept_mulligan(p, true);
        }
        // If the game was never in Mulligan phase (defensive), fall through
        // to an explicit turn-1 transition.
        if self.turn_count == 0 {
            self.turn_count = 1;
            self.memory = 0;
            self.begin_turn();
        }
    }

    // ─── Event accumulator ─────────────────────────────────────────

    /// Allocate the next monotonic event sequence number.
    pub fn next_event_seq(&mut self) -> u64 {
        let s = self.event_seq;
        self.event_seq += 1;
        s
    }

    /// Drain accumulated events, returning them in emission order. The
    /// `HeadlessRunner::step` wrapper calls this after each action so the
    /// PyO3 layer can expose a per-step event list.
    pub fn drain_events(&mut self) -> Vec<crate::events::GameEvent> {
        std::mem::take(&mut self.events)
    }

    /// Borrow accumulated events without draining them. Debug tooling uses
    /// this to checkpoint and assert on incremental event emission.
    pub fn events(&self) -> &[crate::events::GameEvent] {
        &self.events
    }

    /// Trash a single card to `player`'s trash zone, emitting a
    /// [`crate::events::GameEvent::Trash`] event.
    ///
    /// Per the `engine-event-emission` capability spec, every individual
    /// card moving into a trash zone SHALL emit a `Trash` event in
    /// physical-movement order. Token cards (`card.is_token == true`)
    /// are dropped on the floor with no trash entry and no event,
    /// matching the existing token-deletion semantic in
    /// `Player::delete_permanent`.
    ///
    /// This helper is the canonical "trash a card" path for any zone
    /// transition into trash (battle-area cleanup, hand discard, deck
    /// mill, source manipulation, security-stack effects, etc.). New
    /// engine code should prefer this over direct `player.trash.push(...)`
    /// so the event surface stays uniform.
    pub fn trash_card(
        &mut self,
        player: crate::enums::PlayerId,
        card: crate::card_source::CardSource,
    ) {
        if card.is_token {
            // Tokens are removed from game — no trash entry, no event.
            return;
        }
        let card_id = card.card_id(&self.card_data).to_string();
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Trash {
            seq,
            player,
            card_id,
        });
        self.player_mut(player).trash.push(card);
    }

    /// Trash an entire permanent stack (top card + digi_sources + linked
    /// cards, in that physical movement order), emitting one
    /// [`crate::events::GameEvent::Trash`] per card.
    ///
    /// Replaces direct calls to `Player::delete_permanent` from sites
    /// that want event emission. Mirrors `Player::delete_permanent`'s
    /// token-semantic (the whole stack drops on the floor when the top
    /// card is a token — no trash entry, no event) and empty-stack
    /// tolerance (linked cards still flow to trash).
    pub fn trash_permanent_stack(&mut self, player: crate::enums::PlayerId, field_index: usize) {
        let p = self.player_mut(player);
        if field_index >= p.battle_area.len() {
            return;
        }
        let perm = p.battle_area.remove(field_index);

        // Empty-stack guard (parity with Player::delete_permanent): if the
        // stack was already drained by a mid-deletion effect, only linked
        // cards remain to flow to trash.
        if perm.card_sources.is_empty() {
            for card in perm.linked_cards {
                // Linked cards on an empty stack are still real cards —
                // emit per-card.
                let card_id = card.card_id(&self.card_data).to_string();
                let seq = self.next_event_seq();
                self.events.push(crate::events::GameEvent::Trash {
                    seq,
                    player,
                    card_id,
                });
                self.player_mut(player).trash.push(card);
            }
            return;
        }

        // Token semantic: drop the entire stack — no trash, no event.
        // Mirrors Player::delete_permanent's token branch + Python
        // `player.py::delete_permanent`.
        if perm.card_sources[0].is_token {
            return;
        }

        // Normal path: emit + push each card in physical movement order.
        // `card_sources` is stack-top-first per the spec ordering invariant.
        for card in perm.card_sources {
            let card_id = card.card_id(&self.card_data).to_string();
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::Trash {
                seq,
                player,
                card_id,
            });
            self.player_mut(player).trash.push(card);
        }
        for card in perm.linked_cards {
            let card_id = card.card_id(&self.card_data).to_string();
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::Trash {
                seq,
                player,
                card_id,
            });
            self.player_mut(player).trash.push(card);
        }
    }

    // ─── Memory management ─────────────────────────────────────────

    /// Pay memory cost. Returns `true` if affordable (memory stays above
    /// `rules.memory_range.0`).
    ///
    /// Does **not** end the turn even if memory crosses zero. Python's rule:
    /// the turn only ends when `check_turn_end()` is called (typically after
    /// all OnPlay/WhenDigivolving/etc. effects have resolved). Callers should
    /// invoke `check_turn_end()` at the natural resolution boundary.
    pub fn pay_memory(&mut self, cost: u16) -> bool {
        if cost == 0 {
            return true;
        }
        let new_memory = self.memory - cost as i16;
        if new_memory < self.rules.memory_range.0 {
            return false;
        }
        let delta = new_memory - self.memory;
        self.memory = new_memory;
        let seq = self.next_event_seq();
        let player = self.turn_player();
        self.events.push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.memory,
        });
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    /// Pay memory cost **without** the floor check. Used by effect-initiated
    /// flows that explicitly opt out of the affordability constraint
    /// (`ignore_requirements: true`). Always mutates and emits the
    /// `MemoryChange` event — even if the resulting memory dips below
    /// `rules.memory_range.0`.
    ///
    /// Callers must have already decided that the floor check should be
    /// skipped (typically because a printed effect overrides the normal
    /// rules). For ordinary plays, use `pay_memory` instead.
    ///
    /// `cost == 0` is a no-op (returns immediately, no event emitted) —
    /// matches `pay_memory`'s zero-cost short-circuit.
    pub(crate) fn pay_memory_unchecked(&mut self, cost: u16) {
        if cost == 0 {
            return;
        }
        let new_memory = self.memory - cost as i16;
        let delta = new_memory - self.memory;
        self.memory = new_memory;
        let seq = self.next_event_seq();
        let player = self.turn_player();
        self.events.push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.memory,
        });
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// End the turn if memory has crossed to the opponent's side.
    /// Call this after a batch of effects resolves (not synchronously inside
    /// `pay_memory`, which would starve effects of their turn).
    pub fn check_turn_end(&mut self) {
        if self.pending_selection.is_some() {
            return;
        }
        if self.memory < 0 && !self.game_over {
            self.end_turn();
        }
    }

    /// Gain memory for the active player.
    pub fn gain_memory(&mut self, amount: i16) {
        self.gain_memory_for_player(self.turn_player(), amount);
    }

    /// Gain memory for a specific player. The memory counter is stored from
    /// the current turn player's perspective, so non-turn-player gains move
    /// the counter toward the opponent's side.
    pub fn gain_memory_for_player(&mut self, player: PlayerId, amount: i16) {
        let before = self.memory;
        let signed_amount = if player == self.turn_player() {
            amount
        } else {
            -amount
        };
        self.memory = (self.memory + signed_amount)
            .clamp(self.rules.memory_range.0, self.rules.memory_range.1);
        let delta = self.memory - before;
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.memory,
        });
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Set memory to a specific value.
    pub fn set_memory(&mut self, value: i16) {
        let before = self.memory;
        self.memory = value.clamp(self.rules.memory_range.0, self.rules.memory_range.1);
        let delta = self.memory - before;
        let seq = self.next_event_seq();
        let player = self.turn_player();
        self.events.push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.memory,
        });
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    // ─── Scenario staging (add-ui-scenario-test-substrate) ─────────
    //
    // Direct state setters for building an arbitrary mid-game board for
    // tests / the `/debug` HTTP surface. They are inert setup operations,
    // NOT card effects or rule actions — they bypass the play flow on
    // purpose. `DebugRunner`'s staging setters delegate here so there is
    // one implementation; `RustDebugGame` (PyO3) calls these on its
    // wrapped `Game`.

    /// Place a full digivolution stack (bottom-to-top) on `player`'s field
    /// with explicit suspend state and turn-played value. Returns the new
    /// battle-area index. Panics on an unknown card id.
    pub fn stage_place_field_stack(
        &mut self,
        player: PlayerId,
        card_ids: &[&str],
        suspended: bool,
        turn_played: u16,
    ) -> usize {
        assert!(
            !card_ids.is_empty(),
            "stage_place_field_stack requires at least one card id"
        );
        let mut sources = Vec::with_capacity(card_ids.len());
        for card_id in card_ids {
            let data_idx = self
                .card_data
                .iter()
                .position(|c| c.card_id == *card_id)
                .unwrap_or_else(|| {
                    panic!("stage_place_field_stack: unknown card_id {card_id}")
                });
            let next_idx = self.next_card_index();
            let mut card = crate::card_source::CardSource::new(data_idx, player, next_idx);
            card.card_index = next_idx;
            sources.push(card);
        }
        // Bottom-to-top: first id is the bottom source, last is the top
        // card. `Permanent::new` takes the top card; remaining go beneath.
        let top = sources.pop().expect("non-empty checked above");
        let mut perm = crate::permanent::Permanent::new(top, turn_played);
        // Insert the lower sources beneath the top, preserving order.
        for (i, src) in sources.into_iter().enumerate() {
            perm.card_sources.insert(i, src);
        }
        perm.is_suspended = suspended;
        perm.turn_played = turn_played;
        self.players[player as usize].battle_area.push(perm);
        self.players[player as usize].battle_area.len() - 1
    }

    /// Inject a single card directly into a named zone for `player`.
    /// `zone` ∈ {"hand", "deck_top", "security_top", "trash"}. Panics on
    /// an unknown card id; returns `Err` on an unknown zone.
    pub fn stage_inject_card(
        &mut self,
        player: PlayerId,
        card_id: &str,
        zone: &str,
    ) -> Result<(), String> {
        let data_idx = self
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .ok_or_else(|| format!("stage_inject_card: unknown card_id {card_id}"))?;
        let next_idx = self.next_card_index();
        let mut card = crate::card_source::CardSource::new(data_idx, player, next_idx);
        card.card_index = next_idx;
        let p = &mut self.players[player as usize];
        match zone {
            "hand" => p.hand.push(card),
            // Deck top = end of the vec (draw pops from the end).
            "deck_top" => p.deck.push(card),
            // Security top = end of the vec.
            "security_top" => p.security.push(card),
            "trash" => p.trash.push(card),
            other => return Err(format!("stage_inject_card: unknown zone {other}")),
        }
        Ok(())
    }

    /// Make `player` the active turn player, preserving the seesaw and
    /// turn-rotation invariants. Reorders `turn_order` so `player` leads,
    /// points `turn_player_idx` at it, resets `memory_pair` to (active,
    /// next), and realigns any still-pending mulligan order.
    pub fn stage_set_first_player(&mut self, player: PlayerId) {
        if let Some(pos) = self.turn_order.iter().position(|&p| p == player) {
            self.turn_order.rotate_left(pos);
        }
        self.turn_player_idx = 0;
        let next = if self.turn_order.len() >= 2 {
            self.turn_order[1]
        } else {
            self.turn_order[0]
        };
        self.memory_pair = (player, next);
        if !self.mulligan_pending.is_empty() {
            self.mulligan_pending = self.turn_order.clone();
        }
    }

    /// Place a digivolution stack (bottom-to-top) into `player`'s breeding
    /// area, replacing whatever is there. Panics on an unknown card id.
    pub fn stage_place_in_breeding(&mut self, player: PlayerId, card_ids: &[&str]) {
        assert!(
            !card_ids.is_empty(),
            "stage_place_in_breeding requires at least one card id"
        );
        let mut sources = Vec::with_capacity(card_ids.len());
        for card_id in card_ids {
            let data_idx = self
                .card_data
                .iter()
                .position(|c| c.card_id == *card_id)
                .unwrap_or_else(|| {
                    panic!("stage_place_in_breeding: unknown card_id {card_id}")
                });
            let next_idx = self.next_card_index();
            let mut card = crate::card_source::CardSource::new(data_idx, player, next_idx);
            card.card_index = next_idx;
            sources.push(card);
        }
        let top = sources.pop().expect("non-empty checked above");
        let mut perm = crate::permanent::Permanent::new(top, self.turn_count);
        for (i, src) in sources.into_iter().enumerate() {
            perm.card_sources.insert(i, src);
        }
        self.players[player as usize].breeding_area = Some(perm);
    }

    /// Empty a named zone for `player` so staged contents fully replace
    /// whatever was dealt at construction. `zone` ∈ {"hand", "deck",
    /// "security", "trash", "battle_area", "breeding"}. Returns `Err` on
    /// an unknown zone.
    pub fn stage_clear_zone(&mut self, player: PlayerId, zone: &str) -> Result<(), String> {
        let p = &mut self.players[player as usize];
        match zone {
            "hand" => p.hand.clear(),
            "deck" => p.deck.clear(),
            "security" => p.security.clear(),
            "trash" => p.trash.clear(),
            "battle_area" => p.battle_area.clear(),
            "breeding" => p.breeding_area = None,
            other => return Err(format!("stage_clear_zone: unknown zone {other}")),
        }
        Ok(())
    }

    /// Validate that a staged board is internally consistent enough for
    /// the turn machine to operate on. Returns `Err(reason)` on a
    /// rule-illegal staged state so callers can fail loud.
    pub fn stage_validate(&self) -> Result<(), String> {
        if self.turn_order.is_empty() {
            return Err("turn_order is empty".to_string());
        }
        if self.turn_player_idx >= self.turn_order.len() {
            return Err(format!(
                "turn_player_idx {} out of range for turn_order len {}",
                self.turn_player_idx,
                self.turn_order.len()
            ));
        }
        if self.current_phase != GamePhase::Mulligan && !self.mulligan_pending.is_empty() {
            return Err(format!(
                "phase is {:?} but {} player(s) still owe a mulligan decision; \
                 finalize mulligan before setting the phase",
                self.current_phase,
                self.mulligan_pending.len()
            ));
        }
        for (pid, player) in self.players.iter().enumerate() {
            for (i, perm) in player.battle_area.iter().enumerate() {
                if perm.card_sources.is_empty() {
                    return Err(format!(
                        "player {pid} battle_area[{i}] has an empty card stack"
                    ));
                }
            }
        }
        Ok(())
    }

    // ─── Elimination / winner ──────────────────────────────────────

    /// Handle deck-out for a player.
    pub(crate) fn handle_deckout(&mut self, player_id: PlayerId) {
        if self.rules.player_count == 2 {
            // Standard: deck-out = loss
            self.game_over = true;
            let opponents = self.opponents(player_id);
            self.winner = opponents.first().copied();
            self.terminal_outcome_reason = Some(TerminalOutcomeReason::DeckOut);
            self.current_phase = GamePhase::GameOver;
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::GameOver {
                seq,
                winner: self.winner,
                reason: TerminalOutcomeReason::DeckOut,
            });
        } else {
            // Multiplayer: elimination
            self.eliminate_player(player_id);
        }
    }

    /// Eliminate a player (multiplayer modes).
    pub fn eliminate_player(&mut self, player_id: PlayerId) {
        self.players[player_id as usize].is_eliminated = true;

        // Remove from turn order
        self.turn_order.retain(|&p| p != player_id);

        // Check if only one player remains
        if self.turn_order.len() == 1 {
            self.game_over = true;
            self.winner = Some(self.turn_order[0]);
            self.terminal_outcome_reason = Some(TerminalOutcomeReason::EngineDeclared);
            self.current_phase = GamePhase::GameOver;
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::GameOver {
                seq,
                winner: self.winner,
                reason: TerminalOutcomeReason::EngineDeclared,
            });
        }

        // Adjust turn_player_idx if needed
        if self.turn_player_idx >= self.turn_order.len() {
            self.turn_player_idx = 0;
        }
    }

    /// Declare a winner (e.g., after a direct attack on a player with 0 security).
    pub fn declare_winner(&mut self, winner_id: PlayerId) {
        self.declare_winner_with_reason(winner_id, TerminalOutcomeReason::EngineDeclared);
    }

    /// Choose a deterministic winner for non-natural termination.
    ///
    /// Training games should always produce a winner. Timeout ranking uses
    /// visible, stable game-state advantages, then falls back to player order
    /// so exact ties are still deterministic.
    pub fn step_limit_tiebreaker_winner(&self) -> PlayerId {
        fn score(
            game: &Game,
            player: PlayerId,
        ) -> (usize, i32, usize, usize, std::cmp::Reverse<PlayerId>) {
            let p = game.player(player);
            (
                p.security.len(),
                p.total_field_dp(&game.card_data),
                p.deck.len(),
                p.hand.len(),
                std::cmp::Reverse(player),
            )
        }

        self.turn_order
            .iter()
            .copied()
            .max_by_key(|&player| score(self, player))
            .unwrap_or(0)
    }

    /// End the game by step/turn limit while still assigning a winner.
    pub fn declare_step_limit_winner(&mut self) {
        let winner = self.step_limit_tiebreaker_winner();
        self.declare_winner_with_reason(winner, TerminalOutcomeReason::StepLimit);
    }

    /// Concede the game on behalf of `player_id`. The opponent is declared the
    /// winner with `TerminalOutcomeReason::Concede`. Always-legal at every
    /// agent decision point — the action mask reports action `93` as legal in
    /// every phase. Safe to call mid-selection: any pending selection is
    /// cleared, the effect queue is dropped, and any in-progress combat /
    /// attack-timing state is short-circuited by the terminal phase change.
    ///
    /// Event ordering: a `GameEvent::Concede { player }` event is pushed
    /// **before** the `GameOver` event so listeners can observe the concede
    /// before the terminal-outcome notification. This mirrors the
    /// surrender-event-before-declare_winner pattern used by the legacy
    /// Python engine (see `CLAUDE.md` working rule #16).
    ///
    /// No-op if the game is already over.
    pub fn concede(&mut self, player_id: PlayerId) {
        if self.game_over {
            return;
        }
        // Clear pending selection and effect queue so the game terminates
        // cleanly even when concede fires mid-selection or with queued
        // triggered effects waiting to resolve.
        self.pending_selection = None;
        self.effect_queue.clear();
        // Emit the Concede event before declare_winner so its seq is
        // strictly less than the GameOver event's seq.
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Concede {
            seq,
            player: player_id,
        });
        // Resolve the winner: opponent of the conceder. In 2-player play
        // there is exactly one opponent; in multiplayer we fall back to the
        // first non-eliminated opponent in turn order.
        let opponents = self.opponents(player_id);
        let winner_id = opponents.first().copied().unwrap_or(player_id);
        self.declare_winner_with_reason(winner_id, TerminalOutcomeReason::Concede);
    }

    /// Install a `SelectionKind::PlayOrder` prompt with `loser_id` as the
    /// chooser. The engine enters `GamePhase::SelectPlayOrder`; the action
    /// mask reports actions `PLAY_FIRST` (94) and `PLAY_SECOND` (95) as legal
    /// for `loser_id` only. On resolution, the callback writes the chosen
    /// `PlayOrder` to `self.last_play_order_choice` and the standard
    /// selection unwind restores `previous_phase` (typically `GameOver`).
    ///
    /// The wrapper (Python `MatchEnv` for BO3 match training) is expected to
    /// call this method between games of a match, then read
    /// `self.last_play_order_choice` once the selection resolves to decide
    /// which side plays first in the next game.
    ///
    /// Any existing `pending_selection` is dropped — when a game terminates
    /// via a win condition mid-effect (security depleted by a triggered
    /// effect, deck-out during a chain), the engine can leave a
    /// `pending_selection` installed; the BO3 wrapper calls this method
    /// after termination, and we just discard the stale prompt since the
    /// game it belonged to is over.
    pub fn request_play_order_selection(&mut self, loser_id: PlayerId) {
        self.pending_selection = None;
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectPlayOrder;
        self.pending_selection = Some(crate::selection::PendingSelection {
            kind: crate::selection::SelectionKind::PlayOrder,
            selecting_player: loser_id,
            previous_phase,
            valid_action_ids: vec![
                crate::action::space::PLAY_FIRST,
                crate::action::space::PLAY_SECOND,
            ],
            is_optional: false,
            prompt: "Choose play order for the next game".to_string(),
            effect_choices: None,
            // No real card source — this prompt is driven by the BO3 match
            // rules, not by a card effect. We use a sentinel `CardHandle(0)`
            // (CardHandle is a `pub u16` newtype) and `EffectSourceKind::Rule`.
            source_card: crate::card_source::CardHandle(0),
            source_permanent: None,
            source_kind: crate::enums::EffectSourceKind::Rule,
            callback: Box::new(|game: &mut Game, action_id: u16| {
                let picked = if action_id == crate::action::space::PLAY_FIRST {
                    crate::selection::PlayOrder::First
                } else {
                    // PLAY_SECOND (95) is the only other valid id; the
                    // selection installation gates this through valid_action_ids.
                    crate::selection::PlayOrder::Second
                };
                game.last_play_order_choice = Some(picked);
            }),
            on_decline: None,
        });
    }

    /// Convenience entry point that resolves a pending play-order selection
    /// directly (without routing through the action-id interface). Useful for
    /// programmatic testing and for wrapper code that already knows the pick.
    ///
    /// Returns `Err` if no `PlayOrder` selection is currently installed.
    pub fn resolve_play_order_selection(
        &mut self,
        picked: crate::selection::PlayOrder,
    ) -> Result<(), SelectionError> {
        let action_id = match picked {
            crate::selection::PlayOrder::First => crate::action::space::PLAY_FIRST,
            crate::selection::PlayOrder::Second => crate::action::space::PLAY_SECOND,
        };
        let pending = self
            .pending_selection
            .as_ref()
            .ok_or(SelectionError::NoPendingSelection)?;
        if !matches!(pending.kind, crate::selection::SelectionKind::PlayOrder) {
            return Err(SelectionError::NoPendingSelection);
        }
        let player = pending.selecting_player;
        self.resolve_selection(player, action_id)
    }

    /// Declare a winner with an explicit terminal reason.
    pub fn declare_winner_with_reason(
        &mut self,
        winner_id: PlayerId,
        reason: TerminalOutcomeReason,
    ) {
        self.game_over = true;
        self.winner = Some(winner_id);
        self.terminal_outcome_reason = Some(reason);
        self.current_phase = GamePhase::GameOver;
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::GameOver {
            seq,
            winner: self.winner,
            reason,
        });
    }

    /// Allocate a new unique card index (for tokens, etc.).
    pub fn next_card_index(&mut self) -> u16 {
        let idx = self.next_card_index;
        self.next_card_index += 1;
        idx
    }

    /// Advance the card-index counter to `value` if it would move forward.
    /// No-op if `value <= self.next_card_index` — the counter must never
    /// regress, since that would re-issue an index already in circulation.
    ///
    /// Used by test harnesses (e.g. `DebugRunner`) that pre-seed cards with
    /// indices `0..N` and need the game to allocate `N..` for any cards
    /// minted after setup. Centralized here so future invariants around
    /// card-index management have a single chokepoint.
    pub(crate) fn advance_card_index_to(&mut self, value: u16) {
        if value > self.next_card_index {
            self.next_card_index = value;
        }
    }

    // --- Convenience methods that avoid borrow conflicts ---

    /// Suspend a single permanent. Fires `OnSuspend` observers in every
    /// player's battle area if the permanent was not already suspended.
    ///
    /// This is the canonical chokepoint for single-target suspension.
    /// `Player::unsuspend_all` (bulk turn-begin unsuspend) intentionally
    /// bypasses this path — `StartOfYourTurn` is the canonical timing for
    /// turn-start effects.
    pub fn suspend(&mut self, handle: PermanentHandle) {
        let is_breeding = handle.index == crate::action::space::BREEDING_TARGET as u8;
        let event_card = if is_breeding {
            self.players
                .get(handle.player as usize)
                .and_then(|p| p.breeding_area.as_ref())
                .map(|perm| perm.top_card().handle())
        } else {
            self.players
                .get(handle.player as usize)
                .and_then(|p| p.battle_area.get(handle.index as usize))
                .map(|perm| perm.top_card().handle())
        };
        let already = if is_breeding {
            self.players
                .get(handle.player as usize)
                .and_then(|p| p.breeding_area.as_ref())
                .map(|perm| perm.is_suspended)
                .unwrap_or(true)
        } else {
            self.players
                .get(handle.player as usize)
                .and_then(|p| p.battle_area.get(handle.index as usize))
                .map(|perm| perm.is_suspended)
                .unwrap_or(true)
        }; // treat out-of-range as "already suspended" to no-op
        if already {
            return;
        }
        let perm = if is_breeding {
            self.players
                .get_mut(handle.player as usize)
                .and_then(|p| p.breeding_area.as_mut())
        } else {
            self.players
                .get_mut(handle.player as usize)
                .and_then(|p| p.battle_area.get_mut(handle.index as usize))
        };
        if let Some(perm) = perm {
            perm.is_suspended = true;
        }
        self.mark_until_condition_dirty();
        if let Some(card) = event_card {
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnSuspend,
                crate::selection::TriggerSource::EventObserved {
                    player: handle.player,
                    permanent: handle,
                    card,
                },
            );
        }
        self.drain_effect_queue();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Unsuspend a single permanent. Fires `OnUnsuspend` observers in every
    /// player's battle area if the permanent was suspended.
    ///
    /// See `suspend` for the bulk-unsuspend caveat.
    pub fn unsuspend(&mut self, handle: PermanentHandle) {
        let event_card = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
            .map(|perm| perm.top_card().handle());
        let was_suspended = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
            .map(|perm| perm.is_suspended)
            .unwrap_or(false); // treat out-of-range as "not suspended" to no-op
        if !was_suspended {
            return;
        }
        if let Some(perm) = self
            .players
            .get_mut(handle.player as usize)
            .and_then(|p| p.battle_area.get_mut(handle.index as usize))
        {
            perm.is_suspended = false;
        }
        self.mark_until_condition_dirty();
        if let Some(card) = event_card {
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnUnsuspend,
                crate::selection::TriggerSource::EventObserved {
                    player: handle.player,
                    permanent: handle,
                    card,
                },
            );
        }
        self.drain_effect_queue();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Hatch for a player (copies turn_count to avoid borrow conflict).
    /// Fires `OnHatch` observers in every player's battle area after the egg
    /// moves into the breeding area.
    pub fn hatch(&mut self, player_id: PlayerId) -> bool {
        let turn = self.turn_count;
        let ok = self.player_mut(player_id).hatch(turn);
        if ok {
            self.mark_until_condition_dirty();
            let n = self.players.len();
            for pid in 0..n {
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnHatch,
                    crate::selection::TriggerSource::PlayerBattleArea(
                        pid as crate::enums::PlayerId,
                    ),
                );
            }
            self.drain_effect_queue();
            self.reevaluate_until_condition_modifiers_if_dirty();
        }
        ok
    }

    /// Shared core for DNA digivolve. Performs material consumption, hand-card
    /// consumption, stack merging, memory payment (if `cost > 0` and not under
    /// `ignore_requirements`), and trigger firing.
    ///
    /// **Material-stack ordering (canonical):** `target_a.card_sources` are
    /// concatenated, then `target_b.card_sources`, then `from_hand` is pushed
    /// on top. `target_a` should correspond to `DnaCost::requirement1` and
    /// `target_b` to `requirement2`; callers that select materials by user
    /// input must pre-orient via `get_dna_stacking_order`.
    ///
    /// **Trigger surface (in order):**
    /// 1. `WhenDigivolving` on the merged permanent → drain
    /// 2. `OnDnaDigivolve` on the merged permanent → drain
    /// 3. `OnDigivolve` on every player's battle area → drain
    ///
    /// **Index-shift:** if `target_a.player == target_b.player` and
    /// `target_b.index < target_a.index`, the merged permanent ends up at
    /// `target_a.index - 1` (because removing `target_b` first shifts the
    /// remaining slots). Callers should use the returned handle, not
    /// `target_a` directly.
    ///
    /// **Returns** `Some(merged_handle)` on success, `None` on:
    /// - identical targets (`target_a == target_b`)
    /// - either target's index out of range on its player's battle area
    /// - hand index out of range on `hand_owner`
    /// - `cost > 0` and `Game::pay_memory` fails
    ///
    /// The pay-memory-bypass branch (`ignore_requirements && cost > 0`) is
    /// *not* present here — callers that need to bypass the affordability
    /// floor must subtract from `self.memory` before calling (see
    /// `Game::pay_memory_unchecked`). The two callers are:
    /// - `EffectContext::effect_initiated_dna_digivolve` — engine-effect
    ///   wrapper that handles the IR's `(cost, ignore_requirements)` shape
    ///   and invokes the bypass branch when needed.
    /// - `Game::initiate_dna_digivolve`'s stage-2 selection callback — the
    ///   user-action path; passes the printed cost minus
    ///   `BeforePayCost` reductions and never bypasses.
    ///
    /// `grant_digivolve_bonus`: if true, `hand_owner` draws 1 card after the
    /// merge but before triggers fire. The user-action path passes `true`
    /// (matching `digivolve_from_hand`); the effect-initiated path passes
    /// `false`.
    ///
    /// `effect_initiated`: marks the global `OnDigivolve` payload so observers
    /// can distinguish effect-created DNA digivolutions from player-action DNA.
    pub(crate) fn dna_digivolve_inner(
        &mut self,
        target_a: PermanentHandle,
        target_b: PermanentHandle,
        hand_owner: PlayerId,
        hand_index: usize,
        cost: u16,
        grant_digivolve_bonus: bool,
        effect_initiated: bool,
    ) -> Option<PermanentHandle> {
        use crate::enums::EffectTiming;
        use crate::selection::TriggerSource;

        if target_a == target_b {
            return None;
        }
        if (target_a.index as usize) >= self.player(target_a.player).battle_area.len() {
            return None;
        }
        if (target_b.index as usize) >= self.player(target_b.player).battle_area.len() {
            return None;
        }
        if hand_index >= self.player(hand_owner).hand.len() {
            return None;
        }

        if cost > 0 && !self.pay_memory(cost) {
            return None;
        }

        let target_a_index_after = if target_a.player == target_b.player
            && (target_b.index as usize) < (target_a.index as usize)
        {
            (target_a.index as usize) - 1
        } else {
            target_a.index as usize
        };

        // Capture from_stack_top (the OLD top of target_a) before the
        // merge mutates the stack. Needed for `GameEvent::Digivolve`
        // emission below per the `engine-event-emission` spec.
        let from_stack_top = self
            .player(target_a.player)
            .battle_area
            .get(target_a_index_after)
            .map(|p| p.top_card().card_id(&self.card_data).to_string())
            .unwrap_or_default();

        let perm_b = self
            .player_mut(target_b.player)
            .battle_area
            .remove(target_b.index as usize);
        let new_top = self.player_mut(hand_owner).hand.remove(hand_index);
        // Capture top_card_id from the removed-hand source before it moves
        // into the merged permanent's stack.
        let top_card_id = new_top.card_id(&self.card_data).to_string();

        let turn = self.turn_count;
        {
            let perm_a = &mut self.player_mut(target_a.player).battle_area[target_a_index_after];
            perm_a.card_sources.extend(perm_b.card_sources);
            perm_a.card_sources.push(new_top);
            perm_a.turn_digivolved = turn;
        }

        let merged_handle = PermanentHandle {
            player: target_a.player,
            index: target_a_index_after as u8,
        };

        // `GameEvent::Digivolve` for the DNA path — emit BEFORE trigger
        // dispatch so reward components observe the digivolve before any
        // downstream effects fire. `was_blast_dna` is conservatively
        // `false` here: distinguishing Blast from standard DNA requires
        // plumbing the path kind into `dna_digivolve_inner` and is a
        // focused follow-up (the DNA Omnimon profile uses `was_dna` not
        // `was_blast_dna` for primary matching).
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Digivolve {
            seq,
            player: merged_handle.player,
            top_card_id,
            field_index: merged_handle.index,
            from_stack_top,
            was_dna: true,
            was_blast_dna: false,
            memory_paid: cost as i16,
        });

        if grant_digivolve_bonus {
            // Opaque-aware: routes through draw_one_for_player so opaque
            // opponents pull from their RevealSource rather than from the
            // (empty in opaque mode) deck Vec. Errors are logged-and-
            // ignored to preserve the original `-> ()` signature of this
            // function; the symptom of a misaligned reveal source is a
            // downstream parity divergence the replay harness will catch.
            if let Err(e) = self.draw_one_for_player(hand_owner) {
                eprintln!(
                    "[opaque-deck] digivolve bonus draw error for player {}: {}",
                    hand_owner, e
                );
            }
        }

        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(merged_handle),
        );
        self.drain_effect_queue_with_dna_origin(true);

        self.enqueue_triggered(
            EffectTiming::OnDnaDigivolve,
            TriggerSource::Permanent(merged_handle),
        );
        self.drain_effect_queue_with_dna_origin(true);

        let event_card = self
            .player(merged_handle.player)
            .battle_area
            .get(merged_handle.index as usize)
            .map(|perm| perm.top_card().handle())?;
        self.enqueue_triggered(
            EffectTiming::OnDigivolve,
            TriggerSource::Digivolved {
                player: merged_handle.player,
                permanent: merged_handle,
                card: event_card,
                effect_initiated,
                dna_origin: true,
            },
        );
        self.drain_effect_queue();

        // Reward-shaping counters — DNA digivolves stack on the regular
        // counter per spec decision 5: a single `digivolve_reward` line in
        // DigimonEnv always fires, plus a separate `dna_digivolve_bonus`
        // line fires only on DNAs. Effect-initiated DNAs (called via
        // `effect_context::EffectContext::initiate_dna_digivolve`) do not
        // bump — only user-action DNAs (via `Game::initiate_dna_digivolve`)
        // credit, matching the regular-digivolve carve-out. See
        // docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
        if !effect_initiated {
            self.n_digivolutions[merged_handle.player as usize] += 1;
            self.n_dna_digivolutions[merged_handle.player as usize] += 1;
        }

        Some(merged_handle)
    }

    /// DNA digivolve where ONE material lives on the field (`target`) and the
    /// OTHER material is a card in `hand_owner`'s hand (`partner_index`). The
    /// merged permanent is topped with `result_index` (also a hand card —
    /// typically the Omnimon-name level-7 result).
    ///
    /// This is the BT17-095-shaped DNA: the printed text "That Digimon and a
    /// card in the hand may DNA digivolve into a Digimon card with [Omnimon]
    /// in its name in the hand" — only one DNA material is a field permanent;
    /// the second is materialised from hand inline. Mirrors DCGO's
    /// `BT17_095.SuccessProcess` which builds a temporary `Permanent` from the
    /// hand-card partner and runs `PlayCardClass.PlayCard` with `SetJogress`.
    ///
    /// ## Stacking order
    ///
    /// `target.card_sources ++ [hand_partner] ++ [result]`. `target` is the
    /// `requirement1` material; the hand partner is `requirement2`. The merged
    /// permanent stays at `target`'s index (no on-field permanent is removed).
    ///
    /// ## Triggers
    ///
    /// Identical to `dna_digivolve_inner`: `WhenDigivolving` → `OnDnaDigivolve`
    /// → `OnDigivolve` (global), each followed by a queue drain, all carrying
    /// the `dna_origin` marker.
    ///
    /// ## Returns
    ///
    /// `Some(merged_handle)` on success; `None` if `target` is out of range,
    /// either hand index is out of range, the two hand indices coincide, or
    /// `cost > 0` and `pay_memory` fails.
    pub(crate) fn dna_digivolve_hand_partner_inner(
        &mut self,
        target: PermanentHandle,
        hand_owner: PlayerId,
        partner_index: usize,
        result_index: usize,
        cost: u16,
        effect_initiated: bool,
    ) -> Option<PermanentHandle> {
        use crate::enums::EffectTiming;
        use crate::selection::TriggerSource;

        if (target.index as usize) >= self.player(target.player).battle_area.len() {
            return None;
        }
        if partner_index == result_index {
            return None;
        }
        let hand_len = self.player(hand_owner).hand.len();
        if partner_index >= hand_len || result_index >= hand_len {
            return None;
        }

        if cost > 0 && !self.pay_memory(cost) {
            return None;
        }

        // Capture from_stack_top before the merge (for Digivolve event).
        let from_stack_top = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .map(|p| p.top_card().card_id(&self.card_data).to_string())
            .unwrap_or_default();

        // Remove the two hand cards. Remove the higher index first so the
        // lower index is not shifted before its removal.
        let (first, second) = if partner_index > result_index {
            (partner_index, result_index)
        } else {
            (result_index, partner_index)
        };
        let removed_first = self.player_mut(hand_owner).hand.remove(first);
        let removed_second = self.player_mut(hand_owner).hand.remove(second);
        let (partner_source, result_source) = if partner_index > result_index {
            (removed_first, removed_second)
        } else {
            (removed_second, removed_first)
        };
        // Capture top_card_id from the result source before it moves.
        let top_card_id = result_source.card_id(&self.card_data).to_string();

        let turn = self.turn_count;
        {
            let perm = &mut self.player_mut(target.player).battle_area[target.index as usize];
            perm.card_sources.push(partner_source);
            perm.card_sources.push(result_source);
            perm.turn_digivolved = turn;
        }

        let merged_handle = target;

        // `GameEvent::Digivolve` for the BT17-095-shape DNA path (one
        // material on field, one in hand, result also in hand). Same
        // conservative `was_blast_dna: false` as the both-field DNA path.
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Digivolve {
            seq,
            player: merged_handle.player,
            top_card_id,
            field_index: merged_handle.index,
            from_stack_top,
            was_dna: true,
            was_blast_dna: false,
            memory_paid: cost as i16,
        });

        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(merged_handle),
        );
        self.drain_effect_queue_with_dna_origin(true);

        self.enqueue_triggered(
            EffectTiming::OnDnaDigivolve,
            TriggerSource::Permanent(merged_handle),
        );
        self.drain_effect_queue_with_dna_origin(true);

        let event_card = self
            .player(merged_handle.player)
            .battle_area
            .get(merged_handle.index as usize)
            .map(|perm| perm.top_card().handle())?;
        self.enqueue_triggered(
            EffectTiming::OnDigivolve,
            TriggerSource::Digivolved {
                player: merged_handle.player,
                permanent: merged_handle,
                card: event_card,
                effect_initiated,
                dna_origin: true,
            },
        );
        self.drain_effect_queue();

        // Reward-shaping counters — see `dna_digivolve_inner` for rationale.
        // Only user-action DNAs (effect_initiated == false) credit.
        if !effect_initiated {
            self.n_digivolutions[merged_handle.player as usize] += 1;
            self.n_dna_digivolutions[merged_handle.player as usize] += 1;
        }

        Some(merged_handle)
    }

    /// Returns `true` when `card` may digivolve onto `perm` per standard
    /// evo-cost rules: `card` has an `EvoCost` entry whose `level` matches
    /// `perm.top_card()`'s level and whose color is present on
    /// `perm.top_card()`'s color list.
    ///
    /// Memory cost is **not** checked — blast digivolve bypasses memory,
    /// and regular digivolve pays memory at the call site. Mirrors
    /// Python's `can_digivolve(card, base_perm)` validator. Used by
    /// `combat::try_enter_counter` for §2.3 parity.
    pub fn can_digivolve(&self, card: &CardSource, perm: &crate::permanent::Permanent) -> bool {
        let base_top = perm.top_card();
        let Some(base_level) = base_top.digimon_level(&self.card_data) else {
            return false;
        };
        let base_colors = base_top.digimon_colors(&self.card_data);
        card.digivolution_costs(&self.card_data).iter().any(|ec| {
            ec.level == base_level
                && crate::action::mask::evo_color(ec.card_color)
                    .map(|c| base_colors.contains(&c))
                    .unwrap_or(false)
        })
    }

    /// Q3 (`G-DIGIVOLVE-TARGET-RESTRICTION`): a base permanent carrying one or
    /// more `CanOnlyDigivolveInto` modifiers may digivolve ONLY into a card whose
    /// name matches an allowed name. Returns `true` if `card` would be BLOCKED as
    /// a digivolve target of `base_handle` — i.e. at least one restriction entry
    /// is present whose allowed name does not match any of `card`'s names. Each
    /// entry is ANDed (the target must satisfy every restriction). Returns
    /// `false` (not blocked) when no restriction is present — the common case, so
    /// existing cards are unaffected. DCGO parity: `CanNotDigivolveStaticSelfEffect`
    /// (EX10-020 `cardCondition: !EqualsCardName("Apocalymon")`).
    pub(crate) fn digivolve_target_blocked_by_restriction(
        &self,
        base_handle: PermanentHandle,
        card: &CardSource,
    ) -> bool {
        let entries = self
            .modifiers
            .get(base_handle, ModifierType::CanOnlyDigivolveInto);
        if entries.is_empty() {
            return false;
        }
        let names = card.card_names(&self.card_data);
        for entry in entries {
            if let crate::modifiers::ModifierPayload::Name { value, .. } = &entry.payload {
                let allowed = names.iter().any(|n| *n == value.as_str());
                if !allowed {
                    return true;
                }
            }
        }
        false
    }

    // ─── Unified keyword query (Phase 3 Task 2) ──────────────────────

    /// Unified keyword query — returns `true` if the permanent's top card
    /// has `keyword` either printed natively on its face (from
    /// `CardData.keywords`) OR granted by an active modifier.
    ///
    /// This is the canonical engine-wide keyword lookup. Engine code MUST
    /// NOT call `self.modifiers.has_keyword(...)` directly — that only
    /// sees granted keywords and would miss native printed keywords.
    ///
    /// Returns `false` for out-of-range handles (e.g. player index or
    /// battle-area index doesn't exist) so callers don't need a guard.
    pub fn has_keyword(&self, handle: PermanentHandle, keyword: crate::enums::Keyword) -> bool {
        // Modifier-granted (end-of-turn grants, Ally buffs, etc.)
        if self.modifiers.has_keyword(handle, keyword) {
            return true;
        }
        // Native printed on the top card's face.
        let Some(player) = self.players.get(handle.player as usize) else {
            return false;
        };
        let Some(perm) = player.battle_area.get(handle.index as usize) else {
            return false;
        };
        let top = perm.top_card();
        // `data_index` is a direct Vec index — O(1), no iteration needed.
        let card_data = &self.card_data[top.data_index];
        if face_keywords(card_data).contains(&keyword) {
            return true;
        }
        // Inherited keyword grants from digivolution sources. Only cards
        // under the top card contribute inherited text, and any active_when
        // condition must pass before the keyword is considered live.
        let stack_size = perm.card_sources.len();
        let source_ids: Vec<(usize, usize, String, crate::card_source::CardHandle)> = perm
            .card_sources
            .iter()
            .enumerate()
            .map(|(i, s)| {
                (
                    i,
                    s.data_index,
                    s.card_id(&self.card_data).to_string(),
                    s.handle(),
                )
            })
            .collect();
        for (source_index, data_index, src_id, src_handle) in source_ids {
            let is_under = source_index + 1 < stack_size;
            if !is_under {
                continue;
            }
            if inherited_keywords(&self.card_data[data_index]).contains(&keyword) {
                return true;
            }
            let Some(effects) = self.effects_for_card(&src_id, src_handle) else {
                continue;
            };
            for effect in &effects {
                if !effect.declarative || !effect.inherited {
                    continue;
                }
                if effect.granted_keyword != Some(keyword) {
                    continue;
                }
                if let Some(cond) = &effect.condition {
                    let ctx = crate::effect_context::EffectReadContext::new(
                        self,
                        src_handle,
                        Some(handle),
                        handle.player,
                    );
                    if !cond(&ctx) {
                        continue;
                    }
                }
                return true;
            }
        }
        false
    }

    /// Re-install declarative process-backed effects from permanents currently
    /// on the field. Static effect builders still expose pure fields directly;
    /// this dispatcher is for declarative clauses lowered to process closures,
    /// such as filtered auras and player-scoped flood gates.
    pub fn tick_declarative_effects(&mut self) {
        self.modifiers.clear_materialized_declaratives();

        let mut sources = Vec::new();
        for (pid, player) in self.players.iter().enumerate() {
            let player_id = pid as PlayerId;
            for (index, perm) in player.battle_area.iter().enumerate() {
                let handle = PermanentHandle {
                    player: player_id,
                    index: index as u8,
                };
                let top = perm.top_card();
                sources.push((
                    top.card_id(&self.card_data).to_string(),
                    top.handle(),
                    Some(handle),
                    player_id,
                    false,
                ));

                let stack_size = perm.card_sources.len();
                for (source_index, source) in perm.card_sources.iter().enumerate() {
                    if source_index + 1 >= stack_size {
                        continue;
                    }
                    sources.push((
                        source.card_id(&self.card_data).to_string(),
                        source.handle(),
                        Some(handle),
                        player_id,
                        true,
                    ));
                }
            }

            if let Some(perm) = player.breeding_area.as_ref() {
                let handle = PermanentHandle {
                    player: player_id,
                    index: crate::action::space::BREEDING_TARGET as u8,
                };
                let top = perm.top_card();
                sources.push((
                    top.card_id(&self.card_data).to_string(),
                    top.handle(),
                    Some(handle),
                    player_id,
                    false,
                ));
            }

            // Track H §5 — security-zone-sourced auras. Face-up security
            // cards can carry `kind: aura, scope: security` declarative
            // clauses that grant DP/keyword/modifier to filter-matched
            // battle-area permanents while the source remains face-up in
            // the security stack. Source-permanent is `None` because
            // security entries have no battle-area handle; the install
            // closures still target battle-area handles for the matches.
            // Cleanup is automatic — each tick clears materialized
            // declaratives, then re-installs from active sources, so a
            // card leaving security simply stops re-installing on the
            // next tick. Mirrors DCGO `BT21_095.cs:CanUseCondition` →
            // `IsExistInSecurity(card, false)`.
            for card in &player.security {
                if !player.face_up_security.contains(&card.card_index) {
                    continue;
                }
                sources.push((
                    card.card_id(&self.card_data).to_string(),
                    card.handle(),
                    None,
                    player_id,
                    false,
                ));
            }
        }

        for (card_id, source_card, source_permanent, controller, inherited_source) in sources {
            let Some(effects) = self.effects_for_card(&card_id, source_card) else {
                continue;
            };
            for effect in effects {
                if !effect.declarative || effect.inherited != inherited_source {
                    continue;
                }
                if !effect.materializes_declarative_state || effect.process.is_none() {
                    continue;
                }
                // De-dup: a NON-inherited `grant_keyword` whose keyword is already
                // a PRINTED (metadata) keyword on this source is redundant with
                // `card_data` — `face_keywords` (consulted by both
                // `security_attack_keyword_bonus` and `has_keyword`) already counts
                // it, so materializing it too would double-count (e.g. BT21-029's
                // `<Security A. +1>` is both a printed keyword and a `grant_keyword`
                // clause). Inherited grants are NOT in `card_data`, so they must
                // still materialize.
                if !inherited_source {
                    if let Some(kw) = effect.granted_keyword {
                        let already_printed = self
                            .card_data_by_id(&card_id)
                            .is_some_and(|cd| face_keywords(cd).contains(&kw));
                        if already_printed {
                            continue;
                        }
                    }
                }
                if let Some(condition) = &effect.condition {
                    let rctx = crate::effect_context::EffectReadContext::new(
                        self,
                        source_card,
                        source_permanent,
                        controller,
                    );
                    if !condition(&rctx) {
                        continue;
                    }
                }
                if let Some(process) = effect.process.as_ref() {
                    let mut ctx = crate::effect_context::EffectContext::new(
                        self,
                        source_card,
                        source_permanent,
                        controller,
                    );
                    process(&mut ctx);
                }
            }
        }

        // Source-independent floating mass modifiers: re-scan the live candidate
        // set with each descriptor's predicate (relative to its `source_player`)
        // and install a materialized-declarative modifier on every current match
        // — so Digimon entering during the window receive the effect too
        // (G-CONTINUOUS-MASS-DP-DEBUFF). Cleared at the top of every tick along
        // with the source-bound declaratives; the descriptors themselves are
        // pruned at turn-end by `expire_floating_mass_modifiers`.
        if !self.floating_mass_modifiers.is_empty() {
            let floating = self.floating_mass_modifiers.clone();
            for fm in &floating {
                let mut ctx = crate::effect_context::EffectContext::new(
                    self,
                    fm.source_card,
                    None,
                    fm.source_player,
                );
                let matches = crate::dsl_cards::step::permanent_scan::scan(&ctx, &fm.filter, None);
                for h in matches {
                    // `Expiry::Permanent`: the per-permanent materialized entry is
                    // tick-ephemeral (cleared + re-installed every tick), so it must
                    // NOT be expired by the per-turn `expire_end_of_turn` pass — the
                    // floating DESCRIPTOR (pruned by `expire_floating_mass_modifiers`)
                    // is the sole lifetime authority. Using `fm.expiry` here would
                    // expire the entry one turn-end early relative to the descriptor
                    // (`*NextTurn` skips live on the descriptor, not the entry).
                    ctx.add_declarative_modifier(h, fm.modifier, fm.value, Expiry::Permanent);
                }
            }
        }
    }

    /// Register a source-independent continuous mass modifier (see
    /// `crate::floating_modifier`). Computes the `*NextTurn` skip count at
    /// install time (mirroring `ModifierEntry` installs) and materializes it
    /// immediately so it is visible without waiting for the next tick.
    pub fn add_floating_mass_modifier(
        &mut self,
        filter: digimon_dsl::compiled::CompiledPredicate,
        modifier: ModifierType,
        value: i32,
        source_card: crate::card_source::CardHandle,
        source_player: PlayerId,
        expiry: Expiry,
    ) {
        let pending_skips = crate::modifiers::pending_skips_for_install(
            expiry,
            source_player,
            self.turn_player(),
        );
        self.floating_mass_modifiers
            .push(crate::floating_modifier::FloatingMassModifier {
                filter,
                modifier,
                value,
                source_card,
                source_player,
                expiry,
                pending_skips,
            });
        self.tick_declarative_effects();
    }

    /// Prune floating mass modifiers whose turn-relative expiry fires at the end
    /// of `ending_player`'s turn. Mirrors `ModifierRegistry::expire_end_of_turn`
    /// (same `pending_skips` `*NextTurn` skip semantics). Called from
    /// `Game::end_turn` alongside the per-permanent / per-player expiry passes.
    pub fn expire_floating_mass_modifiers(&mut self, ending_player: PlayerId) {
        let had_any = !self.floating_mass_modifiers.is_empty();
        self.floating_mass_modifiers.retain_mut(|fm| {
            // Does THIS turn-end concern this descriptor's expiry?
            let relevant = match fm.expiry {
                // Fires on every turn-end.
                Expiry::EndOfTurn => true,
                // Fires when the ending turn is the source's opponent's.
                Expiry::EndOfOpponentsTurn | Expiry::EndOfOpponentsNextTurn => {
                    ending_player != fm.source_player
                }
                // Fires when the ending turn is the source's own.
                Expiry::EndOfYourTurn | Expiry::EndOfYourNextTurn => {
                    ending_player == fm.source_player
                }
                // Not turn-end-relative — not expired here.
                Expiry::Permanent
                | Expiry::EndOfAttack
                | Expiry::EndOfBattle
                | Expiry::UntilLeaveField
                | Expiry::UntilCondition
                | Expiry::OnceUsed(_) => false,
            };
            if !relevant {
                return true;
            }
            if fm.pending_skips > 0 {
                fm.pending_skips -= 1;
                return true; // `*NextTurn` skip — survive this turn-end
            }
            false // expired → drop
        });
        // Refresh materialized state so a `dp_of`/`effective_dp` read taken right
        // after the turn-end (before the next action's tick) reflects the pruned
        // set: clear the tick-ephemeral materialized entries and re-install only
        // from the descriptors that survived. Scoped to games that actually use
        // floating modifiers (`had_any`) so it never perturbs other games.
        if had_any {
            self.tick_declarative_effects();
        }
    }

    /// Track H Phase 4k — clear all per-permanent state when a
    /// permanent leaves the field, AND prune the corresponding
    /// granted-triggered-effect body registry entries. Wraps the
    /// narrower `ModifierRegistry::clear_permanent` so call sites
    /// don't have to remember the body-registry cleanup separately.
    /// Returns the count of body-registry entries removed (mostly for
    /// tests / instrumentation).
    pub fn clear_permanent_full(&mut self, handle: crate::permanent::PermanentHandle) -> usize {
        let body_ids = self
            .modifiers
            .drain_granted_triggered_ids_on_carrier(handle);
        self.modifiers.clear_permanent(handle);
        let mut removed = 0usize;
        for id in body_ids {
            if self
                .effect_queue
                .iter()
                .any(|queued| queued.granted_effect_id == Some(id))
            {
                continue;
            }
            if self.granted_effect_bodies.remove(id).is_some() {
                removed += 1;
            }
        }
        removed
    }

    /// Soft-remove a permanent whose `card_sources` Vec is now empty after
    /// some effect moved its body card(s) elsewhere. This is the DCGO
    /// `CardObjectController.RemoveField(permanent)` analog — distinct from
    /// `DestroyPermanentsClass.Destroy()` (no OnDeletion, no replacement
    /// window, no trash for the body since it already moved).
    ///
    /// Cleans up: granted-triggered-effect bodies, modifiers, linked cards
    /// (flow to trash + fire `OnLinkedCardTrashed` observers), the battle_area
    /// slot itself, and shifts modifier indices for surviving permanents.
    ///
    /// **Caller must already have moved the body card(s)** — this function
    /// does not extract or route the body. If `src.card_sources` is non-empty
    /// when called, returns `false` and does nothing (defensive: also handles
    /// the case where the slot is already gone).
    ///
    /// Returns `true` if a cleanup happened, `false` if no cleanup was needed
    /// (slot wasn't empty or didn't exist). Callers holding other
    /// `PermanentHandle`s should call [`Self::shift_handle_after_soft_remove`]
    /// to fix up handles after a successful cleanup.
    ///
    /// See `G-PERMANENT-EMPTY-DURING-BATCH-DELETION` (mis-named — actually
    /// the digivolve-from-material zombie class) in
    /// `qa/archetype-qa/engine-gaps.md` for the original surfacing.
    pub(crate) fn soft_remove_if_emptied(
        &mut self,
        src: crate::permanent::PermanentHandle,
    ) -> bool {
        let needs_cleanup = self
            .player(src.player)
            .battle_area
            .get(src.index as usize)
            .map(|p| p.card_sources.is_empty())
            .unwrap_or(false);
        if !needs_cleanup {
            return false;
        }

        // Capture linked cards before the slot is removed.
        let linked_cards = std::mem::take(
            &mut self.player_mut(src.player).battle_area[src.index as usize].linked_cards,
        );
        let had_linked = !linked_cards.is_empty();

        // Remove the empty slot BEFORE any drain — slot must not be visible
        // to subsequent trigger iteration.
        self.clear_permanent_full(src);
        self.modifiers.expire_player_on_permanent_leave(src);
        self.player_mut(src.player)
            .delete_permanent(src.index as usize);
        self.modifiers
            .shift_after_battle_area_remove(src.player, src.index);

        // Linked cards lose their host → trash + OnLinkedCardTrashed observer.
        // Matches the linked-card flow in `trash_single_for_batch`
        // (combat.rs:3740-3768). Runs after the slot is gone so the observer
        // drain is safe.
        if had_linked {
            for card in linked_cards {
                self.player_mut(src.player).trash.push(card);
            }
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnLinkedCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(
                        pid as crate::enums::PlayerId,
                    ),
                );
            }
            self.drain_effect_queue();
        }
        true
    }

    /// Adjust a `PermanentHandle` after a `soft_remove_if_emptied` call.
    /// If `removed_player == handle.player && removed_index < handle.index`,
    /// the battle_area `Vec::remove` shifted `handle.index` down by 1.
    /// Returns the (possibly shifted) handle.
    pub(crate) fn shift_handle_after_soft_remove(
        removed: crate::permanent::PermanentHandle,
        mut handle: crate::permanent::PermanentHandle,
    ) -> crate::permanent::PermanentHandle {
        if removed.player == handle.player && removed.index < handle.index {
            handle.index -= 1;
        }
        handle
    }

    /// Track H §3 — fire all granted triggered effects on `carrier`
    /// whose registered timing matches `timing`. Each body runs
    /// inline with `EffectContext::source_card` set to the grantor
    /// (mirroring DCGO `EffectSourceCard`) and `source_permanent` set
    /// to the carrier (mirroring DCGO `EffectSourcePermanent`).
    ///
    /// Inline-fire model (v1): the body runs synchronously, before
    /// `drain_effect_queue` resolves the rest of the trigger fan-out.
    /// Suitable for grants that mutate state without prompting (memory
    /// gain, modifier installs, etc.). Selection-driving granted
    /// bodies are not yet supported — they belong on the standard
    /// queue/drain path, which is a follow-up.
    pub fn fire_granted_triggered_effects(
        &mut self,
        carrier: crate::permanent::PermanentHandle,
        timing: crate::enums::EffectTiming,
    ) {
        let entries = self.modifiers.granted_triggered_for_timing(carrier, timing);
        for (source_card, source_player, body) in entries {
            // Immunity gate (mirrors the queue dispatcher): skip when the
            // carrier is unaffected by the grantor's effects — `<Progress>` /
            // attack-scoped immunity (Q2) or a general `CannotBeAffected`
            // opponent-effect immunity (Q17). The opponent-effect immunity
            // filter is source-kind-agnostic, so `Digimon` suffices on this
            // fallback path.
            if self.progress_excludes(carrier, Some(source_player))
                || self.permanent_is_unaffected_by_effect(
                    carrier,
                    source_player,
                    crate::enums::EffectSourceKind::Digimon,
                )
            {
                continue;
            }
            // D4 / DCGO: the granted body runs as the carrier's OWN effect
            // (effect_source_player = carrier.player), so a deletion it causes
            // is the carrier-controller's OwnEffect (judge-quiz Q16). The
            // grantor (`source_player`) is used only for the immunity gate.
            let mut ctx = crate::effect_context::EffectContext::new(
                self,
                source_card,
                Some(carrier),
                carrier.player,
            );
            body(&mut ctx);
        }
    }

    /// Returns the `PermanentHandle` of the currently-attacking permanent,
    /// or `None` when no attack is in flight. Reads `pending_attack.attacker`
    /// — the same source the mask and combat-resolution code use.
    ///
    /// Used by `progress_excludes` to gate opponent-effect mutations on
    /// the Progress carrier specifically while it is the attacker.
    pub fn current_attacker(&self) -> Option<PermanentHandle> {
        self.pending_attack.as_ref().map(|p| p.attacker)
    }

    /// Gate predicate for the `<Progress>` keyword **and** the
    /// `ImmunityToOpponentEffects` modifier (both surface the same
    /// "opponent cannot target this with effects while it is the current
    /// attacker" rule; bundling them keeps every opponent-effect call-site
    /// to one branch).
    ///
    /// Returns `true` when:
    ///   - `target` is the current attacker (`current_attacker() == Some(target)`), AND
    ///   - `source` is `Some(pid)` where `pid != target.player`, AND
    ///   - `target` has either `Keyword::Progress` (printed or granted) **or**
    ///     `ModifierType::ImmunityToOpponentEffects`.
    ///
    /// Returns `false` if `source` is `None` (rule-driven mutations: battle,
    /// cost, rule checks). Opponent *effects* are gated; battle damage and
    /// cost-triggered cleanup are not.
    ///
    /// `ImmunityToOpponentEffects` is currently only applied with
    /// attack-scoped expiry (`EndOfAttack` / `EndOfBattle`), so the
    /// `current_attacker` gate is always satisfied when the modifier is
    /// live. If a future card grants the modifier with broader expiry,
    /// split this into `progress_excludes` (Progress only) +
    /// `effect_immunity_excludes` (modifier only) and update both
    /// call-sites; the helpers' shape is identical so the split is
    /// mechanical.
    ///
    /// Callers: `select_opponent_permanent` (selection-time gate, Phase A)
    /// and the script-API mutation entry points on `EffectContext` (Phase B,
    /// broadened in Phase E prep): `delete_permanent`, `return_to_hand`,
    /// `return_to_deck`, `de_digivolve`, `suspend`, and `add_modifier` /
    /// `add_dp_modifier`. The `add_modifier` site is unconditional — every
    /// `ModifierType` and every value (positive, negative, or zero) is gated,
    /// matching DCGO's `CanNotAffected` semantics literally.
    pub fn progress_excludes(
        &self,
        target: PermanentHandle,
        source: Option<crate::enums::PlayerId>,
    ) -> bool {
        let Some(src) = source else { return false };
        if src == target.player {
            return false;
        }
        if self.current_attacker() != Some(target) {
            return false;
        }
        self.has_keyword(target, crate::enums::Keyword::Progress)
            || self.modifiers.has(
                target,
                crate::enums::ModifierType::ImmunityToOpponentEffects,
            )
    }

    pub fn permanent_is_unaffected_by_effect(
        &self,
        target: PermanentHandle,
        effect_controller: crate::enums::PlayerId,
        source_kind: crate::enums::EffectSourceKind,
    ) -> bool {
        use crate::modifiers::EffectControllerFilter;

        self.modifiers
            .get(target, crate::enums::ModifierType::CannotBeAffected)
            .into_iter()
            .any(|entry| {
                let Some(filter) = entry.effect_immunity_filter else {
                    return true;
                };
                let source_kind_matches = filter
                    .source_kind
                    .map(|expected| expected == source_kind)
                    .unwrap_or(true);
                if !source_kind_matches {
                    return false;
                }
                match filter.controller {
                    EffectControllerFilter::Any => true,
                    EffectControllerFilter::OpponentOnly => effect_controller != target.player,
                    EffectControllerFilter::OwnOnly => effect_controller == target.player,
                }
            })
    }

    /// Returns `true` when an effect is currently resolving AND its
    /// controller is not `target`'s controller. The "opponent effect is
    /// targeting me" predicate that drives Mephistomon-style OnDeletion
    /// riders, Scapegoat eligibility (cause ≠ OwnEffect), and the
    /// `was_deleted_by_opponent` accessor.
    ///
    /// Returns `false` when:
    ///   - no effect is currently resolving (`effect_source_player == None`),
    ///   - the resolving effect's controller equals `target.player`.
    ///
    /// Phase B §B5.
    pub fn opponent_sourced_mutation(&self, target: crate::permanent::PermanentHandle) -> bool {
        match self.effect_source_player {
            Some(src) => src != target.player,
            None => false,
        }
    }

    /// Sum the net security-attack modifier contributed by native printed
    /// `<Security A. +N>` and `<Security A. -N>` keywords on `target`.
    /// Called by `resolve_player_security_loop` alongside the existing
    /// `ModifierType::SecurityAttackChange` sum so cards with only the
    /// printed keyword behave correctly without a hand-rolled script.
    pub fn security_attack_keyword_bonus(&self, target: crate::permanent::PermanentHandle) -> i32 {
        use crate::enums::Keyword;
        let Some(player) = self.players.get(target.player as usize) else {
            return 0;
        };
        let Some(perm) = player.battle_area.get(target.index as usize) else {
            return 0;
        };
        // Top-card face keywords count; buried sources only contribute
        // inherited text keywords.
        let mut total = 0i32;
        let stack_size = perm.card_sources.len();
        for (source_index, src) in perm.card_sources.iter().enumerate() {
            let card_data = &self.card_data[src.data_index];
            let keywords = if source_index + 1 == stack_size {
                face_keywords(card_data)
            } else {
                inherited_keywords(card_data)
            };
            for kw in &keywords {
                match kw {
                    Keyword::SecurityAttackPlus(n) => total += *n as i32,
                    Keyword::SecurityAttackMinus(n) => total -= *n as i32,
                    _ => {}
                }
            }
        }
        // Fold in registry-side granted keywords (e.g. an aura's
        // `grant_keyword: SecurityAttackPlus`). Printed keywords above come
        // from `card_sources`; aura grants live in `Modifiers::permanent_keywords`.
        total += self.modifiers.granted_security_attack_keyword_bonus(target);
        total
    }

    pub fn dynamic_dp_aura_bonus(&self, target: crate::permanent::PermanentHandle) -> i32 {
        self.live_declarative_formula_sum(target, false).0
    }

    pub fn static_dp_aura_bonus(&self, target: crate::permanent::PermanentHandle) -> i32 {
        use crate::effect_context::EffectReadContext;

        let Some(permanent) = self
            .players
            .get(target.player as usize)
            .and_then(|player| player.battle_area.get(target.index as usize))
        else {
            return 0;
        };

        let stack_size = permanent.card_sources.len();
        let mut total = 0;
        for (source_index, source) in permanent.card_sources.iter().enumerate() {
            let inherited_source = source_index + 1 < stack_size;
            let card_id = source.card_id(&self.card_data).to_string();
            let Some(effects) = self.effects_for_card(&card_id, source.handle()) else {
                continue;
            };
            for effect in effects {
                if !effect.declarative || effect.inherited != inherited_source {
                    continue;
                }
                if effect.materializes_declarative_state
                    || effect.dp_modifier == 0
                    || effect.dp_modifier_fn.is_some()
                    || effect.applies_to_opponent_security_dp
                {
                    continue;
                }
                let rctx =
                    EffectReadContext::new(self, source.handle(), Some(target), target.player);
                if let Some(condition) = &effect.condition {
                    if !condition(&rctx) {
                        continue;
                    }
                }
                total += effect.dp_modifier;
            }
        }
        total
    }

    pub fn dynamic_security_attack_aura_bonus(
        &self,
        target: crate::permanent::PermanentHandle,
    ) -> Option<i32> {
        let (value, found) = self.live_declarative_formula_sum(target, true);
        found.then_some(value)
    }

    /// True when `target` currently has any Security Attack delta. Printed
    /// and modifier-granted `<Security A. +/-N>` keywords, temporary
    /// `SecurityAttackChange` modifiers, and formula-driven declarative
    /// security-attack auras all count.
    pub fn has_security_attack_change(&self, target: crate::permanent::PermanentHandle) -> bool {
        self.security_attack_keyword_bonus(target) != 0
            || self
                .modifiers
                .sum(target, ModifierType::SecurityAttackChange)
                != 0
            || self
                .dynamic_security_attack_aura_bonus(target)
                .is_some_and(|bonus| bonus != 0)
    }

    /// Shared Digimon-target attack gate for target-scoped combat
    /// restrictions. `CanAttackTargetDefendingPermanent` is the established
    /// affirmative override for target-carried attack bans.
    pub fn attack_target_blocked_by_modifier(
        &self,
        attacker: crate::permanent::PermanentHandle,
        target: crate::permanent::PermanentHandle,
    ) -> bool {
        if self
            .modifiers
            .has(target, ModifierType::CanAttackTargetDefendingPermanent)
        {
            return false;
        }
        if self.modifiers.has(target, ModifierType::CannotAttackTarget) {
            return true;
        }
        self.modifiers.has(
            target,
            ModifierType::CannotBeAttackedBySecurityAttackChanged,
        ) && self.has_security_attack_change(attacker)
    }

    fn live_declarative_formula_sum(
        &self,
        target: crate::permanent::PermanentHandle,
        security_attack: bool,
    ) -> (i32, bool) {
        use crate::effect_context::EffectReadContext;

        let mut sources = Vec::new();
        for (pid, player) in self.players.iter().enumerate() {
            let player_id = pid as PlayerId;
            for (index, perm) in player.battle_area.iter().enumerate() {
                let host = PermanentHandle {
                    player: player_id,
                    index: index as u8,
                };
                let stack_size = perm.card_sources.len();
                for (source_index, source) in perm.card_sources.iter().enumerate() {
                    let inherited_source = source_index + 1 < stack_size;
                    sources.push((
                        source.card_id(&self.card_data).to_string(),
                        source.handle(),
                        Some(host),
                        player_id,
                        inherited_source,
                    ));
                }
            }

            if let Some(perm) = player.breeding_area.as_ref() {
                let host = PermanentHandle {
                    player: player_id,
                    index: crate::action::space::BREEDING_TARGET as u8,
                };
                let top = perm.top_card();
                sources.push((
                    top.card_id(&self.card_data).to_string(),
                    top.handle(),
                    Some(host),
                    player_id,
                    false,
                ));
            }
        }

        let mut total = 0;
        let mut found = false;
        for (card_id, source_card, source_permanent, controller, inherited_source) in sources {
            let Some(effects) = self.effects_for_card(&card_id, source_card) else {
                continue;
            };
            for effect in effects {
                if !effect.declarative || effect.inherited != inherited_source {
                    continue;
                }
                let Some(formula_fn) = (if security_attack {
                    effect.security_attack_fn.as_ref()
                } else {
                    effect.dp_modifier_fn.as_ref()
                }) else {
                    continue;
                };
                let ctx = EffectReadContext::new(self, source_card, source_permanent, controller);
                if let Some(condition) = &effect.condition {
                    if !condition(&ctx) {
                        continue;
                    }
                }
                if let Some(value) = formula_fn(&ctx, target) {
                    if security_attack {
                        total = if found { total.max(value) } else { value };
                    } else {
                        total += value;
                    }
                    found = true;
                }
            }
        }
        (total, found)
    }

    // ─── Effect-listing API (§4.5c) ──────────────────────────────────

    /// Enumerate a card's effects by asking the registry for its impl.
    /// Returns `None` when no impl is registered so hot-path callers (the
    /// mask builder) can skip the match-iterate loop entirely instead of
    /// walking an empty `Vec`.
    ///
    /// Analogous to Python's `CardSource.effect_list(timing)` but expressed
    /// Rust-idiomatically: the registry is owned by `Game`, so this is the
    /// single entry point callers use regardless of whether the card lives
    /// in hand, trash, or a `card_sources` slot. Callers filter the returned
    /// vec by `effect.timing` (e.g. `MainFromHand`).
    ///
    /// The inner `Vec` allocation is driven by `CardEffect::effects(handle)`
    /// re-boxing per-instance closures and is unavoidable with the current
    /// trait shape. The helper does not add an extra empty-case allocation.
    pub fn effects_for_card(
        &self,
        card_id: &str,
        handle: crate::card_source::CardHandle,
    ) -> Option<Vec<crate::effect::Effect>> {
        // Registry effects come first — a hand-authored script owns its
        // slot order. Phase 7 Task 6 appends keyword-derived auto-install
        // replacements (Barrier / Evade / Fragment(N) / Decode) so cards
        // with those printed keywords get the matching WhenWouldBe* process
        // without hand-authoring. Phase D Tasks 4-10 (Fragment(N), ArmorPurge,
        // Save, Decoy, Fortitude, Partition, MaterialSave(N)) are
        // auto-installed.
        let registry_effects = self
            .effect_registry
            .get(card_id)
            .map(|impl_| impl_.effects(handle));

        // Look up CardData for this card_id. The vec scan is O(card_data_len)
        // but is only hit once per effect query, and `effects_for_card` is
        // typically called at state-change fire-sites, not in the mask hot
        // loop — so the cost is acceptable for v1.
        let native_keywords = self
            .card_data
            .iter()
            .find(|cd| cd.card_id == card_id)
            .map(|cd| cd.keywords.clone())
            .unwrap_or_default();
        let mut auto_effects: Vec<crate::effect::Effect> = self
            .card_data
            .iter()
            .find(|cd| cd.card_id == card_id)
            .map(|cd| {
                let mut effects: Vec<crate::effect::Effect> = cd
                    .keywords
                    .iter()
                    .flat_map(|kw| {
                        crate::cards::keyword_effects::keyword_to_auto_effect(*kw, handle)
                    })
                    .collect();

                if self.card_handle_is_under_top(handle) {
                    for kw in inherited_keywords(cd) {
                        effects.extend(
                            crate::cards::keyword_effects::keyword_to_auto_effect(kw, handle)
                                .into_iter()
                                .map(|mut effect| {
                                    effect.inherited = true;
                                    effect
                                }),
                        );
                    }
                }

                effects
            })
            .unwrap_or_default();

        // Declarative `grant_keyword` clauses are semantically equivalent to
        // printed keywords for keyword lookups. If the granted keyword carries
        // replacement behavior (Barrier, Armor Purge, Scapegoat, etc.), also
        // synthesize the keyword's auto-effect so inherited keyword grants can
        // participate in replacement scans. Conditional grants are omitted
        // here for now because `ConditionFn` is boxed and not cloneable; those
        // cards should lower an explicit conditional replacement until the
        // condition-composition surface is added.
        if let Some(es) = registry_effects.as_ref() {
            for grant in es {
                let Some(kw) = grant.granted_keyword else {
                    continue;
                };
                if !grant.declarative || grant.condition.is_some() {
                    continue;
                }
                if !grant.inherited && native_keywords.contains(&kw) {
                    continue;
                }
                auto_effects.extend(
                    crate::cards::keyword_effects::keyword_to_auto_effect(kw, handle)
                        .into_iter()
                        .map(|mut effect| {
                            effect.inherited = grant.inherited;
                            effect
                        }),
                );
            }
        }

        match (registry_effects, auto_effects.is_empty()) {
            (Some(mut es), false) => {
                es.extend(auto_effects);
                Some(es)
            }
            (Some(es), true) => Some(es),
            (None, false) => Some(auto_effects),
            (None, true) => None,
        }
    }

    fn card_handle_is_under_top(&self, handle: crate::card_source::CardHandle) -> bool {
        for player in &self.players {
            for permanent in &player.battle_area {
                let stack_len = permanent.card_sources.len();
                for (source_index, source) in permanent.card_sources.iter().enumerate() {
                    if source.handle() == handle {
                        return source_index + 1 < stack_len;
                    }
                }
            }
            if let Some(permanent) = &player.breeding_area {
                let stack_len = permanent.card_sources.len();
                for (source_index, source) in permanent.card_sources.iter().enumerate() {
                    if source.handle() == handle {
                        return source_index + 1 < stack_len;
                    }
                }
            }
        }
        false
    }

    /// Resolve a `CardHandle` (card_index) to its `CardKind` by scanning all
    /// player zones.
    ///
    /// Used by `source_is_tamer` flood-gate helpers on `EffectContext` /
    /// `EffectReadContext` to discriminate Tamer-sourced effects from
    /// Digimon/Option-sourced ones (matches DCGO `ICardEffect.IsTamerEffect`).
    ///
    /// Returns `None` if no `CardSource` with the given `card_index` is found
    /// in any zone (this should not occur in practice for a live effect).
    pub(crate) fn card_kind_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> Option<crate::enums::CardKind> {
        let target_index = handle.0;
        for player in &self.players {
            // Hand
            if let Some(cs) = player.hand.iter().find(|c| c.card_index == target_index) {
                return Some(self.card_data[cs.data_index].card_kind);
            }
            // Trash
            if let Some(cs) = player.trash.iter().find(|c| c.card_index == target_index) {
                return Some(self.card_data[cs.data_index].card_kind);
            }
            // Battle area (card_sources stacks)
            for perm in &player.battle_area {
                if let Some(cs) = perm
                    .card_sources
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(self.card_data[cs.data_index].card_kind);
                }
                // Linked cards (Tamer equipment)
                if let Some(cs) = perm
                    .linked_cards
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(self.card_data[cs.data_index].card_kind);
                }
            }
            // Breeding area
            if let Some(breeding) = &player.breeding_area {
                if let Some(cs) = breeding
                    .card_sources
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(self.card_data[cs.data_index].card_kind);
                }
            }
            // Security (e.g. when effect fires from security card)
            if let Some(cs) = player
                .security
                .iter()
                .find(|c| c.card_index == target_index)
            {
                return Some(self.card_data[cs.data_index].card_kind);
            }
            // Deck (rare, but possible for mid-search effects)
            if let Some(cs) = player.deck.iter().find(|c| c.card_index == target_index) {
                return Some(self.card_data[cs.data_index].card_kind);
            }
        }
        // Also check revealed_cards pool
        if let Some(cs) = self
            .revealed_cards
            .iter()
            .find(|c| c.card_index == target_index)
        {
            return Some(self.card_data[cs.data_index].card_kind);
        }
        None
    }

    /// Resolve a `CardHandle` to its `&CardData` by scanning all zones —
    /// mirrors `card_kind_for_handle` but returns the full data record so
    /// callers can read name, traits, colors, etc. Used by the DSL predicate
    /// evaluator (`dsl_cards::predicate`).
    ///
    /// Returns `None` if no `CardSource` with the given `card_index` is found.
    pub fn card_data_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> Option<&crate::card_data::CardData> {
        let target_index = handle.0;
        for player in &self.players {
            if let Some(cs) = player.hand.iter().find(|c| c.card_index == target_index) {
                return Some(&self.card_data[cs.data_index]);
            }
            if let Some(cs) = player.trash.iter().find(|c| c.card_index == target_index) {
                return Some(&self.card_data[cs.data_index]);
            }
            for perm in &player.battle_area {
                if let Some(cs) = perm
                    .card_sources
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(&self.card_data[cs.data_index]);
                }
                if let Some(cs) = perm
                    .linked_cards
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(&self.card_data[cs.data_index]);
                }
            }
            if let Some(breeding) = &player.breeding_area {
                if let Some(cs) = breeding
                    .card_sources
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(&self.card_data[cs.data_index]);
                }
            }
            if let Some(cs) = player
                .security
                .iter()
                .find(|c| c.card_index == target_index)
            {
                return Some(&self.card_data[cs.data_index]);
            }
            if let Some(cs) = player.deck.iter().find(|c| c.card_index == target_index) {
                return Some(&self.card_data[cs.data_index]);
            }
        }
        if let Some(cs) = self
            .revealed_cards
            .iter()
            .find(|c| c.card_index == target_index)
        {
            return Some(&self.card_data[cs.data_index]);
        }
        None
    }

    /// Resolve a `CardHandle` to its live `CardSource` instance.
    pub fn card_source_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> Option<&crate::card_source::CardSource> {
        let target_index = handle.0;
        for player in &self.players {
            if let Some(cs) = player.hand.iter().find(|c| c.card_index == target_index) {
                return Some(cs);
            }
            if let Some(cs) = player.trash.iter().find(|c| c.card_index == target_index) {
                return Some(cs);
            }
            for perm in &player.battle_area {
                if let Some(cs) = perm
                    .card_sources
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(cs);
                }
                if let Some(cs) = perm
                    .linked_cards
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(cs);
                }
            }
            if let Some(breeding) = &player.breeding_area {
                if let Some(cs) = breeding
                    .card_sources
                    .iter()
                    .find(|c| c.card_index == target_index)
                {
                    return Some(cs);
                }
            }
            if let Some(cs) = player
                .security
                .iter()
                .find(|c| c.card_index == target_index)
            {
                return Some(cs);
            }
            if let Some(cs) = player.deck.iter().find(|c| c.card_index == target_index) {
                return Some(cs);
            }
        }
        self.revealed_cards
            .iter()
            .find(|c| c.card_index == target_index)
    }

    pub fn provenance_token_for_card(
        &self,
        card: crate::card_source::CardHandle,
    ) -> crate::trigger_context::ProvenanceToken {
        crate::trigger_context::ProvenanceToken::from(card)
    }

    pub fn resolve_provenance_token(
        &self,
        token: crate::trigger_context::ProvenanceToken,
    ) -> Option<crate::trigger_context::EventSubject> {
        if token.0 > u16::MAX as u64 {
            return None;
        }
        let card = crate::card_source::CardHandle(token.0 as u16);
        let target_index = card.0;

        for (player_index, player) in self.players.iter().enumerate() {
            let player_id = player_index as crate::enums::PlayerId;
            for (index, permanent) in player.battle_area.iter().enumerate() {
                if permanent
                    .card_sources
                    .iter()
                    .any(|source| source.card_index == target_index)
                {
                    return Some(crate::trigger_context::EventSubject::Permanent(
                        PermanentHandle {
                            player: player_id,
                            index: index as u8,
                        },
                    ));
                }
                if permanent
                    .linked_cards
                    .iter()
                    .any(|source| source.card_index == target_index)
                {
                    return Some(crate::trigger_context::EventSubject::Card {
                        card,
                        zone: crate::enums::Zone::BattleArea,
                    });
                }
            }
            if let Some(breeding) = &player.breeding_area {
                if breeding
                    .card_sources
                    .iter()
                    .any(|source| source.card_index == target_index)
                {
                    return Some(crate::trigger_context::EventSubject::Permanent(
                        PermanentHandle {
                            player: player_id,
                            index: crate::action::space::BREEDING_TARGET as u8,
                        },
                    ));
                }
            }
            if player
                .hand
                .iter()
                .any(|source| source.card_index == target_index)
            {
                return Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Hand,
                });
            }
            if player
                .trash
                .iter()
                .any(|source| source.card_index == target_index)
            {
                return Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Trash,
                });
            }
            if player
                .security
                .iter()
                .any(|source| source.card_index == target_index)
            {
                return Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Security,
                });
            }
            if player
                .deck
                .iter()
                .any(|source| source.card_index == target_index)
            {
                return Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::Deck,
                });
            }
            if player
                .digitama_deck
                .iter()
                .any(|source| source.card_index == target_index)
            {
                return Some(crate::trigger_context::EventSubject::Card {
                    card,
                    zone: crate::enums::Zone::DigitamaDeck,
                });
            }
        }
        if self
            .revealed_cards
            .iter()
            .any(|source| source.card_index == target_index)
        {
            return Some(crate::trigger_context::EventSubject::Card {
                card,
                zone: crate::enums::Zone::Reveal,
            });
        }
        None
    }

    /// Strict variant of [`resolve_provenance_token`] for "is this played card
    /// still a Digimon on the battle area?" identity checks.
    ///
    /// Returns `Some(handle)` only when the card identified by `token` is
    /// currently the **top card** of a battle-area permanent. Yields `None` if
    /// the card is a digivolution card under a different top, has been removed
    /// from play, or is in any other zone (hand, trash, security, deck,
    /// linked_cards, reveal).
    ///
    /// This is the resolution semantic required by play-verb `bind_as`
    /// bindings ([`crate::dsl_cards::bindings::BindingValue::PlayedPermanent`])
    /// consumed by `return_to_hand` and friends after a `schedule_delayed`
    /// boundary. The permissive [`resolve_provenance_token`] — which returns
    /// `Permanent(handle)` for *any* card in *any* permanent's `card_sources` —
    /// matches DCGO's `IsPermanentExistsOnBattleArea(selectedPermanent)` only
    /// for the specific case where the played card is still the carrier's top;
    /// once the played card became a digivolution card the original
    /// `Permanent` object would have been replaced and the check would fail.
    ///
    /// See change `fix-played-binding-uses-provenance` for the cross-engine
    /// rationale and the BT16-085 + Paildramon scenario this exists to handle.
    pub fn resolve_token_as_battle_area_top(
        &self,
        token: crate::trigger_context::ProvenanceToken,
    ) -> Option<PermanentHandle> {
        if token.0 > u16::MAX as u64 {
            return None;
        }
        let target_index = token.0 as u16;
        for (player_index, player) in self.players.iter().enumerate() {
            for (index, permanent) in player.battle_area.iter().enumerate() {
                if permanent.top_card().card_index == target_index {
                    return Some(PermanentHandle {
                        player: player_index as crate::enums::PlayerId,
                        index: index as u8,
                    });
                }
            }
        }
        None
    }

    // ─── Tensor support: per-source DP + OPT helpers (§3.1 / §3.2) ───

    /// Sum of static `dp_modifier` values from a single source's effects
    /// that pass the inherited/top filter and their condition (if any).
    /// Returns a signed raw DP delta. Tensor writes this divided by DP_NORM.
    pub fn source_dp_contribution(
        &self,
        perm: crate::permanent::PermanentHandle,
        source_index: usize,
    ) -> i32 {
        use crate::effect_context::EffectReadContext;
        let Some(permanent) = self
            .players
            .get(perm.player as usize)
            .and_then(|p| p.battle_area.get(perm.index as usize))
        else {
            return 0;
        };
        let stack_size = permanent.card_sources.len();
        let Some(source) = permanent.card_sources.get(source_index) else {
            return 0;
        };
        let is_under = source_index + 1 < stack_size;
        let card_id = source.card_id(&self.card_data).to_string();
        let Some(impl_) = self.effect_registry.get(&card_id) else {
            return 0;
        };
        let effects = impl_.effects(source.handle());

        let mut total = 0i32;
        for effect in &effects {
            if effect.dp_modifier == 0 && effect.dp_modifier_fn.is_none() {
                continue;
            }
            if is_under != effect.inherited {
                continue;
            }
            let ctx = EffectReadContext::new(self, source.handle(), Some(perm), perm.player);
            if let Some(cond) = &effect.condition {
                if !cond(&ctx) {
                    continue;
                }
            }
            total += effect.dp_modifier;
            if let Some(formula_fn) = effect.dp_modifier_fn.as_ref() {
                if let Some(value) = formula_fn(&ctx, perm) {
                    total += value;
                }
            }
        }
        total
    }

    /// OPT effects on a permanent, counted across its entire digivolution
    /// stack with the same inherited/top filter as `source_dp_contribution`.
    /// Linked card effects are not iterated (residual gap §3.1b).
    pub fn opt_total(&self, perm: crate::permanent::PermanentHandle) -> u32 {
        self.opt_counts(perm).0
    }

    /// Number of OPT effects whose activation count this turn has reached
    /// their `max_per_turn` cap.
    pub fn opt_used(&self, perm: crate::permanent::PermanentHandle) -> u32 {
        self.opt_counts(perm).1
    }

    /// Per-source OPT availability fraction in `[0.0, 1.0]`. `0.0` when the
    /// source has no OPT effects (matches Python's `source_opt_state`).
    pub fn source_opt_state(
        &self,
        perm: crate::permanent::PermanentHandle,
        source_index: usize,
    ) -> f32 {
        let Some(permanent) = self
            .players
            .get(perm.player as usize)
            .and_then(|p| p.battle_area.get(perm.index as usize))
        else {
            return 0.0;
        };
        let stack_size = permanent.card_sources.len();
        let Some(source) = permanent.card_sources.get(source_index) else {
            return 0.0;
        };
        let is_under = source_index + 1 < stack_size;
        let card_id = source.card_id(&self.card_data).to_string();
        let Some(impl_) = self.effect_registry.get(&card_id) else {
            return 0.0;
        };
        let effects = impl_.effects(source.handle());

        let mut total = 0u32;
        let mut available = 0u32;
        for (slot, effect) in effects.iter().enumerate() {
            if effect.max_per_turn == 0 {
                continue;
            }
            if is_under != effect.inherited {
                continue;
            }
            total += 1;
            let used = permanent.activation_count(source.handle(), slot as u8);
            if used < effect.max_per_turn {
                available += 1;
            }
        }

        if total == 0 {
            0.0
        } else {
            available as f32 / total as f32
        }
    }

    /// Shared implementation: `(total_opt_effects, used_opt_effects)` across
    /// every source in the permanent's stack with the inherited/top filter.
    fn opt_counts(&self, perm: crate::permanent::PermanentHandle) -> (u32, u32) {
        let Some(permanent) = self
            .players
            .get(perm.player as usize)
            .and_then(|p| p.battle_area.get(perm.index as usize))
        else {
            return (0, 0);
        };
        let stack_size = permanent.card_sources.len();

        let mut total = 0u32;
        let mut used = 0u32;
        for (source_index, source) in permanent.card_sources.iter().enumerate() {
            let is_under = source_index + 1 < stack_size;
            let card_id = source.card_id(&self.card_data).to_string();
            let Some(impl_) = self.effect_registry.get(&card_id) else {
                continue;
            };
            let effects = impl_.effects(source.handle());
            for (slot, effect) in effects.iter().enumerate() {
                if effect.max_per_turn == 0 {
                    continue;
                }
                if is_under != effect.inherited {
                    continue;
                }
                total += 1;
                let count = permanent.activation_count(source.handle(), slot as u8);
                if count >= effect.max_per_turn {
                    used += 1;
                }
            }
        }
        (total, used)
    }
}

#[cfg(test)]
mod reset_for_replay_tests {
    use super::*;
    use crate::card_data::CardData;
    use crate::card_source::CardSource;
    use crate::debug_runner::DebugRunner;
    use crate::enums::{CardColor, CardKind};

    fn card(id: &str) -> CardData {
        CardData {
            card_id: id.to_string(),
            card_name: id.to_string(),
            card_kind: CardKind::Digimon,
            level: Some(4),
            dp: Some(4000),
            play_cost: 4,
            colors: vec![CardColor::Red],
            traits: Vec::new(),
            evo_costs: Vec::new(),
            dna_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            security_text: String::new(),
            keywords: Vec::new(),
            effect_class_name: id.replace('-', "_"),
            index: 0,
            norm_id: 0.0,
            dual: None,
            ace_overflow: None,
            digixros_aliases: Vec::new(),
            also_treated_as: Vec::new(),
        }
    }

    /// Guard for `Game::reset_for_replay`: dirty a broad set of mutable
    /// fields, reset, and assert every one returns to its `Game::new`
    /// default while the immutable shared state (`card_data`,
    /// `token_registry`) is preserved. If a new mutable field is added to
    /// `Game` and not reset in `reset_for_replay`, extend this test — it is
    /// the lockstep guard called out in the method's doc comment.
    #[test]
    fn reset_for_replay_restores_defaults() {
        let mut r = DebugRunner::builder()
            .add_card(card("A"))
            .add_card(card("B"))
            .start();
        let g = &mut r.game;

        // Immutable shared state we expect preserved across reset.
        let card_data_len = g.card_data.len();
        assert!(card_data_len > 0, "card_data should be populated");
        let token_count = g.token_registry.iter().count();
        assert!(token_count > 0, "token registry should be populated");

        // Dirty a representative spread of mutable / accumulator / transient
        // fields (the plain-typed ones most likely to be forgotten).
        g.turn_count = 7;
        g.n_digivolutions = [3, 5];
        g.n_dna_digivolutions = [1, 2];
        g.n_digivolve_driven_attacks = [4, 4];
        g.digimon_attacks_this_turn = [2, 1];
        g.memory = 6;
        g.game_over = true;
        g.winner = Some(1);
        g.event_seq = 99;
        g.effect_chain_depth = 5;
        g.replacement_depth = 3;
        g.next_granted_effect_id = 42;
        g.pending_player_digivolve_reduction = 9;
        g.in_counter_window = true;
        g.in_replacement_commit = true;
        g.dsl_clause_aborted = true;
        g.draining_deferred = 2;
        g.until_condition_dirty = true;
        g.until_condition_last_cycle_evaluations = 11;
        g.until_condition_total_evaluations = 22;
        g.until_condition_reevaluation_cycles = 33;
        g.next_card_index = 123;
        g.mulligan_used = vec![true, true];
        g.revealed_cards.push(CardSource::new(0, 0, 0));

        g.reset_for_replay();

        // Immutable shared state preserved (not rebuilt / cleared).
        assert_eq!(g.card_data.len(), card_data_len, "card_data preserved");
        assert_eq!(
            g.token_registry.iter().count(),
            token_count,
            "token registry preserved"
        );

        // Mutable state back to Game::new defaults.
        assert_eq!(g.turn_count, 0);
        assert_eq!(g.n_digivolutions, [0, 0]);
        assert_eq!(g.n_dna_digivolutions, [0, 0]);
        assert_eq!(g.n_digivolve_driven_attacks, [0, 0]);
        assert_eq!(g.digimon_attacks_this_turn, [0, 0]);
        assert_eq!(g.memory, 0);
        assert!(!g.game_over);
        assert!(g.winner.is_none());
        assert!(g.terminal_outcome_reason.is_none());
        assert_eq!(g.event_seq, 0);
        assert!(g.events.is_empty());
        assert_eq!(g.effect_chain_depth, 0);
        assert_eq!(g.replacement_depth, 0);
        assert_eq!(g.next_granted_effect_id, 0);
        assert_eq!(g.pending_player_digivolve_reduction, 0);
        assert!(!g.in_counter_window);
        assert!(!g.in_replacement_commit);
        assert!(!g.dsl_clause_aborted);
        assert_eq!(g.draining_deferred, 0);
        assert!(!g.until_condition_dirty);
        assert_eq!(g.until_condition_last_cycle_evaluations, 0);
        assert_eq!(g.until_condition_total_evaluations, 0);
        assert_eq!(g.until_condition_reevaluation_cycles, 0);
        assert_eq!(g.next_card_index, 0);
        assert!(g.revealed_cards.is_empty());
        assert!(g.pending_selection.is_none());
        assert!(g.reveal_source.is_none());
        assert!(g.opaque_data_index_map.is_none());
        assert_eq!(g.current_phase, GamePhase::Mulligan);
        assert!(g.mulligan_used.iter().all(|&u| !u));
        assert!(
            g.players
                .iter()
                .all(|p| p.hand.is_empty() && p.battle_area.is_empty() && p.deck.is_empty()),
            "players reset to fresh (empty zones) — relay re-lays them"
        );
    }
}

#[cfg(test)]
mod current_attacker_tests {
    use crate::card_data::CardData;
    use crate::debug_runner::DebugRunner;
    use crate::enums::{CardColor, CardKind};

    fn card(id: &str) -> CardData {
        CardData {
            card_id: id.to_string(),
            card_name: id.to_string(),
            card_kind: CardKind::Digimon,
            level: Some(4),
            dp: Some(4000),
            play_cost: 4,
            colors: vec![CardColor::Red],
            traits: Vec::new(),
            evo_costs: Vec::new(),
            dna_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            security_text: String::new(),
            keywords: Vec::new(),
            effect_class_name: id.replace('-', "_"),
            index: 0,
            norm_id: 0.0,
            dual: None,
            ace_overflow: None,
            digixros_aliases: Vec::new(),
            also_treated_as: Vec::new(),
        }
    }

    #[test]
    fn current_attacker_is_none_outside_combat() {
        let r = DebugRunner::builder().add_card(card("A")).start();
        assert!(r.game.current_attacker().is_none());
    }

    #[test]
    fn progress_excludes_only_when_attacking_and_opponent_sourced() {
        use crate::enums::{Expiry, Keyword};
        let mut r = DebugRunner::builder()
            .add_card(CardData {
                keywords: vec![Keyword::Progress],
                ..card("PROG")
            })
            .add_card(card("OPP"))
            .start();
        let progress = r.place_on_field(0, "PROG", None);
        let _opp_perm = r.place_on_field(1, "OPP", None);

        // Case 1: not attacking → never excluded.
        assert!(
            !r.game.progress_excludes(progress, Some(1)),
            "not-attacking carrier: no exclusion"
        );

        // Case 2: attacking, but effect is own-sourced → no exclusion.
        //
        // Simulate an in-flight attack by inserting a PendingAttack.
        use crate::selection::{AttackTarget, PendingAttack};
        r.game.pending_attack = Some(PendingAttack {
            attacker: progress,
            original_target: AttackTarget::Player(1),
            effective_target: AttackTarget::Player(1),
            is_blocked: false,
            blocker: None,
            is_vortex: false,
            is_overclock: false,
            declaration_committed: true,
            cancelled: false,
            battle_occurred: false,
            return_phase: crate::enums::GamePhase::Main,
            state: crate::selection::AttackState::Declared,
            counter_depth: 0,
        });
        assert!(
            !r.game.progress_excludes(progress, Some(0)),
            "own-sourced effect on own Progress: no exclusion"
        );
        assert!(
            !r.game.progress_excludes(progress, None),
            "no source player: no exclusion"
        );

        // Case 3: attacking + opponent-sourced → excluded.
        assert!(
            r.game.progress_excludes(progress, Some(1)),
            "opponent-sourced effect on attacking Progress carrier: excluded"
        );

        // Clean up the fake attack state to avoid leaking into later tests.
        r.game.pending_attack = None;

        // Case 4: Progress granted via modifier also triggers.
        let plain = r.place_on_field(0, "OPP", None);
        assert!(!r.game.progress_excludes(plain, Some(1)));
        r.game
            .modifiers
            .grant_keyword(plain, Keyword::Progress, Expiry::EndOfTurn, 0);
        r.game.pending_attack = Some(PendingAttack {
            attacker: plain,
            original_target: AttackTarget::Player(1),
            effective_target: AttackTarget::Player(1),
            is_blocked: false,
            blocker: None,
            is_vortex: false,
            is_overclock: false,
            declaration_committed: true,
            cancelled: false,
            battle_occurred: false,
            return_phase: crate::enums::GamePhase::Main,
            state: crate::selection::AttackState::Declared,
            counter_depth: 0,
        });
        assert!(
            r.game.progress_excludes(plain, Some(1)),
            "modifier-granted Progress should gate the same"
        );
    }

    #[test]
    fn opponent_sourced_mutation_only_when_effect_source_differs() {
        let mut r = DebugRunner::builder()
            .add_card(card("A"))
            .add_card(card("B"))
            .start();
        let a = r.place_on_field(0, "A", None);
        let _b = r.place_on_field(1, "B", None);

        // No effect resolving → false.
        assert!(!r.game.opponent_sourced_mutation(a));

        // Own effect resolving → false.
        r.game.set_effect_source_player_for_test(Some(0));
        assert!(!r.game.opponent_sourced_mutation(a));

        // Opponent effect resolving → true.
        r.game.set_effect_source_player_for_test(Some(1));
        assert!(r.game.opponent_sourced_mutation(a));

        r.game.set_effect_source_player_for_test(None);
    }
}

#[cfg(test)]
mod resolve_token_as_battle_area_top_tests {
    //! Tests for the strict provenance resolver introduced by change
    //! `fix-played-binding-uses-provenance`.

    use crate::card_data::CardData;
    use crate::card_source::CardSource;
    use crate::debug_runner::DebugRunner;
    use crate::enums::{CardColor, CardKind};
    use crate::trigger_context::ProvenanceToken;

    fn lv3_card(id: &str) -> CardData {
        CardData {
            card_id: id.to_string(),
            card_name: id.to_string(),
            card_kind: CardKind::Digimon,
            level: Some(3),
            dp: Some(2000),
            play_cost: 3,
            colors: vec![CardColor::Blue],
            traits: Vec::new(),
            evo_costs: Vec::new(),
            dna_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            security_text: String::new(),
            keywords: Vec::new(),
            effect_class_name: id.replace('-', "_"),
            index: 0,
            norm_id: 0.0,
            dual: None,
            ace_overflow: None,
            digixros_aliases: Vec::new(),
            also_treated_as: Vec::new(),
        }
    }

    fn lv4_card(id: &str) -> CardData {
        let mut c = lv3_card(id);
        c.level = Some(4);
        c.dp = Some(4000);
        c.play_cost = 5;
        c
    }

    #[test]
    fn case_a_played_card_is_battle_area_top_resolves_to_handle() {
        let mut r = DebugRunner::builder().add_card(lv3_card("VEEMON")).start();
        let veemon = r.place_on_field(0, "VEEMON", None);
        let card_index = r.game.players[0].battle_area[veemon.index as usize]
            .top_card()
            .card_index;
        let token = ProvenanceToken(card_index as u64);
        assert_eq!(
            r.game.resolve_token_as_battle_area_top(token),
            Some(veemon),
            "played card is the battle-area top — resolver yields its handle"
        );
    }

    #[test]
    fn case_b_played_card_buried_under_new_top_fizzles() {
        let mut r = DebugRunner::builder()
            .add_card(lv3_card("VEEMON"))
            .add_card(lv4_card("EXVEEMON"))
            .start();
        let veemon = r.place_on_field(0, "VEEMON", None);
        let veemon_card_index = r.game.players[0].battle_area[veemon.index as usize]
            .top_card()
            .card_index;

        // Push ExVeemon as the new top card directly. The Veemon CardSource
        // becomes a digivolution card under ExVeemon.
        let exveemon_data_index = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "EXVEEMON")
            .expect("EXVEEMON registered");
        let next_idx = r.game.next_card_index();
        let exveemon_source = CardSource::new(exveemon_data_index, 0, next_idx);
        r.game.players[0].battle_area[veemon.index as usize]
            .card_sources
            .push(exveemon_source);

        let token = ProvenanceToken(veemon_card_index as u64);
        assert_eq!(
            r.game.resolve_token_as_battle_area_top(token),
            None,
            "played Veemon is now a digivolution card under ExVeemon — strict resolver fizzles"
        );
    }

    #[test]
    fn case_c_played_card_in_trash_fizzles() {
        let mut r = DebugRunner::builder().add_card(lv3_card("VEEMON")).start();
        let veemon_data_index = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "VEEMON")
            .expect("VEEMON registered");
        let next_idx = r.game.next_card_index();
        let veemon_source = CardSource::new(veemon_data_index, 0, next_idx);
        let card_index = veemon_source.card_index;
        r.game.players[0].trash.push(veemon_source);

        let token = ProvenanceToken(card_index as u64);
        assert_eq!(
            r.game.resolve_token_as_battle_area_top(token),
            None,
            "played card is in trash — strict resolver fizzles"
        );
    }

    #[test]
    fn case_d_token_does_not_resolve_anywhere_yields_none() {
        let r = DebugRunner::builder().add_card(lv3_card("VEEMON")).start();
        assert_eq!(
            r.game.resolve_token_as_battle_area_top(ProvenanceToken(99_999)),
            None
        );
        assert_eq!(
            r.game.resolve_token_as_battle_area_top(ProvenanceToken(12345)),
            None,
            "unknown card index yields None"
        );
    }
}
