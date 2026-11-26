//! SD01-05 Rubeusdramon — Digimon Lv.6 Red, Cost 12, DP 12000
//! <Piercing>
//! [When Digivolving] Delete 1 of your opponent's Digimon with 6000 DP or less.

use crate::alysion_common::{action, candidates, fixture, start};
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn sd01_05_piercing_and_when_digivolving_deletes_6000_or_less() {
    let mut r = start(fixture(&["SD01-05"]).hand(0, &["SD01-05"]));
    let base = r.place_stack(0, &["RED-LV5"]);
    r.place_stack(1, &["PLAIN-6K"]);
    r.place_stack(1, &["PLAIN-7K"]);
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert!(r.game.has_keyword(base, Keyword::Piercing));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["PLAIN-6K"]);
    r.choose_prompt_card("PLAIN-6K", None).unwrap();
    assert_eq!(r.battle_area_size(1), 1);
}
