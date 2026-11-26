//! SD01-02 Gemmon — Digimon Lv.3 Red, Cost 3, DP 1000
//! [On Play] Reveal the top 5 cards of your deck. Add 1 red [ALYSION] trait card among them to the hand. Return the rest to the bottom of the deck.
//! Inherited: [Your Turn] This Digimon gets +1000 DP.

use crate::alysion_common::{action, advance_turn, candidates, fixture, ids, start};
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::selection::SelectionKind;

#[test]
fn sd01_02_reveals_5_adds_red_alysion_and_inherited_your_turn_dp() {
    let mut r = start(
        fixture(&["SD01-02"])
            .hand(0, &["SD01-02"])
            .deck(0, &["FILLER", "BLUE-ALYSION", "RED-LV3", "RED-ALYSION", "FILLER"]),
    );
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::Reveal, false, 0);
    assert_eq!(candidates(&r), ["RED-ALYSION"]);
    r.choose_prompt_card("RED-ALYSION", None).unwrap();
    for remaining in (1..=4).rev() {
        r.assert_prompt(SelectionKind::OrderedPermutation { remaining }, false, 0);
        let act = r.pending_selection().unwrap().valid_action_ids[0];
        action(&mut r, 0, act);
    }
    assert_eq!(ids(&r, &r.game.players[0].hand), ["RED-ALYSION"]);
    assert_eq!(r.deck_size(0), 4);

    // Inherited [Your Turn] +1000 DP.
    let host = r.place_stack(0, &["SD01-02", "PLAIN-5K"]);
    assert_eq!(r.effective_dp(host), Some(6000));
    advance_turn(&mut r);
    assert_eq!(r.effective_dp(host), Some(5000));
}
