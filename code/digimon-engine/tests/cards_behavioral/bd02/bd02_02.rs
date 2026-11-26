//! BD02-02 Renamon — Digimon Lv.3 Blue, Cost 3, DP 2000
//! [On Play] Reveal the top 5 cards of your deck. Add 1 blue [ALYSION] trait card among them to the hand. Return the rest to the bottom of the deck.

use crate::alysion_common::{action, candidates, fixture, ids, start};
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::selection::SelectionKind;

#[test]
fn bd02_02_reveals_5_and_adds_blue_alysion() {
    let mut r = start(
        fixture(&["BD02-02"])
            .hand(0, &["BD02-02"])
            .deck(0, &["FILLER", "RED-ALYSION", "BLUE-LV3", "BLUE-ALYSION", "FILLER"]),
    );
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::Reveal, false, 0);
    assert_eq!(candidates(&r), ["BLUE-ALYSION"]);
    r.choose_prompt_card("BLUE-ALYSION", None).unwrap();
    for remaining in (1..=4).rev() {
        r.assert_prompt(SelectionKind::OrderedPermutation { remaining }, false, 0);
        let act = r.pending_selection().unwrap().valid_action_ids[0];
        action(&mut r, 0, act);
    }
    assert_eq!(ids(&r, &r.game.players[0].hand), ["BLUE-ALYSION"]);
    assert_eq!(r.deck_size(0), 4);
}
