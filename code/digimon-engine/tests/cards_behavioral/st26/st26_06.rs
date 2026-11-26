//! ST26-06 Valner Dragnogh — Tamer Blue, Cost 4
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [All Turns] When your Digimon are played or digivolve, if any of them have the [Ice-Snow] trait, by suspending this Tamer, trash the bottom 2 digivolution cards of 1 of your opponent's Digimon. After, if your opponent has no Digimon with digivolution cards, <Draw 1>. (Q7582, Q7583)
//! [Security] Play this card without paying the cost.

use crate::alysion_common::{action, assert_memory_setter, assert_security_play, fixture, start};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::selection::SelectionKind;

#[test]
fn st26_06_memory_setter_and_security_play() {
    assert_memory_setter("ST26-06");
    assert_security_play("ST26-06");
}

#[test]
fn st26_06_suspends_to_strip_two_and_draw_only_when_paid() {
    for accept in [false, true] {
        let mut r = start(fixture(&["ST26-06"]).hand(0, &["ICE-MON"]));
        let tamer = r.place_stack(0, &["ST26-06"]);
        let opp = r.place_stack(1, &["FILLER", "FILLER", "PLAIN-5K"]);
        let hand_before = r.hand_size(0);
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
        if accept {
            let act = r.pending_selection().unwrap().valid_action_ids[0];
            action(&mut r, 0, act);
            r.assert_prompt(SelectionKind::OppField, false, 0);
            r.choose_prompt_card("PLAIN-5K", None).unwrap();
            assert!(r.game.players[0].battle_area[tamer.index as usize].is_suspended);
            assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 1);
            assert_eq!(r.hand_size(0), hand_before); // -1 played, +1 drawn
        } else {
            // Q7582: declining suspend does NOT draw even if opponent has no sources.
            action(&mut r, 0, PASS);
            assert!(!r.game.players[0].battle_area[tamer.index as usize].is_suspended);
            assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 3);
            assert_eq!(r.hand_size(0), hand_before - 1);
        }
    }
}
