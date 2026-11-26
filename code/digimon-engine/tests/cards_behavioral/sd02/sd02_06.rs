//! SD02-06 Valner Dragnogh — Tamer Blue, Cost 4
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [Main] If you have a Digimon with the [ALYSION] trait, by suspending this Tamer, trash the bottom 2 digivolution cards of 1 of your opponent's Digimon.
//! [Security] Play this card without paying the cost.

use crate::alysion_common::{action, assert_memory_setter, assert_security_play, fixture, start};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::selection::SelectionKind;

#[test]
fn sd02_06_memory_setter_security_play_and_main_suspend_strips_two_with_alysion() {
    assert_memory_setter("SD02-06");
    assert_security_play("SD02-06");

    let mut r = start(fixture(&["SD02-06"]));
    let tamer = r.place_stack(0, &["SD02-06"]);
    let opp = r.place_stack(1, &["FILLER", "FILLER", "FILLER", "PLAIN-5K"]);
    let act = FIELD_EFFECT_START
        + (tamer.index as u16) * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN;

    // Without an [ALYSION] Digimon, [Main] is not legal.
    assert!(build_action_mask(&r.game, 0)[act as usize] < 0.5);

    // With an [ALYSION] Digimon, [Main] suspends Tamer and trashes bottom 2 sources.
    r.place_stack(0, &["BLUE-ALYSION"]);
    assert!(build_action_mask(&r.game, 0)[act as usize] > 0.5);
    action(&mut r, 0, act);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert!(r.game.players[0].battle_area[tamer.index as usize].is_suspended);
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 2);
}
