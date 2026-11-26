//! ED01-09 Bond of Rubies — Option Red, Cost 3
//! [Main] You may play 1 red [Gemmon] or [Kanata Hondo] from your hand without paying the cost. Then, place this card in the battle area.
//! [Your Turn] When one of your red [ALYSION] trait Digimon attacks, <Delay>. • That Digimon gains <Security A. +1> and +3000 DP for the turn.
//! [Security] Activate this card's [Main] effects.

use crate::alysion_common::{action, advance_turn, candidates, fixture, start};
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn ed01_09_main_plays_red_gemmon_or_kanata_and_delay_boosts_attacking_red_alysion() {
    let mut r = start(fixture(&["ED01-09"]).hand(0, &["ED01-09", "GEMMON-RED", "KANATA", "RED-TAMER"]));
    let attacker = r.place_stack(0, &["RED-ALYSION"]);

    // [Main] offers GEMMON-RED and KANATA, then places in battle area.
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(candidates(&r), ["GEMMON-RED", "KANATA"]);
    r.choose_prompt_card("KANATA", None).unwrap();
    assert_eq!(r.battle_area_size(0), 3); // RED-ALYSION, KANATA, ED01-09

    // Pass to opponent turn and back so Delay is past its placing turn.
    advance_turn(&mut r);
    advance_turn(&mut r);

    // Attack with red [ALYSION] Digimon -> <Delay> triggers!
    action(&mut r, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    let accept_delay = r.pending_selection().unwrap().valid_action_ids[0];
    action(&mut r, 0, accept_delay); // activate <Delay>
    // Attacker gained +3000 DP (5000 -> 8000) and <Security A. +1> (2 checks!).
    assert_eq!(r.effective_dp(attacker), Some(8000));
    assert_eq!(r.security_count(1), 1); // 3 - 2 = 1
}

#[test]
fn ed01_09_security_activates_main_effect() {
    let mut r = start(
        fixture(&["ED01-09"])
            .hand(1, &["GEMMON-RED"])
            .security(1, &["ED01-09"]),
    );
    let attacker = r.place_stack(0, &["PLAIN-12K"]);
    action(&mut r, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
    r.assert_prompt(SelectionKind::Hand, true, 1);
    r.choose_prompt_card("GEMMON-RED", None).unwrap();
    assert_eq!(r.battle_area_size(1), 2); // GEMMON-RED + ED01-09 in battle area
}
