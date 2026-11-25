//! BT27-088: official Bandai card text.
//!
//! ### Standard digivolve cost circles
//!   - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! ## Printed text (official)
//! ### Effect
//! [Start of Your Main Phase] If you have a Digimon with the [Ice-Snow] trait, gain 1 memory. [All Turns] When your Digimon are played or digivolve, if any of them have the [Ice-Snow] trait, by suspending this Tamer, trash the bottom 2 digivolution cards from 1 of your opponent's Digimon. After, if they don't have a Digimon with digivolution cards, gain 1 memory.
//!
//! ### Security Effect
//! [Security] Play this card without paying the cost.
//!
//! ## Official Q&A
//! - No, you can't. If you don't suspend this card, you can't process the part after "after" in its [All Turns] effect.

use super::bt27_gemmon_fixtures::{fixture, play_from_security};
use super::common::{action, ids, start};
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::selection::SelectionKind;

#[test]
fn valner_can_suspend_for_an_ice_snow_play_and_gains_memory_after_trash() {
    let mut r = start(fixture(&["BT27-088"]).hand(0, &["ICE"]));
    let valner = r.place_stack(0, &["BT27-088"]);
    r.place_stack(1, &["FILLER", "ALYSION"]);
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
    r.execute_branch(0).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("ALYSION", None).unwrap();
    assert!(r.game.players[0].battle_area[valner.index as usize].is_suspended);
    assert_eq!(ids(&r, &r.game.players[1].trash), ["FILLER"]);
    assert_eq!(r.memory(), 8);
}

#[test]
fn valner_main_phase_checks_ice_snow_trait() {
    for (top, expected) in [("ICE", 5), ("FILLER", 4)] {
        let mut r = fixture(&["BT27-088"]).memory(4).start();
        r.place_stack(0, &["BT27-088"]);
        r.place_stack(0, &[top]);
        r.game.enter_main_phase();
        assert_eq!(r.memory(), expected);
    }
}

#[test]
fn valner_can_decline_to_suspend_and_does_not_gain_memory_when_sources_remain() {
    for accept in [false, true] {
        let mut r = start(fixture(&["BT27-088"]).hand(0, &["ICE"]));
        r.place_stack(0, &["BT27-088"]);
        r.place_stack(1, &["FILLER", "FILLER", "FILLER", "ALYSION"]);
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
        if accept {
            r.execute_branch(0).unwrap();
            r.choose_prompt_card("ALYSION", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.game.players[0].battle_area[0].is_suspended, accept);
        assert_eq!(r.game.players[1].trash.len(), if accept { 2 } else { 0 });
        assert_eq!(r.memory(), 7);
    }
}

#[test]
fn security_plays_valner_without_paying() {
    play_from_security("BT27-088");
}

#[test]
fn valner_can_suspend_to_gain_memory_without_any_opposing_digimon() {
    let mut r = start(fixture(&["BT27-088"]).hand(0, &["ICE"]));
    r.place_stack(0, &["BT27-088"]);
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
    r.execute_branch(0).unwrap();
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(r.memory(), 8);
}
