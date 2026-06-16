//! Structured game events emitted during action resolution. Mirrors
//! Python's `digimon_gym/engine/events.py::GameEvent` — a tagged enum
//! consumed by UI animation and replay layers.
//!
//! Emission coverage is currently partial: `MemoryChange`, `Play`, and
//! `GameOver` are wired in by this module's initial landing.
//! `TurnStart`, `PhaseChange`, `Digivolve`, `Attack`, `Trash`, `Mill`,
//! and `SecurityReveal` variants exist on the enum and will be emitted
//! as game-phase and card-migration work wires the corresponding paths.
//!
//! Every event carries a monotonically increasing `seq` allocated by
//! `Game::next_event_seq`. Consumers drain the buffer via
//! `Game::drain_events` (the runner does this around each `step`).

use crate::enums::{GamePhase, PlayerId};
use crate::game::TerminalOutcomeReason;
use crate::permanent::PermanentHandle;

/// A card referenced by an event, carrying both its canonical id and its
/// printed name. Used by events that name one or more cards so log/UI
/// consumers render `[CARD-ID: Name]` without reconstructing the name from
/// (possibly-mutated) board state.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EventCardRef {
    pub card_id: String,
    pub card_name: String,
}

/// Which non-security reveal site produced a [`GameEvent::Reveal`]. Mirrors
/// the DCGO recorder reveal chokepoints (CLAUDE.md rule 27); `SecurityReveal`
/// remains a distinct variant for the security-check path.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum RevealZone {
    /// Top of the main deck revealed by an effect (peek/reveal).
    DeckTop,
    /// Card revealed as it is trashed from the top of the deck (mill).
    TrashFromDeckTop,
    // NOTE: a `Hand` reveal zone is intentionally absent — the engine has no
    // reveal-from-hand primitive and no card in the pool reveals from hand.
    // Re-add a variant (+ emission site) if such a card is ever implemented.
}

/// Tagged event payload. `#[non_exhaustive]` on each variant would force
/// Python consumers to pattern-match defensively forever; we prefer the
/// Rust enum itself `#[non_exhaustive]` instead so new variants can be
/// added without breaking downstream matches.
///
/// Serialized form (via `serde`): each variant is tagged with `type` set
/// to the variant name (matching [`GameEvent::type_str`]) and the
/// variant's fields appear as siblings at the top level. Example:
/// `{"type": "MemoryChange", "seq": 0, "player": 0, "delta": -3, "total": -3}`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum GameEvent {
    /// `memory` just changed by `delta`. Emitted by `Game::gain_memory`,
    /// `pay_memory`, and `set_memory`. `delta` is signed and may be zero
    /// (set_memory can be a no-op; callers can filter if they care).
    MemoryChange {
        seq: u64,
        player: PlayerId,
        delta: i16,
        total: i16,
        /// When the change originates from a card effect (routed through
        /// `EffectContext::gain_memory` / `lose_memory`), the effect's
        /// source card id. `None` for cost payment, turn-pass, and other
        /// structural memory changes with no card source.
        source_card_id: Option<String>,
        /// Printed name paired with `source_card_id` (same `Option` arm).
        source_card_name: Option<String>,
    },

    /// A new turn has started. Emitted after `turn_count` bumps.
    TurnStart {
        seq: u64,
        player: PlayerId,
        turn_count: u16,
    },

    /// The current phase has changed to `phase`.
    PhaseChange {
        seq: u64,
        player: PlayerId,
        phase: GamePhase,
    },

    /// A card entered the battle area from hand.
    ///
    /// `cost_paid` is the actual memory paid AFTER all cost reductions
    /// (tamer-driven, alt-path discounts, etc.). May be zero (or even
    /// negative if a future alt-path grants beyond-free play).
    /// `cost_printed` is the card's printed `play_cost` from `CardData`
    /// at the moment of play — captured here so downstream consumers
    /// (reward components, replay viewers) do not need to re-read
    /// CardData.
    /// `via_alt_path` is the canonical key from
    /// `digimon_dsl::compiled::CompiledAltPathKind::as_key()` when the
    /// card was played through a registered alt-path that bypassed the
    /// printed cost, otherwise `None` (standard PLAY action, with or
    /// without a generic memory cost reduction).
    Play {
        seq: u64,
        player: PlayerId,
        card_id: String,
        /// Printed name of the played card, captured at emission from
        /// `CardData` so log/UI consumers need not reconstruct it.
        card_name: String,
        field_index: u8,
        cost_paid: i16,
        cost_printed: i16,
        via_alt_path: Option<String>,
    },

    /// `player` digivolved `top` onto a permanent at `field_index`.
    /// Emitted by the normal battle-area `Game::digivolve_from_hand` path.
    ///
    /// `was_dna` is `true` for any DNA-style digivolve — the standard
    /// `dna_costs` path, registered end-of-turn DNA alt-paths, Blast DNA,
    /// and xros_req-driven DNA. `false` for normal evo-cost digivolves
    /// and for effect-initiated free digivolves.
    /// `was_blast_dna` is the narrower flag: `true` only when the path
    /// is `CompiledAltPathKind::BlastDnaDigivolve`. Implies `was_dna`.
    /// `memory_paid` is the actual memory cost paid for this digivolve.
    /// May be zero (Blast DNA at cost 0, xros_req at cost 0,
    /// effect-initiated free digivolves) or the printed evo / DNA cost.
    Digivolve {
        seq: u64,
        player: PlayerId,
        top_card_id: String,
        /// Printed name of the card digivolved into (`top_card_id`),
        /// captured at emission from `CardData`.
        card_name: String,
        field_index: u8,
        from_stack_top: String,
        was_dna: bool,
        was_blast_dna: bool,
        memory_paid: i16,
    },

    /// A Digimon declared an attack.
    Attack {
        seq: u64,
        player: PlayerId,
        attacker_field_index: u8,
        target_field_index: Option<u8>,
        target_player: Option<PlayerId>,
        /// Id + name of the attacking permanent's top card, read from
        /// `CardData` at declaration. Non-optional — an attacker always
        /// exists.
        attacker_card_id: String,
        attacker_card_name: String,
        /// Id + name of the defending Digimon's top card when the attack
        /// targets a Digimon (`target_field_index.is_some()`); `None` when
        /// the attack targets the security stack.
        target_card_id: Option<String>,
        target_card_name: Option<String>,
        /// Effective DP (incl. modifiers) of the attacker at declaration,
        /// via `Game::effective_dp`. `None` only if the attacker has no DP.
        attacker_dp: Option<i32>,
        /// Effective DP of the defending Digimon; `None` for a security-stack
        /// attack (no Digimon target).
        target_dp: Option<i32>,
    },

    /// A card was moved to trash from some zone.
    Trash {
        seq: u64,
        player: PlayerId,
        card_id: String,
        /// Printed name of the trashed card, captured at emission.
        card_name: String,
    },

    /// A card was milled (deck→trash from the top of the deck).
    Mill {
        seq: u64,
        player: PlayerId,
        card_id: String,
        /// Printed name of the milled card, captured at emission.
        card_name: String,
    },

    /// A security card was revealed during a security check.
    SecurityReveal {
        seq: u64,
        defender: PlayerId,
        card_id: String,
        /// Printed name of the revealed security card, captured at emission.
        card_name: String,
    },

    /// A card effect committed a target selection. Carries the effect's
    /// source card and the chosen target card(s). Emitted at the selection
    /// commit choke point for every card-bearing selection kind, including
    /// forced single-target picks (no-approximations policy). Non-card
    /// selections (effect-choice menus, trigger-order, play-order) do not
    /// emit this event.
    EffectTarget {
        seq: u64,
        player: PlayerId,
        source_card_id: String,
        source_card_name: String,
        targets: Vec<EventCardRef>,
    },

    /// A card was revealed at a non-security reveal site (see [`RevealZone`]).
    /// One event fires per revealed card, in reveal order.
    Reveal {
        seq: u64,
        player: PlayerId,
        card_id: String,
        card_name: String,
        source_zone: RevealZone,
    },

    /// The game ended. `winner` is `None` on a draw.
    GameOver {
        seq: u64,
        winner: Option<PlayerId>,
        reason: TerminalOutcomeReason,
    },

    /// A player conceded the game. Emitted **before** the subsequent
    /// `GameOver` event so listeners can observe the concede before the
    /// terminal-outcome notification. `player` is the conceding player
    /// (i.e., the loser); the match-level winner is the opponent and
    /// will be carried by the immediately-following `GameOver` event
    /// with `reason = TerminalOutcomeReason::Concede`.
    Concede {
        seq: u64,
        player: PlayerId,
    },

    /// A mandatory pending selection was discarded because no executable
    /// option remained. Emitted in two situations:
    /// 1. **Install-time fizzle** — `install_field_selection` (and other
    ///    selection helpers) found zero targets passing the filter. The
    ///    `source_permanent` identifies the effect source if known.
    /// 2. **Execute-time fizzle (LiveGame safety net)** — `LiveGame::step`
    ///    detected that the only option for a mandatory pending selection
    ///    produced no state change and no events, so the wrapper clears
    ///    the pending and emits this event rather than leave the caller
    ///    stuck.
    ///
    /// `reason` is a short human-readable phrase (e.g., `"no valid target"`
    /// at install, `"no executable target"` at execute).
    EffectFizzled {
        seq: u64,
        source_permanent: Option<PermanentHandle>,
        reason: String,
    },
}

impl GameEvent {
    /// Monotonic sequence number allocated at emission time.
    pub fn seq(&self) -> u64 {
        match self {
            GameEvent::MemoryChange { seq, .. }
            | GameEvent::TurnStart { seq, .. }
            | GameEvent::PhaseChange { seq, .. }
            | GameEvent::Play { seq, .. }
            | GameEvent::Digivolve { seq, .. }
            | GameEvent::Attack { seq, .. }
            | GameEvent::Trash { seq, .. }
            | GameEvent::Mill { seq, .. }
            | GameEvent::SecurityReveal { seq, .. }
            | GameEvent::EffectTarget { seq, .. }
            | GameEvent::Reveal { seq, .. }
            | GameEvent::GameOver { seq, .. }
            | GameEvent::Concede { seq, .. }
            | GameEvent::EffectFizzled { seq, .. } => *seq,
        }
    }

    /// Stable string type name. Matches the `type` tag in the serialized
    /// form (`#[serde(tag = "type")]`) and Python `GameEvent.type` for
    /// the legacy variants.
    pub fn type_str(&self) -> &'static str {
        match self {
            GameEvent::MemoryChange { .. } => "MemoryChange",
            GameEvent::TurnStart { .. } => "TurnStart",
            GameEvent::PhaseChange { .. } => "PhaseChange",
            GameEvent::Play { .. } => "Play",
            GameEvent::Digivolve { .. } => "Digivolve",
            GameEvent::Attack { .. } => "Attack",
            GameEvent::Trash { .. } => "Trash",
            GameEvent::Mill { .. } => "Mill",
            GameEvent::SecurityReveal { .. } => "SecurityReveal",
            GameEvent::EffectTarget { .. } => "EffectTarget",
            GameEvent::Reveal { .. } => "Reveal",
            GameEvent::GameOver { .. } => "GameOver",
            GameEvent::Concede { .. } => "Concede",
            GameEvent::EffectFizzled { .. } => "EffectFizzled",
        }
    }
}
