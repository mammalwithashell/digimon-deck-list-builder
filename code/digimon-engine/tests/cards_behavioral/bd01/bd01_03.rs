//! BD01-03 Liskmon — Digimon Lv.4 Red, Cost 5, DP 5000
//! [When Digivolving] Delete 1 of your opponent's Digimon with 2000 DP or less.

use crate::alysion_common::{action, candidates, fixture, start};
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::selection::SelectionKind;

#[test]
fn bd01_03_when_digivolving_deletes_2000_dp_or_less() {
    let mut r = start(fixture(&["BD01-03"]).hand(0, &["BD01-03"]));
    let base = r.place_stack(0, &["RED-LV3"]);
    r.place_stack(1, &["PLAIN-2K"]);
    r.place_stack(1, &["PLAIN-3K"]);
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["PLAIN-2K"]);
    r.choose_prompt_card("PLAIN-2K", None).unwrap();
    assert_eq!(r.battle_area_size(1), 1);
}
