//! ST26-02 Renamon — Digimon Lv.3 Blue, Cost 3, DP 1000
//! (Rule) Trait: Has [Ice-Snow] Type.
//! [When Moving] [On Play] Trash the bottom digivolution card of 1 of your opponent's Digimon. Then, if your opponent has no Digimon with digivolution cards, <Draw 1>. (Q7573)
//! Inherited: [When Attacking] If this Digimon has the [Ice-Snow] trait, you may change the attack target to 1 of your opponent's unsuspended Digimon with the fewest digivolution cards. (Q7574, Q7575)

use crate::alysion_common::{action, fixture, ids, start};
use digimon_engine::action::space::{encode_attack, MOVE_FROM_BREEDING, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn st26_02_on_play_and_when_moving_strip_bottom_and_draw_when_no_opp_sources() {
    // [On Play] strips last source -> opponent now has no Digimon with sources -> Draw 1.
    let mut r = start(fixture(&["ST26-02"]).hand(0, &["ST26-02"]));
    let opp = r.place_stack(1, &["FILLER", "PLAIN-5K"]);
    let hand_before = r.hand_size(0);
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].battle_area[opp.index as usize].card_sources), ["PLAIN-5K"]);
    assert_eq!(r.hand_size(0), hand_before); // -1 played +1 drawn

    // [When Moving] with no opponent Digimon at all (Q7573) still draws 1.
    let mut r2 = fixture(&["ST26-02"]).start();
    r2.set_phase(digimon_engine::enums::GamePhase::Breeding);
    r2.place_in_breeding(0, "ST26-02");
    let hand2 = r2.hand_size(0);
    action(&mut r2, 0, MOVE_FROM_BREEDING);
    assert_eq!(r2.hand_size(0), hand2 + 1);
}

#[test]
fn st26_02_inherited_redirects_to_unsuspended_fewest_sources() {
    for accept in [false, true] {
        let mut r = start(fixture(&["ST26-02"]));
        let attacker = r.place_stack(0, &["ST26-02", "ICE-MON"]);
        let fewest = r.place_stack(1, &["PLAIN-2K"]); // 0 sources
        r.place_stack(1, &["FILLER", "PLAIN-12K"]);   // 1 source
        action(&mut r, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
        r.assert_prompt(SelectionKind::OppField, true, 0);
        if accept {
            r.choose_prompt_card("PLAIN-2K", None).unwrap();
            assert_eq!(r.battle_area_size(1), 1); // PLAIN-2K deleted in battle
            assert_eq!(r.security_count(1), 3);
            let _ = fewest;
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.battle_area_size(1), 2);
            assert_eq!(r.security_count(1), 2);
        }
    }
}
