//! BD01-05 Reptilimon — Digimon Lv.5 Red, Cost 7, DP 6000
//! Inherited: [Your Turn] [Once Per Turn] When any of your opponent's Digimon are deleted, this Digimon gains <Security A. +1> for the turn.

use crate::alysion_common::{advance_turn, delete_perm, fixture, start};

#[test]
fn bd01_05_inherited_gains_sa_plus_1_opt_on_opp_digimon_deletion_your_turn() {
    let mut r = start(fixture(&["BD01-05"]));
    let host = r.place_stack(0, &["BD01-05", "PLAIN-12K"]);
    let opp1 = r.place_stack(1, &["PLAIN-2K"]);
    let _opp2 = r.place_stack(1, &["PLAIN-3K"]);

    assert_eq!(r.game.effective_security_strike(host), 1);
    delete_perm(&mut r, opp1);
    assert_eq!(r.game.effective_security_strike(host), 2);

    // OPT: second deletion in same turn does NOT grant another SA+1.
    delete_perm(&mut r, digimon_engine::permanent::PermanentHandle { player: 1, index: 0 });
    assert_eq!(r.game.effective_security_strike(host), 2);

    // Expires at end of turn, and opponent-turn deletions do not trigger.
    advance_turn(&mut r);
    assert_eq!(r.game.effective_security_strike(host), 1);
    let opp3 = r.place_stack(1, &["PLAIN-2K"]);
    delete_perm(&mut r, opp3);
    assert_eq!(r.game.effective_security_strike(host), 1);
}
