//! UI-state serialization — builds a `serde_json::Value` tree that
//! matches the dict shape emitted by Python's
//! `digimon_gym/engine/game/serialization.py::to_ui_json`.
//!
//! Consumed by the PyO3 layer (PyDict conversion) and, transitively,
//! by `state_filter.py` and the React frontend.
//!
//! Player IDs are translated to the Python 1/2 convention at this layer
//! so downstream consumers don't need to know about the Rust 0-indexed
//! internal convention.

use serde_json::{json, Map, Value};

use crate::action::space::SECURITY_TARGET;
use crate::card_data::CardData;
use crate::enums::{CardColor, CardKind, GamePhase};
use crate::game::Game;
use crate::permanent::Permanent;
use crate::player::Player;
use crate::selection::{AttackTarget, PendingAttack, PendingSelectionView};

/// Translate a Rust `PlayerId` (0 / 1) into the Python 1 / 2 convention.
fn py_pid(rust_pid: u8) -> i64 {
    (rust_pid as i64) + 1
}

/// Map a Rust `GamePhase` variant to Python's `GamePhase.value` integer.
///
/// Python's `GamePhase` enum (`digimon_gym/engine/data/enums.py`):
///   Start=0, Draw=1, Breeding=2, Main=3, End=4,
///   SelectTarget=5, SelectMaterial=6, BlockTiming=7, CounterTiming=8,
///   SelectTrash=9, SelectSource=10, SelectHand=11, SelectReveal=12,
///   SelectEffectChoice=13, SelectSecurity=14, EndOfTurnAction=15,
///   AllianceTiming=16, Mulligan=17
///
/// Rust-only variants with no direct Python equivalent:
///   Unsuspend → treated as Start (0); an Unsuspend phase is never serialised
///               in practice because Rust auto-resolves it without pausing.
///   GameOver  → reused from Python's End (4) since the game is over either way.
fn phase_int(p: GamePhase) -> i64 {
    match p {
        GamePhase::Mulligan => 17,
        GamePhase::Unsuspend => 0, // no Python equivalent; Start=0 is nearest
        GamePhase::Draw => 1,
        GamePhase::Breeding => 2,
        GamePhase::Main => 3,
        GamePhase::EndTurn => 4,
        GamePhase::SelectTarget => 5,
        GamePhase::SelectMaterial => 6,
        GamePhase::SelectTrash => 9,
        GamePhase::SelectSource => 10,
        GamePhase::SelectHand => 11,
        GamePhase::SelectReveal => 12,
        GamePhase::SelectSecurity => 14,
        GamePhase::EffectChoice => 13,
        GamePhase::BlockTiming => 7,
        GamePhase::CounterTiming => 8,
        GamePhase::AllianceTiming => 16,
        GamePhase::EndOfTurnAction => 15,
        GamePhase::GameOver => 4, // no Python equivalent; End=4 is nearest
        // Phase 4 variants — no Python equivalent yet; reuse SelectTarget (5)
        // as a placeholder. Tasks 2-5 will add proper Python-side values.
        GamePhase::SelectUnion => 5,
        GamePhase::SelectPermutation => 5,
        GamePhase::SelectBudgeted => 5,
    }
}

/// Build the player-level dict. Matches `player_ui_data()` in Python.
fn player_ui_data(player: &Player, data: &[CardData], game: &Game) -> Value {
    let hand_ids: Vec<&str> = player.hand.iter().map(|c| c.card_id(data)).collect();

    let hand_cards: Vec<Value> = player
        .hand
        .iter()
        .map(|cs| {
            let cd = &data[cs.data_index];
            json!({
                "cardId": cs.card_id(data),
                "cardName": cd.card_name,
                "playCost": cd.play_cost,
                "level": cd.level,
                "dp": cd.dp,
                "colors": colors_of(cd),
                "cardKind": kind_int(cd),
                "evoCosts": evo_costs_of(cd),
            })
        })
        .collect();

    // Security IDs: null for face-down cards, card_id string for face-up.
    // `face_up_security` contains `card_index` values (u16) of revealed cards.
    let security_ids: Vec<Value> = player
        .security
        .iter()
        .map(|cs| {
            if player.face_up_security.contains(&cs.card_index) {
                json!(cs.card_id(data))
            } else {
                Value::Null
            }
        })
        .collect();

    let security_face_up: Vec<bool> = player
        .security
        .iter()
        .map(|cs| player.face_up_security.contains(&cs.card_index))
        .collect();

    let battle_area: Vec<Value> = player
        .battle_area
        .iter()
        .map(|p| perm_data(p, data, game))
        .collect();

    let breeding_area = player
        .breeding_area
        .as_ref()
        .map(|p| perm_data(p, data, game))
        .unwrap_or(Value::Null);

    let trash_ids: Vec<&str> = player.trash.iter().map(|c| c.card_id(data)).collect();

    // Player-relative memory: turn player sees +gauge, opponent sees -gauge.
    // Matches Python's `game._get_memory_for(p)` in serialization.py:300.
    let memory: i64 = if player.id == game.turn_player() {
        game.memory as i64
    } else {
        -(game.memory as i64)
    };

    json!({
        "id": py_pid(player.id),
        "memory": memory,
        "handCount": player.hand.len(),
        "handIds": hand_ids,
        "handCards": hand_cards,
        "securityCount": player.security.len(),
        "securityIds": security_ids,
        "securityFaceUp": security_face_up,
        "deckCount": player.deck.len(),
        "eggDeckCount": player.digitama_deck.len(),
        "battleAreaCount": player.battle_area.len(),
        "battleArea": battle_area,
        "breedingArea": breeding_area,
        "trashIds": trash_ids,
    })
}

fn colors_of(cd: &CardData) -> Vec<i64> {
    cd.colors.iter().map(|c| color_to_python_int(*c)).collect()
}

/// Map Rust `CardColor` → Python `CardColor` enum int value. Hand-written
/// because the two enums have different declaration orders.
///
/// Python (`digimon_gym/engine/data/enums.py`):
///   Red=0, Blue=1, Yellow=2, Green=3, White=4, Black=5, Purple=6
///
/// Rust (`digimon-engine/src/enums.rs`):
///   Red=0, Blue=1, Yellow=2, Green=3, Black=4, Purple=5, White=6
///
/// White, Black, and Purple are in different positions, so `as u8` gives
/// wrong values for those three. This mapping must be kept in sync with
/// Python's `CardColor` enum.
fn color_to_python_int(c: CardColor) -> i64 {
    match c {
        CardColor::Red => 0,
        CardColor::Blue => 1,
        CardColor::Yellow => 2,
        CardColor::Green => 3,
        CardColor::White => 4,
        CardColor::Black => 5,
        CardColor::Purple => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_to_python_int_mapping() {
        assert_eq!(color_to_python_int(CardColor::Red), 0);
        assert_eq!(color_to_python_int(CardColor::Blue), 1);
        assert_eq!(color_to_python_int(CardColor::Yellow), 2);
        assert_eq!(color_to_python_int(CardColor::Green), 3);
        assert_eq!(color_to_python_int(CardColor::White), 4);
        assert_eq!(color_to_python_int(CardColor::Black), 5);
        assert_eq!(color_to_python_int(CardColor::Purple), 6);
    }
}

fn kind_int(cd: &CardData) -> i64 {
    // Match Python CardKind int values: 0=Digimon, 1=Tamer, 2=Option, 3=DigiEgg.
    match cd.card_kind {
        CardKind::Digimon => 0,
        CardKind::Tamer => 1,
        CardKind::Option => 2,
        CardKind::DigiEgg => 3,
    }
}

fn evo_costs_of(cd: &CardData) -> Vec<Value> {
    // EvoCost fields: card_color: u8, level: u8, memory_cost: u16.
    // Key names match Python's serialization.py:313 — {"color", "level", "cost"}.
    // card_color is a raw u8 (same integer value Python's enum.value emits).
    cd.evo_costs
        .iter()
        .map(|ec| {
            json!({
                "color": ec.card_color,
                "level": ec.level,
                "cost": ec.memory_cost,
            })
        })
        .collect()
}

/// Per-permanent dict. Card-script-specific fields (keyword breakdown,
/// dp breakdown sources, effect text) are populated with neutral defaults —
/// shape parity is the goal; richness arrives with card migration.
fn perm_data(perm: &Permanent, data: &[CardData], _game: &Game) -> Value {
    let top = perm.top_card();
    let top_data = &data[top.data_index];
    let base_dp = top_data.dp.unwrap_or(0);
    let level = top_data.level.unwrap_or(0);
    let colors = colors_of(top_data);

    let sources: Vec<Value> = perm
        .card_sources
        .iter()
        .enumerate()
        .map(|(i, cs)| {
            let cd = &data[cs.data_index];
            json!({
                "cardId": cs.card_id(data),
                "cardName": cd.card_name,
                "isTop": i + 1 == perm.card_sources.len(),
                "optState": 0.0,
                "dpContribution": 0,
                "mainEffectText": "",
                "inheritedEffectText": "",
                "colors": colors_of(cd),
            })
        })
        .collect();

    let linked_card_ids: Vec<&str> = perm.linked_cards.iter().map(|c| c.card_id(data)).collect();

    json!({
        "topCardId": top.card_id(data),
        "topCardName": top_data.card_name,
        "dp": base_dp,
        "level": level,
        "isSuspended": perm.is_suspended,
        "sourceCount": perm.card_sources.len(),
        "keywords": Vec::<String>::new(),
        "keywordBreakdown": json!({ "innate": [], "gained": [] }),
        "securityAttackModifier": 0,
        "linkedCardIds": linked_card_ids,
        "sources": sources,
        "mainEffectText": "",
        "inheritedEffects": [],
        "dpBreakdown": json!({
            "base": base_dp,
            "sources": [],
            "temporary": 0.0,
            "aura": 0,
            "total": base_dp,
        }),
        "turnPlayed": perm.turn_played,
        "colors": colors,
    })
}

/// Build the pendingSelection dict from a resolved `PendingSelectionView`.
///
/// Extracted as a private helper symmetric with `player_ui_data` / `perm_data`
/// so it is unit-testable and keeps `to_ui_json` lean.
///
/// "kind" is a deliberate Rust-additive key not present in Python's
/// serialization.py:338 output — it surfaces the SelectionKind variant string
/// so typed WebSocket/UI consumers can route selection prompts without
/// re-deriving the kind from phase + validIndices alone.
fn pending_selection_data(v: &PendingSelectionView) -> Value {
    let mut m = Map::new();
    m.insert("kind".into(), Value::String(v.kind_str()));
    // phase: int matching Python's GamePhase.value (not a debug string).
    m.insert("phase".into(), Value::from(phase_int(v.previous_phase)));
    m.insert(
        "selectingPlayer".into(),
        Value::from(py_pid(v.selecting_player)),
    );
    m.insert(
        "validIndices".into(),
        Value::Array(v.valid_action_ids.iter().map(|i| json!(*i)).collect()),
    );
    m.insert("isOptional".into(), Value::from(v.is_optional));
    m.insert("prompt".into(), Value::String(v.prompt.clone()));
    if let Some(choices) = v.effect_choices.as_ref() {
        m.insert(
            "effectChoices".into(),
            Value::Array(
                choices
                    .iter()
                    .map(|c| json!({"label": c.label, "actionId": c.action_id}))
                    .collect(),
            ),
        );
    }
    Value::Object(m)
}

/// Build the pendingAttack dict.
///
/// Shape matches Python's serialization.py:370-373:
///   attacker_slot: index of the attacker in the turn-player's battle_area.
///   target_slot: index in enemy battle_area (Digimon target) or
///     SECURITY_TARGET for a direct/player attack.
fn pending_attack_data(pa: &PendingAttack, _game: &Game) -> Value {
    let attacker_slot = pa.attacker.index as i64;
    let target_slot: i64 = match pa.effective_target {
        AttackTarget::Digimon(ph) => ph.index as i64,
        AttackTarget::Player(_) => SECURITY_TARGET as i64,
    };
    json!({
        "attackerSlot": attacker_slot,
        "targetSlot": target_slot,
    })
}

/// Build the full UI-state dict. `state_filter.py` consumes this directly.
pub fn to_ui_json(game: &Game) -> Value {
    let pending_sel_value = game
        .pending_selection
        .as_ref()
        .map(|s| pending_selection_data(&s.view()))
        .unwrap_or(Value::Null);

    let pending_attack = game
        .pending_attack
        .as_ref()
        .map(|pa| pending_attack_data(pa, game))
        .unwrap_or(Value::Null);

    let revealed: Vec<Value> = game
        .revealed_cards
        .iter()
        .map(|cs| json!({"cardId": cs.card_id(&game.card_data), "owner": py_pid(cs.owner)}))
        .collect();

    // memoryGauge: from player1's perspective. Matches Python's
    // `game._get_memory_for(game.player1)` at serialization.py:379.
    // Positive when player1 is the turn player, negative otherwise.
    let memory_gauge: i64 = if game.players[0].id == game.turn_player() {
        game.memory as i64
    } else {
        -(game.memory as i64)
    };

    json!({
        "turnCount": game.turn_count,
        "currentPhase": phase_int(game.current_phase),
        "currentPlayer": py_pid(game.turn_player()),
        "memoryGauge": memory_gauge,
        "isGameOver": game.game_over,
        "winner": game.winner.map(py_pid),
        "player1": player_ui_data(&game.players[0], &game.card_data, game),
        "player2": player_ui_data(&game.players[1], &game.card_data, game),
        "revealedCards": revealed,
        "pendingSelection": pending_sel_value,
        "pendingAttack": pending_attack,
    })
}
