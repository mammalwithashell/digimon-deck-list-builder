//! SD02-05 Yukinamon — Digimon Lv.6 Blue, Cost 12, DP 10000
//! [Your Turn] This Digimon can also attack your opponent's unsuspended Digimon with fewer digivolution cards than this Digimon.
//! [All Turns] Other than against Security Digimon, compare the number of digivolution cards instead of DP in this Digimon's battles.

use crate::alysion_common::{action, fixture, start};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};

#[test]
fn sd02_05_attacks_unsuspended_fewer_sources_and_battles_by_source_count_not_security() {
    let mut r = start(fixture(&["SD02-05"]).security(1, &["PLAIN-12K"]));
    // Yukinamon: 10000 DP, 2 digivolution cards.
    let yuki = r.place_stack(0, &["FILLER", "FILLER", "SD02-05"]);
    // Opponent 1: 12000 DP, 1 digivolution card (unsuspended) -> legal attack target AND Yukinamon wins battle (2 > 1)!
    let opp_fewer = r.place_stack(1, &["FILLER", "PLAIN-12K"]);
    // Opponent 2: 12000 DP, 2 digivolution cards (unsuspended) -> NOT a legal attack target.
    let opp_equal = r.place_stack(1, &["FILLER", "FILLER", "PLAIN-12K"]);

    r.game.tick_declarative_effects();
    let mask = build_action_mask(&r.game, 0);
    assert!(mask[encode_attack(yuki.index as u16, opp_fewer.index as u16) as usize] > 0.5);
    assert!(mask[encode_attack(yuki.index as u16, opp_equal.index as u16) as usize] < 0.5);

    action(&mut r, 0, encode_attack(yuki.index as u16, opp_fewer.index as u16));
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.battle_area_size(1), 1); // opp_fewer deleted despite 12000 > 10000 DP!

    // Against Security Digimon (12000 DP), normal DP comparison applies -> Yukinamon (10000 DP) is deleted.
    r.game.players[0].battle_area[0].is_suspended = false;
    action(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert_eq!(r.battle_area_size(0), 0);
}
