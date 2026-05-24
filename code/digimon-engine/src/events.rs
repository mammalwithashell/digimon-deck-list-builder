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
    Play {
        seq: u64,
        player: PlayerId,
        card_id: String,
        field_index: u8,
    },

    /// `player` digivolved `top` onto a permanent at `field_index`.
    /// Emitted by the normal battle-area `Game::digivolve_from_hand` path.
    Digivolve {
        seq: u64,
        player: PlayerId,
        top_card_id: String,
        field_index: u8,
        from_stack_top: String,
    },

    /// A Digimon declared an attack.
    /// (Variant defined for future wiring — not emitted yet.)
    Attack {
        seq: u64,
        player: PlayerId,
        attacker_field_index: u8,
        target_field_index: Option<u8>,
        target_player: Option<PlayerId>,
    },

    /// A card was moved to trash from some zone.
    /// (Variant defined for future wiring — not emitted yet.)
    Trash {
        seq: u64,
        player: PlayerId,
        card_id: String,
    },

    /// A card was milled (deck→trash from the top of the deck).
    /// (Variant defined for future wiring — not emitted yet.)
    Mill {
        seq: u64,
        player: PlayerId,
        card_id: String,
    },

    /// A security card was revealed during a security check.
    /// (Variant defined for future wiring — not emitted yet.)
    SecurityReveal {
        seq: u64,
        defender: PlayerId,
        card_id: String,
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
            GameEvent::GameOver { .. } => "GameOver",
            GameEvent::Concede { .. } => "Concede",
            GameEvent::EffectFizzled { .. } => "EffectFizzled",
        }
    }
}
