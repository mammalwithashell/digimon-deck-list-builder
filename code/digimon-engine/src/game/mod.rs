mod native_play_observation;
pub use native_play_observation::NativePlayEntry;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::card_data::CardData;
use crate::card_source::{CardHandle, CardSource};
use crate::cards::{build_registry, CardEffectRegistry};
use crate::dsl_cards::formula_registry::FormulaExtensionRegistry;
use crate::enums::{CardKind, EffectSourceKind, Expiry, GamePhase, ModifierType, PlayerId};
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
    /// §18-3-2 (Ver. 4.3): "If an infinite loop occurs and neither player has
    /// the ability to stop it, that game ends in a draw." Emitted with no
    /// winner, after a `GameEvent::InfiniteLoop`.
    InfiniteLoopDraw,
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
            Self::InfiniteLoopDraw => "infinite_loop_draw",
        }
    }

    pub fn result(self) -> &'static str {
        match self {
            Self::Crash | Self::UnknownDraw | Self::InfiniteLoopDraw => "draw",
            Self::SecurityAttack | Self::DeckOut | Self::EngineDeclared | Self::UnknownWin => "win",
            Self::StepLimit => "win",
            Self::Concede => "win",
        }
    }
}

pub(crate) fn face_keywords(card_data: &CardData) -> Vec<crate::enums::Keyword> {
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

/// Keywords printed in a link card's link effect: a Digimon gains them while the
/// card is one of its link cards (general_rule.pdf 4-3-5), and only then.
fn link_keywords(card_data: &CardData) -> Vec<crate::enums::Keyword> {
    if card_data.link_text.is_empty() {
        return Vec::new();
    }
    crate::card_data::parse_printed_keywords("", &card_data.link_text, "")
}

/// One stashed member of a parked multi-permanent deletion batch
/// (G-ENGINE-BATCH-DELETION-PARK-DROPS-REST).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingDeletionBatchRest {
    pub(crate) receipt: Option<u64>,
    pub(crate) parked_member: Option<(PlayerId, crate::card_source::CardHandle)>,
    pub(crate) player: PlayerId,
    pub(crate) card: crate::card_source::CardHandle,
    pub(crate) cause: crate::replacement::ReplacementCause,
    /// The parked optional replacement was also active for this member.
    pub(crate) covered_by_parked: bool,
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
    /// Cleanup of a native Delay body captured in the simultaneous start pool.
    /// Its due turn has already advanced; no phase scan resumes after cleanup.
    CapturedStartTurnDelay {
        carrier_identity: u64,
    },
    /// Standard `<Delay>` activated by a player `[Main]`-phase action
    /// (PUPPETS-G009). The Option's `DelayEffect` body installed a pending
    /// selection; once it resolves, the Option is trashed as the activation
    /// cost. No turn-keyed scan resumes — this kind only carries the deferred
    /// trash of the activated Option.
    MainPhaseActivation,
    /// The turn-start unsuspend-phase `OnUnsuspend` batch
    /// (G-ENGINE-PHASE-UNSUSPEND-NO-ONUNSUSPEND) parked on an observer's
    /// selection. No Delay scan resumes — once the batch's queue settles the
    /// turn machine continues into the draw phase
    /// (`continue_begin_turn_after_unsuspend_phase`).
    UnsuspendPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedOptionLifecycleResume {
    pub(crate) turn: u16,
    pub(crate) kind: DelayedOptionLifecycleResumeKind,
    pub(crate) pending_delete_key: Option<(PlayerId, u16)>,
    pub(crate) skip_key: Option<(PlayerId, u16)>,
}

/// Where the turn machine resumes after a decision in a turn-boundary
/// simultaneous pool (§18-1-2: pending processing + triggered effects)
/// parked it. The boundary does not pass until that pool has resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TurnBoundaryResume {
    /// Whatever `rotate_turn_player`'s expiries queued (the `[End of
    /// Opponent's Turn]` effects resolve earlier, in the end-of-turn pool):
    /// finish the rotation (memory floor, turn advance, seesaw flip, `begin_turn`).
    FinishTurnRotation { ending_player: PlayerId },
    /// The `UntilNextUnsuspend` pool at the start of a turn: continue into
    /// the per-turn reset and the unsuspend phase.
    BeginTurnAfterUntilNextUnsuspend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EndTurnResume {
    pub(crate) ending_player: PlayerId,
    pub(crate) memory_before_end_effects: i16,
}

/// An `<Overclock>` attack whose cost deletion parked a decision (an
/// optional replacement such as <Armor Purge>, or a deletion trigger), queued
/// in [`Game::parked_overclock_attacks`]. It is declared once the decision has
/// resolved and no attack is in progress, and only if the cost's deletion
/// happened: `cost_receipt` (a `deletion_receipts` id) must hold the cost
/// Digimon's stack `cost_identity` (§16-34-3, §15-7-1, §15-8-5-2). The
/// attacker is re-found by its top card (`source_card`), never by the old
/// slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParkedOverclockAttack {
    pub(crate) player: PlayerId,
    /// The `<Overclock>` carrier's top card.
    pub(crate) source_card: CardHandle,
    /// The player the attack is declared on.
    pub(crate) opponent: PlayerId,
    /// `capture_digivolve_host_identity` of the cost Digimon.
    pub(crate) cost_identity: u64,
    pub(crate) cost_receipt: u64,
}

impl ParkedOverclockAttack {
    /// The tuple the former single slot (`pending_overclock_attack:
    /// Option<(..)>`) hashed; the verification digest keeps that shape.
    pub(crate) fn digest_tuple(&self) -> (PlayerId, CardHandle, PlayerId, u64, u64) {
        (
            self.player,
            self.source_card,
            self.opponent,
            self.cost_identity,
            self.cost_receipt,
        )
    }
}

/// An end-of-turn attack keyword's processing (`<Engage>`, `<Execute>`,
/// `<Vortex>`, `<Overclock>`) whose trigger activated in the `[End of Your
/// Turn]` pool while another attack was already declared and not yet ended
/// (or an `<Overclock>` attack was queued, [`ParkedOverclockAttack`]).
/// Only one attack is in progress at a time (§11-1-4: the declared attack's
/// timings follow once the pool has resolved), so its offer waits until that
/// attack has ended (`Game::resume_pending_end_turn`). The carrier is kept by
/// stack identity, so a slot shift in between cannot misdirect it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeferredEndOfTurnAttackOffer {
    pub(crate) player: PlayerId,
    /// `Permanent::battle_identity` of the carrier.
    pub(crate) carrier_identity: u64,
    pub(crate) keyword: crate::enums::Keyword,
    /// The card whose keyword triggered (printed, inherited or granted).
    pub(crate) source_card: CardHandle,
    pub(crate) source_kind: crate::enums::EffectSourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingWouldPlayResume {
    pub(crate) player: PlayerId,
    pub(crate) card: crate::card_source::CardHandle,
    pub(crate) effective_cost: u16,
    pub(crate) origin: PendingWouldPlayOrigin,
    pub(crate) effect_initiated: bool,
    /// Controller of the effect that declared this play, retained across replacement parking.
    pub(crate) play_controller: PlayerId,
    /// PUPPETS-G030 — when `true`, the just-played permanent's own `[On Play]`
    /// effects are NOT enqueued for this play event. Used by BT5-106's
    /// [Security] clause ("Any [On Play] effects on Digimon played with this
    /// effect don't activate."). Scoped strictly to the played permanent and
    /// this single play event: other permanents' On Play, and every other
    /// timing (OnEnterFieldAnyone / OnAllyPlayed), are unaffected.
    pub(crate) suppress_on_play: bool,
}

/// A simultaneous source play whose individual preparation completed. The card
/// remains under its captured physical carrier until actual entry.
#[derive(Debug, Clone)]
pub(crate) struct PreparedSourcePlay {
    pub(crate) on_play_suppressor: Option<(PlayerId, crate::enums::EffectSourceKind)>,
    pub(crate) declaration: PendingWouldPlayResume,
    pub(crate) assembly: Option<(crate::card_source::CardHandle, Vec<crate::card_source::CardHandle>)>,
}

/// Open `OnDiscardHand` batch window (G-ENGINE-ON-DISCARD-HAND). Accumulates
/// the trashing player(s) whose hand lost cards to an effect within a single
/// effect body, plus the controller of the causing effect, so the trigger
/// fires ONCE per batch (DCGO `DiscardHands()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingHandDiscard {
    /// The controller of the effect that opened this window (the "cause").
    pub(crate) cause_controller: PlayerId,
    /// Distinct trashing players who lost ≥1 hand card during this window, in
    /// first-seen order. Almost always a single player, but an effect that
    /// trashes from both hands in one body fires one event per affected owner.
    pub(crate) trashed_players: Vec<PlayerId>,
    /// Every card this window moved hand→trash, with its owner, in trash
    /// order. Drives the self-scoped "When this card is trashed from the
    /// hand" trigger (BT26-069 Dobermon; DCGO `CanTriggerOnTrashSelfHand`):
    /// on flush, each still-in-trash card's `scope: trash` `on_discard_hand`
    /// clauses are enqueued. G-ENGINE-SELF-TRASHED-FROM-HAND.
    pub(crate) trashed_cards: Vec<(PlayerId, crate::card_source::CardHandle)>,
}

/// One host's share of an open `OnAddDigivolutionCards` batch window
/// (G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS). Accumulates every card an effect
/// placed into that host's digivolution cards within a single effect body so
/// the trigger fires ONCE per host per batch (rule 15-5-2; DCGO
/// `AddDigivolutionCardsBottom(list, cardEffect)` fires once per list —
/// `<Material Save N>` moves N cards in one call, and this engine's
/// per-card `place_card_under_permanent_bottom` loop must coalesce to match).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingAddedSources {
    /// The host's STABLE identity — its top card at note time. Re-resolved to
    /// a positional handle at flush time (battle-area indices shift when a
    /// sibling leaves play mid-body, e.g. the Tamer that `<Mind Link>` folds
    /// under a Digimon); a host that has left play by then fires nothing
    /// (DCGO `Permanent.TopCard != null` gate in `CanTriggerOnAddDigivolutionCard`).
    pub(crate) host_card: crate::card_source::CardHandle,
    /// The placing effect (controller + source card) — DCGO's non-null
    /// hashtable `CardEffect`. A window already open for the same host under
    /// a DIFFERENT controller is flushed first so two effects never merge.
    pub(crate) cause: crate::trigger_context::EffectAttribution,
    /// Added cards in placement order.
    pub(crate) cards: Vec<crate::card_source::CardHandle>,
}

/// One physical security play's completion, retained across replacement prompts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SecurityPlayReceipt {
    pub(crate) id: u64,
    pub(crate) player: u8,
    pub(crate) card: CardHandle,
    pub(crate) original_index: usize,
    pub(crate) committed: Option<(PermanentHandle, u64)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingWouldPlayOrigin {
    Hand,
    Trash {
        index: usize,
    },
    SecurityTop {
        was_face_up: bool,
    },
    SecurityRecorded { was_face_up: bool, index: usize, receipt: u64 },
    SecurityAt { was_face_up: bool, index: usize },
    Reveal {
        index: usize,
    },
    Source {
        permanent: PermanentHandle,
        host_identity: u64,
        source_index: usize,
        suspended: bool,
        receipt: Option<u64>,
    },
}

// Preserve archival origin Debug bytes; V4 hashes the typed extension separately.
impl std::fmt::Debug for PendingWouldPlayOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hand => f.write_str("Hand"),
            Self::Trash { index } => f.debug_struct("Trash").field("index", index).finish(),
            Self::SecurityTop { was_face_up } | Self::SecurityAt { was_face_up, .. }
            | Self::SecurityRecorded { was_face_up, .. } => f.debug_struct("SecurityTop").field("was_face_up", was_face_up).finish(),
            Self::Reveal { index } => f.debug_struct("Reveal").field("index", index).finish(),
            Self::Source { permanent, host_identity, source_index, suspended, receipt } => f.debug_struct("Source")
                .field("permanent", permanent).field("host_identity", host_identity).field("source_index", source_index)
                .field("suspended", suspended).field("receipt", receipt).finish(),
        }
    }
}

impl PendingWouldPlayOrigin {
    pub(crate) fn source_play_receipt(&self) -> Option<u64> {
        match self {
            Self::Source { receipt, .. } => *receipt,
            Self::SecurityRecorded { receipt, .. } => Some(*receipt),
            _ => None,
        }
    }
    pub(crate) fn enters_suspended(&self) -> bool {
        matches!(
            self,
            Self::Source {
                suspended: true,
                ..
            }
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PendingSecurityMove {
    pub(crate) kind: SecurityMoveKind,
    pub(crate) donor: (PlayerId, u64),
    pub(crate) card: crate::card_source::CardHandle,
    pub(crate) player: PlayerId,
    pub(crate) position: crate::enums::StackPosition,
    pub(crate) face_up: bool,
    pub(crate) observer: PlayerId,
    pub(crate) leave_complete: bool,
    /// Exact optional-processing payment operation; independent nested moves have none.
    pub(crate) result_receipt: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SecurityMoveKind {
    Permanent,
    TopCard,
    SourceCard,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingFieldTopSourceMove {
    pub(crate) result_receipt: Option<u64>,
    pub(crate) donor: (PlayerId, u64),
    pub(crate) target: (PlayerId, u64),
    pub(crate) target_breeding: bool,
    pub(crate) donor_card: crate::card_source::CardHandle,
    pub(crate) observer: PlayerId,
    pub(crate) face_down: bool,
    pub(crate) top: bool,
    pub(crate) effect_source_card: Option<crate::card_source::CardHandle>,
    pub(crate) source_kind: crate::enums::EffectSourceKind,
    pub(crate) cause: crate::replacement::ReplacementCause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingWouldLinkResume {
    pub(crate) host: PermanentHandle,
    pub(crate) card: crate::card_source::CardHandle,
}

/// Where a player-declared DigiLink Shape-B Digimon-link comes from — the two
/// origins the printed keyword names ("Plug this card from the hand or battle
/// area sideways into the specified Digimon"). Mirrors the `root` DCGO's
/// `ILinkCard.LinkCard` derives from the linking card's zone: `None` (a whole
/// standing permanent, absorbed via `IPlacePermanentToLinkCards`) or `Hand`
/// (a single card attached via `Permanent.AddLinkCard`). The trash /
/// under-stack / re-link roots stay effect-driven (`link_chosen_card_into_host`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DigimonLinkOrigin {
    /// An un-linked standing Digimon in the battle area — the whole permanent
    /// is absorbed (under-sources trashed, top card becomes the linked card).
    Standing(PermanentHandle),
    /// A Digimon card in `player`'s hand — it is lifted out of the hand and
    /// attached as-is; nothing is absorbed and no `[On Play]` fires.
    Hand(PlayerId),
}

/// Fire-site continuation for a DigiLink Shape-B Digimon-link whose
/// `WhenWouldLink` replacement parked an interactive selection. Carries the
/// linking card's origin, the chosen host, the link cost, and the linking
/// card's handle (the `WhenWouldLink` replacement subject) so the resume can
/// re-validate the source before committing the attach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingDigimonLink {
    pub(crate) origin: DigimonLinkOrigin,
    pub(crate) host: PermanentHandle,
    pub(crate) cost: u16,
    pub(crate) card: crate::card_source::CardHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingWouldDigivolveResume {
    /// Cost effects share the temporary identity, but only the owned would-
    /// digivolve window may commit this declaration after costs are complete.
    pub(crate) window_started: bool,
    pub(crate) host_identity: u64,
    /// Exact modifier entries owned by this window; never remove other effects.
    pub(crate) treated_identity_orders: Option<(u64, Option<u64>)>,
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
///   `rotate_turn_player`, `pass_turn`, `install_overclock_selection`,
///   `fire_end_of_your_turn`.
/// - `game_actions.rs` — player mutators: `play_from_hand`,
///   `digivolve_from_hand`, `move_from_breeding`, `activate_*_main`,
///   `initiate_dna_digivolve`.
/// Per-game memo of `effects_for_card` results, keyed by
/// `(data_index, card_handle, under_top)`. The registry effect list a card
/// derives is a pure function of those inputs (`CardEffect::effects(handle)`
/// reads no game state; `under_top` is the only state input and is in the key),
/// so entries are **never invalidated** for a game's lifetime. Stored behind a
/// `RefCell` so the lazy fill works on the `&self` `effects_for_card` path;
/// values are `Arc` so `Game` stays `Send` (`RustHeadlessGame` is a
/// non-`unsendable` `#[pyclass]`) and a cache hit is a refcount bump — no vec or
/// closure clone. See the `cache-effects-for-card` change.
///
/// `Clone` (make-engine-cloneable): the memo is pure derived data keyed by pure
/// inputs, valid for a cloned `Game` (same Arc-shared registry/card_data), so a
/// forked search node may inherit it. Cloning deep-copies the map structure;
/// the `Arc` values are refcount bumps. The clone's `RefCell` is independent.
#[derive(Default, Clone)]
pub struct EffectsCache(
    std::cell::RefCell<
        std::collections::HashMap<
            (usize, crate::card_source::CardHandle, bool),
            Option<std::sync::Arc<Vec<crate::effect::Effect>>>,
        >,
    >,
    /// Registry-only memo for [`Game::registry_effects`]: `CardEffect::effects`
    /// is a pure function of `(card, handle)`, so no invalidation is needed.
    std::cell::RefCell<
        std::collections::HashMap<
            (usize, crate::card_source::CardHandle),
            Option<std::sync::Arc<Vec<crate::effect::Effect>>>,
        >,
    >,
);

impl std::fmt::Debug for EffectsCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectsCache")
            .field("entries", &self.0.borrow().len())
            .finish()
    }
}

impl EffectsCache {
    fn get(
        &self,
        key: &(usize, crate::card_source::CardHandle, bool),
    ) -> Option<Option<std::sync::Arc<Vec<crate::effect::Effect>>>> {
        self.0.borrow().get(key).cloned()
    }

    fn insert(
        &self,
        key: (usize, crate::card_source::CardHandle, bool),
        value: Option<std::sync::Arc<Vec<crate::effect::Effect>>>,
    ) {
        self.0.borrow_mut().insert(key, value);
    }
}

#[derive(Debug, Clone)]
pub struct Game {
    pub(crate) belief_knowledge: Option<[crate::determinize::beliefs::KnowledgeLedger; 2]>,
    pub(crate) belief_knowledge_error: Option<String>,
    /// Exam-only recorder state; cloned but excluded from gameplay/digests.
    pub(crate) movement_observer:
        std::cell::RefCell<Option<crate::movement_observation::MovementObserver>>,
    pub rules: Rules,
    /// Per-game memo for `effects_for_card` (see `EffectsCache`). Not part of
    /// the observable game state — pure derived data, never snapshotted.
    pub(crate) effects_cache: EffectsCache,
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
    /// `CardStore` wraps an `Arc<Vec<CardData>>` shared across games (see
    /// `card_store`); `Deref`-transparent so reads are unchanged.
    pub card_data: crate::card_store::CardStore,
    /// Compiled DSL alternate digivolution paths keyed by result card id.
    /// Shared via `Arc` (built once with the card store).
    #[cfg(feature = "dsl-yaml-loader")]
    pub(crate) alt_path_registry:
        std::sync::Arc<HashMap<String, Vec<digimon_dsl::compiled::CompiledAltPath>>>,
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
    pub(crate) next_card_choice_id: u64,
    pub(crate) security_shuffle_generations: [u64; 2],
    /// Temporary public knowledge: owner, physical card, arrival, shuffle epoch.
    /// This never changes the card's rules-facing face-up state.
    pub(crate) public_security_reveals: Vec<(PlayerId, crate::card_source::CardHandle, u64, u64)>,
    pub(crate) resolved_card_choice: Option<crate::owned_card_choice::ResolvedCardChoice>,
    /// Coexistence-phase data continuation paired with `pending_selection`
    /// (make-engine-cloneable Phase 2). When `Some`, `resolve_generic_selection`
    /// runs the data-frame VM (`crate::resume::run_resume`) instead of the
    /// legacy `pending_selection.callback`. Set and cleared in lock-step with
    /// `pending_selection`; parallels `dsl_outer_tail` (continuation-as-data).
    /// Always `None` today — populated as cards are ported onto the VM.
    pub pending_selection_resume: Option<crate::resume::ResumeStack>,
    /// Resume-side continuation channel (make-engine-cloneable Phase 2,
    /// defunctionalized 2026-07-01). Continuation wrappers (play-cost /
    /// digivolve-reducer / option-reducer continuations, the DigiXros
    /// leave-window resume, the `<Partition>` second-source play) defer their
    /// post-selection continuation here as plain data
    /// ([`crate::resume::AfterSelectionHook`]); `resolve_generic_selection`
    /// drains it after each resume resolution. Being data, armed hooks
    /// survive `Game::clone` faithfully.
    pub after_selection_resume_hooks: crate::resume::ResumeContinuationHooks,
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
    pub(crate) digixros_leaving_limbo: Vec<(
        crate::enums::PlayerId,
        crate::permanent::PermanentHandle,
        crate::permanent::Permanent,
    )>,
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
    /// Resolutions in the current chain: incremented per drain step, reset to
    /// 0 when the queue empties, and restarted (to 1, keeping the "inside a
    /// chain" marker that `cancel_pending_attack_from_effect_checked` and
    /// `reevaluate_until_condition_modifiers_if_dirty` read) whenever a
    /// player decision resolves, so a loop its owner keeps choosing to repeat
    /// (§18-3-3-3) is never mistaken for an unbounded one. Past
    /// `effect_queue::MAX_CHAIN_DEPTH` (a far backstop behind the §18-3
    /// signature watch) the game ends in the §18-3-2 draw.
    pub(crate) effect_chain_depth: u16,
    /// §18-3 infinite-loop watch: how many times each loop signature
    /// (`Game::loop_signature`) has been seen since the last DECLINABLE
    /// decision resolved or the outermost drain settled with nothing parked.
    /// See `game::infinite_loop`. It survives forced decisions, so a clone
    /// taken at a forced prompt carries a non-empty watch; clones stay
    /// deterministic because it is a `BTreeMap` of fixed-key hashes over
    /// handles and values (no pointers, RNG or allocation order).
    pub(crate) loop_watch: std::collections::BTreeMap<u64, u8>,
    /// Steps since the loop watch last started over; signatures are taken
    /// only from `game::infinite_loop::LOOP_WATCH_START` steps on.
    pub(crate) loop_watch_steps: u16,
    /// Marks the pending prompt as an optional decision when it is not
    /// `is_optional` (DSL choose-one, `<Ascension>` Yes/No); consumed when the
    /// next decision resolves. See `game::infinite_loop::OptionalDecisionMark`.
    pub(crate) optional_decision_mark: Option<infinite_loop::OptionalDecisionMark>,
    /// Last §15-4-5 trigger-batch id handed out by the drainer (see
    /// `QueuedEffect::trigger_batch`). Monotonic within a game.
    pub(crate) trigger_batch_seq: u32,

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
    /// Private cloneable receipts, filled at commit before entry observers.
    pub(crate) source_play_receipts: Vec<(
        u64,
        crate::card_source::CardHandle,
        Option<(PermanentHandle, u64)>,
    )>,
    pub(crate) next_source_play_receipt: u64,
    pub(crate) security_play_receipts: Vec<SecurityPlayReceipt>,
    pub(crate) next_security_play_receipt: u64,
    /// Receipt-owned preparation slots; no ambient batch flag can capture nested plays.
    pub(crate) prepared_source_plays: Vec<(u64, Option<PreparedSourcePlay>)>,
    pub(crate) deletion_receipts: Vec<(u64, Vec<(PermanentHandle, Option<i32>, bool, (u16, u64))>)>,
    pub(crate) next_deletion_receipt: u64,
    pub(crate) return_receipts: Vec<(u64, Vec<crate::game_actions::ReturnedCard>)>,
    pub(crate) next_return_receipt: u64,
    pub(crate) field_source_receipts: Vec<(u64, Option<crate::card_source::CardHandle>)>,
    pub(crate) current_deletion_receipt: Option<u64>,
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
    pub(crate) pending_assembly_materials: Option<(
        crate::card_source::CardHandle,
        Vec<crate::card_source::CardHandle>,
    )>,
    /// Fire-site continuation for optional `WhenWouldLink` replacements whose
    /// subject is the pending Link Option card.
    #[doc(hidden)]
    pub(crate) pending_would_link_resume: Option<PendingWouldLinkResume>,
    pub(crate) pending_field_top_source_moves: Vec<PendingFieldTopSourceMove>,
    pub(crate) pending_security_moves: Vec<PendingSecurityMove>,
    /// Open "by placing X in security" condition receipts: (top-card handle,
    /// processed). Marked processed when the move of that card completes —
    /// into security, or under Digi-Egg / token rules (§15-1-9) — and taken
    /// by `SecurityPlacementCancelFinalize` (BT24-040).
    pub(crate) security_placement_receipts: Vec<(u16, bool)>,
    pub(crate) permanent_security_placement_receipts: Vec<(u64, bool)>,
    pub(crate) security_trash_receipts: Vec<(u16, bool)>,
    pub(crate) pending_security_trashes: Vec<crate::game_actions::security_trash::SecurityTrashMove>,
    /// The host chosen for a from-hand Plug-In Option's §6-5-1-4 link
    /// declaration, pinned between the host pick (§10-1-3-1) and the plug-in
    /// (§10-1-3-3) so `dispose_option`'s `Link` arm attaches to it instead of
    /// asking a second time. Set by
    /// `run_option_hand_link_host_selection_step`, consumed by
    /// `dispose_option`. `G-ENGINE-OPTION-HAND-LINK-COST-TIMING`.
    #[doc(hidden)]
    pub(crate) pending_option_link_host: Option<PermanentHandle>,
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
    pub(crate) effect_digivolve_scopes: Vec<crate::game_actions::digivolve::EffectDigivolveScope>,
    pub(crate) effect_dna_would_scopes: Vec<crate::game_actions::effect_dna::EffectDnaWouldScope>,

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

    /// Armed by `use_option_from` once an Option's use cost is paid; consumed
    /// by `fire_on_use_option_observers` AFTER the Option's own body has
    /// fully resolved. Lives on `Game` (not on `PendingOption`) because a body
    /// may CLAIM `pending_option` mid-resolution (`place_self_under_permanent`,
    /// `add_this_option_to_hand`, link plug-ins) — the board-wide "when a
    /// player uses an Option card" observers (BT3-096, BT25-091) must still
    /// fire once the claimed body finishes.
    /// Carries the using player and the used card so the observers'
    /// trigger context can expose them (`event_card` / `source_player`):
    /// "When YOU use [TS] trait Option cards" (BT25-091) needs both.
    /// G-ENGINE-ON-USE-OPTION-EVENT-CARD.
    pub(crate) on_use_option_armed: Option<(PlayerId, crate::card_source::CardHandle)>,

    /// Set by `play_option_core` just before the used Option's `[Main]`
    /// (`OptionMain`) body is enqueued; cleared by
    /// `complete_option_body_if_done` once the body has fully resolved and the
    /// Option has been disposed. While set, triggered effects the body caused
    /// (e.g. an opponent's `[On Deletion]`) are held back so the used Option is
    /// trashed FIRST, uninterrupted by rule checks or other effects: §9-1-5
    /// (Ver. 4.3) "A used Option card is immediately trashed as pending
    /// processing as soon as the timing when its 1st [Main] effect has been
    /// resolved ... Other effects and rule checks don't interrupt the period
    /// between [Main] effect activation and trashing."
    /// DCGO: `UseOptionClass.UseOption` calls `AddTrashCard` right after the
    /// `OptionSkill` process, before the stacked triggers resolve.
    /// G-ENGINE-OPTION-TRASH-AFTER-TRIGGERED-EFFECTS.
    pub(crate) option_body_pending: bool,
    /// §9-1-5: the deferred-drain depth the in-flight Option's `[Main]` body
    /// was started under (`play_option_core`), refreshed to the depth outside
    /// the resolving callback whenever one of the body's selections resolves.
    /// The body has finished -- and the Option is disposed -- as soon as the
    /// drain is back at that depth: `draining_deferred == outer`, not `== 0`,
    /// so an Option an EFFECT uses from inside its own selection callback
    /// (EX7-073 using P-180: the body runs with `draining_deferred > 0`) is
    /// trashed in the step that finishes its body too, before any rule check
    /// (F-ENGINE-EFFECT-USED-OPTION-MAIN-IS-ORDERABLE residual).
    pub(crate) option_body_outer_deferred: u32,
    /// §9-1-5 "as soon as the timing when its 1st [Main] effect has been
    /// resolved": set when the in-flight Option's first `OptionMain` entry has
    /// run; an Option with two `[Main]` effects (none in the current pool --
    /// the fixture `MAIN_TWO` pins the mechanism) is disposed after the
    /// first, and the second resolves as the pending effect of a card in the
    /// trash (how it activates has no rule text, §9-1-9-3 names only the 1st;
    /// owner ruling pending).
    pub(crate) option_first_main_resolved: bool,

    /// VARIABLE-amount interactive cost reduction driven by the printed play
    /// cost of a permanent deleted DURING an interactive cost's `pay_cost`
    /// (`G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST`). BT13-103 Akihiro
    /// Kurata: "by deleting 1 of your [Gizmon]-name Digimon, reduce the play
    /// cost by the play cost of the deleted Digimon" (DCGO
    /// `permanent.CostJustBeforeRemoveField`). The reduction amount is not known
    /// up-front (`inspect_cost_reduction_candidate` runs BEFORE the interactive
    /// select+delete), so the dedicated `delete_for_cost_reduction` pay_cost step
    /// snapshots the deleted permanent's printed cost here (pre-removal, rule 25)
    /// and the play-cost continuation adds it once the parked select resolves.
    /// `Some(0)` from a 0-cost deletion credits nothing (DCGO `if (reduceCost <=
    /// 0) yield break`). `None` means "no deferred variable reduction". Consumed
    /// (`take`) by `resume_play_cost_continuation_after_pending`.
    pub(crate) pending_cost_reduction_amount_override: Option<i32>,
    pub(crate) cost_payment_receipts: Vec<crate::game_actions::CostPaymentReceipt>,
    pub(crate) next_cost_payment_receipt: u64,
    pub(crate) active_cost_payment_receipt: Option<u64>,
    pub(crate) preferred_play_cost_effect: Option<crate::game_actions::CostReductionKey>,
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

    /// Spec §7.5 once-per-event guard. Records `(timing, stable subject)` pairs
    /// already offered for the current interrupting cause so a
    /// redirected route does not re-fire the same timing for the same subject
    /// (e.g. `WhenWouldLeaveBattleArea` super-timing double-fire when a
    /// `Redirected(Deck)` outcome on `WhenWouldBeDeleted` routes through
    /// `return_to_deck`, which would otherwise re-invoke
    /// `WhenWouldLeaveBattleArea` for the same permanent).
    ///
    /// Fresh deletion calls isolate this ledger from enclosing events. Owned
    /// windows retain consumed candidates across selections and reconstruct
    /// their commit guard; permanent keys survive compaction and distinguish
    /// replayed incarnations. See `replacement::with_deletion_event` and
    /// `replacement::run_commit_with_flag`.
    #[doc(hidden)]
    pub replacement_fired: std::collections::HashSet<(
        crate::enums::EffectTiming,
        crate::replacement::ReplacementSubjectIdentity,
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

    /// Companion to `in_replacement_commit`: `true` while the commit being
    /// continued is for a replacement that actually FIRED (a non-`None`
    /// outcome — cancel / redirect / substitute / custom). Only meaningful
    /// while `in_replacement_commit` is set.
    ///
    /// `try_replace_impl` consults it to pick the §7.5 guard strictness: a
    /// FIRED commit blocks ANY prior-fired subject regardless of timing (the
    /// v1 double-prompt protection for redirect routes — pinned by
    /// `dispatcher_guard.rs::commit_continuation_broadening_blocks_different_timing_v1_known_limitation`),
    /// while a DECLINED / no-op commit (outcome `None`: the event proceeds
    /// unchanged) blocks only exact `(timing, subject)` re-dispatch so the
    /// remaining not-yet-offered Would* windows still open. Rule 15-8-5-4:
    /// each simultaneous immediate-type effect "can be activated one at a
    /// time until the cause ... is resolved" — only the already ACTIVATED
    /// one is spent; an offered-and-declined one must not consume the event
    /// (declined `<Decode>` must not eat `<Evade>`).
    #[doc(hidden)]
    pub(crate) replacement_commit_fired: bool,

    /// The exact `(timing, subject)` window a callback-commit continuation is
    /// currently committing, or `None` outside a commit.
    ///
    /// The §7.5 once-per-event guard normally remembers an offered window in
    /// `replacement_fired`, but that set is cleared on any dispatcher entry at
    /// `depth == 0` — and an OPTIONAL replacement unwinds its fire-site when
    /// the selection installs, so the event's OWN later work re-enters at
    /// depth 0 and can wipe its record. The commit then re-offered the very
    /// window it was committing, the player accepted, and the event never
    /// terminated. This key does not depend on that bookkeeping surviving: a
    /// commit may never re-offer the window it is committing, full stop.
    pub(crate) replacement_commit_key: Option<(
        crate::enums::EffectTiming,
        crate::replacement::ReplacementSubjectIdentity,
    )>,

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
    /// panic-safe `catch_unwind` scope at the fire-site. The fallback for
    /// `EffectContext::deletion_cause()` / `was_deleted_by_effect()` /
    /// `was_deleted_by_opponent()`: all three read the deleted object's
    /// snapshot cause first (`observed_deletion_cause`, rule 25), since this
    /// slot is already restored when the `[On Deletion]` bundle is deferred.
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

    /// `<Overclock>` attacks whose cost deletion parked a decision (a
    /// replacement such as <Armor Purge>, or a deletion trigger), in the order
    /// the costs were paid (FIFO; see [`ParkedOverclockAttack`]). The front
    /// entry is declared once nothing is pending and no attack is in progress
    /// (§11-2-4: "A new attack declaration can't be made during an attack"),
    /// and only if its cost was actually deleted (§16-34-3, §15-7-1,
    /// §15-8-5-2): `Game::resume_pending_overclock_attack`, from the
    /// `resolve_selection` tail and when an attack ends (through
    /// `resume_pending_end_turn`). While one is queued, a later end-of-turn
    /// attack keyword's processing waits behind it
    /// (`deferred_end_of_turn_attack_offers`). Plain data: survives
    /// `Game::clone`.
    #[doc(hidden)]
    pub(crate) parked_overclock_attacks: Vec<ParkedOverclockAttack>,
    /// Overclock source cards whose optional cost was declined this end of
    /// turn. <Overclock> is a trigger that activates once per end of turn
    /// (§16-34-2), so nothing re-offers it and no engine path reads the set.
    /// It is kept (cleared on rotation) because the verification digest
    /// hashes it: dropping it would change the digest of every recorded game
    /// in which an <Overclock> was declined (replay goldens).
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
    /// **Stack (G-NESTED-PARKED-REPLACEMENT, 2026-10-01):** a replacement
    /// process can itself cause an event whose replacement window parks
    /// (e.g. `<Guard>` deletes its carrier, whose own `<Decode>` window then
    /// prompts). Parked replacements are pushed and drained LIFO — the inner
    /// event commits before the outer replacement's outcome is committed.
    /// Plain data (clone-safe for `Game: Clone`).
    ///
    /// Outcome writers (`EffectContext::cancel_leave` etc.) address only the
    /// TOP entry and only when it belongs to the currently-running scope —
    /// see `parked_replacement_floor`.
    ///
    /// **Coexistence with `dsl_outer_tail`** (Phase 2d): independent slots for
    /// independent concerns. Phase C §4.1.
    #[doc(hidden)]
    pub(crate) parked_replacement: Vec<crate::replacement::ParkedReplacement>,

    /// Stack depth of `parked_replacement` at the entry of the replacement
    /// process currently running synchronously (0 outside any process). A
    /// process running while an OUTER replacement is parked must not write
    /// its outcome into that outer entry: entries at index `< floor` belong
    /// to enclosing events. Saved/restored by `replacement::run_effect_inner`.
    #[doc(hidden)]
    pub(crate) parked_replacement_floor: usize,

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

    /// Battle events waiting for replacement/deletion choices to settle.
    /// A stack preserves nested effect-driven battles and clones by value.
    pub(crate) pending_battle_resolutions: Vec<crate::combat::PendingBattleResolution>,
    /// Completed declared-attack battle, consumed when its state machine
    /// resumes after the battle's triggered choices have finished.
    pub(crate) pending_attack_battle_result: Option<crate::combat::AttackResult>,
    /// A security battle's winner referent (identity + last-known
    /// information, `Game::battle_winner_referent`) taken when its
    /// battle-result triggers were enqueued, carried to the end of the battle
    /// timing (§14-2-5) that follows them
    /// (`SecurityPhase::EndOfSecurityBattleTimingDrain`).
    pub(crate) security_battle_end_referent: Option<(
        Option<(PlayerId, u64)>,
        Option<std::sync::Arc<crate::trigger_context::DeletedObjectSnapshot>>,
    )>,
    pub(crate) next_battle_identity: u64,
    /// Stack identity (`Permanent::battle_identity`) of the declared
    /// attacker, captured when the declaration's triggers are enqueued
    /// (`fire_on_attack`). `Game::live_attack_attacker` uses it to tell the
    /// attacker from a later occupant of its compacted slot.
    pub(crate) attack_attacker_identity: Option<(PlayerId, u64)>,
    /// The declared attacker's top card, captured with
    /// `attack_attacker_identity`. §11-1-5: the counter timing still occurs
    /// after the attacker leaves the battle area, and its prompt names the
    /// attacking card from this stored context instead of the attacker's
    /// (possibly reused) battle-area slot.
    pub(crate) attack_attacker_card: Option<CardHandle>,
    /// Stack identity of the in-flight attack's effective Digimon target,
    /// captured at the declaration and on every target switch (block,
    /// redirect, `<Raid>`). §11-2-6: `Game::live_attack_target_digimon` uses
    /// it so a removed target stays the target instead of the Digimon that
    /// compacts into its slot. `None` for a player target.
    pub(crate) attack_target_identity: Option<(PlayerId, u64)>,
    /// Scratch cache scoped to one `enqueue_triggered` call (empty outside
    /// it): the event Digimon's last-known snapshot, built once per event
    /// and shared by every context of that event.
    pub(crate) event_snapshot_cache:
        Option<((PlayerId, u64), std::sync::Arc<crate::trigger_context::DeletedObjectSnapshot>)>,
    /// Optional harness ledger, excluded from gameplay and verification state.
    pub(crate) native_play_observation: Option<native_play_observation::NativePlayObservation>,

    /// The REST of a multi-permanent deletion batch whose replacement stage
    /// parked on one member's optional replacement prompt (e.g. the attacker's
    /// `<Evade>` in a tie — kill list `[defender, attacker]`). The parked
    /// member is committed by the replacement accept/decline callback; the
    /// other members (identified by their top card, since indices can shift)
    /// are deleted with the same cause once the selection settles
    /// (`resume_pending_deletion`). Before this, the batch was dropped on the
    /// park and the other members silently survived
    /// (G-ENGINE-BATCH-DELETION-PARK-DROPS-REST, found 2026-10-01 by EX13-076).
    #[doc(hidden)]
    pub(crate) pending_deletion_batch_rest: Vec<PendingDeletionBatchRest>,
    /// The parked member of that batch (owner, top card) — still on the
    /// battle area at resume time ⇒ its replacement cancelled the leave, so
    /// rest members covered by the SAME replacement activation don't leave
    /// either (one activation covers the whole simultaneous batch, as in
    /// DCGO's list-based `WhenRemoveField`).
    #[doc(hidden)]
    pub(crate) pending_deletion_batch_parked:
        Option<(crate::enums::PlayerId, crate::card_source::CardHandle)>,

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
    /// via `EffectContext::schedule_delayed` and moved into the effect queue by
    /// `Game::enqueue_scheduled_for_timing` when their `when:` timing occurs,
    /// joining that timing's printed triggers in one pool (§18-1-2).
    pub scheduled_effects: Vec<crate::scheduled_effects::ScheduledEffect>,

    /// PUPPETS-G003 — provenance-keyed deletions scheduled for this turn's
    /// end. An effect that plays a Digimon and must delete *that specific
    /// permanent* at turn end ("At turn end, delete the Digimon this effect
    /// played") pushes one entry here, keyed by a stable `ProvenanceToken`
    /// (the played card's identity) rather than a battle-area index that can
    /// shift. Enqueued into the end-of-turn pool by `Game::enqueue_provenance_deletions_at_turn_end`
    /// from `fire_end_of_your_turn`; the queue is cleared each turn so a
    /// played permanent that already left is a silent no-op.
    pub scheduled_provenance_deletions: Vec<crate::scheduled_effects::ScheduledProvenanceDeletion>,
    /// PUPPETS-G016 — provenance-keyed deletions scheduled for the end of the
    /// **opponent's** turn. Mirror of `scheduled_provenance_deletions` but
    /// enqueued into the non-turn player's `EndOfOpponentsTurn` half of the
    /// end-of-turn pool (`enqueue_end_of_opponents_turn_pool`)
    /// (`Game::enqueue_provenance_deletions_at_opponents_turn_end`, §18-1-2)
    /// rather than the `EndOfYourTurn` one.
    /// Used by P-165 ShoeShoemon ("At the end of your opponent's turn, delete
    /// that token").
    pub scheduled_provenance_deletions_opp:
        Vec<crate::scheduled_effects::ScheduledProvenanceDeletion>,
    /// Continuation for a delayed-option lifecycle paused by a DelayEffect or
    /// delete/replacement selection. Re-entered from `resolve_selection`.
    pub(crate) pending_delayed_option_lifecycle: Option<DelayedOptionLifecycleResume>,
    /// Stable key `(owner, bottom_card_index)` of a turn-scheduled `<Delay>`
    /// Option whose §16-17-2 cost the controller just DECLINED.
    ///
    /// The turn scan (`resolve_delayed_options_matching`) trashes the Option
    /// after running its body, because the trash IS the Delay's cost. When the
    /// cost is declined the body never ran, so the trash must not happen either
    /// (§15-7-2) — this key tells the two delete sites to reschedule the Option
    /// to its next window instead of deleting it. Cleared as soon as it is
    /// consumed; only ever holds one key because exactly one Delay is resolved
    /// at a time.
    pub(crate) declined_delay_option: Option<(PlayerId, u16)>,
    pub(crate) pending_delayed_option_lifecycle_stack: Vec<DelayedOptionLifecycleResume>,
    /// Continuation for the regular EndTurn state machine when an
    /// EndOfYourTurn effect parks a player selection.
    pub(crate) pending_end_turn_resume: Option<EndTurnResume>,
    /// Continuation for a turn boundary whose §18-1-2 pool parked on a
    /// decision (`resume_pending_turn_boundary`).
    pub(crate) pending_turn_boundary_resume: Option<TurnBoundaryResume>,
    /// Parked Breeding → Main phase advance. Set by `enter_main_phase` when a
    /// breeding-action trigger (e.g. an `[When Moving]` selection fired by
    /// `move_from_breeding`) parked a `PendingSelection` before the phase
    /// advance ran. Without this, `enter_main_phase` would set the phase to
    /// Main mid-selection and the selection's `previous_phase = Breeding`
    /// restore would later drop the turn machine back into a phantom second
    /// Breeding phase (double `StartOfYourMainPhase` — user bug 2026-07-09).
    /// Re-entered from `resolve_selection` via `resume_pending_enter_main`.
    pub(crate) pending_enter_main_after_selection: bool,
    /// An end-of-turn attack (`<Engage>`, `<Execute>`, `<Vortex>`,
    /// `<Overclock>`; `PendingAttack::return_phase == EndOfTurnAction`) has
    /// ended while processing was still resolving (inside a selection's
    /// resolution or an effect drain). `settle_end_of_turn_attack` re-checks
    /// the end-of-turn keywords once everything has settled and lets the end
    /// of turn continue (§16-44-1). Plain data: survives `Game::clone`.
    pub(crate) end_of_turn_attack_settle_pending: bool,
    /// The end-of-turn attack keyword (`<Engage>`, `<Execute>`, `<Vortex>`)
    /// whose trigger body declared the attack in progress, if any. Set at the
    /// declaration (`run_attack_declaration_decision_step`), cleared when the
    /// attack ends. `<Execute>`'s "At the end of the attack, this Digimon is
    /// deleted" applies to THAT attack only (§16-38-1, §16-38-4).
    pub(crate) end_of_turn_attack_keyword: Option<crate::enums::Keyword>,
    /// End-of-turn attack keyword offers waiting for the attack in progress to
    /// end (see [`DeferredEndOfTurnAttackOffer`]), in activation order.
    pub(crate) deferred_end_of_turn_attack_offers: Vec<DeferredEndOfTurnAttackOffer>,

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
    /// A source-move event is open (`Game::begin_source_move_event`): every
    /// `OnDigivolutionCardTrashed` / `OnDigivolutionCardReturnedToDeckBottom`
    /// entry enqueued meanwhile is stamped
    /// (`TriggerContext::in_source_move_event`). One effect step trashing
    /// several digivolution cards is ONE event: its observers are coalesced
    /// to one trigger per observer when the event closes (§15-5-2), and a
    /// later event triggers them again (§15-4-2-4). Coalescing selects the
    /// stamped entries wherever they sit, so a queue `retain_mut` or swap
    /// mid-event cannot misattribute entries. Transient: always `false`
    /// between steps. G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS.
    pub(crate) source_move_event_open: bool,
    /// 6-1-4-1: memory crossed to the opponent's side while an effect was
    /// still resolving (inside a drain or a deferred-drain scope), so
    /// `check_turn_end` held the turn end back. The outermost
    /// `drain_effect_queue` exit re-runs `check_turn_end` once that
    /// processing has finished. G-ENGINE-TURN-END-MID-EFFECT.
    pub(crate) turn_end_check_deferred: bool,
    /// Depth of `resolve_generic_selection` calls in progress; while > 0,
    /// `check_turn_end` defers (G-ENGINE-TURN-END-MID-EFFECT).
    pub(crate) selection_resolution_depth: u16,
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

    /// Transient: `true` while the OnPlay trigger for a just-played permanent
    /// is being dispatched IF that play was effect-driven (`PlaySource::ByEffect`).
    /// Read by the `OnPlay` trigger context so a card's `played_by_effect`
    /// predicate can gate "if played by an effect, …" (BT25-080 Witchmon main
    /// clause; G-ENGINE-ON-DISCARD-HAND). Set by `fire_play_event_triggers`
    /// around `fire_on_play`, cleared immediately after. `false` for normal
    /// hand plays and digivolutions. Mirrors DCGO `CardEffectCommons.IsByEffect`.
    pub(crate) pending_play_effect_initiated: bool,

    /// Open coalescing window for the `OnDiscardHand` batch trigger
    /// (G-ENGINE-ON-DISCARD-HAND). While `Some`, effect-caused hand→trash calls
    /// append the trashing player; the window is flushed (fires
    /// `OnDiscardHand` once) when the effect body that opened it completes.
    /// `None` outside a discard window. Mirrors DCGO `DiscardHands()` collecting
    /// the whole discard list before firing the trigger once.
    pub(crate) pending_hand_discard: Option<PendingHandDiscard>,
    /// Exact effect-milled cards, released after the causing effect completes.
    pub(crate) pending_deck_discards: Vec<(PlayerId, CardHandle, u64, crate::trigger_context::EffectAttribution)>,

    /// Open coalescing window for the `OnAddToDeck` batch trigger
    /// (G-ENGINE-ON-ADD-TO-DECK): the controller of the effect that added
    /// cards to a deck during the current effect body. Flushed (fires the
    /// trigger once) at the outermost drain, alongside `pending_hand_discard`.
    pub(crate) pending_deck_add: Option<PlayerId>,

    /// Open coalescing windows for the `OnAddDigivolutionCards` batch trigger
    /// (G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS), one per host permanent that an
    /// effect placed sources under during the current effect body. Flushed
    /// (fires the trigger once per host) at the outermost drain, alongside
    /// `pending_hand_discard`. Empty outside a placement window. Plain data,
    /// so it clones with the `Game` (rule 28 clone-safety).
    pub(crate) pending_added_sources: Vec<PendingAddedSources>,

    /// The placing effect's carrier for an in-flight effect-driven
    /// security→digivolution-source placement (`place_as_source_observed`'s
    /// Security branch hands the card to `fire_effect_security_removal`,
    /// whose completion in `complete_effect_security_removal` is where the
    /// card actually enters the stack and the `OnAddDigivolutionCards` batch
    /// is noted). Set just before the removal is fired, consumed (taken) when
    /// the `BottomSource` / `TopSource` destination is seated. `None`
    /// otherwise. G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS.
    pub(crate) pending_security_source_cause_card: Option<crate::card_source::CardHandle>,
    /// `true` only around the `OnLoseSecurity` enqueue inside
    /// `fire_effect_security_removal` (every caller of which is an EFFECT
    /// removal — the combat security check enqueues its own drain). Read by
    /// `trigger_context_for_source` so `event_is_effect_initiated` can tell
    /// "removed from by effects" apart from a security check (BT26-089 Kyo
    /// Sawashiro; DCGO `CardEffectCommons.IsByEffect`).
    /// G-ENGINE-LOSE-SECURITY-BY-EFFECT.
    pub(crate) pending_security_loss_effect_initiated: bool,
    /// Face-down flag for an in-flight effect-driven security→digivolution
    /// source placement (`place_as_source_observed`'s Security branch);
    /// consumed when `complete_effect_security_removal` seats the card
    /// (BT26-025 Liollmon "place your top security card face down under …").
    /// G-DSL-SECURITY-TOP-AS-SOURCE.
    pub(crate) pending_security_source_face_down: bool,

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
    /// Shared via `Arc` with the card store (see `card_store`).
    pub(crate) card_id_index: std::sync::Arc<std::collections::HashMap<String, usize>>,
}

// Tier-1 impl Game mechanic clusters (see docs/RUST_ENGINE_API.md §3).
mod handles;
pub mod infinite_loop;
mod lifecycle;
mod memory;
mod opt;
mod queries;
mod setup;
mod snapshot;
mod staging;
mod succession;
mod suspend;
mod triggers;
mod until_condition;
mod verification;

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
        self.players[pid as usize].put_in_hand(card);
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
        let hand: Vec<_> = p.hand.drain(..).collect();
        p.put_all_on_deck_top(hand);
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
    /// Strict transport entry point. Serial engine/RL callers resolve the
    /// current prompt; asynchronous callers must echo its exact page token.
    pub fn resolve_selection_checked(
        &mut self,
        player: PlayerId,
        action_id: u16,
        expected: crate::card_choice_page::CardChoiceToken,
    ) -> Result<(), crate::selection::SelectionError> {
        let current = self.pending_selection.as_ref().and_then(|p| p.card_choice.as_ref())
            .map(|owner| owner.token());
        if current != Some(expected) { return Err(crate::selection::SelectionError::InvalidAction); }
        self.resolve_selection(player, action_id)
    }

    pub fn resolve_selection(
        &mut self,
        player: PlayerId,
        action_id: u16,
    ) -> Result<(), SelectionError> {
        if self.pending_selection.is_none() {
            return Err(SelectionError::NoPendingSelection);
        }
        // A finished game accepts no further decision: only the between-games
        // play-order choice (`SelectionKind::PlayOrder`, installed by the BO3
        // match wrapper after `game_over`) resolves.
        if self.game_over
            && self
                .pending_selection
                .as_ref()
                .is_some_and(|sel| sel.kind != crate::selection::SelectionKind::PlayOrder)
        {
            return Err(SelectionError::GameOver);
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
            .put_in_trash(removed);
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
        // <Overflow> is OWNER-relative: "lose X memory" (4-1-4) is applied to
        // the card's owner (4-19-1; DCGO `AceOverflowClass.Overflow()` ->
        // `cardSource.Owner.AddMemory(-OverflowMemory)`, CardController.cs
        // ~6151). The raw `self.memory` seesaw is turn-player relative, so
        // adding the signed value directly made a non-turn-player owner GAIN
        // memory. Simultaneous instances resolve turn player's first (4-19-5).
        let turn_player = self.turn_player();
        let mut penalties: Vec<(PlayerId, i16)> = sources
            .iter()
            .filter(|source| !source.is_token)
            .filter_map(|source| {
                let value = self.card_data.get(source.data_index)?.ace_overflow?;
                Some((source.owner, value as i16))
            })
            .filter(|(_, value)| *value != 0)
            .collect();
        penalties.sort_by_key(|(owner, _)| if *owner == turn_player { 0 } else { 1 });
        for (owner, value) in penalties {
            // `ace_overflow` is stored signed (e.g. -4): a gain of -4 for the
            // owner is "lose 4 memory".
            self.gain_memory_for_player(owner, value);
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
            .put_in_hand(removed);
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
            effect_initiated: true,
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
                effect_initiated: true,
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

    /// The battle-area permanent whose TOP card is `card`, if any.
    pub fn permanent_with_top_card(
        &self,
        card: crate::card_source::CardHandle,
    ) -> Option<PermanentHandle> {
        for (pid, player) in self.players.iter().enumerate() {
            for (i, perm) in player.battle_area.iter().enumerate() {
                if perm.card_sources.last().is_some_and(|c| c.handle() == card) {
                    return Some(PermanentHandle {
                        player: pid as PlayerId,
                        index: i as u8,
                    });
                }
            }
        }
        None
    }

    /// Effect-initiated SIMULTANEOUS free play of `controller`'s hand / trash
    /// cards (`play_cards_free`, BT26-081 Mervamon "play up to 8 play cost's
    /// total worth of [Iliad] trait cards from your hand or trash without
    /// paying the costs" — DCGO `PlayPermanentCards` over the whole list).
    /// Only Digimon / Tamer cards are played; a card no longer in its zone,
    /// or one an effect forbids playing / without a free field slot, is
    /// skipped. All cards enter before any of their play triggers resolve.
    pub fn play_cards_from_zones_free(
        &mut self,
        controller: PlayerId,
        cards: &[crate::card_source::CardHandle],
    ) -> usize {
        use crate::enums::{CardKind, CardSourceRef};
        let mut taken_cards: Vec<CardSource> = Vec::new();
        let mut required_slots = 0usize;
        for &card in cards {
            let Some(src) = self.locate_card_source_ref(card) else {
                continue;
            };
            let in_own_zone = matches!(
                src,
                CardSourceRef::Hand(p, _) | CardSourceRef::Trash(p, _) if p == controller
            );
            if !in_own_zone {
                continue;
            }
            if !matches!(
                self.card_kind_for_handle(card),
                Some(CardKind::Digimon) | Some(CardKind::Tamer)
            ) {
                continue;
            }
            if !self.can_play_card_from_effect_without_cost(controller, card, required_slots + 1) {
                continue;
            }
            let _ = src;
            if let Some(taken) = self.take_card_by_handle(card) {
                required_slots += 1;
                taken_cards.push(taken);
            }
        }

        let turn = self.turn_count;
        let mut entered = Vec::with_capacity(taken_cards.len());
        for card_source in taken_cards {
            let emitted_card_id = card_source.card_id(&self.card_data).to_string();
            let emitted_card_name = card_source.card_name(&self.card_data).to_string();
            let cost_printed = self.card_data[card_source.data_index].play_cost as i16;
            let player = self.player_mut(controller);
            player
                .battle_area
                .push(crate::permanent::Permanent::new(card_source, turn));
            let field_index = player.battle_area.len() - 1;
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::Play {
                seq,
                player: controller,
                card_id: emitted_card_id,
                card_name: emitted_card_name,
                field_index: field_index as u8,
                cost_paid: 0,
                cost_printed,
                via_alt_path: None,
                effect_initiated: true,
            });
            entered.push(field_index);
        }
        let n = entered.len();
        for field_index in entered {
            self.fire_play_event_triggers(controller, field_index, true, false);
        }
        n
    }

    pub fn can_play_card_from_effect_without_cost(
        &self,
        player_id: PlayerId,
        card: crate::card_source::CardHandle,
        required_slots: usize,
    ) -> bool {
        self.can_play_card_from_effect_without_cost_for_controller(
            player_id,
            card,
            required_slots,
            self.effect_source_player.unwrap_or(player_id),
        )
    }

    /// Read-only target eligibility with an explicit destination and causing
    /// effect controller. Selection masks must not depend on ambient resolution
    /// provenance, which can be absent or belong to an outer effect.
    pub fn can_play_card_from_effect_without_cost_for_controller(
        &self,
        destination: PlayerId,
        card: crate::card_source::CardHandle,
        required_slots: usize,
        controller: PlayerId,
    ) -> bool {
        let Some(player) = self.players.get(destination as usize) else {
            return false;
        };
        if self.players.get(controller as usize).is_none()
            || player.battle_area.len() + required_slots > self.rules.field_slots as usize
        {
            return false;
        }
        let Some(data) = self.card_data_for_handle(card) else {
            return false;
        };
        if self.card_data_play_is_prohibited(controller, data) {
            return false;
        }
        let card_kind = data.card_kind;
        let restriction = match card_kind {
            CardKind::Digimon | CardKind::Dual | CardKind::Token => {
                ModifierType::CannotPlayDigimonByEffect
            }
            CardKind::Tamer => ModifierType::CannotPlayTamerByEffect,
            _ => return true,
        };
        !self.modifiers.player_has(controller, restriction)
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

    /// Infer provenance for an effect-driven deletion. An attack or security
    /// check can contain resolving card effects; neither ambient procedure
    /// turns an effect deletion into battle deletion. Actual normal and
    /// security battles pass `ReplacementCause::Battle` explicitly.
    ///
    /// Consumed by the deletion fire-site in `combat::delete_permanent_with_effects`.
    pub(crate) fn infer_deletion_cause(
        &self,
        target_handle: crate::permanent::PermanentHandle,
    ) -> crate::replacement::ReplacementCause {
        self.infer_effect_cause(target_handle.player)
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
    /// `Battle` is never inferred from an ambient attack. Battle fire-sites
    /// provide their cause explicitly; deletion uses this same effect path.
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
        self.parked_replacement.push(parked);
    }

    /// Test-only getter for the parked-replacement outcome. The
    /// `parked_replacement` field is `pub(crate)`, so behavioral tests
    /// under `digimon-engine/tests/` cannot read it directly. Returns
    /// `None` when no replacement is parked.
    #[doc(hidden)]
    pub fn parked_replacement_outcome_for_test(
        &self,
    ) -> Option<crate::replacement::ReplacementOutcome> {
        self.parked_replacement.last().map(|p| p.outcome)
    }

    /// Test-only: number of parked replacements currently stacked
    /// (G-NESTED-PARKED-REPLACEMENT).
    #[doc(hidden)]
    pub fn parked_replacement_depth_for_test(&self) -> usize {
        self.parked_replacement.len()
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
    /// engine code should prefer this over a bare `Player::put_in_trash`
    /// so the event surface stays uniform; both place the card face-up
    /// (§3-6-3).
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
        self.player_mut(player).put_in_trash(card);
    }

    /// Record that an EFFECT (controlled by `cause_controller`) just trashed a
    /// card from `trashing_player`'s hand, opening or extending the
    /// `OnDiscardHand` batch window (G-ENGINE-ON-DISCARD-HAND). Called from
    /// `EffectContext::trash_from_hand_by_index` — the single effect-caused
    /// hand→trash choke point. The trigger is NOT fired here; it fires once when
    /// the window is flushed (`flush_pending_hand_discard`) after the causing
    /// effect body completes, mirroring DCGO `DiscardHands()` collecting the
    /// full discard list before firing `OnDiscardHand`.
    ///
    /// A window already open with a DIFFERENT `cause_controller` is flushed
    /// first, so two nested effect bodies never merge their discards.
    pub(crate) fn note_effect_hand_discard(
        &mut self,
        cause_controller: crate::enums::PlayerId,
        trashing_player: crate::enums::PlayerId,
    ) {
        if let Some(window) = self.pending_hand_discard.as_ref() {
            if window.cause_controller != cause_controller {
                self.flush_pending_hand_discard();
            }
        }
        let window =
            self.pending_hand_discard
                .get_or_insert_with(|| crate::game::PendingHandDiscard {
                    cause_controller,
                    trashed_players: Vec::new(),
                    trashed_cards: Vec::new(),
                });
        if !window.trashed_players.contains(&trashing_player) {
            window.trashed_players.push(trashing_player);
        }
    }

    /// Record that an EFFECT controlled by `cause_controller` just added a
    /// (non-Digi-Egg) card to a deck from outside it, opening or extending the
    /// `OnAddToDeck` batch window (G-ENGINE-ON-ADD-TO-DECK). A window open
    /// with a DIFFERENT controller is flushed first so two effect bodies never
    /// merge. The trigger fires when the window is flushed after the causing
    /// effect body completes (`flush_pending_deck_add`).
    pub(crate) fn note_effect_deck_add(&mut self, cause_controller: crate::enums::PlayerId) {
        if let Some(open) = self.pending_deck_add {
            if open != cause_controller {
                self.flush_pending_deck_add();
            }
        }
        self.pending_deck_add = Some(cause_controller);
    }

    /// `note_effect_deck_add` for Game-level deck sinks: records only when an
    /// effect is currently resolving (`effect_source_player`), so rule-driven
    /// deck moves (mulligan, setup) never fire the observer.
    pub(crate) fn note_effect_deck_add_if_resolving(&mut self) {
        if let Some(cause) = self.effect_source_player {
            self.note_effect_deck_add(cause);
        }
    }

    /// Fire the `OnAddToDeck` batch trigger for the open window, then close it.
    /// G-ENGINE-ON-ADD-TO-DECK.
    pub(crate) fn flush_pending_deck_add(&mut self) {
        let Some(cause_controller) = self.pending_deck_add.take() else {
            return;
        };
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnAddToDeck,
            crate::selection::TriggerSource::DeckGained { cause_controller },
        );
    }

    /// Fire the `OnDiscardHand` batch trigger (once per affected owner) for the
    /// open discard window, then close it. No-op if no window is open or it
    /// recorded no discards. G-ENGINE-ON-DISCARD-HAND.
    pub(crate) fn flush_pending_hand_discard(&mut self) {
        let Some(window) = self.pending_hand_discard.take() else {
            return;
        };
        let cause_controller = window.cause_controller;
        for trashing_player in window.trashed_players {
            let cards = window
                .trashed_cards
                .iter()
                .filter(|(owner, _)| *owner == trashing_player)
                .map(|(_, card)| *card)
                .collect();
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnDiscardHand,
                crate::selection::TriggerSource::HandDiscarded {
                    player: trashing_player,
                    cause_controller,
                    cards,
                },
            );
        }
        // "When this card is trashed from the hand" (BT26-069 Dobermon):
        // the trashed cards' own `scope: trash` clauses.
        // G-ENGINE-SELF-TRASHED-FROM-HAND.
        for (owner, card) in window.trashed_cards {
            self.enqueue_self_trashed_from_hand(owner, card, cause_controller);
        }
    }

    /// Record the specific card an effect just moved from `owner`'s hand to
    /// the trash into the open `OnDiscardHand` window (call right after
    /// `note_effect_hand_discard`). G-ENGINE-SELF-TRASHED-FROM-HAND.
    pub(crate) fn note_effect_hand_discard_card(
        &mut self,
        owner: crate::enums::PlayerId,
        card: crate::card_source::CardHandle,
    ) {
        if let Some(window) = self.pending_hand_discard.as_mut() {
            window.trashed_cards.push((owner, card));
        }
    }

    /// Record that an EFFECT (`cause`) just placed `cards` into `host`'s
    /// digivolution cards, opening or extending that host's
    /// `OnAddDigivolutionCards` batch window
    /// (G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS). Call this from EVERY
    /// effect-driven source-placement facade AFTER the `card_sources`
    /// mutation — and from NO rule-driven one: normal / DNA digivolution,
    /// DigiXros + Assembly material consumption and App Fusion place sources
    /// with a `null` `cardEffect` in DCGO (`CardController.cs:1736`,
    /// `SelectDigiXrosClass.cs:910`, `SelectAssemblyClass.cs:302`,
    /// `SelectAppFusionEffect.cs:234`) and `CanTriggerOnAddDigivolutionCard`
    /// requires a non-null one, so they never trigger it.
    ///
    /// The trigger is NOT fired here; it fires once per host when the window
    /// is flushed (`flush_pending_added_sources`) after the placing effect
    /// body completes — DCGO `AddDigivolutionCards*` stacks the skill once
    /// per added LIST (Permanent.cs:1133 / 1237), and rule 15-5-2 makes one
    /// condition met several times at once a single triggering. Tokens
    /// never enter a stack (DCGO skips `IsToken` cards and token hosts), so a
    /// caller that placed nothing simply passes an empty `cards`.
    ///
    /// `host` may be a battle-area or breeding handle; it is re-resolved by
    /// top-card identity at flush time.
    pub(crate) fn note_effect_added_sources(
        &mut self,
        host: crate::permanent::PermanentHandle,
        cards: Vec<crate::card_source::CardHandle>,
        cause: crate::trigger_context::EffectAttribution,
    ) {
        if cards.is_empty() {
            return;
        }
        let Some(host_card) = self.stable_top_card_of(host) else {
            return;
        };
        if let Some(window) = self
            .pending_added_sources
            .iter_mut()
            .find(|w| w.host_card == host_card)
        {
            if window.cause.controller == cause.controller {
                window.cards.extend(cards);
                return;
            }
            // A different controller's effect is now placing under the same
            // host: fire the earlier batch on its own.
            let earlier = window.clone();
            self.pending_added_sources
                .retain(|w| w.host_card != host_card);
            self.fire_added_sources_batch(earlier);
        }
        self.pending_added_sources.push(PendingAddedSources {
            host_card,
            cause,
            cards,
        });
    }

    /// Fire the `OnAddDigivolutionCards` batch trigger once per host for every
    /// open placement window, then close them all. No-op when nothing is
    /// pending. G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS.
    pub(crate) fn flush_pending_added_sources(&mut self) {
        let windows = std::mem::take(&mut self.pending_added_sources);
        for window in windows {
            self.fire_added_sources_batch(window);
        }
    }

    fn fire_added_sources_batch(&mut self, window: PendingAddedSources) {
        // Re-resolve the host by its stable top card; a host that has left
        // play since the placement fires nothing (DCGO `TopCard != null`).
        let Some(host) = self.permanent_handle_by_top_card(window.host_card) else {
            return;
        };
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnAddDigivolutionCards,
            crate::selection::TriggerSource::SourcesAddedToStack {
                host,
                host_card: window.host_card,
                cards: window.cards,
                cause: window.cause,
            },
        );
    }

    /// The top card of a battle-area or breeding permanent, or `None` when
    /// the slot is empty / missing.
    pub(crate) fn stable_top_card_of(
        &self,
        handle: crate::permanent::PermanentHandle,
    ) -> Option<crate::card_source::CardHandle> {
        let player = self.players.get(handle.player as usize)?;
        let perm = if handle.index == crate::action::space::BREEDING_TARGET as u8 {
            player.breeding_area.as_ref()?
        } else {
            player.battle_area.get(handle.index as usize)?
        };
        perm.card_sources.last().map(|card| card.handle())
    }

    /// Locate the battle-area or breeding permanent whose TOP card is
    /// `top_card` (stable-identity → positional handle). `None` when no such
    /// permanent is in play.
    pub(crate) fn permanent_handle_by_top_card(
        &self,
        top_card: crate::card_source::CardHandle,
    ) -> Option<crate::permanent::PermanentHandle> {
        for (pid, player) in self.players.iter().enumerate() {
            if let Some(index) = player
                .battle_area
                .iter()
                .position(|perm| perm.card_sources.last().map(|c| c.handle()) == Some(top_card))
            {
                return Some(crate::permanent::PermanentHandle {
                    player: pid as PlayerId,
                    index: index as u8,
                });
            }
            if player
                .breeding_area
                .as_ref()
                .and_then(|perm| perm.card_sources.last())
                .map(|c| c.handle())
                == Some(top_card)
            {
                return Some(crate::permanent::PermanentHandle {
                    player: pid as PlayerId,
                    index: crate::action::space::BREEDING_TARGET as u8,
                });
            }
        }
        None
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
        self.invalidate_captured_security_check_source(crate::permanent::PermanentHandle {
            player,
            index: field_index as u8,
        });

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
                self.player_mut(player).put_in_trash(card);
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
            self.player_mut(player).put_in_trash(card);
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
            self.player_mut(player).put_in_trash(card);
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
        // The first terminal outcome stands (see `finish_game`).
        if self.game_over {
            return;
        }
        if self.rules.player_count == 2 {
            // Standard: deck-out = loss. One terminal path (`finish_game`).
            let winner = self.opponents(player_id).first().copied();
            self.finish_game(winner, TerminalOutcomeReason::DeckOut);
        } else {
            // Multiplayer: elimination
            self.eliminate_player(player_id);
        }
    }

    /// Allocate a new unique card index (for tokens, etc.).
    pub(crate) fn reset_hypothetical_indices(&mut self, next:u16) { self.next_card_index=next; }

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
    /// **Material-stack ordering:** the printed left DNA requirement goes
    /// above the right (8-2-2-2), preserving each material's internal order
    /// (8-2-2-3). Determine that order independently of which carrier survives
    /// compaction. A recipe-free/requirements-ignoring caller retains its
    /// explicit bottom-to-top A-then-B order when no printed recipe matches.
    ///
    /// **Simultaneous trigger surface:** `WhenDigivolving` and
    /// `OnDnaDigivolve` on the merged permanent plus global `OnDigivolve`
    /// observers are queued together before the controller chooses their order.
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
    /// §8-2-3-3 — "places the Digimon for the DNA digivolution on top of the
    /// stack, draws 1 card, and the DNA digivolution process is resolved" —
    /// so `hand_owner` ALWAYS draws 1 after the merge and before triggers
    /// fire, however the DNA digivolution was initiated. This used to be
    /// gated behind a `grant_digivolve_bonus` flag that the effect-initiated
    /// callers passed as `false`; since §8-2-2-4 makes every DNA digivolution
    /// effect-driven, that silently skipped the draw for most real DNA lines.
    ///
    /// `effect_initiated`: marks the global `OnDigivolve` payload so observers
    /// can distinguish effect-created DNA digivolutions from player-action DNA.
    /// Reset the old field objects only after the DNA route and payment succeed.
    /// Rules 8-2-2-1: the result inherits neither old state nor gained effects.
    /// This is not deletion, unsuspension, or effect-caused link trashing.
    pub(crate) fn reset_dna_material_objects(&mut self, materials: &[PermanentHandle]) {
        if let Some(attack) = self.pending_attack.as_mut() {
            if matches!(attack.effective_target,
                crate::selection::AttackTarget::Digimon(h) if materials.contains(&h)) {
                attack.effective_target_removed = true;
            }
        }
        self.invalidate_battle_winners_for_dna(materials);
        self.invalidate_pending_field_effects(materials);
        let mut all_links = Vec::new();
        for &handle in materials {
            self.clear_permanent_full(handle);
            let turn = self.turn_count;
            let links = {
                let perm = &mut self.player_mut(handle.player).battle_area[handle.index as usize];
                perm.is_suspended = false;
                // Every attack gate tests equality with current turn, including turn zero.
                perm.turn_played = turn.wrapping_sub(1);
                perm.turn_digivolved = 0;
                perm.attacks_this_turn = 0;
                perm.is_attacking = false;
                perm.effect_activations.clear();
                perm.option_state = crate::permanent::OptionState::Standard;
                for source in &mut perm.card_sources {
                    source.also_treated_as.clear();
                    source.clear_reveal_overlay();
                }
                std::mem::take(&mut perm.linked_cards)
            };
            all_links.extend(links);
        }
        let overflow_sources = all_links.clone();
        for mut card in all_links {
            let owner = card.owner;
            let card_id = card.card_id(&self.card_data).to_string();
            let card_name = card.card_name(&self.card_data).to_string();
            card.also_treated_as.clear();
            card.clear_reveal_overlay();
            self.player_mut(owner).put_in_trash(card);
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::Trash {
                seq, player: owner, card_id, card_name,
            });
        }
        // Apply all simultaneous link Overflow owner-relatively, turn player first.
        self.apply_ace_overflow_for_sources(&overflow_sources);
    }

    /// Field materials are new cards under8-2-2-1-2; hand/trash inputs
    /// change areas under3-1-3-1. Drop only effect-granted instance overlays.
    /// Physical identity, printed/static aliases and face-down source state stay.
    pub(crate) fn clear_dna_card_overlays(&mut self, result: PermanentHandle) {
        for card in &mut self.player_mut(result.player).battle_area[result.index as usize].card_sources {
            card.also_treated_as.clear();
            card.clear_reveal_overlay();
        }
    }

    pub(crate) fn dna_digivolve_inner(
        &mut self,
        target_a: PermanentHandle,
        target_b: PermanentHandle,
        hand_owner: PlayerId,
        hand_index: usize,
        cost: u16,
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

        // Our source vectors are bottom-to-top. Recipe requirement1 is the
        // upper material, not the first block to append. Effect callers may
        // supply the same legal pair in either order, so keep the carrier and
        // its physical identity separate from the stack orientation.
        let a_is_upper = {
            let result = &self.player(hand_owner).hand[hand_index];
            let extras_a = self.dna_material_extras(target_a, result);
            let extras_b = self.dna_material_extras(target_b, result);
            crate::dna_digivolve::get_dna_stacking_order_ext(
                &self.card_data[result.data_index],
                (&self.player(target_a.player).battle_area[target_a.index as usize], &extras_a),
                (&self.player(target_b.player).battle_area[target_b.index as usize], &extras_b),
                &self.card_data,
            )
            .map(|(a_is_upper, _)| a_is_upper)
            .unwrap_or(false)
        };

        // Effect DNA can resolve on either player's turn. Memory is stored
        // from the turn player's perspective, so the hand owner pays toward
        // their opponent even when the hand owner is the non-turn player.
        if cost > 0 {
            if hand_owner == self.turn_player() {
                if !self.pay_memory(cost) {
                    return None;
                }
            } else {
                if i32::from(self.memory) + i32::from(cost) > i32::from(self.rules.memory_range.1) {
                    return None;
                }
                self.lose_memory_for_player(hand_owner, cost as i16);
            }
        }

        self.reset_dna_material_objects(&[target_a, target_b]);

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
        // Field compaction changes surviving observers' positional handles,
        // but does not turn them into new cards (15-4-4-3).
        self.shift_effect_queue_after_battle_area_remove(target_b.player, target_b.index);
        let new_top = self.player_mut(hand_owner).hand.remove(hand_index);
        // Capture top_card_id from the removed-hand source before it moves
        // into the merged permanent's stack.
        let top_card_id = new_top.card_id(&self.card_data).to_string();
        let top_card_name = new_top.card_name(&self.card_data).to_string();

        // The committed result identity must survive effects that move or
        // delete the merged permanent before OnDigivolve observers dispatch.
        let event_card = new_top.handle();

        let turn = self.turn_count;
        {
            let perm_a = &mut self.player_mut(target_a.player).battle_area[target_a_index_after];
            if a_is_upper {
                let mut sources = perm_b.card_sources;
                sources.append(&mut perm_a.card_sources);
                perm_a.card_sources = sources;
            } else {
                perm_a.card_sources.extend(perm_b.card_sources);
            }
            perm_a.push_top(new_top);
            perm_a.turn_digivolved = turn;
        }

        let merged_handle = PermanentHandle {
            player: target_a.player,
            index: target_a_index_after as u8,
        };
        self.clear_dna_card_overlays(merged_handle);

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

        // §8-2-3-3: the DNA digivolution procedure draws 1 card.
        // Opaque-aware: routes through draw_one_for_player so opaque
        // opponents pull from their RevealSource rather than from the
        // (empty in opaque mode) deck Vec. Errors are logged-and-
        // ignored to preserve the original `-> ()` signature of this
        // function; the symptom of a misaligned reveal source is a
        // downstream parity divergence the replay harness will catch.
        if let Err(e) = self.draw_one_for_player(hand_owner) {
            eprintln!(
                "[opaque-deck] DNA digivolve draw error for player {}: {}",
                hand_owner, e
            );
        }

        self.fire_digivolve_event_triggers(merged_handle, event_card, effect_initiated, true);

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
    /// The field-plus-hand DNA commit keeps the hand partner in hand until
    /// the merge. It never creates a temporary battle-area permanent or fires
    /// play effects for that material. The printed left requirement is the
    /// upper material and each material's internal source order is preserved.
    /// The field slot survives while its old object state is reset.
    ///
    /// Evolution triggers are queued together with the DNA origin marker.
    /// Both hand cards must be distinct and owned by the field controller;
    /// memory payment is checked from that controller's perspective.
    pub(crate) fn dna_digivolve_hand_partner_with_order(
        &mut self,
        target: PermanentHandle,
        hand_owner: PlayerId,
        partner_index: usize,
        result_index: usize,
        cost: u16,
        effect_initiated: bool,
        field_is_upper: bool,
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

        if target.player != hand_owner || !self.pay_memory_for(hand_owner, cost) {
            return None;
        }

        self.reset_dna_material_objects(&[target]);

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
            if field_is_upper {
                perm.card_sources.insert(0, partner_source);
            } else {
                perm.card_sources.push(partner_source);
            }
            perm.push_top(result_source);
            perm.turn_digivolved = turn;
        }

        let merged_handle = target;
        self.clear_dna_card_overlays(merged_handle);

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

        // §8-2-3-3: the DNA digivolution procedure draws 1 card. Same rule as
        // `dna_digivolve_inner`; these partner variants (hand partner / trash
        // partner) are DNA digivolutions too, so they draw identically.
        if let Err(e) = self.draw_one_for_player(hand_owner) {
            eprintln!(
                "[opaque-deck] DNA digivolve draw error for player {}: {}",
                hand_owner, e
            );
        }

        let event_card = self
            .player(merged_handle.player)
            .battle_area
            .get(merged_handle.index as usize)
            .map(|perm| perm.top_card().handle())?;
        self.fire_digivolve_event_triggers(merged_handle, event_card, effect_initiated, true);

        // Reward-shaping counters — see `dna_digivolve_inner` for rationale.
        // Only user-action DNAs (effect_initiated == false) credit.
        if !effect_initiated {
            self.n_digivolutions[merged_handle.player as usize] += 1;
            self.n_dna_digivolutions[merged_handle.player as usize] += 1;
        }

        Some(merged_handle)
    }

    /// DNA digivolve where ONE material lives on the field (`target`), the
    /// OTHER material lives in `hand_owner`'s **trash** (`trash_index`), and the
    /// merged permanent is topped with a **hand** card (`result_index`).
    ///
    /// This is the BT18-015 / BT18-073 `[On Deletion]` shape:
    ///   "1 of your [Machinedramon] **in play** and 1 [Kimeramon] **in the
    ///    trash** may DNA Digivolve into [Millenniummon] **in the hand**."
    ///
    /// DCGO (`BT18_015.cs` / `BT18_073.cs`) materialises the trash material via
    /// `CardObjectController.CreateNewPermanent` — a **pure placement that fires
    /// NO triggers** (no `[On Play]`, no `OnEnterField`) — then jogress-merges
    /// with `PlayCardClass(..., payCost: true, activateETB: true)`.
    ///
    /// We fold the two steps into one atomic merge so the trash material never
    /// exists as an independently-played permanent: it moves straight from
    /// trash into the merged stack. This exactly reproduces DCGO's
    /// observer-firing surface — only the merged TOP's `WhenDigivolving` /
    /// `OnDnaDigivolve` / `OnDigivolve` fire (below), never the trash
    /// material's own play triggers.
    ///
    /// ## Stacking order
    ///
    /// `target.card_sources ++ [trash_material] ++ [result]`. `target` is the
    /// `requirement1` material; the trash material is `requirement2`. The merged
    /// permanent stays at `target`'s index (no on-field permanent is removed).
    ///
    /// ## Triggers
    ///
    /// Identical to `dna_digivolve_hand_partner_with_order`: self and global evolution
    /// triggers are queued together before draining with the DNA origin marker.
    ///
    /// ## Returns
    ///
    /// `Some(merged_handle)` on success; `None` if `target` is out of range,
    /// `trash_index` is out of range on `hand_owner`'s trash, `result_index` is
    /// out of range on `hand_owner`'s hand, or `cost > 0` and `pay_memory`
    /// fails.
    pub(crate) fn dna_digivolve_trash_partner_inner(
        &mut self,
        target: PermanentHandle,
        hand_owner: PlayerId,
        trash_index: usize,
        result_index: usize,
        cost: u16,
        effect_initiated: bool,
    ) -> Option<PermanentHandle> {
        use crate::enums::EffectTiming;
        use crate::selection::TriggerSource;

        if (target.index as usize) >= self.player(target.player).battle_area.len() {
            return None;
        }
        if trash_index >= self.player(hand_owner).trash.len() {
            return None;
        }
        if result_index >= self.player(hand_owner).hand.len() {
            return None;
        }

        if cost > 0 && !self.pay_memory(cost) {
            return None;
        }

        self.reset_dna_material_objects(&[target]);

        // Capture from_stack_top before the merge (for Digivolve event).
        let from_stack_top = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .map(|p| p.top_card().card_id(&self.card_data).to_string())
            .unwrap_or_default();

        // Remove the trash material and the hand result. These live in distinct
        // zones so index shifting between them is not a concern.
        let trash_material = self.player_mut(hand_owner).trash.remove(trash_index);
        let result_source = self.player_mut(hand_owner).hand.remove(result_index);

        let top_card_id = result_source.card_id(&self.card_data).to_string();
        let top_card_name = result_source.card_name(&self.card_data).to_string();

        let turn = self.turn_count;
        {
            let perm = &mut self.player_mut(target.player).battle_area[target.index as usize];
            perm.card_sources.push(trash_material);
            perm.push_top(result_source);
            perm.turn_digivolved = turn;
        }

        let merged_handle = target;
        self.clear_dna_card_overlays(merged_handle);

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

        // §8-2-3-3: the DNA digivolution procedure draws 1 card. Same rule as
        // `dna_digivolve_inner`; these partner variants (hand partner / trash
        // partner) are DNA digivolutions too, so they draw identically.
        if let Err(e) = self.draw_one_for_player(hand_owner) {
            eprintln!(
                "[opaque-deck] DNA digivolve draw error for player {}: {}",
                hand_owner, e
            );
        }

        let event_card = self
            .player(merged_handle.player)
            .battle_area
            .get(merged_handle.index as usize)
            .map(|perm| perm.top_card().handle())?;
        self.fire_digivolve_event_triggers(merged_handle, event_card, effect_initiated, true);

        // Reward-shaping counters — see `dna_digivolve_inner` for rationale.
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
        payload: Option<crate::modifiers::ModifierPayload>,
    ) {
        self.add_floating_mass_modifier_scoped(filter, modifier, value, source_card,
            source_player, expiry, effect_immunity, payload, false);
    }

    pub fn add_floating_mass_modifier_scoped(
        &mut self,
        filter: digimon_dsl::compiled::CompiledPredicate,
        modifier: ModifierType,
        value: i32,
        source_card: crate::card_source::CardHandle,
        source_player: PlayerId,
        expiry: Expiry,
        effect_immunity: Option<crate::modifiers::EffectImmunityFilter>,
        payload: Option<crate::modifiers::ModifierPayload>,
        opponent_only: bool,
    ) {
        let source_kind = self.effect_source_kind_for_handle(source_card);
        self.add_floating_mass_modifier_scoped_from(filter, modifier, value, source_card,
            source_player, expiry, effect_immunity, payload, opponent_only, source_kind, None);
    }

    /// Install with the resolving effect's captured category and carrier.
    pub fn add_floating_mass_modifier_scoped_from(
        &mut self,
        filter: digimon_dsl::compiled::CompiledPredicate,
        modifier: ModifierType,
        value: i32,
        source_card: crate::card_source::CardHandle,
        source_player: PlayerId,
        expiry: Expiry,
        effect_immunity: Option<crate::modifiers::EffectImmunityFilter>,
        payload: Option<crate::modifiers::ModifierPayload>,
        opponent_only: bool,
        source_kind: crate::enums::EffectSourceKind,
        source_identity: Option<(PlayerId, u64)>,
    ) {
        let pending_skips =
            crate::modifiers::pending_skips_for_install(expiry, source_player, self.turn_player());
        self.floating_mass_modifiers
            .push(crate::floating_modifier::FloatingMassModifier {
                opponent_only,
                filter,
                modifier,
                value,
                source_card,
                source_player,
                source_kind,
                source_identity,
                expiry,
                pending_skips,
                effect_immunity,
                payload,
                keyword: None,
            });
        self.tick_declarative_effects();
    }

    /// Continuous mass KEYWORD grant (G-DSL-CONTINUOUS-MASS-KEYWORD-GRANT):
    /// every declarative tick grants `keyword` to each battle-area permanent
    /// matching `filter` (relative to `source_player`) until `expiry`, so a
    /// matching permanent that enters during the window is covered too.
    pub fn add_floating_mass_keyword(
        &mut self,
        filter: digimon_dsl::compiled::CompiledPredicate,
        keyword: crate::enums::Keyword,
        source_card: crate::card_source::CardHandle,
        source_player: PlayerId,
        expiry: Expiry,
    ) {
        let source_kind = self.effect_source_kind_for_handle(source_card);
        self.add_floating_mass_keyword_from(filter, keyword, source_card, source_player,
            expiry, source_kind, None);
    }

    /// Install with the resolving effect's captured category and carrier.
    pub fn add_floating_mass_keyword_from(
        &mut self,
        filter: digimon_dsl::compiled::CompiledPredicate,
        keyword: crate::enums::Keyword,
        source_card: crate::card_source::CardHandle,
        source_player: PlayerId,
        expiry: Expiry,
        source_kind: crate::enums::EffectSourceKind,
        source_identity: Option<(PlayerId, u64)>,
    ) {
        let pending_skips =
            crate::modifiers::pending_skips_for_install(expiry, source_player, self.turn_player());
        self.floating_mass_modifiers
            .push(crate::floating_modifier::FloatingMassModifier {
                opponent_only: false,
                filter,
                modifier: ModifierType::ChangeDp,
                value: 0,
                source_card,
                source_player,
                source_kind,
                source_identity,
                expiry,
                pending_skips,
                effect_immunity: None,
                payload: None,
                keyword: Some(keyword),
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
        self.shift_pending_attack_after_battle_area_remove(src.player, src.index);
        self.shift_effect_queue_after_battle_area_remove(src.player, src.index);

        // Linked cards lose their host → trash + OnLinkedCardTrashed observer.
        // Matches the linked-card flow in `trash_single_for_batch`
        // (combat.rs:3740-3768). Runs after the slot is gone so the observer
        // drain is safe.
        if had_linked {
            for card in linked_cards {
                self.player_mut(src.player).put_in_trash(card);
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

    /// "Add N to this Digimon's DP deletion effects' maximums" total for the
    /// battle-area permanent `host`: the sum of every self-scoped
    /// `Effect::dp_delete_effect_max_delta` in `host`'s card stack whose
    /// scope matches the card's position (inherited ⇔ under the top card) and
    /// whose condition (`active_when`) holds — read live, so a card that just
    /// became a digivolution source counts immediately — plus any
    /// materialized `ModifierType::ChangeDPDeleteEffectMaxDP` entries on
    /// `host` installed by other routes (filtered-target auras, steps).
    ///
    /// Added to the DP cap of every deletion effect whose source permanent is
    /// `host` (DCGO `Player.MaxDP_DeleteEffect` →
    /// `ChangeDPDeleteEffectMaxDPClass.ChangeMaxDP`). Non-battle-area handles
    /// (breeding sentinel, stale indices) yield 0.
    /// G-ENGINE-DP-DELETION-MAX-MODIFIER.
    pub fn dp_delete_effect_max_bonus(&self, host: PermanentHandle) -> i32 {
        use crate::effect_context::EffectReadContext;

        let materialized = self.modifiers.dp_delete_effect_max_delta(host);
        let Some(perm) = self
            .players
            .get(host.player as usize)
            .and_then(|p| p.battle_area.get(host.index as usize))
        else {
            return materialized;
        };
        let stack_size = perm.card_sources.len();
        let sources: Vec<(String, crate::card_source::CardHandle, bool)> = perm
            .card_sources
            .iter()
            .enumerate()
            .map(|(i, src)| {
                (
                    src.card_id(&self.card_data).to_string(),
                    src.handle(),
                    i + 1 < stack_size,
                )
            })
            .collect();
        let mut total = materialized;
        for (card_id, handle, inherited_source) in sources {
            let Some(effects) = self.effects_for_card(&card_id, handle) else {
                continue;
            };
            for effect in effects.iter() {
                let Some(delta) = effect.dp_delete_effect_max_delta else {
                    continue;
                };
                if !effect.declarative || effect.inherited != inherited_source {
                    continue;
                }
                let ctx = EffectReadContext::new(self, handle, Some(host), host.player);
                if let Some(condition) = &effect.condition {
                    if !condition(&ctx) {
                        continue;
                    }
                }
                total += delta;
            }
        }
        total
    }

    /// The read-time gate of `ModifierRegistry::get`, for a live formula
    /// aura of category `source_kind` controlled by `controller` reaching
    /// `target`: not hidden by the `<Progress>` shield (an opponent's effect
    /// on the shielded attacker, §16-39-1) and not covered by an immunity on
    /// `target` (§15-15-5-1).
    fn declarative_formula_can_affect(
        &self,
        target: crate::permanent::PermanentHandle,
        controller: PlayerId,
        source_kind: EffectSourceKind,
        source_identity: Option<(PlayerId, u64)>,
    ) -> bool {
        let shielded =
            self.modifiers.progress_shield() == Some(target) && controller != target.player;
        !shielded && !self.modifiers.is_unaffected_by_from(target, controller, source_kind, source_identity)
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
                    // §15-2-1: a top card's aura is an effect of its printed
                    // category; an inherited one is the Digimon's (§15-3-2).
                    let kind = if inherited_source {
                        EffectSourceKind::Digimon
                    } else {
                        EffectSourceKind::for_card_kind(self.card_data[source.data_index].card_kind)
                    };
                    sources.push((
                        source.card_id(&self.card_data).to_string(),
                        source.handle(),
                        Some(host),
                        player_id,
                        inherited_source,
                        false,
                        kind,
                    ));
                }
                // `<Succession>`: an adopted source's top-scope formula auras.
                for source_index in self.succession_source_indices(host) {
                    let source = &perm.card_sources[source_index];
                    sources.push((
                        source.card_id(&self.card_data).to_string(),
                        source.handle(),
                        Some(host),
                        player_id,
                        false,
                        true,
                        EffectSourceKind::Digimon,
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
                    false,
                    EffectSourceKind::for_card_kind(self.card_data[top.data_index].card_kind),
                ));
            }
        }

        let mut total = 0;
        let mut found = false;
        for (
            card_id,
            source_card,
            source_permanent,
            controller,
            inherited_source,
            adopted_copy,
            source_kind,
        ) in sources
        {
            // §15-15-5-1 / §16-39-1: a formula aura is an effect; while
            // `target` is unaffected by this controller's effects of this
            // category (a covering immunity, or the `<Progress>` shield on an
            // attacker against its opponent's effects) its value does not
            // reach `target`, and applies again the moment the gate lifts
            // (§15-15-5-2). The registry-level predicates are used (not
            // `effect_can_affect`, whose `<Progress>` half re-derives the
            // shield through keyword reads that can read DP) -- the shield the
            // registry holds IS the read-time truth `Game` keeps refreshed.
            // G-ENGINE-PROGRESS-SHIELD-BYPASSED-BY-TIMING-DISABLE-AND-FORMULA-AURAS.
            if !self.declarative_formula_can_affect(target, controller, source_kind, source_permanent.and_then(|h| self.permanent_effect_identity(h))) {
                continue;
            }
            let Some(effects) = self.effects_for_card(&card_id, source_card) else {
                continue;
            };
            for effect in effects.iter() {
                if !effect.declarative || effect.inherited != inherited_source {
                    continue;
                }
                if adopted_copy && !Game::is_adoptable_effect(effect) {
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
                    // DP-minus protection is per contribution, before summing:
                    // positive and own-scoped auras must still apply.
                    if !security_attack && value < 0 && self.dp_minus_is_blocked(target, controller) {
                        continue;
                    }
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
            // A link card's ESS is the host Digimon's effect (§4-3-5).
            if !self.declarative_formula_can_affect(target, controller, EffectSourceKind::Digimon, self.permanent_effect_identity(host)) {
                continue;
            }
            let Some(effects) = self.effects_for_card(&card_id, source_card) else {
                continue;
            };
            for effect in effects.iter() {
                // DigiLink Shape-B (G-LINK-INHERITED-ESS): a link card's
                // continuous DP / Security-Attack formula ESS reaches its HOST
                // under BOTH the `.linked()` (BT25-101) and `.inherited()`
                // conventions. A clause is `.linked()` XOR `.inherited()`, so
                // accepting either cannot double-count; the `card_sources`
                // walk above never touches `linked_cards`.
                if !effect.declarative || !(effect.linked || effect.inherited) {
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
                    // DP-minus protection is per contribution, before summing:
                    // positive and own-scoped auras must still apply.
                    if !security_attack && value < 0 && self.dp_minus_is_blocked(target, controller) {
                        continue;
                    }
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
    /// Memoized per-card effect derivation — the engine's hottest call
    /// (56–72% of bare-engine runtime; ~742–1715 calls/step). The registry
    /// effect list is a pure function of `(card_id, handle, under_top)`, so it
    /// is cached on `Game` for the game's lifetime (see `EffectsCache`) and a
    /// repeat query is a refcount bump rather than a fresh `impl_.effects(handle)`
    /// closure re-box. Returns a shared `Arc<Vec<Effect>>`; callers that must
    /// mutate the vec use `effects_for_card_owned`.
    pub fn effects_for_card(
        &self,
        card_id: &str,
        handle: crate::card_source::CardHandle,
    ) -> Option<std::sync::Arc<Vec<crate::effect::Effect>>> {
        let under_top = self.card_handle_is_under_top(handle);
        // Key on the canonical data_index (Copy, no per-call String alloc) rather
        // than the &str card_id. A handle maps 1:1 to a card instance, so this is
        // also unique; data_index canonicalizes any duplicate card_id rows.
        let Some(&data_index) = self.card_id_index.get(card_id) else {
            // Unknown card_id (defensive — shouldn't happen): build uncached.
            return self
                .build_effects_for_card(card_id, handle, under_top)
                .map(std::sync::Arc::new);
        };
        let key = (data_index, handle, under_top);

        if let Some(cached) = self.effects_cache.get(&key) {
            // Differential oracle (debug/test only): the cache is provably pure
            // in (card_id, handle, under_top), so a hit must equal a fresh build;
            // this catches a caller passing a card_id that doesn't match `handle`.
            #[cfg(debug_assertions)]
            {
                let fresh = self.build_effects_for_card(card_id, handle, under_top);
                let proj = |v: &[crate::effect::Effect]| -> Vec<String> {
                    v.iter().map(|e| format!("{e:?}")).collect()
                };
                debug_assert_eq!(
                    cached.as_ref().map(|a| proj(&a[..])),
                    fresh.as_ref().map(|v| proj(&v[..])),
                    "effects_for_card cache divergence: card_id={card_id} handle={handle:?} under_top={under_top}"
                );
            }
            return cached;
        }

        let built = self
            .build_effects_for_card(card_id, handle, under_top)
            .map(std::sync::Arc::new);
        self.effects_cache.insert(key, built.clone());
        built
    }

    /// Memoized `effect_registry.get(card_id).effects(handle)`: the registry
    /// list alone, WITHOUT the keyword auto-effects `effects_for_card` appends.
    /// For per-source OPT / DP queries (observation tensor) whose counts and
    /// slot keys are defined over the registry list.
    pub(crate) fn registry_effects(
        &self,
        card_id: &str,
        handle: crate::card_source::CardHandle,
    ) -> Option<std::sync::Arc<Vec<crate::effect::Effect>>> {
        let build = || {
            self.effect_registry
                .get(card_id)
                .map(|impl_| std::sync::Arc::new(impl_.effects(handle)))
        };
        let Some(&data_index) = self.card_id_index.get(card_id) else {
            return build();
        };
        let key = (data_index, handle);
        if let Some(cached) = self.effects_cache.1.borrow().get(&key) {
            return cached.clone();
        }
        let built = build();
        self.effects_cache.1.borrow_mut().insert(key, built.clone());
        built
    }

    /// Owned, un-memoized variant of [`Game::effects_for_card`] for the few
    /// callers that mutate the returned vec (`game_actions/cost.rs`,
    /// `game_actions/options.rs`). Rebuilds every call — acceptable because
    /// those sites are low-frequency (cost calc, not the per-step hot path).
    pub(crate) fn effects_for_card_owned(
        &self,
        card_id: &str,
        handle: crate::card_source::CardHandle,
    ) -> Option<Vec<crate::effect::Effect>> {
        let under_top = self.card_handle_is_under_top(handle);
        self.build_effects_for_card(card_id, handle, under_top)
    }

    /// Build the per-card effect list from the registry + keyword synthesis.
    /// Owned and un-memoized; `under_top` is threaded in so the memoized wrapper
    /// keys on it without this body recomputing `card_handle_is_under_top`.
    fn build_effects_for_card(
        &self,
        card_id: &str,
        handle: crate::card_source::CardHandle,
        under_top: bool,
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
        // FACE keywords only. `CardData::keywords` is parsed from ALL THREE
        // printed text fields (`CardData::load`), so reading it here put a
        // card's INHERITED keyword on its face: Gazimon (BT25-078) as the top
        // card carried a live <Retaliation> it does not print (DCGO registers
        // it `isInheritedEffect: true` only). `face_keywords` is the same
        // split `has_keyword` already uses; for a DSL-pack card with empty
        // text it returns `keywords` unchanged.
        let native_keywords = cd_opt.map(face_keywords).unwrap_or_default();
        let mut auto_effects: Vec<crate::effect::Effect> = cd_opt
            .map(|cd| {
                let mut effects: Vec<crate::effect::Effect> = native_keywords
                    .iter()
                    .flat_map(|kw| {
                        crate::cards::keyword_effects::keyword_to_auto_effect(*kw, handle)
                    })
                    .collect();

                // NOT gated on `under_top`. Every consumer already filters on
                // `Effect::inherited` (registry `scope: inherited` clauses and
                // grant-synthesized bodies are in the list unconditionally), so
                // the gate bought nothing -- and it made the list's SLOT LAYOUT
                // depend on where the card is. An [On Deletion] entry is queued
                // by `effect_slot` while the card is under the top and resolved
                // after the stack is in the trash (rule 25), where `under_top`
                // is false: the printed inherited keyword's slot fell off the
                // end of the shorter list and the trigger silently resolved to
                // nothing. A printed inherited <Retaliation> on a card with no
                // grant clause never deleted anything in a live game.
                let _ = under_top;
                {
                    for kw in inherited_keywords(cd) {
                        // De-dup against an unconditional `scope: inherited`
                        // `kind: grant_keyword` clause for the SAME keyword.
                        // The real game loads `inherited_text` from
                        // `cards.json`, so a DSL card whose inherited keyword
                        // is authored as a grant clause (BT25-078 Gazimon's
                        // inherited <Retaliation>) had the keyword's trigger
                        // body synthesized TWICE -- once here, once by the
                        // grant loop below. Both queued at the deletion, the
                        // pair opened a phantom TriggerOrder prompt, and the
                        // park outlived the battle the keyword needed. DCGO
                        // registers exactly one `RetaliationSelfEffect`
                        // (BT25_078.cs); 16-12-4 [3.6 numbering; deleted in 4.3 — see changelog-3.6-to-4.3.md] treats instances per
                        // printed keyword, and this card prints one. The
                        // embedded DSL pack leaves the text fields empty, which
                        // is why only live games / the exam ever saw this.
                        let covered_by_inherited_grant =
                            registry_effects.as_ref().is_some_and(|es| {
                                es.iter().any(|e| {
                                    e.declarative
                                        && e.inherited
                                        && e.condition.is_none()
                                        && e.granted_keyword == Some(kw)
                                })
                            });
                        if covered_by_inherited_grant {
                            continue;
                        }
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

                // Printed LINK keywords reach the host only while this card is
                // one of its link cards (4-3-5), so they synthesize as
                // `.linked()` auto-effects: the linked-card trigger and
                // replacement passes collect them, the top-card and inherited
                // scans skip them. De-duped against an unconditional
                // `scope: linked` grant clause for the same keyword, as the
                // loop above is against a `scope: inherited` one, so a DSL
                // card's link keyword fires once in a live game, as it does in
                // the embedded pack (whose text fields are empty).
                for kw in link_keywords(cd) {
                    let covered_by_linked_grant = registry_effects.as_ref().is_some_and(|es| {
                        es.iter().any(|e| {
                            e.declarative
                                && e.linked
                                && e.condition.is_none()
                                && e.granted_keyword == Some(kw)
                        })
                    });
                    if covered_by_linked_grant {
                        continue;
                    }
                    effects.extend(
                        crate::cards::keyword_effects::keyword_to_auto_effect(kw, handle)
                            .into_iter()
                            .map(|mut effect| {
                                effect.linked = true;
                                effect
                            }),
                    );
                }

                effects
            })
            .unwrap_or_default();

        // Declarative `grant_keyword` clauses are semantically equivalent to
        // printed keywords for keyword lookups. If the granted keyword carries
        // triggered or replacement behavior (Execute, Barrier, Armor Purge,
        // Scapegoat, etc.), also synthesize the keyword's auto-effects so
        // keyword grants participate in the triggered-effect enqueue and the
        // replacement scans.
        //
        // task_69f10a66 Family 1: CONDITIONAL grants (an `active_when` aura
        // like EX12-004 Onibimon's "[Your Turn] this [TB] Digimon gains
        // <Execute>") synthesize too — `ConditionFn` is `Arc`-shared, so the
        // grant's condition is cloned onto each synthesized auto-effect
        // (AND-composed with any condition the auto-effect already carries,
        // e.g. Armor Purge's stack-size gate). The condition is EVALUATED at
        // fire/collect time (`run_queued_effect_inner` / `collect_candidates`),
        // never here, so the built list stays a pure function of
        // `(card_id, handle, under_top)` and the `effects_for_card` memo (and
        // its debug differential oracle) remain valid. Without this, a
        // conditionally granted trigger keyword installed NO body anywhere:
        // granted <Execute> never fired at end of turn and the printed "may
        // attack" choice never reached the action space (rule-17 violation —
        // §16-38-2/-3, §15-9-2-2).
        if let Some(es) = registry_effects.as_ref() {
            for grant in es {
                let Some(kw) = grant.granted_keyword else {
                    continue;
                };
                if !grant.declarative {
                    continue;
                }
                // A face grant of a keyword the card prints on its face is
                // redundant; an inherited or link grant reaches another Digimon.
                if !grant.inherited && !grant.linked && native_keywords.contains(&kw) {
                    continue;
                }
                let grant_condition = grant.condition.clone();
                auto_effects.extend(
                    crate::cards::keyword_effects::keyword_to_auto_effect(kw, handle)
                        .into_iter()
                        .map(|mut effect| {
                            effect.inherited = grant.inherited;
                            // A `scope: linked` grant (Link-ESS) fires for the
                            // host through the linked-card scans, never as the
                            // card's own face effect (4-3-5): BT24-067
                            // Hackmon's link-box <Retaliation>.
                            effect.linked = grant.linked;
                            if let Some(gc) = grant_condition.clone() {
                                effect.condition = Some(match effect.condition.take() {
                                    Some(own) => std::sync::Arc::new(
                                        move |rctx: &crate::effect_context::EffectReadContext| {
                                            gc(rctx) && own(rctx)
                                        },
                                    ),
                                    None => gc,
                                });
                            }
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
            let Some(effects) =
                self.registry_effects(source.card_id(&self.card_data), source.handle())
            else {
                continue;
            };
            let counts = Self::source_opt_counts(permanent, source.handle(), is_under, &effects);
            total += counts.0;
            used += counts.1;
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
            link_text: String::new(),
            requirement_text: String::new(),
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
        g.replacement_commit_fired = true;
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
        assert!(!g.replacement_commit_fired);
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
            link_text: String::new(),
            requirement_text: String::new(),
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
            effective_target_removed: false,
            is_blocked: false,
            blocker: None,
            is_vortex: false,
            is_overclock: false,
            declaration_committed: true,
            cancelled: false,
            battle_occurred: false,
            battle_defender_deleted: false,
            effect_battle_piercing_pending: false,
            security_check_started: false,
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
            effective_target_removed: false,
            is_blocked: false,
            blocker: None,
            is_vortex: false,
            is_overclock: false,
            declaration_committed: true,
            cancelled: false,
            battle_occurred: false,
            battle_defender_deleted: false,
            effect_battle_piercing_pending: false,
            security_check_started: false,
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
            link_text: String::new(),
            requirement_text: String::new(),
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
        r.game.players[0].put_in_trash(veemon_source);

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
            r.game
                .resolve_token_as_battle_area_top(ProvenanceToken(99_999)),
            None
        );
        assert_eq!(
            r.game
                .resolve_token_as_battle_area_top(ProvenanceToken(12345)),
            None,
            "unknown card index yields None"
        );
    }
}
