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
use crate::enums::{CardKind, GamePhase};
use crate::game::Game;
use crate::permanent::Permanent;
use crate::player::Player;
use crate::selection::AttackTarget;

/// Translate a Rust `PlayerId` (0 / 1) into the Python 1 / 2 convention.
fn py_pid(rust_pid: u8) -> i64 {
    (rust_pid as i64) + 1
}

/// Stable string for a phase variant. Uses the Rust `Debug` spelling
/// (PascalCase). Consumers can normalise case if needed.
fn phase_str(p: GamePhase) -> String {
    format!("{:?}", p)
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

    json!({
        "id": py_pid(player.id),
        "memory": game.memory,
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

fn colors_of(cd: &CardData) -> Vec<String> {
    cd.colors.iter().map(|c| format!("{:?}", c)).collect()
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

/// Build the full UI-state dict. `state_filter.py` consumes this directly.
pub fn to_ui_json(game: &Game) -> Value {
    let ps = game.pending_selection.as_ref().map(|s| s.view());
    let pending_sel_value = match ps {
        None => Value::Null,
        Some(v) => {
            let mut m = Map::new();
            // "kind" is a deliberate Rust-additive key not present in Python's
            // serialization.py:338 output — it surfaces the SelectionKind variant
            // string so typed WebSocket/UI consumers can route selection prompts
            // without re-deriving the kind from phase + validIndices alone.
            m.insert("kind".into(), Value::String(v.kind_str()));
            m.insert("phase".into(), Value::String(v.previous_phase_str()));
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
    };

    let pending_attack = game
        .pending_attack
        .as_ref()
        .map(|pa| {
            // Shape matches Python's serialization.py:370-373.
            // attacker_slot: index of the attacker in the turn-player's battle_area.
            // target_slot: index in enemy battle_area (Digimon target) or
            //   SECURITY_TARGET (14) for a direct/player attack — mirrors Python's
            //   isinstance(pa.effective_target, Permanent) branch.
            let attacker_slot = pa.attacker.index as i64;
            let target_slot: i64 = match pa.effective_target {
                AttackTarget::Digimon(ph) => ph.index as i64,
                AttackTarget::Player(_) => SECURITY_TARGET as i64,
            };
            json!({
                "attackerSlot": attacker_slot,
                "targetSlot": target_slot,
            })
        })
        .unwrap_or(Value::Null);

    let revealed: Vec<Value> = game
        .revealed_cards
        .iter()
        .map(|cs| json!({"cardId": cs.card_id(&game.card_data), "owner": py_pid(cs.owner)}))
        .collect();

    json!({
        "turnCount": game.turn_count,
        "currentPhase": phase_str(game.current_phase),
        "currentPlayer": py_pid(game.turn_player()),
        "memoryGauge": game.memory,
        "isGameOver": game.game_over,
        "winner": game.winner.map(py_pid),
        "player1": player_ui_data(&game.players[0], &game.card_data, game),
        "player2": player_ui_data(&game.players[1], &game.card_data, game),
        "revealedCards": revealed,
        "pendingSelection": pending_sel_value,
        "pendingAttack": pending_attack,
    })
}
