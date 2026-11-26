//! ST26-04 Sekkamon — Digimon Lv.5 Blue, Cost 7, DP 7000
//! (Rule) Trait: Has [Ice-Snow] Type.
//! <Iceclad>
//! [On Play] [When Digivolving] Trash any 3 digivolution cards from your opponent's Digimon. Then, 1 of their Digimon or Tamers can't suspend until their turn ends.
//! Inherited: [End of Your Turn] [Once Per Turn] Trash the bottom 2 digivolution cards of 1 of your opponent's Digimon. If this effect didn't trash, this [Ice-Snow] trait Digimon may unsuspend. (Q7578, Q7579)

use crate::alysion_common::{action, advance_turn, fixture, start};
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::enums::{Keyword, ModifierType};
use digimon_engine::selection::SelectionKind;

#[test]
fn st26_04_strips_3_any_sources_and_locks_suspend_until_opp_turn_end() {
    let mut r = start(fixture(&["ST26-04"]).hand(0, &["ST26-04"]));
    let base = r.place_stack(0, &["BLUE-LV4"]);
    let opp = r.place_stack(1, &["FILLER", "FILLER", "FILLER", "PLAIN-5K"]);
    let tamer = r.place_stack(1, &["BLUE-TAMER"]);

    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert!(r.game.has_keyword(base, Keyword::Iceclad));
    // Trash 3 sources from opponent's Digimon.
    for _ in 0..3 {
        let act = r.pending_selection().unwrap().valid_action_ids[0];
        action(&mut r, 0, act);
    }
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 1);
    // Lock 1 opponent Digimon or Tamer.
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("BLUE-TAMER", None).unwrap();
    assert!(r.game.modifiers.has(tamer, ModifierType::CannotSuspend));
    advance_turn(&mut r);
    assert!(r.game.modifiers.has(tamer, ModifierType::CannotSuspend));
    advance_turn(&mut r);
    assert!(!r.game.modifiers.has(tamer, ModifierType::CannotSuspend));
}

#[test]
fn st26_04_inherited_end_of_turn_unsuspends_ice_snow_if_nothing_trashed() {
    let mut r = start(fixture(&["ST26-04"]));
    let ice = r.place_stack(0, &["ST26-04", "ICE-MON"]);
    r.game.players[0].battle_area[ice.index as usize].is_suspended = true;
    r.place_stack(1, &["PLAIN-5K"]); // 0 sources -> nothing trashed

    action(&mut r, 0, PASS);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.choose_prompt_card("ICE-MON", None).unwrap();
    assert!(!r.game.players[0].battle_area[ice.index as usize].is_suspended);
}
