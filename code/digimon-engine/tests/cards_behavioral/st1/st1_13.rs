//! ST1-13 Shadow Wing — Option, Red, Cost 1.
//!
//! # Printed text (official Bandai DB, data/card_bundles/ST1-13.md)
//!
//! ```text
//! [Main] 1 of your Digimon gets +3000 DP for the turn.
//! [Security] All of your Digimon gain <Security Attack +1> (This Digimon
//! checks 1 additional security card) until the end of your next turn.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST1/Red/ST1_13.cs
//!   SecuritySkill → CardEffectCommons.ChangeDigimonSAttackPlayerEffect(
//!       permanentCondition: IsPermanentExistsOnOwnerBattleAreaDigimon,
//!       changeValue: 1, effectDuration: UntilOwnerTurnEnd, ...)
//!   — a PLAYER-scoped continuous effect whose predicate is re-evaluated live,
//!   so Digimon that enter the battle area during the window are covered too.
//!
//! # Rules basis
//! general_rule.pdf 15-11-2-2 / 15-11-2-3 (docs/digimon-rules/digest.md §
//! Targets): "all" is OVERALL processing — it does not choose targets and it
//! "also catches later-added matches"; overall processing tracks the LIVE set.
//! A one-shot scan at resolution time is therefore wrong for this card: a
//! Digimon played while the grant window is open must also gain
//! <Security Attack +1>.

use digimon_engine::debug_runner::DebugRunner;

/// P1 owns ST1-13 as their sole security card and one Digimon on the field.
/// P0 attacks P1 with a Digimon, flipping ST1-13 and firing its [Security]
/// effect. Extra ST1-03 copies sit in P1's hand/deck for the follow-up plays.
fn security_flip_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST1-13")
        .expect("ST1-13 in embedded DSL pack")
        .dsl_card("ST1-03")
        .expect("ST1-03 in embedded DSL pack")
        .memory(10)
        .security(1, &["ST1-13"])
        .hand(1, &["ST1-03"])
        .deck(1, &["ST1-03", "ST1-03", "ST1-03"])
        .deck(0, &["ST1-03", "ST1-03", "ST1-03"])
        .start()
}

fn resolve_all_pending(runner: &mut DebugRunner) {
    for _ in 0..8 {
        if runner.pending_selection().is_none() {
            return;
        }
        runner.auto_resolve().expect("pending selection resolves");
    }
}

/// Baseline: the [Security] grant reaches a Digimon that was ALREADY on the
/// owner's field when the effect resolved.
#[test]
fn st1_13_security_grants_security_attack_to_digimon_on_field_at_resolution() {
    let mut runner = security_flip_runner();
    let attacker = runner.place_on_field(0, "ST1-03", Some(0));
    let defender_digimon = runner.place_on_field(1, "ST1-03", Some(0));

    runner.attack_player(attacker, 1, false);
    resolve_all_pending(&mut runner);

    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.effective_security_strike(defender_digimon),
        2,
        "a Digimon on the owner's field when ST1-13's [Security] resolved must \
         check 1 additional security card (base 1 + 1)"
    );
}

/// Regression (user report): "blanket effects aren't applying to newly played
/// Digimon". Rule 15-11-2-2/-3 — "all of your Digimon gain X until <window>"
/// is overall processing over the LIVE set, and DCGO implements it as a
/// player-scoped continuous effect. A Digimon the owner plays DURING the
/// window (their next turn) must also have <Security Attack +1>.
#[test]
fn st1_13_security_grant_applies_to_digimon_played_during_the_window() {
    let mut runner = security_flip_runner();
    let attacker = runner.place_on_field(0, "ST1-03", Some(0));
    let _on_field = runner.place_on_field(1, "ST1-03", Some(0));

    runner.attack_player(attacker, 1, false);
    resolve_all_pending(&mut runner);

    // Pass the turn to the ST1-13 owner (P1) — still inside "until the end of
    // your next turn".
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "it is now the owner's turn");
    // Normalize phase + memory so the follow-up play is unconditionally legal.
    runner.set_phase(digimon_engine::enums::GamePhase::Main);
    runner.game_mut().memory = 10;

    // P1 plays a NEW Digimon from hand during the window.
    let field_index = runner.play(1, 0).expect("P1 plays ST1-03 from hand");
    let newcomer = runner.perm_handle(1, field_index);

    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.effective_security_strike(newcomer),
        2,
        "a Digimon played while ST1-13's [Security] window is open must also \
         gain <Security Attack +1> (overall processing tracks the live set — \
         rule 15-11-2-2/-3; DCGO ChangeDigimonSAttackPlayerEffect)"
    );
}

/// The grant window closes at the end of the owner's next turn: afterwards
/// every own Digimon is back to the base single check.
#[test]
fn st1_13_security_grant_expires_at_end_of_owners_next_turn() {
    let mut runner = security_flip_runner();
    let attacker = runner.place_on_field(0, "ST1-03", Some(0));
    let on_field = runner.place_on_field(1, "ST1-03", Some(0));

    runner.attack_player(attacker, 1, false);
    resolve_all_pending(&mut runner);

    // End P0's turn → owner's (P1's) turn. The grant is still live.
    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.effective_security_strike(on_field),
        2,
        "the grant lasts through the owner's next turn"
    );

    // End the owner's turn → window closed.
    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.effective_security_strike(on_field),
        1,
        "the <Security Attack +1> grant expires at the end of the owner's next turn"
    );
}
