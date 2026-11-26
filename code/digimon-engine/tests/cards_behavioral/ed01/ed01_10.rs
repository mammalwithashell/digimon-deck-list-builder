//! ED01-10 Bond of Snowflakes — Option Blue, Cost 3
//! [Main] You may play 1 blue [Renamon] or [Valner Dragnogh] from your hand without paying the cost. Then, place this card in the battle area.
//! [End of Your Turn] <Delay> • Choose 1 of your blue [ALYSION] trait Digimon and it attacks 1 of your opponent's Digimon with fewer digivolution cards than it. It can also attack your opponent's unsuspended Digimon with this effect. (Q7586)
//! [Security] Activate this card's [Main] effects.

use crate::alysion_common::{action, advance_turn, candidates, fixture, start};
use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn ed01_10_main_only_blue_renamon_or_valner_and_eot_delay_attacks_strictly_fewer_sources() {
    let mut r = start(
        fixture(&["ED01-10"]).hand(0, &["ED01-10", "RENAMON-BLUE", "RENAMON-YEL", "VALNER"]),
    );
    let attacker = r.place_stack(0, &["FILLER", "FILLER", "BLUE-ALYSION"]); // 2 sources, 5000 DP
    let opp_fewer = r.place_stack(1, &["FILLER", "PLAIN-2K"]); // 1 source (unsuspended)
    let _opp_equal = r.place_stack(1, &["FILLER", "FILLER", "PLAIN-2K"]); // 2 sources (Q7586: NOT valid!)

    // [Main] offers blue Renamon and Valner, NOT yellow Renamon.
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(candidates(&r), ["RENAMON-BLUE", "VALNER"]);
    r.choose_prompt_card("VALNER", None).unwrap();

    // Advance past placing turn.
    advance_turn(&mut r);
    advance_turn(&mut r);

    // End of Your Turn -> <Delay> triggers.
    action(&mut r, 0, PASS);
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    let accept_delay = r.pending_selection().unwrap().valid_action_ids[0];
    action(&mut r, 0, accept_delay); // activate <Delay>
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("BLUE-ALYSION", None).unwrap();

    // Target prompt: ONLY `opp_fewer` is valid (not equal-source Digimon per Q7586, not security).
    let valid_ids = r
        .pending_selection()
        .expect("forced attack target prompt")
        .valid_action_ids
        .clone();
    assert_eq!(
        valid_ids,
        vec![encode_attack(attacker.index as u16, opp_fewer.index as u16)]
    );
    action(&mut r, 0, valid_ids[0]);
    assert_eq!(r.battle_area_size(1), 1); // opp_fewer deleted in battle
}

#[test]
fn ed01_10_security_activates_main_effect() {
    let mut r = start(
        fixture(&["ED01-10"])
            .hand(1, &["RENAMON-BLUE"])
            .security(1, &["ED01-10"]),
    );
    let attacker = r.place_stack(0, &["PLAIN-12K"]);
    action(&mut r, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
    r.assert_prompt(SelectionKind::Hand, true, 1);
    r.choose_prompt_card("RENAMON-BLUE", None).unwrap();
    assert_eq!(r.battle_area_size(1), 2);
}
