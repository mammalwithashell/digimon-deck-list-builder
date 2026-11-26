//! SD02-07 Pale Death — Option Blue, Cost 3
//! [Main] Return 1 of your opponent's level 4 or lower Digimon to the hand.
//! [Security] Activate this card's [Main] effect.

use crate::alysion_common::{action, candidates, fixture, start};
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn sd02_07_main_and_security_bounce_lv4_or_lower_digimon() {
    let mut r = start(fixture(&["SD02-07"]).hand(0, &["SD02-07"]));
    r.place_stack(0, &["BLUE-LV3"]); // blue color req
    r.place_stack(1, &["PLAIN-4K"]); // Lv.4
    r.place_stack(1, &["PLAIN-6K"]); // Lv.5
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["PLAIN-4K"]);
    r.choose_prompt_card("PLAIN-4K", None).unwrap();
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.hand_size(1), 1);

    // [Security] Activate [Main] effect.
    let mut r2 = start(fixture(&["SD02-07"]).security(1, &["SD02-07"]));
    let attacker = r2.place_stack(0, &["PLAIN-4K"]); // Lv.4
    action(&mut r2, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
    r2.assert_prompt(SelectionKind::OppField, false, 1);
    r2.choose_prompt_card("PLAIN-4K", None).unwrap();
    assert_eq!(r2.battle_area_size(0), 0);
    assert_eq!(r2.hand_size(0), 1);
}
