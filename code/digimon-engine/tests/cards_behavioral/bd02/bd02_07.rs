//! BD02-07 Yukinamon — Digimon Lv.6 Blue, Cost 12, DP 11000
//! [When Digivolving] Trash the bottom 3 digivolution cards of 1 of your opponent's Digimon. Afterwards, until the end of your opponent's turn, this Digimon gains <Blocker>.
//! [All Turns] Other than against Security Digimon, compare the number of digivolution cards instead of DP in this Digimon's battles.

use crate::alysion_common::{action, advance_turn, fixture, start};
use digimon_engine::action::space::{encode_attack, encode_digivolve};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn bd02_07_strips_3_gains_blocker_until_opp_turn_end_and_battles_by_sources() {
    let mut r = start(fixture(&["BD02-07"]).hand(0, &["BD02-07"]));
    let base = r.place_stack(0, &["FILLER", "BLUE-LV5"]); // 2 sources after digivolve
    let opp = r.place_stack(1, &["FILLER", "FILLER", "FILLER", "PLAIN-12K"]); // 3 sources, 12000 DP

    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-12K", None).unwrap();
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 1);
    assert!(r.game.has_keyword(base, Keyword::Blocker));

    // Battle: Yukinamon (11000 DP, 2 sources) vs PLAIN-12K (12000 DP, 0 sources) -> Yukinamon wins!
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    action(&mut r, 0, encode_attack(base.index as u16, opp.index as u16));
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.battle_area_size(1), 0);

    // Blocker persists through opponent's turn, expires at start of our next turn.
    advance_turn(&mut r);
    assert!(r.game.has_keyword(base, Keyword::Blocker));
    advance_turn(&mut r);
    assert!(!r.game.has_keyword(base, Keyword::Blocker));
}
