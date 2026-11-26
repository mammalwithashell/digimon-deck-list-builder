//! BD02-03 Tenkomon — Digimon Lv.4 Blue, Cost 5, DP 5000
//! Inherited: [When Attacking] [Once Per Turn] Trash the bottom digivolution card of 1 of your opponent's Digimon.

use crate::alysion_common::{action, fixture, start};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn bd02_03_inherited_when_attacking_opt_trashes_bottom_source() {
    let mut r = start(fixture(&["BD02-03"]));
    let host = r.place_stack(0, &["BD02-03", "PLAIN-5K"]);
    let opp = r.place_stack(1, &["FILLER", "FILLER", "PLAIN-5K"]);

    action(&mut r, 0, encode_attack(host.index as u16, SECURITY_TARGET));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 2);

    // OPT: second attack in same turn does not strip again.
    r.game.players[0].battle_area[host.index as usize].is_suspended = false;
    action(&mut r, 0, encode_attack(host.index as u16, SECURITY_TARGET));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 2);
}
