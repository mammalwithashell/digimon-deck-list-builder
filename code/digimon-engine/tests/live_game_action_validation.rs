//! Cross-cutting integration tests for the `LiveGame` action contract.
//!
//! These tests pin the spec scenarios in `live-game-surface/spec.md`
//! (proposal `enforce-live-game-action-contracts`) by driving a real
//! `LiveGame` constructed via a minimal card pool. The in-module tests
//! in `live_game.rs` cover the same scenarios; this file exists to
//! exercise the public crate surface so the contracts hold for
//! downstream consumers (CLI, MCP server).

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::enums::GamePhase;
use digimon_engine::live_game::{ActionResult, AttackTarget, LiveGame};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::view::Perspective;

fn minimal_db() -> HashMap<String, CardData> {
    let json = r#"{
        "ST1-01": {
            "card_id": "ST1-01", "card_name_eng": "Botamon",
            "card_effect_class_name": "ST1_01", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "ST1-03": {
            "card_id": "ST1-03", "card_name_eng": "Koromon",
            "card_effect_class_name": "ST1_03", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["Rookie"], "attribute_eng": ["Free"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

fn test_deck() -> Vec<String> {
    std::iter::repeat("ST1-01".to_string())
        .take(5)
        .chain(std::iter::repeat("ST1-03".to_string()).take(45))
        .collect()
}

fn make_game() -> LiveGame {
    let db = minimal_db();
    LiveGame::from_decks(test_deck(), test_deck(), Some(7), &db).expect("from_decks succeeds")
}

#[test]
fn step_with_unknown_action_id_is_rejected() {
    let mut lg = make_game();
    let r = lg.step(0xFFFE);
    assert!(!r.ok, "unknown action_id must be rejected");
}

#[test]
fn end_turn_during_mulligan_does_not_advance_turn() {
    let mut lg = make_game();
    let st_before = lg.state(Perspective::God);
    let turn_before = st_before.turn_count;
    let phase_before = st_before.phase.clone();
    let r = lg.end_turn();
    let st_after = lg.state(Perspective::God);
    assert!(!r.ok, "end_turn during Mulligan must be rejected");
    assert_eq!(
        st_after.turn_count, turn_before,
        "turn_count must not advance"
    );
    assert_eq!(st_after.phase, phase_before, "phase must not change");
}

#[test]
fn legal_actions_inactive_player_returns_empty() {
    let lg = make_game();
    let active = lg.current_decision_player();
    let other = 1 - active;
    let acts_other = lg.legal_actions(other);
    assert!(
        acts_other.is_empty(),
        "inactive player must see 0 legal actions, got {}",
        acts_other.len()
    );
}

#[test]
fn events_emitted_are_structured_json() {
    let mut lg = make_game();
    // Drive through mulligan + first turn to fire events.
    let _ = lg.step(0);
    let _ = lg.step(0);
    let _ = lg.pass_turn();
    let active = lg.current_decision_player();
    let r = lg.play(active, 0);
    let v = serde_json::to_value(&r).expect("ActionResult serializes");
    let evs = v["events_emitted"]
        .as_array()
        .expect("events_emitted is array");
    for ev in evs {
        assert!(ev.is_object(), "event entry must be object, got {:?}", ev);
        assert!(
            ev.get("type").and_then(|x| x.as_str()).is_some(),
            "event entry must have string `type`: {:?}",
            ev
        );
    }
}

#[test]
fn digivolve_with_no_legal_action_returns_rejection() {
    let mut lg = make_game();
    let host = PermanentHandle {
        player: 0,
        index: 0,
    };
    let r = lg.digivolve(host, 0);
    assert!(!r.ok, "digivolve with no legal action must be rejected");
    assert!(r.error.is_some());
}

#[test]
fn attack_with_no_legal_action_returns_rejection() {
    let mut lg = make_game();
    let attacker = PermanentHandle {
        player: 0,
        index: 0,
    };
    let r = lg.attack(attacker, AttackTarget::Security);
    assert!(!r.ok, "attack with no legal action must be rejected");
    assert!(r.error.is_some());
}

#[test]
fn game_phase_mulligan_is_initial_phase() {
    let lg = make_game();
    assert_eq!(
        lg.state(Perspective::God).phase,
        format!("{:?}", GamePhase::Mulligan)
    );
}

#[allow(dead_code)]
fn _action_result_compiles(r: ActionResult) -> bool {
    r.ok
}
