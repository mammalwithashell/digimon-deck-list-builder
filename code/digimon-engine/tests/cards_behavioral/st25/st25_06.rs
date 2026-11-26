//! ST25-06 Kanata Hondo — Tamer Red, Cost 4
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [Your Turn] When your Digimon are played or digivolve, if any of them have the [ALYSION] trait, by suspending this Tamer, for the turn, 1 of your Digimon isn't affected by your opponent's Option effects and gets +3000 DP.
//! [Security] Play this card without paying the cost.

use crate::alysion_common::{action, assert_memory_setter, assert_security_play, fixture, start};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::enums::EffectSourceKind;
use digimon_engine::selection::SelectionKind;

#[test]
fn st25_06_memory_setter_and_security_play() {
    assert_memory_setter("ST25-06");
    assert_security_play("ST25-06");
}

#[test]
fn st25_06_suspends_on_alysion_play_to_grant_option_immunity_and_3000_dp() {
    for accept in [false, true] {
        let mut r = start(fixture(&["ST25-06"]).hand(0, &["RED-ALYSION"]));
        let tamer = r.place_stack(0, &["ST25-06"]);
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
        if accept {
            let act = r.pending_selection().unwrap().valid_action_ids[0];
            action(&mut r, 0, act);
            r.assert_prompt(SelectionKind::OwnField, false, 0);
            r.choose_prompt_card("RED-ALYSION", None).unwrap();
            let played = digimon_engine::permanent::PermanentHandle { player: 0, index: 1 };
            assert!(r.game.players[0].battle_area[tamer.index as usize].is_suspended);
            assert_eq!(r.effective_dp(played), Some(8000));
            assert!(!r.game.effect_can_affect_from(played, 1, EffectSourceKind::Option, None));
            assert!(r.game.effect_can_affect_from(played, 1, EffectSourceKind::Digimon, None));
        } else {
            action(&mut r, 0, PASS);
            let played = digimon_engine::permanent::PermanentHandle { player: 0, index: 1 };
            assert!(!r.game.players[0].battle_area[tamer.index as usize].is_suspended);
            assert_eq!(r.effective_dp(played), Some(5000));
            assert!(r.game.effect_can_affect_from(played, 1, EffectSourceKind::Option, None));
        }
    }
}
