//! SD02-03 Tenkomon — Digimon Lv.4 Blue, Cost 6, DP 5000
//! [When Digivolving] <Draw 1>
//! Inherited: [Your Turn] This Digimon gets +1000 DP.

use crate::alysion_common::{action, advance_turn, fixture, start};
use digimon_engine::action::space::encode_digivolve;

#[test]
fn sd02_03_when_digivolving_draws_and_inherited_your_turn_dp() {
    let mut r = start(fixture(&["SD02-03"]).hand(0, &["SD02-03"]));
    let base = r.place_stack(0, &["BLUE-LV3"]);
    let hand_before = r.hand_size(0);
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    // -1 digivolved +1 standard digivolve draw +1 [When Digivolving] draw = hand_before + 1.
    assert_eq!(r.hand_size(0), hand_before + 1);

    let host = r.place_stack(0, &["SD02-03", "PLAIN-5K"]);
    assert_eq!(r.effective_dp(host), Some(6000));
    advance_turn(&mut r);
    assert_eq!(r.effective_dp(host), Some(5000));
}
