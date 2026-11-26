//! BD02-04 Sekkamon — Digimon Lv.5 Blue, Cost 7, DP 6000
//! Inherited: [When Attacking] [Once Per Turn] If your opponent has no Digimon with digivolution cards, this Digimon unsuspends. (Q7584)

use crate::alysion_common::{action, fixture, start};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};

#[test]
fn bd02_04_inherited_unsuspends_opt_when_opponent_has_no_sourced_digimon() {
    // Opponent has a Digimon WITH a digivolution card -> stays suspended.
    let mut r = start(fixture(&["BD02-04"]));
    let host = r.place_stack(0, &["BD02-04", "PLAIN-5K"]);
    r.place_stack(1, &["FILLER", "PLAIN-5K"]);
    action(&mut r, 0, encode_attack(host.index as u16, SECURITY_TARGET));
    assert!(r.game.players[0].battle_area[host.index as usize].is_suspended);

    // Opponent has no Digimon at all (Q7584) -> unsuspends once per turn!
    let mut r2 = start(fixture(&["BD02-04"]));
    let host2 = r2.place_stack(0, &["BD02-04", "PLAIN-5K"]);
    action(&mut r2, 0, encode_attack(host2.index as u16, SECURITY_TARGET));
    assert!(!r2.game.players[0].battle_area[host2.index as usize].is_suspended);
    // Second attack same turn -> stays suspended (OPT).
    action(&mut r2, 0, encode_attack(host2.index as u16, SECURITY_TARGET));
    assert!(r2.game.players[0].battle_area[host2.index as usize].is_suspended);
}
