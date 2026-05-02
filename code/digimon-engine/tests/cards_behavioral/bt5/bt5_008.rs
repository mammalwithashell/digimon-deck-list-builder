//! BT5-008 Gaossmon — Digimon, Lv.3, DP 2000, Cost 3, Red.
//! Traits: Reptile.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [Your Turn] Your other [Gaossmon] all get +3000 DP.
//! [Opponent's Turn] Your opponent can't reduce digivolution costs.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT5/Red/BT5_008.cs
//!
//! # Patterns this test covers
//! - D4  declarative aura (own-turn, named-card filter, +3000 DP)
//! - D6  flood gate / opponent restriction (opponent-turn, cannot-reduce-digivolve-cost)
//!
//! # Clause summary
//!
//! | # | Clause              | Timing/Kind     | Scope  | DSL shape                                       |
//! |---|---------------------|-----------------|--------|-------------------------------------------------|
//! | 1 | [Your Turn] +3000   | aura            | FaceUp | kind: aura, active_when: your_turn, name_contains: "Gaossmon", other: true, dp_modifier: 3000 |
//! | 2 | [Opp Turn] no cost reduction | flood_gate / declarative | FaceUp | kind: flood_gate, target_player: opponent, modifier: CannotReduceDigivolveCost |
//!
//! # Known gaps
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | Clause 1 filtered aura runtime | G-DECLARATIVE-KEYWORD — `EffectTiming::Declarative` is never enqueued or fired by the engine; the filtered aura's process closure (ctx.add_dp_modifier) is compiled but never called; ChangeDp modifier is never installed at runtime | BLOCKED — all behavioral aura tests are #[ignore]'d; structural tests pass |
//! | Clause 1 self-exclusion | G-OTHER-PREDICATE-UNEVALUATED — `other: true` in CompiledPredicate is compiled but `eval_permanent_fields` does not check it; aura would fire on self too (over-fires) if G-DECLARATIVE-KEYWORD were closed | BLOCKED — #[ignore]'d; secondary to G-DECLARATIVE-KEYWORD |
//! | Clause 2 digivolve-cost gate | Player-targeted DSL and `CannotReduceDigivolveCost` enforcement are available. Remaining passive runtime blocker is G-DECLARATIVE-KEYWORD: declarative field effects are compiled but not globally dispatched, so this static floodgate is not installed from field state yet. | PARTIAL — structural DSL-native tests pass; behavioral passive test remains ignored |

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledPlayerRef, CompiledScope,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

// ─── Helper builders ─────────────────────────────────────────────────────────

/// Build a Gaossmon-named Digimon suitable as an "other Gaossmon" target.
fn make_gaossmon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_name = "Gaossmon".to_string();
    c.traits = vec!["Reptile".to_string()];
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// Build a non-Gaossmon Digimon (should NOT be buffed by clause 1).
fn make_non_gaossmon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_name = "SomeOtherDigimon".to_string();
    c.traits = vec!["Reptile".to_string()];
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// Standard runner: BT5-008 Gaossmon + one extra Gaossmon in hand + filler deck.
fn gaossmon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT5-008")
        .expect("BT5-008 found in embedded DSL pack")
        .add_card(make_gaossmon("GAOSSMON-2"))
        .add_card(make_non_gaossmon("NON-GAOSSMON"))
        .add_card(make_test_card("FILLER", "FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start()
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

/// BT5-008 must compile to at least 2 declarative clauses (aura + player-targeted flood_gate).
#[test]
fn bt5_008_compiles_to_at_least_two_declarative_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("BT5-008")
        .expect("BT5-008 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT5-008")
        .expect("BT5-008 compiled card present");

    assert!(
        compiled.effects.len() >= 2,
        "BT5-008 must have at least 2 clauses (aura + player-targeted flood_gate); got {}",
        compiled.effects.len()
    );

    // All clauses must be Declarative (no Triggered clauses — both effects are static/passive).
    for clause in &compiled.effects {
        assert!(
            matches!(clause, CompiledClause::Declarative(_)),
            "BT5-008 must have only declarative clauses; found triggered clause: {:?}",
            clause
        );
    }

    let flood_gate_clause = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
            modifier,
            target_player,
            ..
        }) => Some((modifier.as_str(), *target_player)),
        _ => None,
    });

    assert_eq!(
        flood_gate_clause,
        Some((
            "CannotReduceDigivolveCost",
            Some(CompiledPlayerRef::Opponent),
        )),
        "BT5-008 clause 2 must be a player-targeted CannotReduceDigivolveCost flood_gate"
    );
}

/// Clause 0 must be a Declarative::Aura with FaceUp scope and dp_modifier = +3000.
#[test]
fn bt5_008_has_your_turn_aura_clause_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("BT5-008")
        .expect("BT5-008 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT5-008")
        .expect("BT5-008 compiled card present");

    let aura_clause = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) => Some((*scope, *dp_modifier)),
        _ => None,
    });

    let (scope, dp) = aura_clause.expect(
        "BT5-008 must have a Declarative::Aura clause ([Your Turn] other Gaossmon +3000 DP)",
    );

    assert_eq!(
        scope,
        CompiledScope::FaceUp,
        "aura clause must be FaceUp scope (active while BT5-008 is on the field)"
    );
    assert_eq!(
        dp,
        Some(3000),
        "aura dp_modifier must be +3000 (printed text: +3000 DP)"
    );
}

// ─── Section 2: Clause 1 behavioral — [Your Turn] Gaossmon aura ──────────────

/// Positive: On your turn, another Gaossmon on the field gets +3000 DP
/// while BT5-008 is in play.
///
/// Blocked by G-DECLARATIVE-KEYWORD: the filtered aura lowers to
/// `Effect::declarative(card).process(|ctx| { ... ctx.add_dp_modifier(h, 3000, Expiry::Permanent) })`,
/// but `EffectTiming::Declarative` is NEVER enqueued or fired by the engine.
/// The process closure that installs `ChangeDp` modifiers is therefore never called.
/// As a result, `runner.dp_of(gaossmon_2)` returns the base DP (2000) — the +3000
/// modifier was never installed at runtime.
///
/// Additionally, due to G-OTHER-PREDICATE-UNEVALUATED, the `other: true` self-exclusion
/// predicate is also not enforced (see separate test). Both gaps must close for the
/// full behavioral contract to be satisfied.
#[test]
#[ignore = "pending: G-DECLARATIVE-KEYWORD — EffectTiming::Declarative is never enqueued or fired by \
            the engine; filtered aura process closure (ctx.add_dp_modifier) is never called; \
            ChangeDp modifier is compiled but not installed at runtime"]
fn bt5_008_aura_buffs_other_gaossmon_on_your_turn() {
    let mut runner = gaossmon_runner();

    // Place BT5-008 on P0's field.
    let gaossmon_1 = runner.place_on_field(0, "BT5-008", None);
    // Place a second Gaossmon on P0's field.
    let gaossmon_2 = runner.place_on_field(0, "GAOSSMON-2", None);

    // On P0's turn, the aura should give GAOSSMON-2 +3000 DP.
    let dp_gaossmon_2 = runner.dp_of(gaossmon_2).unwrap_or(0);
    assert_eq!(
        dp_gaossmon_2,
        2000 + 3000,
        "other Gaossmon must get +3000 DP on your turn; got {}",
        dp_gaossmon_2
    );

    let _ = gaossmon_1;
}

/// Negative: On your turn, a non-Gaossmon Digimon on P0's field does NOT get buffed.
///
/// Blocked by G-DECLARATIVE-KEYWORD: same root cause as the positive test.
/// With the declarative process never firing, non-Gaossmon also shows base DP —
/// but that's because the aura does nothing, not because filtering works correctly.
/// This test would vacuously pass if run, but that would be a false positive.
#[test]
#[ignore = "pending: G-DECLARATIVE-KEYWORD — filtered aura process never fires; non-Gaossmon check \
            would vacuously pass (correct result, wrong reason — declarative tick blocked)"]
fn bt5_008_aura_does_not_buff_non_gaossmon() {
    let mut runner = gaossmon_runner();

    // Place BT5-008 on P0's field.
    runner.place_on_field(0, "BT5-008", None);
    // Place a non-Gaossmon Digimon on P0's field.
    let non_gaossmon = runner.place_on_field(0, "NON-GAOSSMON", None);

    // NON-GAOSSMON should NOT get +3000 DP from BT5-008's aura.
    let dp_non_gaossmon = runner.dp_of(non_gaossmon).unwrap_or(0);
    assert_eq!(
        dp_non_gaossmon,
        2000, // base DP, no buff
        "non-Gaossmon must NOT receive +3000 DP from BT5-008; got {}",
        dp_non_gaossmon
    );
}

/// Negative: On opponent's turn, the [Your Turn] aura condition fails —
/// the other Gaossmon should NOT get +3000 DP.
///
/// Blocked by G-DECLARATIVE-KEYWORD: same root cause. The active_when condition
/// closure is never called, so the turn-gating cannot be verified at runtime.
#[test]
#[ignore = "pending: G-DECLARATIVE-KEYWORD — declarative process never fires; active_when condition \
            cannot be verified; opponent-turn negative test would vacuously pass"]
fn bt5_008_aura_does_not_fire_on_opponents_turn() {
    let mut runner = gaossmon_runner();

    // Place BT5-008 on P0's field.
    let gaossmon_1 = runner.place_on_field(0, "BT5-008", None);
    // Place a second Gaossmon on P0's field.
    let gaossmon_2 = runner.place_on_field(0, "GAOSSMON-2", None);

    // Advance to P1's turn.
    runner.end_turn();

    // The aura should NOT fire on P1's turn (active_when: your_turn).
    let dp_gaossmon_2 = runner.dp_of(gaossmon_2).unwrap_or(0);
    assert_eq!(
        dp_gaossmon_2,
        2000, // base DP, no buff
        "other Gaossmon must NOT get +3000 DP on opponent's turn; got {}",
        dp_gaossmon_2
    );

    let _ = gaossmon_1;
}

/// G-OTHER-PREDICATE-UNEVALUATED: BT5-008's aura should NOT buff itself
/// (the `other: true` predicate should exclude the source card), but due to
/// the engine gap the aura currently also applies to BT5-008 itself.
///
/// This test documents the EXPECTED behavior (self NOT buffed). It is #[ignore]'d
/// because the engine currently over-fires (BT5-008 buffs itself too).
#[test]
#[ignore = "BLOCKED: G-OTHER-PREDICATE-UNEVALUATED — eval_permanent_fields does not check pred.other; BT5-008 incorrectly buffs itself (expected: self excluded from aura target)"]
fn bt5_008_aura_does_not_buff_self() {
    let mut runner = gaossmon_runner();

    // Place BT5-008 on P0's field.
    let gaossmon_1 = runner.place_on_field(0, "BT5-008", None);

    // On P0's turn, BT5-008 should NOT buff itself (other: true exclusion).
    let dp_self = runner.dp_of(gaossmon_1).unwrap_or(0);
    assert_eq!(
        dp_self,
        2000, // base DP, no self-buff
        "BT5-008 must NOT buff itself (other: true); got {}",
        dp_self
    );
}

// ─── Section 3: Clause 2 behavioral — [Opponent's Turn] no digivolution cost reduction ──

/// Clause 2 is DSL-native now, but runtime behavior is still blocked by
/// G-DECLARATIVE-KEYWORD: passive declarative field effects are not dispatched globally.
/// Once the declarative pass exists, BT5-008 on P0's field during P1's turn should
/// install `CannotReduceDigivolveCost` on P1 and suppress P1's digivolution cost
/// reductions without affecting play-cost reducers.
#[test]
#[ignore = "BLOCKED: G-DECLARATIVE-KEYWORD — passive declarative flood_gate compiles, but field-state declarative effects are not globally dispatched yet"]
fn bt5_008_opponent_cannot_reduce_digivolution_costs_while_in_play() {
    // This test would need:
    // 1. A cost-reduction effect on P1's side (e.g., a BeforePayCost script).
    // 2. BT5-008 on P0's field.
    // 3. End P0's turn (P1's turn starts).
    // 4. P1 attempts to digivolve → cost reduction should be suppressed.
    //
    // The player-scoped DSL shape and per-cost-type enforcement now exist; the
    // remaining missing piece is the passive declarative dispatcher that installs
    // this static field floodgate while BT5-008 is face-up and active.
    assert!(
        false,
        "placeholder — remove when G-DECLARATIVE-KEYWORD is closed"
    );
}
