//! BT22-057 -- synthetic-board behavioural tests for the Keramon line
//! (dcgo-replay hunt). Fixtures: `keramon_line_support`.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::keramon_line_support::*;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn bt22_057_inherited_substitute_works_the_same() {
    let mut r = full_runner();
    let host = r.place_stack(0, &["BT22-057", "EX6-043"]);
    r.place_on_field(0, "TOKEN_DIABOROMON", Some(0));
    r.game.delete_permanent_with_effects(host);
    r.game.drain_effect_queue();
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 0), vec!["EX6-043"]);
}

#[test]
fn bt22_057_when_digivolving_plays_arata_sanada_from_hand_free() {
    for seat in 0..=1u8 {
        let mut r = full_runner();
        r.game.turn_player_idx = seat as usize;
        r.game.memory_pair = (seat, 1 - seat);
        let host = r.place_on_field(seat, "BLACK3", Some(0));
        r.add_to_hand(seat, "ARATA");
        digivolve_via_action(&mut r, seat, "BT22-057", host);
        assert!(r.pending_is_optional(), "[When Digivolving] ... you may (seat {seat})");
        r.accept_optional_trigger().unwrap();
        let _ = r.auto_resolve();
        assert_eq!(tamers_on_field(&r, seat), 1, "[Arata Sanada] was played");
        assert_eq!(hand_ids(&r, seat), vec!["INERT"], "only the digivolve draw remains");
        assert_eq!(r.memory(), 8 - 2, "only the digivolve cost; the Tamer was free");
    }
}

#[test]
fn bt22_057_when_digivolving_can_be_declined() {
    let mut r = full_runner();
    let host = r.place_on_field(0, "BLACK3", Some(0));
    r.add_to_hand(0, "ARATA");
    digivolve_via_action(&mut r, 0, "BT22-057", host);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(tamers_on_field(&r, 0), 0);
    assert_eq!(hand_ids(&r, 0), vec!["ARATA", "INERT"], "Arata stays; INERT is the digivolve draw");
}

#[test]
fn bt22_057_when_digivolving_blocked_by_two_tamers() {
    let mut r = full_runner();
    let host = r.place_on_field(0, "BLACK3", Some(0));
    r.place_on_field(0, "ARATA", Some(0));
    r.place_on_field(0, "TAMER-PLAIN", Some(0));
    r.add_to_hand(0, "ARATA");
    digivolve_via_action(&mut r, 0, "BT22-057", host);
    assert!(
        r.pending_selection().is_none(),
        "2 Tamers: the condition 'if you have 1 or fewer Tamers' fails"
    );
    assert_eq!(tamers_on_field(&r, 0), 2);
}

#[test]
fn bt22_057_when_digivolving_allowed_with_exactly_one_tamer() {
    let mut r = full_runner();
    let host = r.place_on_field(0, "BLACK3", Some(0));
    r.place_on_field(0, "TAMER-PLAIN", Some(0));
    r.add_to_hand(0, "ARATA");
    digivolve_via_action(&mut r, 0, "BT22-057", host);
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(tamers_on_field(&r, 0), 2);
}

#[test]
fn bt22_057_when_digivolving_needs_arata_in_hand() {
    let mut r = full_runner();
    let host = r.place_on_field(0, "BLACK3", Some(0));
    r.add_to_hand(0, "TAMER-PLAIN");
    digivolve_via_action(&mut r, 0, "BT22-057", host);
    assert!(r.pending_selection().is_none());
}
