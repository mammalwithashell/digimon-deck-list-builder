//! BD02-01 Viximon — Digi-Egg Lv.2 Blue
//! Inherited: [All Turns] [Once Per Turn] When effects trash your opponent's Digimon's digivolution cards, if this Digimon has the [ALYSION] trait, <Draw 1>

use crate::alysion_common::{action, fixture, start};
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::selection::SelectionKind;

#[test]
fn bd02_01_inherited_draws_opt_when_effects_trash_opponent_digimon_sources() {
    let mut r = start(fixture(&["BD02-01", "SD02-02"]).hand(0, &["SD02-02", "SD02-02"]));
    let _host = r.place_stack(0, &["BD02-01", "BLUE-ALYSION"]);
    r.place_stack(1, &["FILLER", "FILLER", "PLAIN-5K"]);
    let hand_before = r.hand_size(0);

    // Playing SD02-02 trashes 1 bottom source of opponent's Digimon -> BD02-01 triggers Draw 1!
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.hand_size(0), hand_before); // -1 played +1 drawn

    // OPT: second SD02-02 in same turn trashes another source, does NOT draw again.
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.hand_size(0), hand_before - 1);
}
