//! Capture round-trip coverage for the add-scenario-capture-mcp change.
//!
//! Proves `Game::to_scenario()` is the inverse of the staging importer:
//! a non-trivial staged board, captured and re-applied via
//! `Game::apply_scenario()`, reproduces the identical full-information
//! zones and scalar state. Also pins the captured fixture's schema shape.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, GamePhase};
use serde_json::Value;

fn lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

/// A runner over a fixed synthetic pool (no DigiEggs, so digitama stays
/// empty and the deck pool is a clean multiset).
fn fresh_runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(lv4("D-A"))
        .add_card(lv4("D-B"))
        .add_card(lv4("D-C"))
        .add_card(lv4("D-D"))
        .add_card(lv4("D-E"))
        .add_card(lv4("D-F"))
        .start();
    r.skip_mulligan();
    r
}

/// Build a non-trivial board on a fresh runner: field stacks on both
/// players (one suspended with a turn_played override), a breeding stack,
/// a trash card, off-default memory / phase / turn / first-player.
fn staged_board() -> DebugRunner {
    let mut r = fresh_runner();
    r.set_phase(GamePhase::Main);
    r.set_turn(4);
    r.set_first_player(1);
    r.place_field_stack(0, &["D-A", "D-B"], true, 3);
    r.place_field_stack(1, &["D-C"], false, 0);
    r.place_in_breeding(0, "D-D");
    r.inject_trash(0, "D-E");
    r.game.set_memory(3);
    r
}

fn sorted_deck(fixture: &Value, pid: &str) -> Vec<String> {
    let mut v: Vec<String> = fixture["decks"][pid]
        .as_array()
        .expect("decks[pid] is an array")
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect();
    v.sort();
    v
}

#[test]
fn capture_round_trips_zones_and_scalar_state() {
    let a = staged_board();
    let captured = a.game.to_scenario();

    // Re-stage onto an independently-constructed game over the same pool.
    let mut b = fresh_runner();
    b.game
        .apply_scenario(&captured)
        .expect("captured fixture must re-stage cleanly");
    let restaged = b.game.to_scenario();

    // Full per-player zone arrangement must match (the importer always
    // emits every zone key, so B's zones are fully determined by capture).
    assert_eq!(
        captured["zones"], restaged["zones"],
        "re-staged zones must equal the captured zones exactly"
    );

    // Scalar state must match.
    assert_eq!(captured["state"], restaged["state"], "scalar state must round-trip");

    // Deck pool must match as a multiset (order may differ; content cannot).
    for pid in ["1", "2"] {
        assert_eq!(
            sorted_deck(&captured, pid),
            sorted_deck(&restaged, pid),
            "deck pool for player {pid} must round-trip as a multiset"
        );
    }

    // Spot-check the substantive board details survived.
    assert_eq!(captured["state"]["memory"], 3);
    assert_eq!(captured["state"]["turn"], 4);
    assert_eq!(captured["state"]["phase"], "Main");
    let p1_field = &captured["zones"]["1"]["field"];
    assert_eq!(p1_field[0]["stack"], serde_json::json!(["D-A", "D-B"]));
    assert_eq!(p1_field[0]["is_suspended"], true);
    assert_eq!(p1_field[0]["turn_played"], 3);
    assert_eq!(captured["zones"]["1"]["breeding"], serde_json::json!(["D-D"]));
    assert_eq!(captured["zones"]["1"]["trash"], serde_json::json!(["D-E"]));
}

#[test]
fn captured_fixture_has_the_documented_schema_shape() {
    let captured = staged_board().game.to_scenario();

    // Required top-level keys.
    for key in ["schema_version", "decks", "seed", "state", "zones", "assertions"] {
        assert!(captured.get(key).is_some(), "captured fixture missing '{key}'");
    }
    assert_eq!(captured["schema_version"], 1);

    // Scalar state shape.
    for key in ["memory", "phase", "turn", "first_player"] {
        assert!(
            captured["state"].get(key).is_some(),
            "captured state missing '{key}'"
        );
    }

    // Assertion buckets present and empty on capture.
    assert_eq!(captured["assertions"]["engine"], serde_json::json!([]));
    assert_eq!(captured["assertions"]["ui"], serde_json::json!([]));

    // Per-player decks + zones, with every zone key present.
    for pid in ["1", "2"] {
        assert!(captured["decks"].get(pid).is_some(), "missing decks[{pid}]");
        let z = &captured["zones"][pid];
        for zk in ["hand", "deck_top", "security", "trash", "field", "breeding"] {
            assert!(z.get(zk).is_some(), "zones[{pid}] missing '{zk}'");
        }
    }
}
