//! ST25-02 Gemmon — Digimon Lv.3 Red, Cost 3, DP 1000
//! [Digivolve] Lv.2 w/[Gemmon] in text: Cost 0
//! [When Moving] [On Play] 1 of your Digimon gets +3000 DP for the turn.
//! Inherited: [Your Turn] This Digimon gets +2000 DP.

use crate::alysion_common::{action, advance_turn, assert_gemmon_text_digivolve, fixture, start};
use digimon_engine::action::space::{MOVE_FROM_BREEDING, PLAY_HAND_START};
use digimon_engine::selection::SelectionKind;

#[test]
fn st25_02_alt_digivolve_from_gemmon_text_lv2() {
    assert_gemmon_text_digivolve("ST25-02", 2, 0);
}

#[test]
fn st25_02_on_play_and_when_moving_boost_dp_for_turn_and_inherited_your_turn() {
    // [On Play] +3000 DP for turn.
    let mut r = start(fixture(&["ST25-02"]).hand(0, &["ST25-02"]));
    let ally = r.place_stack(0, &["PLAIN-5K"]);
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.effective_dp(ally), Some(8000));
    advance_turn(&mut r);
    assert_eq!(r.effective_dp(ally), Some(5000));

    // [When Moving] +3000 DP for turn.
    let mut r2 = fixture(&["ST25-02"]).start();
    r2.set_phase(digimon_engine::enums::GamePhase::Breeding);
    r2.place_in_breeding(0, "ST25-02");
    action(&mut r2, 0, MOVE_FROM_BREEDING);
    r2.assert_prompt(SelectionKind::OwnField, false, 0);
    r2.choose_prompt_card("ST25-02", None).unwrap();
    assert_eq!(r2.effective_dp(digimon_engine::permanent::PermanentHandle { player: 0, index: 0 }), Some(4000));

    // Inherited: [Your Turn] +2000 DP.
    let mut r3 = start(fixture(&["ST25-02"]));
    let host = r3.place_stack(0, &["ST25-02", "PLAIN-5K"]);
    assert_eq!(r3.effective_dp(host), Some(7000));
    advance_turn(&mut r3);
    assert_eq!(r3.effective_dp(host), Some(5000));
}
