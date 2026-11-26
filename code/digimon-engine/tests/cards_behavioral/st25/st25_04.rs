//! ST25-04 Reptilimon — Digimon Lv.5 Red, Cost 7, DP 7000
//! [Digivolve] Lv.4 w/[Gemmon] in text: Cost 3
//! <Engage>
//! [On Play] [When Digivolving] 1 of your Digimon gets +3000 DP for the turn. Then, if that Digimon has 12000 DP or more, it also gains <Security A. +1>. (Q7552: for the turn.)
//! Inherited: [Your Turn] This Digimon with [Gemmon] in its text gains <Engage>.

use crate::alysion_common::{action, advance_turn, assert_gemmon_text_digivolve, fixture, start};
use digimon_engine::action::space::{encode_digivolve, PLAY_HAND_START};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn st25_04_alt_digivolve_from_gemmon_text_lv4() {
    assert_gemmon_text_digivolve("ST25-04", 4, 3);
}

#[test]
fn st25_04_boosts_dp_and_conditionally_grants_sa_plus_1_for_turn_plus_engage() {
    // Target reaching >= 12000 DP gains <Security A. +1> for the turn (Q7552).
    let mut r = start(fixture(&["ST25-04"]).hand(0, &["ST25-04"]));
    let big = r.place_stack(0, &["PLAIN-12K"]);
    let small = r.place_stack(0, &["PLAIN-5K"]);
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("PLAIN-12K", None).unwrap();
    assert_eq!(r.effective_dp(big), Some(15000));
    assert_eq!(r.game.effective_security_strike(big), 2);
    assert_eq!(r.game.effective_security_strike(small), 1);

    // Expires at end of turn.
    advance_turn(&mut r);
    assert_eq!(r.effective_dp(big), Some(12000));
    assert_eq!(r.game.effective_security_strike(big), 1);

    // Target staying below 12000 DP gets +3000 DP only, no SA+1.
    let mut r2 = start(fixture(&["ST25-04"]).hand(0, &["ST25-04"]));
    let base = r2.place_stack(0, &["RED-LV4"]);
    action(&mut r2, 0, encode_digivolve(0, base.index as u16));
    r2.assert_prompt(SelectionKind::OwnField, false, 0);
    r2.choose_prompt_card("ST25-04", None).unwrap();
    assert_eq!(r2.effective_dp(base), Some(10000));
    assert_eq!(r2.game.effective_security_strike(base), 1);
    assert!(r2.game.has_keyword(base, Keyword::Engage));

    // Inherited [Your Turn] <Engage> only with [Gemmon] in text.
    let mut r3 = start(fixture(&["ST25-04", "ST25-02"]));
    let gemmon = r3.place_stack(0, &["ST25-04", "ST25-02"]);
    let plain = r3.place_stack(0, &["ST25-04", "PLAIN-5K"]);
    r3.game.tick_declarative_effects();
    assert!(r3.game.has_keyword(gemmon, Keyword::Engage));
    assert!(!r3.game.has_keyword(plain, Keyword::Engage));
    // Passing opens Engage EoT attack window; declining (PASS) finishes the turn.
    action(&mut r3, 0, digimon_engine::action::space::PASS);
    if r3.pending_selection().is_some() {
        action(&mut r3, 0, digimon_engine::action::space::PASS);
    }
    assert_eq!(r3.game.turn_player(), 1);
    assert!(!r3.game.has_keyword(gemmon, Keyword::Engage));
}
