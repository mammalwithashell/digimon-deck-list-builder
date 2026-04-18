//! Shape parity: Rust `to_ui_json` must produce every top-level key the
//! Python side produces, plus the correct player-ID convention (1/2).

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::serialization::to_ui_json;

#[test]
fn to_ui_json_has_all_top_level_keys() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .add_card(make_test_card("ST1-01", "Egg"))
        .add_card(make_test_card("ST1-03", "Filler"))
        .hand(0, &["TEST-001"])
        .start();
    let value = to_ui_json(&r.game);
    let obj = value.as_object().expect("root is an object");
    for key in [
        "turnCount",
        "currentPhase",
        "currentPlayer",
        "memoryGauge",
        "isGameOver",
        "winner",
        "player1",
        "player2",
        "revealedCards",
        "pendingSelection",
        "pendingAttack",
    ] {
        assert!(obj.contains_key(key), "missing top-level key {:?}", key);
    }
}

#[test]
fn player_ids_use_python_convention() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .start();
    let value = to_ui_json(&r.game);
    assert_eq!(value["player1"]["id"], serde_json::json!(1));
    assert_eq!(value["player2"]["id"], serde_json::json!(2));
    let cp = value["currentPlayer"].as_i64().unwrap();
    assert!(cp == 1 || cp == 2, "currentPlayer must be 1 or 2, got {}", cp);
}

#[test]
fn player_ui_data_has_full_key_set() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .start();
    let value = to_ui_json(&r.game);
    let p1 = value["player1"].as_object().expect("player1 is object");
    for key in [
        "id",
        "memory",
        "handCount",
        "handIds",
        "handCards",
        "securityCount",
        "securityIds",
        "securityFaceUp",
        "deckCount",
        "eggDeckCount",
        "battleAreaCount",
        "battleArea",
        "breedingArea",
        "trashIds",
    ] {
        assert!(p1.contains_key(key), "player1 missing {:?}", key);
    }
}

#[test]
fn pending_selection_serializes_when_installed() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-010", "PilotDelete"))
        .add_card(make_test_card("ALLY", "Ally"))
        .hand(0, &["TEST-010"])
        .memory(3)
        .start();
    r.place_on_field(1, "ALLY", Some(0));
    r.play(0, 0);

    let value = to_ui_json(&r.game);
    let ps = value["pendingSelection"].as_object().expect("pendingSelection is object");
    assert_eq!(ps["selectingPlayer"], serde_json::json!(1));
    assert!(ps["validIndices"].is_array());
    assert_eq!(ps["isOptional"], serde_json::json!(true));
    assert!(ps["prompt"].is_string());
    assert_eq!(ps["kind"], serde_json::json!("OppField"));
}

#[test]
fn pending_selection_null_when_no_selection() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .start();
    let value = to_ui_json(&r.game);
    assert!(
        value["pendingSelection"].is_null(),
        "pendingSelection must be null without a prompt; got {}",
        value["pendingSelection"]
    );
}
