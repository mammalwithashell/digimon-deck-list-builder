//! BT22-059 -- synthetic-board behavioural tests for the Keramon line
//! (dcgo-replay hunt). Fixtures: `keramon_line_support`.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::keramon_line_support::*;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

// -------------------------------------------------------------- BT22-059
#[test]
fn bt22_059_on_play_deletes_an_opponent_digimon_with_play_cost_5_or_less_only() {
    let mut r = full_runner();
    let inf = r.place_on_field(0, "BT22-059", Some(0));
    r.place_on_field(1, "OPP-C4", Some(0));
    r.place_on_field(1, "OPP-C6", Some(0));
    r.fire_on_play(0, inf.index as usize);
    // Only the cost-4 Digimon is a legal target; the pick is mandatory.
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 1), vec!["OPP-C6"]);
}

#[test]
fn bt22_059_on_play_with_no_target_does_nothing() {
    let mut r = full_runner();
    let inf = r.place_on_field(0, "BT22-059", Some(0));
    r.place_on_field(1, "OPP-C6", Some(0));
    r.fire_on_play(0, inf.index as usize);
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 1), vec!["OPP-C6"]);
}

#[test]
fn bt22_059_diaboromon_token_is_not_a_cost_5_or_less_target() {
    // The token prints play cost 14.
    let mut r = full_runner();
    let inf = r.place_on_field(0, "BT22-059", Some(0));
    r.place_on_field(1, "TOKEN_DIABOROMON", Some(0));
    r.fire_on_play(0, inf.index as usize);
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 1), vec!["TOKEN_DIABOROMON"]);
}

#[test]
fn bt22_059_protects_itself_only_while_arata_or_adam_is_in_play() {
    for tamer in ["ARATA", "ADAM"] {
        let mut r = full_runner();
        let inf = r.place_on_field(0, "BT22-059", Some(0));
        r.place_on_field(0, tamer, Some(0));
        r.place_on_field(1, "OPP-C4", Some(0));
        r.fire_on_play(0, inf.index as usize);
        let _ = r.auto_resolve();
        assert!(r.modifiers().has(inf, ModifierType::ImmuneFromDPMinus), "{tamer}");
        assert!(r.modifiers().has(inf, ModifierType::CannotBeReturnedToHand), "{tamer}");
        assert!(r.modifiers().has(inf, ModifierType::CannotBeReturnedToDeck), "{tamer}");
    }
    let mut r = full_runner();
    let inf = r.place_on_field(0, "BT22-059", Some(0));
    r.place_on_field(0, "TAMER-PLAIN", Some(0));
    r.place_on_field(1, "OPP-C4", Some(0));
    r.fire_on_play(0, inf.index as usize);
    let _ = r.auto_resolve();
    assert!(!r.modifiers().has(inf, ModifierType::ImmuneFromDPMinus));
    assert!(!r.modifiers().has(inf, ModifierType::CannotBeReturnedToHand));
}

#[test]
fn bt22_059_protection_still_applies_when_the_delete_found_no_target() {
    // "Then, if you have [Arata Sanada] or [Eater Adam]" is independent of the delete.
    let mut r = full_runner();
    let inf = r.place_on_field(0, "BT22-059", Some(0));
    r.place_on_field(0, "ARATA", Some(0));
    r.fire_on_play(0, inf.index as usize);
    let _ = r.auto_resolve();
    assert!(r.modifiers().has(inf, ModifierType::CannotBeReturnedToDeck));
}

#[test]
fn bt22_059_when_digivolving_runs_the_same_body() {
    let mut r = full_runner();
    let host = r.place_on_field(0, "CS-LV4", Some(0));
    r.place_on_field(1, "OPP-C4", Some(0));
    digivolve_via_action(&mut r, 0, "BT22-059", host);
    let _ = r.auto_resolve();
    assert!(field_top_ids(&r, 1).is_empty());
}

#[test]
fn bt22_059_inherited_plays_a_token_when_another_unidentified_digimon_is_deleted() {
    for seat in 0..=1u8 {
        let mut r = full_runner();
        r.game.turn_player_idx = seat as usize;
        r.game.memory_pair = (seat, 1 - seat);
        // BT22-059 as the SOURCE under a plain host (inherited text).
        r.place_stack(seat, &["BT22-059", "BLACK3"]);
        let unid = r.place_on_field(seat, "UNID-LV3", Some(0));
        r.game.delete_permanent_with_effects(unid);
        r.game.drain_effect_queue();
        assert!(r.pending_is_optional(), "[Once Per Turn] ... you may (seat {seat})");
        r.accept_optional_trigger().unwrap();
        let _ = r.auto_resolve();
        assert_eq!(token_on_field(&r, seat).len(), 1);
    }
}

#[test]
fn bt22_059_inherited_ignores_non_unidentified_and_opponent_deletions() {
    let mut r = full_runner();
    r.place_stack(0, &["BT22-059", "BLACK3"]);
    let plain = r.place_on_field(0, "PLAIN-LV3", Some(0));
    r.game.delete_permanent_with_effects(plain);
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert!(token_on_field(&r, 0).is_empty(), "not an [Unidentified] Digimon");

    let mut r = full_runner();
    r.place_stack(0, &["BT22-059", "BLACK3"]);
    let theirs = r.place_on_field(1, "UNID-LV3", Some(0));
    r.game.delete_permanent_with_effects(theirs);
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert!(token_on_field(&r, 0).is_empty(), "opponent's Digimon");
}

#[test]
fn bt22_059_inherited_is_once_per_turn() {
    let mut r = full_runner();
    r.place_stack(0, &["BT22-059", "BLACK3"]);
    let a = r.place_on_field(0, "UNID-LV3", Some(0));
    r.place_on_field(0, "UNID-LV3", Some(0));
    r.game.delete_permanent_with_effects(a);
    r.game.drain_effect_queue();
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(token_on_field(&r, 0).len(), 1);
    let b = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "UNID-LV3")
        .unwrap();
    r.game
        .delete_permanent_with_effects(PermanentHandle { player: 0, index: b as u8 });
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert_eq!(token_on_field(&r, 0).len(), 1, "second deletion: locked out");
}

#[test]
fn bt22_059_inherited_counts_a_deleted_diaboromon_token() {
    // The token is an [Unidentified] Digimon.
    let mut r = full_runner();
    r.place_stack(0, &["BT22-059", "BLACK3"]);
    let tok = r.place_on_field(0, "TOKEN_DIABOROMON", Some(0));
    r.game.delete_permanent_with_effects(tok);
    r.game.drain_effect_queue();
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(token_on_field(&r, 0).len(), 1);
}
