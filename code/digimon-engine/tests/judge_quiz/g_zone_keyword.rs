//! Cluster G — zone/keyword scoping.
//!
//! Questions (see `card-resolution.md`):
//!   Q3  Puppetmon (EX10-020) `[All Turns]` doesn't function in the breeding
//!       area, so it can digivolve into Quartzmon (BT12-057) — judge: YES.
//!   Q4  Aldamon (AD1-002) given `<Security A. +1>` by Atomic Inferno (BT4-098)
//!       and `<Security A. −1>` by Holy Flame (ST3-15): base 1 + 1 − 1 = 1
//!       check; one done ⇒ no more — judge: NO another check.  (Extends the
//!       `mid_attack_security_attack_recompute.rs` live-net-strike rule.)
//!
//! Scenarios authored under tasks §9.

#![allow(unused_imports)]

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, ModifierType, PlayerId};

/// A plain L4 Red Digimon used as security filler (3000 DP, no effect). Mirrors
/// the `filler` helper in `mid_attack_security_attack_recompute.rs`.
fn sec_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.colors = vec![CardColor::Red];
    c
}

/// Drive every pending selection by declining (PASS when offered, else the first
/// valid action). Used to clear Aldamon's optional `[End of Attack]` trigger,
/// which fires after the security checks are already counted and so cannot affect
/// the assertion.
fn drive_declining(runner: &mut DebugRunner) {
    let mut steps = 0;
    while runner.pending_selection().is_some() && steps < 100 {
        steps += 1;
        let (player, action) = {
            let sel = runner.pending_selection().unwrap();
            let id = if sel.valid_action_ids.contains(&PASS) {
                PASS
            } else {
                sel.valid_action_ids.first().copied().unwrap_or(PASS)
            };
            (sel.selecting_player, id)
        };
        runner
            .execute_action(player, action)
            .unwrap_or_else(|e| panic!("drive_declining failed at step {steps}: {e:?}"));
    }
    assert!(steps < 100, "selection loop exceeded 100 steps");
}

/// Q3 — Puppetmon (EX10-020) [All Turns] doesn't function in the breeding area, so
/// it can digivolve into Quartzmon (BT12-057). Judge: YES.
#[test]
#[ignore = "BLOCKED-CARD: needs EX10-020 (Puppetmon), BT12-057 (Quartzmon)."]
fn q3_breeding_area_effect_inactive_allows_digivolve() {}

/// Q4 — Aldamon (AD1-002) given `<Security A. +1>` by Atomic Inferno (BT4-098) and
/// `<Security A. −1>` by Holy Flame (ST3-15): base 1 + 1 − 1 = 1 check; one done ⇒
/// no more. Judge: **NO another check.**
///
/// REALIZATION — this is the live-card form of the *reduction* case that
/// `mid_attack_security_attack_recompute.rs` Test 3 deferred (it needed "a
/// security-card effect that reduces SecurityAttackChange from inside an
/// OnLoseSecurity callback" — exactly Holy Flame's `[Security]` effect).
///
/// Board: P0's Aldamon (Hybrid Lv5) is granted `<Security A. +1>` by Atomic
/// Inferno's `[Main]` (net strike would be base 1 + 1 = 2). P1's security stack is
/// `[filler, Holy Flame]` with Holy Flame on TOP (last element). Aldamon attacks
/// the player:
///   • Check 1 flips Holy Flame (an Option → no battle). Its `[Security]` effect
///     gives "all of your opponent's Digimon" (= Aldamon) `<Security A. −1>`.
///   • The engine re-reads the attacker's live Security Attack (DCGO parity, see
///     `current_security_strike`): net is now 1, and 1 card is already checked ⇒
///     the loop STOPS. No second check.
/// Judge-correct outcome: exactly 1 security card removed (1 of 2 remains).
#[test]
fn q4_security_attack_net_modifiers_one_check() {
    let p0: PlayerId = 0;
    let p1: PlayerId = 1;

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-002")
        .expect("AD1-002 (Aldamon) in embedded DSL pack")
        .dsl_card("BT4-098")
        .expect("BT4-098 (Atomic Inferno) in embedded DSL pack")
        .dsl_card("ST3-15")
        .expect("ST3-15 (Holy Flame) in embedded DSL pack")
        .add_card(sec_filler("SEC-FILLER"))
        .add_card(sec_filler("DECK-FILLER"))
        .hand(0, &["BT4-098"])
        // Top of security = last element ⇒ Holy Flame is checked first.
        .security(1, &["SEC-FILLER", "ST3-15"])
        .deck(0, &["DECK-FILLER"; 10])
        .deck(1, &["DECK-FILLER"; 10])
        .memory(10)
        .start();
    runner.skip_mulligan();

    // Aldamon is P0's attacker.
    let aldamon = runner.place_on_field(p0, "AD1-002", Some(0));

    // Play Atomic Inferno's [Main]; Aldamon is the only Hybrid Digimon, so the
    // (mandatory) target selection has exactly one legal pick.
    runner.play(p0, 0);
    let pick = runner
        .pending_selection()
        .expect("Atomic Inferno [Main] surfaces a Hybrid-Digimon selection")
        .valid_action_ids
        .first()
        .copied()
        .expect("Aldamon is a legal Hybrid target");
    runner.execute_action(p0, pick).expect("select Aldamon");
    runner.auto_resolve().expect("resolve Atomic Inferno grants");

    // Pre-attack: Aldamon carries the +1 from Atomic Inferno (would check 2).
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(aldamon, ModifierType::SecurityAttackChange),
        1,
        "Atomic Inferno's [Main] granted Aldamon <Security A. +1>"
    );
    assert_eq!(runner.security_count(p1), 2, "P1 security starts at 2");

    let _ = runner.game.attack_player(aldamon, p1, false);
    drive_declining(&mut runner);

    // Diagnostic: Holy Flame's [Security] −1 must have landed on Aldamon, netting
    // the live Security Attack back to 0 (= base-1 strike). If this is still +1 the
    // failure is "Holy Flame didn't target the attacker", not the recompute.
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(aldamon, ModifierType::SecurityAttackChange),
        0,
        "Holy Flame's [Security] <Security A. −1> must apply to the attacker (net 0)"
    );

    // Judge ruling: exactly ONE security card checked — the −1 is observed on the
    // mid-attack recompute, so no second check happens (1 of 2 remains).
    assert_eq!(
        runner.security_count(p1),
        1,
        "net +1−1 = 1 check; the mid-attack recompute must NOT check a second card \
         (judge: NO another check)"
    );
}

/// Q4 CONTROL — proves the pin above is not a false pass. With Atomic Inferno's
/// `<Security A. +1>` on Aldamon but NO Holy Flame in security, the attacker's
/// strike is 2 and it checks BOTH security cards. If the granted +1 were silently
/// ignored by the attack loop, this control would check only 1 (leaving 1) and so
/// would fail — guaranteeing that Q4's "1 remaining" is genuinely caused by Holy
/// Flame's mid-attack reduction, not by the +1 never registering.
#[test]
fn q4_control_atomic_inferno_plus_one_alone_checks_two() {
    let p0: PlayerId = 0;
    let p1: PlayerId = 1;

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-002")
        .expect("AD1-002 (Aldamon) in embedded DSL pack")
        .dsl_card("BT4-098")
        .expect("BT4-098 (Atomic Inferno) in embedded DSL pack")
        .add_card(sec_filler("SEC-FILLER"))
        .add_card(sec_filler("DECK-FILLER"))
        .hand(0, &["BT4-098"])
        // Two plain fillers, no Holy Flame: nothing reduces the strike mid-attack.
        .security(1, &["SEC-FILLER", "SEC-FILLER"])
        .deck(0, &["DECK-FILLER"; 10])
        .deck(1, &["DECK-FILLER"; 10])
        .memory(10)
        .start();
    runner.skip_mulligan();

    let aldamon = runner.place_on_field(p0, "AD1-002", Some(0));

    runner.play(p0, 0);
    let pick = runner
        .pending_selection()
        .expect("Atomic Inferno [Main] surfaces a Hybrid-Digimon selection")
        .valid_action_ids
        .first()
        .copied()
        .expect("Aldamon is a legal Hybrid target");
    runner.execute_action(p0, pick).expect("select Aldamon");
    runner.auto_resolve().expect("resolve Atomic Inferno grants");

    assert_eq!(runner.security_count(p1), 2, "P1 security starts at 2");

    let _ = runner.game.attack_player(aldamon, p1, false);
    drive_declining(&mut runner);

    // Strike 2 against a 2-card stack with no reduction ⇒ both checked ⇒ 0 remain.
    assert_eq!(
        runner.security_count(p1),
        0,
        "Atomic Inferno's granted <Security A. +1> must extend the loop to 2 checks \
         (control for q4_security_attack_net_modifiers_one_check)"
    );
}
