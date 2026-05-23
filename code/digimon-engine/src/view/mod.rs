//! Debug / tooling view layer.
//!
//! Provides `serde::Serialize`-derived, self-contained views over `Game`
//! state for the `digimon-engine-cli` and `digimon-engine-mcp` consumers.
//!
//! These views are intentionally distinct from:
//! - `serialization::to_ui_json` (frontend-shaped, Python-1/2 player IDs,
//!   lossy on modifier detail)
//! - `selection::PendingSelectionView` (FFI-shaped, no Serialize, raw
//!   action IDs without labels)
//!
//! Each view is a POD struct owning its data. Construct via
//! `View::from_game(&game, perspective)` — no borrowed references in the
//! struct itself, so the JSON wire shape is stable.

use serde::Serialize;

use crate::card_data::CardData;
use crate::enums::{CardColor, CardKind, EffectSourceKind, GamePhase, PlayerId};
use crate::events::GameEvent;
use crate::game::{Game, TerminalOutcomeReason};
use crate::modifiers::ModifierEntry;
use crate::permanent::{Permanent, PermanentHandle};
use crate::player::Player;
use crate::selection::{EffectChoiceEntry, PendingSelection, QueuedEffect, SelectionKind};

mod tests;

/// Selects which player's perspective filters opponent-hidden information.
///
/// - `Player(p)` redacts opponent hand card IDs, opponent security card IDs,
///   and any other zone the rules treat as hidden to `p`.
/// - `God` exposes everything — the unfiltered debugging view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Perspective {
    Player(PlayerId),
    God,
}

impl Perspective {
    /// True when `viewer` should see the contents of zones owned by `zone_owner`.
    /// God sees all; otherwise only the viewing player sees their own private zones.
    pub fn sees_private(self, zone_owner: PlayerId) -> bool {
        match self {
            Perspective::God => true,
            Perspective::Player(viewer) => viewer == zone_owner,
        }
    }
}

// ── StateView ────────────────────────────────────────────────────────────

/// Top-level game state view — phase, turn, memory, terminal outcome.
/// Phase 1.2 of the engine-debug-mcp change.
#[derive(Debug, Clone, Serialize)]
pub struct StateView {
    pub phase: String,
    pub turn_count: u16,
    pub memory: i16,
    pub memory_pair: (PlayerId, PlayerId),
    pub turn_player: PlayerId,
    pub game_over: bool,
    pub winner: Option<PlayerId>,
    pub terminal_outcome_reason: Option<String>,
    /// Most recent event sequence number — useful for tools that want to
    /// know "anything new since I last asked?"
    pub event_seq: u64,
}

impl StateView {
    pub fn from_game(game: &Game, _perspective: Perspective) -> Self {
        Self {
            phase: format!("{:?}", game.current_phase),
            turn_count: game.turn_count,
            memory: game.memory,
            memory_pair: game.memory_pair,
            turn_player: game.turn_player(),
            game_over: game.game_over,
            winner: game.winner,
            terminal_outcome_reason: game.terminal_outcome_reason.map(|r| r.as_str().to_string()),
            event_seq: game.event_seq,
        }
    }
}

// ── HandView ─────────────────────────────────────────────────────────────

/// A single card visible to the perspective.
#[derive(Debug, Clone, Serialize)]
pub struct HandCardEntry {
    pub hand_index: usize,
    pub card_id: String,
    pub card_name: String,
    pub kind: String,
    pub play_cost: u16,
}

/// Hand contents from a chosen perspective.
/// God / owner: full card list. Other players: count only.
#[derive(Debug, Clone, Serialize)]
pub struct HandView {
    pub player: PlayerId,
    pub count: usize,
    /// Populated only when the perspective is allowed to see the hand.
    /// `None` means the viewer sees the count but not the cards.
    pub cards: Option<Vec<HandCardEntry>>,
}

impl HandView {
    pub fn from_game(game: &Game, player: PlayerId, perspective: Perspective) -> Self {
        let p = game.player(player);
        let count = p.hand.len();
        let cards = if perspective.sees_private(player) {
            Some(
                p.hand
                    .iter()
                    .enumerate()
                    .map(|(idx, cs)| {
                        let cd = &game.card_data[cs.data_index];
                        HandCardEntry {
                            hand_index: idx,
                            card_id: cd.card_id.clone(),
                            card_name: cd.card_name.clone(),
                            kind: format!("{:?}", cd.card_kind),
                            play_cost: cd.play_cost,
                        }
                    })
                    .collect(),
            )
        } else {
            None
        };
        Self {
            player,
            count,
            cards,
        }
    }
}

// ── SecurityView ─────────────────────────────────────────────────────────

/// Security stack from a chosen perspective.
/// God: full card_id list (bottom-up: index 0 = bottom). Player view of own
/// security: count only (the rules don't let a player look at their own
/// security stack). Opponent: count only.
#[derive(Debug, Clone, Serialize)]
pub struct SecurityView {
    pub player: PlayerId,
    pub count: usize,
    /// God view only. Bottom-up ordering matches engine internals.
    pub card_ids: Option<Vec<String>>,
}

impl SecurityView {
    pub fn from_game(game: &Game, player: PlayerId, perspective: Perspective) -> Self {
        let p = game.player(player);
        let count = p.security.len();
        let card_ids = match perspective {
            Perspective::God => Some(
                p.security
                    .iter()
                    .map(|cs| game.card_data[cs.data_index].card_id.clone())
                    .collect(),
            ),
            Perspective::Player(_) => None,
        };
        Self {
            player,
            count,
            card_ids,
        }
    }
}

// ── ModifierSummary + FieldView ──────────────────────────────────────────

/// Compact, JSON-friendly modifier summary. Drops closure fields
/// (`replacement_condition`, `until_condition`, etc.) that aren't
/// serializable; downstream debuggers see the type + value + expiry.
#[derive(Debug, Clone, Serialize)]
pub struct ModifierSummary {
    pub modifier_type: String,
    pub value: i32,
    pub expiry: String,
    pub source_permanent: Option<PermanentHandle>,
    pub source_player: PlayerId,
    pub immunity_source_kind: Option<String>,
    pub install_order: u64,
}

impl ModifierSummary {
    pub fn from_entry(entry: &ModifierEntry) -> Self {
        // ModifierEntry has no top-level source_kind; the closest field is on
        // its effect_immunity_filter (for CannotBeAffected-style gates).
        let immunity_source_kind = entry
            .effect_immunity_filter
            .and_then(|f| f.source_kind)
            .map(|k| format!("{:?}", k));
        Self {
            modifier_type: format!("{:?}", entry.modifier),
            value: entry.value,
            expiry: format!("{:?}", entry.expiry),
            source_permanent: entry.source_permanent,
            source_player: entry.source_player,
            immunity_source_kind,
            install_order: entry.install_order,
        }
    }
}

/// Per-permanent debug view.
#[derive(Debug, Clone, Serialize)]
pub struct PermanentView {
    pub handle: PermanentHandle,
    pub top_card_id: String,
    pub top_card_name: String,
    pub kind: String,
    pub level: Option<u8>,
    pub dp: Option<i32>,
    pub colors: Vec<String>,
    /// Whole digivolution stack bottom-up. Index 0 = bottom source; last = top.
    pub stack_card_ids: Vec<String>,
    pub linked_card_ids: Vec<String>,
    pub is_suspended: bool,
    pub turn_played: u16,
    pub turn_digivolved: u16,
    pub attacks_this_turn: u8,
    pub is_attacking: bool,
    pub modifiers: Vec<ModifierSummary>,
}

impl PermanentView {
    fn from_permanent(game: &Game, handle: PermanentHandle, perm: &Permanent) -> Self {
        let data = &game.card_data;
        let top = perm
            .card_sources
            .last()
            .expect("permanent has at least one card source");
        let top_cd = &data[top.data_index];
        let stack_card_ids = perm
            .card_sources
            .iter()
            .map(|cs| data[cs.data_index].card_id.clone())
            .collect();
        let linked_card_ids = perm
            .linked_cards
            .iter()
            .map(|cs| data[cs.data_index].card_id.clone())
            .collect();
        let modifiers: Vec<ModifierSummary> = game
            .modifiers
            .permanent_modifiers_iter(handle)
            .map(ModifierSummary::from_entry)
            .collect();
        let colors: Vec<String> = top_cd
            .colors
            .iter()
            .map(|c: &CardColor| format!("{:?}", c))
            .collect();
        Self {
            handle,
            top_card_id: top_cd.card_id.clone(),
            top_card_name: top_cd.card_name.clone(),
            kind: format!("{:?}", top_cd.card_kind),

            level: top_cd.level,
            dp: top_cd.dp,
            colors,
            stack_card_ids,
            linked_card_ids,
            is_suspended: perm.is_suspended,
            turn_played: perm.turn_played,
            turn_digivolved: perm.turn_digivolved,
            attacks_this_turn: perm.attacks_this_turn,
            is_attacking: perm.is_attacking,
            modifiers,
        }
    }
}

/// Battle area + breeding area view for one player.
#[derive(Debug, Clone, Serialize)]
pub struct FieldView {
    pub player: PlayerId,
    pub battle_area: Vec<PermanentView>,
    /// Breeding area is public to both players per game rules.
    pub breeding: Vec<PermanentView>,
    pub trash_count: usize,
    pub deck_count: usize,
    pub egg_deck_count: usize,
}

impl FieldView {
    pub fn from_game(game: &Game, player: PlayerId, _perspective: Perspective) -> Self {
        let p = game.player(player);
        let battle_area: Vec<PermanentView> = p
            .battle_area
            .iter()
            .enumerate()
            .map(|(idx, perm)| {
                let handle = PermanentHandle {
                    player,
                    index: idx as u8,
                };
                PermanentView::from_permanent(game, handle, perm)
            })
            .collect();
        let breeding: Vec<PermanentView> = breeding_permanents(p)
            .into_iter()
            .enumerate()
            .map(|(idx, perm)| {
                let handle = PermanentHandle {
                    player,
                    // Breeding handles are NOT real battle-area indices; we
                    // expose 255-idx as a sentinel so consumers don't mistake
                    // them for battle-area targets.
                    index: u8::MAX - idx as u8,
                };
                PermanentView::from_permanent(game, handle, perm)
            })
            .collect();
        Self {
            player,
            battle_area,
            breeding,
            trash_count: p.trash.len(),
            deck_count: p.deck.len(),
            egg_deck_count: egg_deck_len(p),
        }
    }
}

fn breeding_permanents(player: &Player) -> Vec<&Permanent> {
    // `Player::breeding_area` is `Option<Permanent>` — at most one digi-egg
    // chick lives there. Collect into a slice-like Vec to keep the FieldView
    // shape uniform with the multi-permanent `battle_area`.
    player.breeding_area.iter().collect()
}

fn egg_deck_len(player: &Player) -> usize {
    player.digitama_deck.len()
}

// ── PendingSelectionView (decoded variant) ───────────────────────────────

/// A single legal option in a pending selection, decoded into a label.
#[derive(Debug, Clone, Serialize)]
pub struct PendingSelectionOption {
    pub action_id: u16,
    pub label: String,
}

/// Decoded, serde-friendly view of `PendingSelection`. Distinct from
/// `selection::PendingSelectionView` (FFI-shaped, no labels, no serde).
///
/// Carries human-readable option labels so an agent reading the view can
/// reason about choices without re-implementing the action decoder.
#[derive(Debug, Clone, Serialize)]
pub struct PendingSelectionDebugView {
    pub kind: String,
    pub selecting_player: PlayerId,
    pub previous_phase: String,
    pub is_optional: bool,
    pub prompt: String,
    pub source_card_id: Option<String>,
    pub source_permanent: Option<PermanentHandle>,
    pub source_kind: String,
    pub options: Vec<PendingSelectionOption>,
    /// Effect-choice menu entries when `kind == EffectChoice`. None for other
    /// selection kinds.
    pub effect_choices: Option<Vec<EffectChoiceSummary>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectChoiceSummary {
    pub label: String,
    pub action_id: u16,
}

impl PendingSelectionDebugView {
    pub fn from_pending(game: &Game, pending: &PendingSelection) -> Self {
        // Resolving CardHandle -> card_id directly requires a zone scan;
        // for v1 we leave the top-level source_card_id None and let
        // consumers cross-reference via source_permanent + FieldView.
        let source_card_id = None;
        let options: Vec<PendingSelectionOption> = pending
            .valid_action_ids
            .iter()
            .map(|&action_id| {
                let explanation = crate::action::explain::explain_action(
                    game,
                    pending.selecting_player,
                    action_id,
                );
                PendingSelectionOption {
                    action_id,
                    label: explanation.label,
                }
            })
            .collect();
        let effect_choices = pending
            .effect_choices
            .as_ref()
            .map(|entries| entries.iter().map(effect_choice_summary).collect());
        Self {
            kind: format!("{:?}", pending.kind),
            selecting_player: pending.selecting_player,
            previous_phase: format!("{:?}", pending.previous_phase),
            is_optional: pending.is_optional,
            prompt: pending.prompt.clone(),
            source_card_id,
            source_permanent: pending.source_permanent,
            source_kind: format!("{:?}", pending.source_kind),
            options,
            effect_choices,
        }
    }
}

fn effect_choice_summary(entry: &EffectChoiceEntry) -> EffectChoiceSummary {
    EffectChoiceSummary {
        label: entry.label.clone(),
        action_id: entry.action_id,
    }
}

// ── EffectQueueView ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct EffectQueueEntry {
    pub source_card_id: String,
    pub source_kind: String,
    pub timing: String,
    pub controller: PlayerId,
    pub is_optional: bool,
    pub effect_slot: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectQueueView {
    pub pending: Vec<EffectQueueEntry>,
}

impl EffectQueueView {
    pub fn from_game(game: &Game) -> Self {
        let pending: Vec<EffectQueueEntry> =
            game.effect_queue.iter().map(queue_entry_summary).collect();
        Self { pending }
    }
}

fn queue_entry_summary(qe: &QueuedEffect) -> EffectQueueEntry {
    EffectQueueEntry {
        source_card_id: qe.card_id.clone(),
        source_kind: format!("{:?}", qe.source_kind),
        timing: format!("{:?}", qe.timing),
        controller: qe.controller,
        is_optional: qe.is_optional,
        effect_slot: qe.effect_slot,
    }
}

// ── ModifierView (per-permanent) ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ModifierView {
    pub handle: PermanentHandle,
    pub modifiers: Vec<ModifierSummary>,
}

impl ModifierView {
    pub fn from_game(game: &Game, handle: PermanentHandle) -> Self {
        let modifiers = game
            .modifiers
            .permanent_modifiers_iter(handle)
            .map(ModifierSummary::from_entry)
            .collect();
        Self { handle, modifiers }
    }
}

// ── EventLogView ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct EventLogView {
    pub events: Vec<EventEntry>,
    pub last_seq: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventEntry {
    /// Debug representation of the GameEvent variant. Full structured
    /// serialization of `GameEvent` is out of scope for v1 — debug
    /// strings keep the view stable while GameEvent itself evolves.
    pub event: String,
}

impl EventLogView {
    /// Return events with `seq > since_seq`. When `since_seq` is `None`,
    /// returns all events in the buffer.
    pub fn from_game(game: &Game, since_seq: Option<u64>) -> Self {
        // Event sequence numbers are monotonic per-game; engine tracks
        // event_seq globally. The events Vec contains all retained events;
        // we filter by index since each push corresponds to one seq tick.
        let total_emitted = game.event_seq;
        let retained = game.events.len() as u64;
        let first_retained_seq = total_emitted.saturating_sub(retained);
        let cutoff = since_seq.unwrap_or(first_retained_seq);
        let events = game
            .events
            .iter()
            .enumerate()
            .filter_map(|(idx, ev)| {
                let seq = first_retained_seq + idx as u64 + 1;
                if seq > cutoff {
                    Some(EventEntry {
                        event: format!("{:?}", ev),
                    })
                } else {
                    None
                }
            })
            .collect();
        Self {
            events,
            last_seq: total_emitted,
        }
    }
}

// ── Unused-import suppressors (referenced by future tasks) ──────────────
//
// `CardData`, `CardKind`, `EffectSourceKind`, `GamePhase`, `GameEvent`,
// `SelectionKind`, `TerminalOutcomeReason` may be unused in the v1 view
// surface but are anticipated by the spec; the `let _ = ...` lines keep
// the imports honest while leaving them available for follow-up tasks.

#[allow(dead_code)]
fn _import_witnesses() {
    let _ = std::any::type_name::<CardData>();
    let _ = std::any::type_name::<CardKind>();
    let _ = std::any::type_name::<EffectSourceKind>();
    let _ = std::any::type_name::<GamePhase>();
    let _ = std::any::type_name::<GameEvent>();
    let _ = std::any::type_name::<SelectionKind>();
    let _ = std::any::type_name::<TerminalOutcomeReason>();
}
