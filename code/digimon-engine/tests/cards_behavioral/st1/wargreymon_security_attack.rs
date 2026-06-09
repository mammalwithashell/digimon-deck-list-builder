//! ST1-11 WarGreymon — `<Security A.>` count + the "no win on overflow" rule.
//!
//! WarGreymon's printed effect:
//!   [Your Turn] For every 2 digivolution cards this Digimon has, it gains
//!   <Security A. +1>. (It checks 1 additional security card per +1.)
//!
//! These tests use the natural ST-1 Gaia Red line the user described:
//!   Agumon (ST1-03, Lv3) → Coredramon (ST1-06, Lv4) →
//!   MetalGreymon (ST1-09, Lv5) → WarGreymon (ST1-11, Lv6).
//!
//! That is a 4-card stack with **3 digivolution materials**, so
//! `floor(3 / 2) = 1` extra check → total checks = base 1 + 1 = **2**.
//!
//! # Rule reference — winning via security
//!
//! You win by attacking a player **who has 0 security cards at the moment the
//! attack connects** (general_rule.pdf 11-5-1-2 "Security knockout"). The
//! security-check loop itself NEVER ends the game: with `<Security A. +N>`, if
//! the defender runs out of security mid-multi-check the extra checks simply do
//! not happen — the attack ends and the game continues.
//!
//! DCGO authority (`AttackProcess.cs:418-431` + `CardController.cs:3950-4252`):
//! the 0-security win is checked ONCE before `ISecurityCheck.SecurityCheck()`;
//! inside the loop, running out of security is an `else { break; }` — no win.

use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{Expiry, Keyword};

/// Natural ST-1 WarGreymon stack: Agumon → Coredramon → MetalGreymon →
/// WarGreymon (bottom → top). 3 materials → 2 security checks.
const WARGREYMON_LINE: [&str; 4] = ["ST1-03", "ST1-06", "ST1-09", "ST1-11"];

fn wargreymon_builder() -> DebugRunnerBuilder {
    let mut builder = DebugRunner::builder();
    // The WarGreymon line + a vanilla Digimon (Biyomon, 3000 DP) used as
    // inert security filler — it battles WarGreymon (12000 DP), loses, and is
    // trashed without parking on any [Security] effect.
    for id in [
        "ST1-01", "ST1-02", "ST1-03", "ST1-05", "ST1-06", "ST1-07", "ST1-09", "ST1-11",
    ] {
        builder = builder
            .dsl_card(id)
            .unwrap_or_else(|err| panic!("{id} must be in embedded DSL pack: {err}"));
    }
    builder
}

/// Bug #1: WarGreymon on the natural 3-material stack must check exactly 2
/// security cards — `1 + floor(3 / 2)` — not 4.
#[test]
fn wargreymon_st1_natural_stack_checks_exactly_two_security() {
    let mut runner = wargreymon_builder()
        .security(1, &["ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-02"])
        .start();
    let wargreymon = runner.place_stack(0, &WARGREYMON_LINE);

    // Sanity: the effective strike is base 1 + floor(3/2) = 2.
    assert_eq!(
        runner.game.dynamic_security_attack_aura_bonus(wargreymon),
        Some(2),
        "WarGreymon's BASE-INCLUSIVE security_attack_fn on a 3-material stack is \
         1 + floor(3/2) = 2"
    );

    let result = runner.attack_player(wargreymon, 1, false);

    assert_eq!(
        runner.security_count(1),
        3,
        "WarGreymon checks exactly 2 security cards (5 - 2 = 3 remain), not 4"
    );
    assert!(!runner.game_over(), "the opponent still has security");
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
}

/// Bug #1 (with the digi-egg): the real stack the user played also had the
/// ST1-01 Koromon digi-egg at the bottom — Koromon → Agumon → Coredramon →
/// MetalGreymon → WarGreymon. That is **4 digivolution cards** (the egg counts),
/// so `floor(4 / 2) = 2` extra checks → total = base 1 + 2 = **3**, not 4.
#[test]
fn wargreymon_st1_egg_stack_checks_exactly_three_security() {
    let mut runner = wargreymon_builder()
        .security(1, &["ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-02"])
        .start();
    // Koromon egg at the bottom → 4 digivolution materials under WarGreymon.
    let wargreymon = runner.place_stack(0, &["ST1-01", "ST1-03", "ST1-06", "ST1-09", "ST1-11"]);

    assert_eq!(
        runner.game.dynamic_security_attack_aura_bonus(wargreymon),
        Some(3),
        "WarGreymon over a Koromon-rooted 4-material stack is 1 + floor(4/2) = 3"
    );

    let result = runner.attack_player(wargreymon, 1, false);

    assert_eq!(
        runner.security_count(1),
        2,
        "WarGreymon checks exactly 3 security cards (5 - 3 = 2 remain), not 4"
    );
    assert!(!runner.game_over(), "the opponent still has security");
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
}

/// The user's actual on-board stack (desktop screenshot): the natural ST-1
/// line uses ST1-07 Greymon (cost 5, Lv4) as the Lv4 — NOT Coredramon (cost 4).
/// With the Koromon egg that is 4 materials, so WarGreymon's own formula grants
/// +2 AND Greymon's inherited `<Security A. +1>` grants another +1:
///   base 1 + floor(4/2) = 2 + 1 = **4** checks.
///
/// Both the engine's combat count AND the value the UI renders must be 4. This
/// is the screenshot regression: combat correctly checked 4, but the desktop /
/// browser DTOs showed "Security Attack: 2" because the display computation
/// omitted WarGreymon's `security_attack_fn` formula aura — making correct
/// gameplay look broken. `Game::effective_security_strike` is the single
/// read-only source both display paths now use.
#[test]
fn wargreymon_greymon_stack_security_attack_is_four_in_combat_and_display() {
    let mut runner = wargreymon_builder()
        .security(1, &["ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-02"])
        .start();
    // Koromon → Agumon → Greymon (ST1-07) → MetalGreymon → WarGreymon.
    let wargreymon = runner.place_stack(0, &["ST1-01", "ST1-03", "ST1-07", "ST1-09", "ST1-11"]);

    // The UI serializes already-ticked state (declarative grants materialized),
    // so reproduce that before reading the display value.
    runner.game.tick_declarative_effects();

    // The value the UI renders must equal the real combat strike (4).
    assert_eq!(
        runner.game.effective_security_strike(wargreymon),
        4,
        "WarGreymon (4 materials → +2) + Greymon inherited <Security A. +1> = 4"
    );

    let result = runner.attack_player(wargreymon, 1, false);

    assert_eq!(
        runner.security_count(1),
        1,
        "combat checks exactly 4 security (5 - 4 = 1 remains)"
    );
    assert!(!runner.game_over());
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
}

/// The user's *actual* on-board stack (desktop screenshot): the Lv4 is the
/// vanilla **Birdramon (ST1-05)** — empty inherited effect, no `<Security A.>`.
/// Koromon → Agumon → Birdramon → MetalGreymon → WarGreymon is 4 materials with
/// NO keyword source, so a clean stack checks exactly `1 + floor(4/2) = 3`.
///
/// This pins the exact-stack expectation: any count above 3 in a real game is
/// an EXTRA `<Security A.>` coming from outside the stack (a Tamer/effect or a
/// stale modifier), not from these cards.
#[test]
fn wargreymon_st1_birdramon_egg_stack_checks_exactly_three() {
    let mut runner = wargreymon_builder()
        .security(1, &["ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-02"])
        .start();
    let wargreymon = runner.place_stack(0, &["ST1-01", "ST1-03", "ST1-05", "ST1-09", "ST1-11"]);
    runner.game.tick_declarative_effects();

    // No keyword "+1" — Birdramon is vanilla. The whole value is the formula.
    assert_eq!(
        runner.game.security_attack_keyword_bonus(wargreymon),
        0,
        "no card in the stack grants a flat <Security A.> keyword"
    );
    assert_eq!(
        runner.game.effective_security_strike(wargreymon),
        3,
        "WarGreymon formula only: 1 + floor(4/2) = 3"
    );

    let result = runner.attack_player(wargreymon, 1, false);
    assert_eq!(
        runner.security_count(1),
        2,
        "a clean Birdramon stack checks exactly 3 (5 - 3 = 2 remain)"
    );
    assert!(!runner.game_over());
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
}

/// The complete screenshot scenario. The vanilla Birdramon line (4 materials →
/// WarGreymon formula +2 = base 3) PLUS an external `<Security A. +1>` — the
/// keyword ST1-13 Shadow Wing's `[Security]` effect grants to all your Digimon
/// "until the end of your next turn" (and which rides through digivolution onto
/// WarGreymon). 3 + 1 = 4 checks; the opponent loses 4 of 5 security, and the
/// UI must render 4 — the old display showed 2 because it omitted WarGreymon's
/// own formula aura.
#[test]
fn wargreymon_birdramon_stack_with_shadow_wing_grant_checks_four() {
    let mut runner = wargreymon_builder()
        .security(1, &["ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-02"])
        .start();
    let wargreymon = runner.place_stack(0, &["ST1-01", "ST1-03", "ST1-05", "ST1-09", "ST1-11"]);

    // Shadow Wing [Security]: grant SecurityAttackPlus(1) to the user's Digimon.
    runner.game.modifiers.grant_keyword(
        wargreymon,
        Keyword::SecurityAttackPlus(1),
        Expiry::EndOfYourNextTurn,
        0,
    );
    runner.game.tick_declarative_effects();

    // Display value (UI) must equal the real combat strike: formula 3 + grant 1.
    assert_eq!(
        runner.game.effective_security_strike(wargreymon),
        4,
        "WarGreymon formula (4 materials → 3) + Shadow Wing <Security A. +1> = 4"
    );

    let result = runner.attack_player(wargreymon, 1, false);
    assert_eq!(
        runner.security_count(1),
        1,
        "combat checks exactly 4 security (5 - 4 = 1 remains) — matches the screenshot"
    );
    assert!(!runner.game_over());
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
}

/// Bug #2: with `<Security A. +1>`, attacking a player who has exactly 1
/// security checks that 1 card and ends — the "extra" check finds an empty
/// stack and does NOT win the game. You can only win by attacking a player who
/// already has 0 security.
#[test]
fn wargreymon_security_attack_overflow_does_not_win_with_one_security() {
    let mut runner = wargreymon_builder().security(1, &["ST1-02"]).start();
    let wargreymon = runner.place_stack(0, &WARGREYMON_LINE);

    let result = runner.attack_player(wargreymon, 1, false);

    assert_eq!(
        runner.security_count(1),
        0,
        "the single security card is checked and trashed"
    );
    assert!(
        !runner.game_over(),
        "overflowing <Security A. +1> past an emptied stack must NOT win the game"
    );
    assert_eq!(runner.winner(), None, "no winner is declared");
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "the attack ends having survived the security check, not GameWon"
    );
}

/// Control: attacking a player who already has 0 security IS a win (the
/// genuine security knockout). This guards against an over-broad fix to bug #2.
#[test]
fn wargreymon_attacking_zero_security_player_wins() {
    let mut runner = wargreymon_builder().security(1, &[]).start();
    let wargreymon = runner.place_stack(0, &WARGREYMON_LINE);

    let result = runner.attack_player(wargreymon, 1, false);

    assert!(
        runner.game_over(),
        "attacking a player with 0 security is a direct hit and wins the game"
    );
    assert_eq!(runner.winner(), Some(0));
    assert_eq!(result, AttackResult::GameWon);
}
