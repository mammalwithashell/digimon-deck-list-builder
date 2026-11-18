//! BT5-063 -- synthetic-board behavioural tests for the Keramon line
//! (dcgo-replay hunt). Fixtures: `keramon_line_support`.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::keramon_line_support::*;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

// --------------------------------------------------------------- BT5-063
#[test]
fn bt5_063_when_digivolving_plays_arata_when_none_in_play() {
    let mut r = full_runner();
    let host = r.place_on_field(0, "BLACK3", Some(0));
    r.add_to_hand(0, "ARATA");
    digivolve_via_action(&mut r, 0, "BT5-063", host);
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(tamers_on_field(&r, 0), 1);
}

#[test]
fn bt5_063_when_digivolving_blocked_when_arata_already_in_play() {
    let mut r = full_runner();
    let host = r.place_on_field(0, "BLACK3", Some(0));
    r.place_on_field(0, "ARATA", Some(0));
    r.add_to_hand(0, "ARATA");
    digivolve_via_action(&mut r, 0, "BT5-063", host);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt5_063_inherited_rush_goes_to_other_digimon_with_the_same_name_on_your_turn() {
    use digimon_engine::enums::Keyword;
    let mut r = full_runner();
    // Carrier: Kurisarimon (BT5-063) is the source under a plain host, so the
    // carrier's name is the HOST's name ("Other Black Rookie").
    let carrier = r.place_stack(0, &["BT5-063", "BLACK3"]);
    let twin = r.place_on_field(0, "BLACK3", Some(0));
    let stranger = r.place_on_field(0, "PLAIN-LV3", Some(0));
    let theirs = r.place_on_field(1, "BLACK3", Some(0));
    r.game.materialize_declaratives_full();
    assert!(r.game.has_keyword(twin, Keyword::Rush), "same name as the carrier");
    assert!(!r.game.has_keyword(stranger, Keyword::Rush), "different name");
    assert!(!r.game.has_keyword(carrier, Keyword::Rush), "'other' Digimon only");
    assert!(!r.game.has_keyword(theirs, Keyword::Rush), "only your Digimon");
}

#[test]
fn bt5_063_inherited_rush_is_your_turn_only() {
    use digimon_engine::enums::Keyword;
    let mut r = full_runner();
    r.place_stack(0, &["BT5-063", "BLACK3"]);
    let twin = r.place_on_field(0, "BLACK3", Some(0));
    r.game.turn_player_idx = 1;
    r.game.memory_pair = (1, 0);
    r.game.materialize_declaratives_full();
    assert!(!r.game.has_keyword(twin, Keyword::Rush));
}
