//! EX6-043 -- synthetic-board behavioural tests for the Keramon line
//! (dcgo-replay hunt). Fixtures: `keramon_line_support`.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::keramon_line_support::*;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

// --------------------------------------------------------------- EX6-043
#[test]
fn ex6_043_start_of_main_phase_and_when_digivolving_play_a_token() {
    let mut r = full_runner();
    let host = r.place_on_field(0, "BT17-055", Some(0));
    digivolve_via_action(&mut r, 0, "EX6-043", host);
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(token_on_field(&r, 0).len(), 1);
}

#[test]
fn ex6_043_other_diaboromon_named_digimon_get_blocker_and_jamming_but_not_itself() {
    use digimon_engine::enums::Keyword;
    let mut r = full_runner();
    let me = r.place_on_field(0, "EX6-043", Some(0));
    let tok = r.place_on_field(0, "TOKEN_DIABOROMON", Some(0));
    let other = r.place_on_field(0, "PLAIN-LV3", Some(0));
    let theirs = r.place_on_field(1, "TOKEN_DIABOROMON", Some(0));
    r.game.materialize_declaratives_full();
    assert!(r.game.has_keyword(tok, Keyword::Blocker));
    assert!(r.game.has_keyword(tok, Keyword::Jamming));
    assert!(!r.game.has_keyword(me, Keyword::Blocker), "other Digimon only");
    assert!(!r.game.has_keyword(other, Keyword::Blocker));
    assert!(!r.game.has_keyword(theirs, Keyword::Blocker), "only your Digimon");
}

#[test]
fn ex6_043_opponent_play_may_activate_its_when_digivolving_effect_once_per_turn() {
    let mut r = full_runner();
    r.place_on_field(0, "EX6-043", Some(0));
    let opp = r.place_on_field(1, "OPP-C4", None);
    r.fire_play_event_triggers(1, opp.index as usize, false, false);
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    // The re-fired [When Digivolving] is itself a 'you may'.
    if r.pending_is_optional() {
        r.accept_optional_trigger().unwrap();
    }
    let _ = r.auto_resolve();
    assert_eq!(token_on_field(&r, 0).len(), 1);

    let opp2 = r.place_on_field(1, "OPP-C6", None);
    r.fire_play_event_triggers(1, opp2.index as usize, false, false);
    assert!(r.pending_selection().is_none(), "[Once Per Turn]");
}

#[test]
fn ex6_043_does_not_activate_on_its_own_sides_play() {
    let mut r = full_runner();
    r.place_on_field(0, "EX6-043", Some(0));
    let mine = r.place_on_field(0, "PLAIN-LV3", None);
    r.fire_play_event_triggers(0, mine.index as usize, false, false);
    assert!(r.pending_selection().is_none());
}
