//! Tests for the view layer.
//!
//! Phase 1.10-1.11 of the engine-debug-mcp change. The property test asserts
//! that every `Perspective::Player` view's serialized JSON is a *subset*
//! (key-wise) of the corresponding `Perspective::God` view — i.e. perspective
//! filtering only ever *drops* keys, never adds or renames them.

#![cfg(test)]

use super::*;
use crate::debug_runner::DebugRunner;
use crate::enums::PlayerId;

// ── helpers ──────────────────────────────────────────────────────────────

fn fixture_game() -> DebugRunner {
    // A minimal game built from the default DebugRunnerBuilder — no scripted
    // cards, just the engine baseline. Sufficient for view-shape checks.
    DebugRunner::new()
}

/// `god` keys ⊇ `player` keys, recursively for objects.
fn assert_key_subset(player: &serde_json::Value, god: &serde_json::Value, path: &str) {
    use serde_json::Value;
    match (player, god) {
        (Value::Object(p), Value::Object(g)) => {
            for k in p.keys() {
                assert!(
                    g.contains_key(k),
                    "player view has key '{path}.{k}' that god view lacks"
                );
                assert_key_subset(&p[k], &g[k], &format!("{path}.{k}"));
            }
        }
        (Value::Array(p), Value::Array(g)) => {
            // Compare per-element when both arrays have entries; player may
            // have fewer hidden entries (e.g. opponent permanents are public,
            // so they should match length-wise here).
            let n = p.len().min(g.len());
            for i in 0..n {
                assert_key_subset(&p[i], &g[i], &format!("{path}[{i}]"));
            }
        }
        _ => {
            // Leaves — value differences are allowed; we only check that
            // the player view never *adds* a key god doesn't have.
        }
    }
}

// ── state ───────────────────────────────────────────────────────────────

#[test]
fn state_view_serializes() {
    let r = fixture_game();
    let view = StateView::from_game(&r.game, Perspective::God);
    let json = serde_json::to_value(&view).expect("state view serializes");
    assert!(json.is_object());
    let obj = json.as_object().unwrap();
    for required in &[
        "phase",
        "turn_count",
        "memory",
        "memory_pair",
        "turn_player",
        "game_over",
        "winner",
        "terminal_outcome_reason",
        "event_seq",
    ] {
        assert!(obj.contains_key(*required), "missing field {required}");
    }
}

// ── hand ────────────────────────────────────────────────────────────────

#[test]
fn hand_view_god_includes_cards_field() {
    let r = fixture_game();
    let view = HandView::from_game(&r.game, 0, Perspective::God);
    let json = serde_json::to_value(&view).unwrap();
    let obj = json.as_object().unwrap();
    assert!(obj.contains_key("count"));
    assert!(obj.contains_key("cards"));
    // God always reveals the cards array.
    assert!(!obj["cards"].is_null());
}

#[test]
fn hand_view_opponent_perspective_redacts_cards() {
    let r = fixture_game();
    // Viewer is player 0; ask for player 1's hand.
    let view = HandView::from_game(&r.game, 1, Perspective::Player(0));
    let json = serde_json::to_value(&view).unwrap();
    assert!(json["count"].is_number());
    // `cards` is None → serializes as null.
    assert!(json["cards"].is_null());
}

#[test]
fn hand_view_own_perspective_reveals_cards() {
    let r = fixture_game();
    let view = HandView::from_game(&r.game, 0, Perspective::Player(0));
    let json = serde_json::to_value(&view).unwrap();
    assert!(!json["cards"].is_null());
}

// ── security ────────────────────────────────────────────────────────────

#[test]
fn security_view_god_includes_card_ids() {
    let r = fixture_game();
    let view = SecurityView::from_game(&r.game, 0, Perspective::God);
    let json = serde_json::to_value(&view).unwrap();
    assert!(json["count"].is_number());
    assert!(!json["card_ids"].is_null());
}

#[test]
fn security_view_player_perspective_redacts_card_ids() {
    let r = fixture_game();
    // Even the owner cannot peek at their own security stack per rules.
    let view = SecurityView::from_game(&r.game, 0, Perspective::Player(0));
    let json = serde_json::to_value(&view).unwrap();
    assert!(json["card_ids"].is_null());
}

// ── field ───────────────────────────────────────────────────────────────

#[test]
fn field_view_serializes_with_empty_battle_area() {
    let r = fixture_game();
    let view = FieldView::from_game(&r.game, 0, Perspective::God);
    let json = serde_json::to_value(&view).unwrap();
    assert!(json["battle_area"].is_array());
    assert!(json["breeding"].is_array());
    assert!(json["trash_count"].is_number());
    assert!(json["deck_count"].is_number());
    assert!(json["egg_deck_count"].is_number());
}

// ── perspective subset property ─────────────────────────────────────────

#[test]
fn perspective_subset_invariant_holds_across_views() {
    let r = fixture_game();
    let viewer: PlayerId = 0;

    // StateView is perspective-insensitive but the invariant still holds.
    let state_god = serde_json::to_value(StateView::from_game(&r.game, Perspective::God)).unwrap();
    let state_pl =
        serde_json::to_value(StateView::from_game(&r.game, Perspective::Player(viewer))).unwrap();
    assert_key_subset(&state_pl, &state_god, "state");

    // HandView for opponent: player view strictly drops fields.
    let hand_god = serde_json::to_value(HandView::from_game(&r.game, 1, Perspective::God)).unwrap();
    let hand_pl =
        serde_json::to_value(HandView::from_game(&r.game, 1, Perspective::Player(viewer))).unwrap();
    assert_key_subset(&hand_pl, &hand_god, "hand(opponent)");

    // SecurityView for own player: drops card_ids.
    let sec_god =
        serde_json::to_value(SecurityView::from_game(&r.game, viewer, Perspective::God)).unwrap();
    let sec_pl = serde_json::to_value(SecurityView::from_game(
        &r.game,
        viewer,
        Perspective::Player(viewer),
    ))
    .unwrap();
    assert_key_subset(&sec_pl, &sec_god, "security(own)");

    // FieldView: battle area is public, perspective filtering is a no-op.
    let field_god =
        serde_json::to_value(FieldView::from_game(&r.game, 0, Perspective::God)).unwrap();
    let field_pl = serde_json::to_value(FieldView::from_game(
        &r.game,
        0,
        Perspective::Player(viewer),
    ))
    .unwrap();
    assert_key_subset(&field_pl, &field_god, "field");
}

// ── effect queue + event log smoke ──────────────────────────────────────

#[test]
fn effect_queue_view_serializes_when_empty() {
    let r = fixture_game();
    let view = EffectQueueView::from_game(&r.game);
    let json = serde_json::to_value(&view).unwrap();
    assert!(json["pending"].is_array());
}

#[test]
fn event_log_view_serializes() {
    let r = fixture_game();
    let view = EventLogView::from_game(&r.game, None);
    let json = serde_json::to_value(&view).unwrap();
    assert!(json["events"].is_array());
    assert!(json["last_seq"].is_number());
}
