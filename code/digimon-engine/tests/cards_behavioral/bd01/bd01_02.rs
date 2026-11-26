//! BD01-02 Gemmon — Digimon Lv.3 Red, Cost 3, DP 2000
//! Inherited: [Your Turn] Add 1000 to this [ALYSION] trait Digimon's DP deletion effect's maximum. (Q7567, Q7568)

use crate::alysion_common::{action, candidates, fixture, start};
use digimon_engine::action::space::{encode_digivolve};
use digimon_engine::selection::SelectionKind;

#[test]
fn bd01_02_inherited_adds_1000_to_alysion_dp_deletion_max_live_on_digivolve() {
    // Digivolving BD01-03 Liskmon (delete <= 2000 DP) onto BD01-02 Gemmon raises cap to 3000 DP (Q7567).
    let mut r = start(fixture(&["BD01-02", "BD01-03"]).hand(0, &["BD01-03"]));
    let base = r.place_stack(0, &["BD01-02"]);
    r.place_stack(1, &["PLAIN-3K"]);
    r.place_stack(1, &["PLAIN-4K"]);
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["PLAIN-3K"]);
    r.choose_prompt_card("PLAIN-3K", None).unwrap();
    assert_eq!(r.battle_area_size(1), 1);
}
