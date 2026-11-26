//! BD02-08 Valner Dragnogh — Tamer Blue, Cost 3
//! [Start of Your Main Phase] If you have a blue Digimon with the [ALYSION] trait, gain 1 memory.
//! [Main] By suspending this Tamer, 1 of your blue Digimon with the [ALYSION] trait gains <Piercing> for the turn.
//! [Security] Play this card without paying the cost.

use crate::alysion_common::{action, advance_turn, assert_security_play, candidates, fixture, start};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn bd02_08_start_of_main_memory_main_piercing_and_security_play() {
    assert_security_play("BD02-08");

    let mut r = start(fixture(&["BD02-08"]));
    let tamer = r.place_stack(0, &["BD02-08"]);
    let blue_a = r.place_stack(0, &["BLUE-ALYSION"]);
    let _red_a = r.place_stack(0, &["RED-ALYSION"]);

    // [Main] suspend Tamer -> 1 blue [ALYSION] Digimon gains <Piercing> for the turn.
    action(
        &mut r,
        0,
        FIELD_EFFECT_START + (tamer.index as u16) * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN,
    );
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), ["BLUE-ALYSION"]);
    r.choose_prompt_card("BLUE-ALYSION", None).unwrap();
    assert!(r.game.players[0].battle_area[tamer.index as usize].is_suspended);
    assert!(r.game.has_keyword(blue_a, Keyword::Piercing));

    advance_turn(&mut r);
    assert!(!r.game.has_keyword(blue_a, Keyword::Piercing));

    // [Start of Your Main Phase] +1 memory with blue [ALYSION] Digimon.
    r.game.set_memory(-3);
    r.game.end_turn();
    assert_eq!(r.memory(), 4);
}
