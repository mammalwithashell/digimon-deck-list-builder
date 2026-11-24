//! BT27-086: official Bandai card text.
//!
//! ### Standard digivolve cost circles
//!   - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! ## Printed text (official)
//! ### Effect
//! [Start of Your Main Phase] If you have a Digimon with [Gemmon] in its text, gain 1 memory. [Your Turn] When your Digimon are played or digivolve, if any of them have the [ALYSION] trait, by suspending this Tamer, 1 of your Digimon gets +3000 DP for the turn. After, if you have a Digimon with 8000 DP or more, ＜Draw 1＞.
//!
//! ### Security Effect
//! [Security] Play this card without paying the cost.
//!
//! ## Official Q&A
//! - A [Main] effect shown on the DUAL card's Option side will activate. Activated effects on the Option side are treated as Option card effects.

use super::bt27_gemmon_fixtures::{fixture, play_from_security};
use super::common::{action, start};
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::selection::SelectionKind;

#[test]
fn kanata_can_suspend_for_an_alysion_play_to_buff_and_draw() {
    let mut r = start(fixture(&["BT27-086"]).hand(0, &["ALYSION"]));
    let kanata = r.place_stack(0, &["BT27-086"]);
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
    r.execute_branch(0).unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("ALYSION", None).unwrap();
    assert!(r.game.players[0].battle_area[kanata.index as usize].is_suspended);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn kanata_main_phase_checks_gemmon_text() {
    for (top, expected) in [("BT27-006", 5), ("FILLER", 4)] {
        let mut r = fixture(&["BT27-006", "BT27-086"]).memory(4).start();
        r.place_stack(0, &["BT27-086"]);
        r.place_stack(0, &[top]);
        r.game.enter_main_phase();
        assert_eq!(r.memory(), expected);
    }
}

#[test]
fn kanata_can_decline_to_suspend_and_does_not_draw_below_8000() {
    for accept in [false, true] {
        let mut r = start(fixture(&["BT27-086"]).hand(0, &["ALYSION"]));
        r.place_stack(0, &["BT27-086"]);
        let ally = r.place_stack(0, &["FILLER"]);
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
        if accept {
            r.execute_branch(0).unwrap();
            r.choose_prompt_card("FILLER", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.game.players[0].battle_area[0].is_suspended, accept);
        assert_eq!(r.effective_dp(ally), Some(if accept { 5000 } else { 2000 }));
        assert_eq!(r.hand_size(0), 0);
    }
}

#[test]
fn security_plays_kanata_without_paying() {
    play_from_security("BT27-086");
}
