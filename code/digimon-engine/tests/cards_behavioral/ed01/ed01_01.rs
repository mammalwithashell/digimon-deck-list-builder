//! ED01-01 Rubeusdramon — Digimon Lv.6 Red, Cost 13, DP 13000
//! [When Digivolving] <Blitz>
//! [When Attacking] Delete up to 6000 DP total worth of your opponent's Digimon. (Q7569)
//! [Your Turn] While you have [Kanata Hondo], this Digimon gains <Piercing> and can also attack your opponent's unsuspended Digimon. (Q7570)

use crate::alysion_common::{action, fixture, start};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, ATTACK_START, PASS, SECURITY_TARGET,
};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn ed01_01_blitz_dp_budget_delete_and_kanata_piercing_unsuspended() {
    // [When Digivolving] <Blitz> when digivolution crosses memory to opponent's side.
    let mut r = start(fixture(&["ED01-01"]).hand(0, &["ED01-01"]).memory(2));
    let base = r.place_stack(0, &["RED-LV5"]);
    r.place_stack(1, &["PLAIN-3K"]);
    r.place_stack(1, &["PLAIN-3K"]);
    action(&mut r, 0, encode_digivolve(0, base.index as u16)); // cost 5 -> memory -3
    // Blitz attack target prompt.
    action(&mut r, 0, encode_attack(base.index as u16, SECURITY_TARGET));
    // [When Attacking] Delete up to 6000 DP total (min 1 pick per Q7569).
    r.assert_prompt(SelectionKind::DpBudget { remaining_dp: 6000, picked: 0 }, false, 0);
    action(&mut r, 0, ATTACK_START);
    // Optional second pick (Q7569: may stop at 1 pick).
    r.assert_prompt(SelectionKind::DpBudget { remaining_dp: 3000, picked: 1 }, true, 0);
    action(&mut r, 0, PASS);
    assert_eq!(r.battle_area_size(1), 1);

    // [Your Turn] While you have [Kanata Hondo], gains <Piercing> and can attack unsuspended.
    let mut r2 = start(fixture(&["ED01-01"]));
    let rubeus = r2.place_stack(0, &["ED01-01"]);
    let opp_unsuspended = r2.place_stack(1, &["PLAIN-5K"]);
    r2.game.tick_declarative_effects();
    assert!(!r2.game.has_keyword(rubeus, Keyword::Piercing));
    assert!(build_action_mask(&r2.game, 0)[encode_attack(rubeus.index as u16, opp_unsuspended.index as u16) as usize] < 0.5);

    r2.place_stack(0, &["KANATA"]);
    r2.game.tick_declarative_effects();
    assert!(r2.game.has_keyword(rubeus, Keyword::Piercing));
    assert!(build_action_mask(&r2.game, 0)[encode_attack(rubeus.index as u16, opp_unsuspended.index as u16) as usize] > 0.5);
}
