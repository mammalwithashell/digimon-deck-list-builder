//! Security Digimon scoping — surface R. Ledger rows OR-4, OR-5.
//!
//! OR-4: "Do [On Deletion] effects activate when Security Digimon are deleted?
//! No. Security Digimon cannot activate any effects unless they are [Security]
//! effects." (General Rules/FAQ, Other Rulings.)
//!
//! Vehicle: EX9-027 Kokeshimon — "[On Deletion] By trashing 1 card in hand, 1
//! of your opponent's Digimon gets -4000 DP for the turn." If this wrongly fired
//! when EX9-027 is deleted during a security check, the −4000 would land on the
//! ATTACKER (the security holder's opponent) and a hand card would be trashed —
//! both directly observable.

#![allow(unused_imports)]

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, ModifierType, PlayerId};
use digimon_engine::modifiers::ModifierEntry;

/// OR-4 — a Digimon deleted as a SECURITY Digimon (lost a security check battle)
/// must NOT activate its non-[Security] [On Deletion] effect.
#[test]
fn or4_security_digimon_on_deletion_does_not_fire() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX9-027")
        .expect("EX9-027 (Kokeshimon) in embedded DSL pack")
        .dsl_card("AD1-001")
        .expect("AD1-001 (Greymon) in embedded DSL pack")
        // Defender (seat 1) holds EX9-027 in security and a hand card (the
        // On-Deletion trash cost), so the effect's condition WOULD be satisfiable.
        .security(1, &["EX9-027"])
        .hand(1, &["AD1-001"])
        .start();

    let p = r.turn_player();
    assert_eq!(p, 0, "attacker is the turn player (seat 0)");
    let opp: PlayerId = 1;

    // Attacker (5000 DP > EX9-027's 4000) — Rush-granted so it can attack.
    let atk = r.place_on_field(0, "AD1-001", Some(0));
    r.game.modifiers.add(
        atk,
        ModifierEntry::simple(ModifierType::GrantRush, 0, Expiry::Permanent, 0),
    );

    let atk_dp_before = r.effective_dp(atk);
    let opp_hand_before = r.hand_size(opp);
    let opp_trash_before = r.trash_size(opp);

    // Attack the player → security check reveals EX9-027 → it loses the security
    // battle and is deleted.
    let _ = r.attack_player(atk, opp, false);
    let _ = r.auto_resolve();

    // Precondition the test depends on: EX9-027 really WAS checked and disposed
    // (security consumed, card now in trash) — otherwise "On-Deletion didn't
    // fire" would be vacuously true.
    assert_eq!(
        r.security_count(opp),
        0,
        "precondition: the security check must have consumed EX9-027"
    );
    assert!(
        r.trash_size(opp) > opp_trash_before,
        "precondition: the checked security Digimon must have been disposed to trash"
    );

    assert_eq!(
        r.effective_dp(atk),
        atk_dp_before,
        "FAQ OR-4: a security Digimon's [On Deletion] (−4000 DP to the attacker) must NOT fire"
    );
    assert_eq!(
        r.hand_size(opp),
        opp_hand_before,
        "FAQ OR-4: the [On Deletion] trash cost must NOT be paid by a security Digimon"
    );
}
