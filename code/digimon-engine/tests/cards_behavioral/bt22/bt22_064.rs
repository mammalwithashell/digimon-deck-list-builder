//! BT22-064 -- synthetic-board behavioural tests for the Keramon line
//! (dcgo-replay hunt). Fixtures: `keramon_line_support`.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::keramon_line_support::*;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn bt22_064_alt_route_costs_3_standard_route_costs_4() {
    // [Infermon] / Lv.5 w/[CS]: cost 3 (only the alt route fits a red [Infermon]).
    let mut r = full_runner();
    let inf = r.place_on_field(0, "INF-NAMED", Some(0));
    digivolve_via_action(&mut r, 0, "BT22-064", inf);
    assert!(r.pending_is_optional(), "[When Digivolving] optional token prompt");
    r.decline_optional_trigger().unwrap();
    assert_eq!(r.memory(), 8 - 3);

    // A black Lv.5 named [Infermon] fits BOTH routes: the player chooses 3 or 4.
    let mut r = full_runner();
    let black5 = r.place_on_field(0, "BT17-055", Some(0));
    digivolve_via_action(&mut r, 0, "BT22-064", black5);
    choose_cost_route(&mut r, 4);
    r.decline_optional_trigger().unwrap();
    assert_eq!(r.memory(), 8 - 4);

    // A black Lv.5 that is neither [Infermon] nor [CS]: only the standard cost 4.
    let mut r = full_runner();
    let plain = r.place_on_field(0, "PLAIN-LV5", Some(0));
    assert!(!can_digivolve(&mut r, 0, "BT22-064", plain), "red non-CS non-Infermon Lv.5: no route");
}

// -------------------------------------------------------------- BT22-064
#[test]
fn bt22_064_when_digivolving_may_play_a_token() {
    let mut r = full_runner();
    let host = r.place_on_field(0, "CS-LV5", Some(0));
    digivolve_via_action(&mut r, 0, "BT22-064", host);
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(token_on_field(&r, 0).len(), 1);
}

#[test]
fn bt22_064_has_alliance() {
    let mut r = full_runner();
    let d = r.place_on_field(0, "BT22-064", Some(0));
    assert!(r.game.has_keyword(d, digimon_engine::enums::Keyword::Alliance));
}

#[test]
fn bt22_064_deletes_lowest_play_cost_when_another_unidentified_digimon_is_played() {
    let mut r = full_runner();
    r.place_on_field(0, "BT22-064", Some(0));
    r.place_on_field(1, "OPP-C4", Some(0));
    r.place_on_field(1, "OPP-C6", Some(0));
    let unid = r.place_on_field(0, "UNID-LV3", None);
    r.fire_play_event_triggers(0, unid.index as usize, false, false);
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 1), vec!["OPP-C6"], "the cost-4 Digimon is the lowest");
}

#[test]
fn bt22_064_lowest_play_cost_counts_a_diaboromon_token_at_14() {
    let mut r = full_runner();
    r.place_on_field(0, "BT22-064", Some(0));
    r.place_on_field(1, "TOKEN_DIABOROMON", Some(0));
    let unid = r.place_on_field(0, "UNID-LV3", None);
    r.fire_play_event_triggers(0, unid.index as usize, false, false);
    let _ = r.auto_resolve();
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the opponent's only Digimon (a cost-14 Diaboromon Token) has the lowest play cost"
    );
}

#[test]
fn bt22_064_ignores_non_unidentified_and_its_own_play() {
    let mut r = full_runner();
    r.place_on_field(1, "OPP-C4", Some(0));
    let me = r.place_on_field(0, "BT22-064", None);
    r.fire_play_event_triggers(0, me.index as usize, false, false);
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 1), vec!["OPP-C4"], "'other' Digimon only");

    let mut r = full_runner();
    r.place_on_field(0, "BT22-064", Some(0));
    r.place_on_field(1, "OPP-C4", Some(0));
    let plain = r.place_on_field(0, "PLAIN-LV3", None);
    r.fire_play_event_triggers(0, plain.index as usize, false, false);
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 1), vec!["OPP-C4"], "needs the [Unidentified] trait");

    let mut r = full_runner();
    r.place_on_field(0, "BT22-064", Some(0));
    r.place_on_field(1, "OPP-C4", Some(0));
    let theirs = r.place_on_field(1, "UNID-LV3", None);
    r.fire_play_event_triggers(1, theirs.index as usize, false, false);
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 1).len(), 2, "an opponent's Unidentified Digimon doesn't count");
}

#[test]
fn bt22_064_lowest_play_cost_delete_is_once_per_turn() {
    let mut r = full_runner();
    r.place_on_field(0, "BT22-064", Some(0));
    r.place_on_field(1, "OPP-C4", Some(0));
    r.place_on_field(1, "OPP-C6", Some(0));
    r.place_on_field(1, "OPP-C9", Some(0));
    let a = r.place_on_field(0, "UNID-LV3", None);
    r.fire_play_event_triggers(0, a.index as usize, false, false);
    let _ = r.auto_resolve();
    let b = r.place_on_field(0, "UNID-LV3", None);
    r.fire_play_event_triggers(0, b.index as usize, false, false);
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 1), vec!["OPP-C6", "OPP-C9"], "second play: locked out");
}
