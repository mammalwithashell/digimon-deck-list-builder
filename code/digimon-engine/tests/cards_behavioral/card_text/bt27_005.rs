//! Source: `data/card_bundles/BT27-005.md` (printed text).
//!
//! ### Inherited Effect
//! [Your Turn] [Once Per Turn] When any of your black Digimon are played, ＜Draw 1＞.

use super::common::*;

const CARD: &str = "BT27-005";

#[test]
fn own_black_digimon_play_draws_once() {
    let mut r = start(builder(CARD).hand(0, &["TEXT-3", "TEXT-4"]));
    r.place_stack(0, &[CARD, "TEXT-5"]);
    r.play(0, 0);
    assert_eq!(r.hand_size(0), 2);
    r.play(0, 0);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn playing_a_tamer_does_not_draw() {
    let mut r = start(builder(CARD).hand(0, &["TEXT-TAMER"]));
    r.place_stack(0, &[CARD, "TEXT-5"]);
    r.play(0, 0);
    assert_eq!(r.hand_size(0), 0);
}
