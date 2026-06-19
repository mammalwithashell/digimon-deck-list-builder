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

/// Fire-site continuation for a DigiLink Shape-B Digimon-link whose
/// `WhenWouldLink` replacement parked an interactive selection. Carries the
/// linking standing Digimon, the chosen host, the link cost, and the linking
/// card's handle (the `WhenWouldLink` replacement subject) so the resume can
/// re-validate the source before committing the absorb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingDigimonLink {
    pub(crate) source: PermanentHandle,
    pub(crate) host: PermanentHandle,
    pub(crate) cost: u16,
    pub(crate) card: crate::card_source::CardHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingWouldDigivolveResume {
    pub(crate) player: PlayerId,
    pub(crate) permanent: PermanentHandle,
    pub(crate) card: crate::card_source::CardHandle,
    pub(crate) effective_cost: u16,
    /// Whether the player chose an App-Fusion route for this digivolution. The
    /// commit path consumes the host's linked cards only when this is set.
    /// Threaded from the chosen route so the commit honors the player's pick
    /// (`pending_digivolve_route_choice`) rather than re-deriving the min route.
    pub(crate) app_fusion: bool,
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
    /// fired. Carries the host and the just-linked card so `OnLink` fires with
    /// the `Linked` trigger source identifying exactly the card that attached.
    #[doc(hidden)]
    pub(crate) pending_option_placed_link_resume:
        Option<(PermanentHandle, crate::card_source::CardHandle)>,
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
    /// Fire-site continuation for a DigiLink Shape-B Digimon-link whose
    /// `WhenWouldLink` replacement parked an interactive selection.
    #[doc(hidden)]
    pub(crate) pending_digimon_link: Option<PendingDigimonLink>,
    /// The host a card is about to link ONTO during the active `WhenWouldLink`
    /// replacement window. Set by `begin_digimon_link` right before
    /// `try_replace`, cleared when the link resolves/aborts. Read by a
    /// host-side reducer effect's `condition` (via `EffectContext::
    /// pending_link_host`) so it can verify "...link to THIS Digimon" before
    /// offering its optional cost reduction (Gap 5 — BT25-004 / BT25-045).
    #[doc(hidden)]
    pub(crate) pending_link_host: Option<PermanentHandle>,
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

    /// Reduction (in memory) granted by an already-resolved FIELD-HOSTED
    /// INTERACTIVE digivolve cost reducer (its `pay_cost` installs a selection
    /// — e.g. ST23-03 Cougarmon's "by trashing the bottom face-down card under
    /// a Tamer, reduce the cost by 2"). Set by the interactive reducer prompt's
    /// parked-selection success continuation
    /// (`try_prompt_interactive_digivolve_cost_reducer`), read by the digivolve
    /// cost calculation on re-entry and cleared once consumed. `0` means "no
    /// interactive field reduction". Distinct from
    /// `pending_player_digivolve_reduction` (the player-scoped BT3-103 store) so
    /// the two can both contribute to a single digivolution without colliding.
    /// `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
    pub(crate) pending_interactive_digivolve_reduction: i32,

    /// Reduction (in memory) granted by an already-resolved FIELD-HOSTED
    /// INTERACTIVE Option-use cost reducer (its `pay_cost` installs a selection
    /// — e.g. BT25-049 Armalizamon's "by trashing the bottom face-down card
    /// under a Tamer, reduce the cost by 3"). Set by the interactive reducer
    /// prompt's parked-selection success continuation
    /// (`try_prompt_interactive_option_use_cost_reducer`), read by
    /// `play_option_core` on re-entry and cleared once consumed. `0` means "no
    /// interactive Option-use reduction".
    /// `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
    pub(crate) pending_interactive_option_use_reduction: i32,

    /// True once `play_option_core` has installed the interactive Option-use
    /// cost-reducer prompt for the in-flight Option play, so the re-entry (after
    /// the accept/decline gate OR the parked Tamer-pick resolves) does NOT
    /// re-offer the same reducer. A decline-surviving re-entry signal — unlike a
    /// `pending_interactive_option_use_reduction == 0` check, a 0-credit DECLINE
    /// (or a 0-amount paid reducer) still clears the prompt, preventing the
    /// accept/decline gate from re-prompting forever. Mirrors the digivolve
    /// path's `player_reducer_resolved` flag. Reset once consumed.
    /// `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
    pub(crate) interactive_option_use_reducer_prompted: bool,
    /// The digivolution route (cost + app-fusion-ness) the player CHOSE when a
    /// hand card offered more than one distinct-cost route over the target base
    /// (rule 17 — no auto-selection of the cheapest). Set by the cost-choice
    /// `EffectChoice` callback, which re-enters `digivolve_from_hand_inner`;
    /// peeked there to pin the printed cost; consumed (`take`) when the
    /// `PendingWouldDigivolveResume` is built. Cleared at the public
    /// `digivolve_from_hand` entry so each fresh user attempt starts clean (the
    /// cost-choice callback bypasses that entry, preserving the pick across the
    /// reducer/replacement re-entries). `None` means "no pending choice".
    pub(crate) pending_digivolve_route_choice: Option<crate::dna_digivolve::DigivolveRouteMatch>,

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

    /// Set to `true` while an effect-driven Option USE is resolving from a
    /// triggered/effect body (the unified `play_or_use_from_hand_with_cost`
    /// helper). `play_option_core`'s Main-phase gate is LIFTED while this is
    /// set, because the effect text ("you may play or use 1 … card") grants the
    /// use regardless of phase — e.g. BT25-041 fires from `[When Attacking]`,
    /// which is not the Main phase, yet must still be able to use an Option.
    /// Counter-window Option plays use the separate `in_counter_window` bypass;
    /// this flag is the effect-body analogue. Set/cleared around the
    /// `play_option_core` call inside `EffectContext::use_option_from_hand_with_cost`.
    /// `G-PLAY-OR-USE-FROM-HAND`.
    #[doc(hidden)]
    pub(crate) effect_driven_option_use: bool,

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

    /// Short-lived channel carrying the FRESHEST bindings of the
    /// just-resolved DSL selection chain. Set by
    /// `run_tail_preserving_trigger_context` when an inner tail completes;
    /// cleared-then-consumed by `wrap_pending_selection_with_tail`'s composed
    /// callbacks so a wrapped outer tail sees the picks a nested selection
    /// made after the wrap-time snapshot (a sibling `binding_exists` /
    /// `binding_absent` would otherwise read a stale absence —
    /// G-OPT-REFUND-ON-DECLINE). Never read across resolutions: the wrapper
    /// clears it before the original callback and takes it immediately after.
    #[doc(hidden)]
    pub dsl_resolved_tail_bindings: Option<crate::dsl_cards::bindings::Bindings>,

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
    /// G-PLAY-ENTERS-SUSPENDED (Q28 / EX5-060): the next play commit enters
    /// the battle area suspended. Set by the `play_from_trash_free`
    /// `suspended: true` DSL arm; consumed (and cleared) at the
    /// entry-commit site.
    pub(crate) play_enters_suspended: bool,
    /// Identity of the effect requesting `suppress_on_play` for the next
    /// play — `(controller, source kind)`. The suppression is an EFFECT on
    /// the played Digimon, so `fire_play_event_triggers` consults
    /// `permanent_is_unaffected_by_effect` against this identity before
    /// honoring the skip (judge-quiz Q28: Gankoomon X's protection lets the
    /// played Ciel's [On Play] activate through Dragomon's rider).
    pub(crate) on_play_suppressor: Option<(PlayerId, crate::enums::EffectSourceKind)>,

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

    /// Card-id → `card_data` index, materialized once at construction. O(1)
    /// replacement for the per-step `card_data.iter().find(...)` linear scan.
    pub(crate) card_id_index: std::collections::HashMap<String, usize>,
}

// Tier-1 impl Game mechanic clusters (see docs/RUST_ENGINE_API.md §3).
mod handles;
mod lifecycle;
mod memory;
mod opt;
mod queries;
mod setup;
mod snapshot;
mod staging;
mod suspend;
mod triggers;
mod until_condition;

impl Game {
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

    // ─── Mulligan ────────────────────────────────────────────────────

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
        let emitted_card_name = top_card.card_name(&self.card_data).to_string();
        let cost_printed = self.card_data[top_card.data_index].play_cost as i16;
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Play {
            seq,
            player: player_id,
            card_id: emitted_card_id,
            card_name: emitted_card_name,
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
            let emitted_card_name = card_source.card_name(&self.card_data).to_string();
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
                card_name: emitted_card_name,
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

    // ─── Event accumulator ─────────────────────────────────────────

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
        let card_name = card.card_name(&self.card_data).to_string();
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Trash {
            seq,
            player,
            card_id,
            card_name,
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
                let card_name = card.card_name(&self.card_data).to_string();
                let seq = self.next_event_seq();
                self.events.push(crate::events::GameEvent::Trash {
                    seq,
                    player,
                    card_id,
                    card_name,
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
            let card_name = card.card_name(&self.card_data).to_string();
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::Trash {
                seq,
                player,
                card_id,
                card_name,
            });
            self.player_mut(player).trash.push(card);
        }
        for card in perm.linked_cards {
            let card_id = card.card_id(&self.card_data).to_string();
            let card_name = card.card_name(&self.card_data).to_string();
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::Trash {
                seq,
                player,
                card_id,
                card_name,
            });
            self.player_mut(player).trash.push(card);
        }
    }

    // ─── Memory management ─────────────────────────────────────────

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
            source_card_id: None,
            source_card_name: None,
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
        let top_card_name = new_top.card_name(&self.card_data).to_string();

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
            card_name: top_card_name,
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
        let top_card_name = result_source.card_name(&self.card_data).to_string();

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
            card_name: top_card_name,
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
        effect_immunity: Option<crate::modifiers::EffectImmunityFilter>,
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
                effect_immunity,
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

    /// Returns the `PermanentHandle` of the currently-attacking permanent,
    /// or `None` when no attack is in flight. Reads `pending_attack.attacker`
    /// — the same source the mask and combat-resolution code use.
    ///
    /// Used by `progress_excludes` to gate opponent-effect mutations on
    /// the Progress carrier specifically while it is the attacker.
    pub fn current_attacker(&self) -> Option<PermanentHandle> {
        self.pending_attack.as_ref().map(|p| p.attacker)
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

        // DigiLink Shape-B (G-LINK-INHERITED-ESS): a link card's `.linked()`
        // dynamic DP / Security-Attack formula Link-ESS contributes to its host.
        // The loop above scans `card_sources` / breeding only; fold in every
        // host's `linked_cards` with the host as `source_permanent`.
        let mut linked_sources: Vec<(
            String,
            crate::card_source::CardHandle,
            PermanentHandle,
            PlayerId,
        )> = Vec::new();
        for (pid, player) in self.players.iter().enumerate() {
            let player_id = pid as PlayerId;
            for (index, perm) in player.battle_area.iter().enumerate() {
                let host = PermanentHandle {
                    player: player_id,
                    index: index as u8,
                };
                for linked in &perm.linked_cards {
                    linked_sources.push((
                        linked.card_id(&self.card_data).to_string(),
                        linked.handle(),
                        host,
                        player_id,
                    ));
                }
            }
        }
        for (card_id, source_card, host, controller) in linked_sources {
            let Some(effects) = self.effects_for_card(&card_id, source_card) else {
                continue;
            };
            for effect in effects {
                if !effect.declarative || !effect.linked {
                    continue;
                }
                let Some(formula_fn) = (if security_attack {
                    effect.security_attack_fn.as_ref()
                } else {
                    effect.dp_modifier_fn.as_ref()
                }) else {
                    continue;
                };
                let ctx = EffectReadContext::new(self, source_card, Some(host), controller);
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
        // One O(1) lookup, reused for both keyword reads (was two O(~4000)
        // linear scans for the same card — the per-step hot path: ~793
        // effects_for_card calls/step, 94% of step time).
        let cd_opt = self.card_data_by_id(card_id);
        let native_keywords = cd_opt.map(|cd| cd.keywords.clone()).unwrap_or_default();
        let mut auto_effects: Vec<crate::effect::Effect> = cd_opt
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

    // ─── Tensor support: per-source DP + OPT helpers (§3.1 / §3.2) ───

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
