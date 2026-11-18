//! EX6-036 -- synthetic-board behavioural tests for the Keramon line
//! (dcgo-replay hunt). Fixtures: `keramon_line_support`.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::keramon_line_support::*;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn ex6_036_on_play_adds_a_diaboromon_tamer_and_an_unidentified_card_and_trashes_the_rest() {
    let mut r = search_runner(&["INERT", "UNID-1", "TAMER-DIA"]);
    let k = r.place_on_field(0, "EX6-036", None);
    r.fire_on_play(0, k.index as usize);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["TAMER-DIA", "UNID-1"]);
    assert_eq!(trash_ids(&r, 0), vec!["INERT"], "the rest is trashed, not returned");
    assert_eq!(r.deck_size(0), 0);
}

#[test]
fn ex6_036_accepts_an_option_with_diaboromon_in_its_text_and_rejects_a_plain_tamer() {
    let mut r = search_runner(&["TAMER-PLAIN", "UNID-1", "OPT-DIA"]);
    let k = r.place_on_field(0, "EX6-036", None);
    r.fire_on_play(0, k.index as usize);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["OPT-DIA", "UNID-1"]);
    assert_eq!(trash_ids(&r, 0), vec!["TAMER-PLAIN"]);

    let mut r = search_runner(&["TAMER-PLAIN", "UNID-1", "INERT"]);
    let k = r.place_on_field(0, "EX6-036", None);
    r.fire_on_play(0, k.index as usize);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["UNID-1"], "no Tamer/Option with [Diaboromon] in text");
    assert_eq!(trash_ids(&r, 0), vec!["INERT", "TAMER-PLAIN"]);
}
