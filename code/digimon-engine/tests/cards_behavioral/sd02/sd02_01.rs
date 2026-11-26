//! SD02-01 Viximon — Digi-Egg Lv.2 Blue
//! Inherited: [When Attacking] [Once Per Turn] If this Digimon has the [ALYSION] trait, trash the bottom digivolution card of 1 of your opponent's Digimon.

use crate::alysion_common::{action, fixture, start};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn sd02_01_inherited_when_attacking_opt_strips_bottom_only_with_alysion_trait() {
    let mut r = start(fixture(&["SD02-01"]));
    let alysion = r.place_stack(0, &["SD02-01", "BLUE-ALYSION"]);
    let plain = r.place_stack(0, &["SD02-01", "PLAIN-5K"]);
    let opp = r.place_stack(1, &["FILLER", "FILLER", "PLAIN-5K"]);

    // Non-[ALYSION] host does NOT trigger.
    action(&mut r, 0, encode_attack(plain.index as u16, SECURITY_TARGET));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 3);

    // [ALYSION] host triggers and trashes bottom digivolution card.
    action(&mut r, 0, encode_attack(alysion.index as u16, SECURITY_TARGET));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 2);

    // OPT: unsuspending and attacking again in same turn does not trigger a second time.
    r.game.players[0].battle_area[alysion.index as usize].is_suspended = false;
    action(&mut r, 0, encode_attack(alysion.index as u16, SECURITY_TARGET));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 2);
}
