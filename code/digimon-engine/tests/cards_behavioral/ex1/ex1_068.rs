//! EX1-068 Ice Wall! — Option, Blue, Cost 1, traits: (none).
//!
//! # Card text (cards.json)
//!
//! **[Main]** All of your opponent's Digimon gain "[When Attacking] lose 2
//! memory" until the end of their next turn.
//!
//! **Inherited (Security):** [Security] Gain 2 memory.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/EX1/Blue/EX1_068.cs`
//!
//! # Implementation status — PARTIAL (BLOCKED on [Main])
//!
//! - **[Main] grant "[When Attacking] lose 2 memory" to opponent Digimon
//!   until end of their next turn** — BLOCKED on the DSL gap
//!   `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT`. The DSL today exposes
//!   grants for STATIC effects only (`grant_keyword`,
//!   `add_modifier`/`add_dp_modifier`, `grant_effect_immunity`); it has no
//!   verb to install a TRIGGERED effect (a clause whose `when:` fires
//!   later, here `when_attacking`) on a permanent the SOURCE does NOT
//!   control with a turn-scoped expiry. The Python engine handles this
//!   via `permanent.grant_temp_effect(effect, expiry_turn)`; the Rust
//!   engine has the modifier-registry + expiry substrate but no typed
//!   `GrantedTriggeredEffect` slot, no `CompiledStep::GrantTriggeredEffect`,
//!   and no inline-clause-as-step authoring shape. See the YAML header for
//!   the full gap analysis and the proposed `grant_triggered_effect:` step
//!   shape. Per no-approximations, the entire [Main] clause is OMITTED
//!   from the YAML rather than half-implemented (a "lose 2 memory whenever
//!   the opponent attacks" approximation would over-fire on opponent
//!   Digimon played AFTER this Option resolves — DCGO's per-Permanent
//!   foreach loop runs ONCE at resolution time and snapshots the eligible
//!   Digimon set).
//!
//!   **NOTE (2026-05-11):** The DSL gap `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT`
//!   is now CLOSED — `grant_triggered_effect` step is fully implemented
//!   (see `qa/dsl-vocab-gaps.md` Track H Phase 4e, `code/digimon-engine/src/dsl_cards/step/grant_triggered.rs`,
//!   and the fixture in `tests/dsl/group6_auras.rs::dsl_grant_triggered_effect_step_grants_when_attacking_body_to_filter_matches`).
//!   The YAML has not yet been updated to use it (that is a separate YAML edit).
//!   When the YAML gains the [Main] clause, the `ex1_068_has_one_clause_pending_main_gap`
//!   and `ex1_068_no_main_from_hand_clause_pending_gap` tests will correctly
//!   fail and force the author to update them.
//!
//! - **[Security] Gain 2 memory** — IMPLEMENTED via `gain_memory: 2` under
//!   `when: on_security`. Identical shape to ST2-13 Hammer Spark Clause 2.
//!
//! # Patterns this test covers
//!
//! - YAML parses and compiles, registers as Option/Blue/cost-1.
//! - Single triggered clause exists (the [Security] clause), present in
//!   the expected shape — `OnSecurity` timing, mandatory (no [Security]
//!   effect is opt-out per RULES_CONTEXT.md §16), `GainMemory(2)` step.
//! - Asserts the [Main] clause is OMITTED (only 1 effect on the card)
//!   and that NO `MainFromHand` clause exists — defending the no-
//!   approximations decision in code so a future "let's just add a
//!   placeholder lose_memory: 2 step" change fails the test.
//! - End-to-end gain_memory: 2 fires when the security effect runs via
//!   a full attack-player security reveal, mirroring the ST2-13 tests.
//!
//! # Faithfulness audit (per clause)
//!
//! 1. **[Main] grant clause** — BLOCKED (DSL gap closed at engine level;
//!    YAML not yet updated). Tests `ex1_068_no_main_from_hand_clause_pending_gap`
//!    and `ex1_068_has_one_clause_pending_main_gap` document the omission
//!    and prevent accidental half-implementation.
//!
//! 2. **[Security] Gain 2 memory** — `when: on_security` + `process:
//!    [gain_memory: 2]` mirrors DCGO `card.Owner.AddMemory(2,
//!    activateClass)` with `SetIsSecurityEffect(true)`. Mandatory
//!    (`optional: false`); no [OPT]. End-to-end behavioral test
//!    `ex1_068_security_reveal_gains_2_memory` confirms the clause
//!    fires correctly when P0 attacks P1's security stack.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCard, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "EX1-068";
const YAML: &str = include_str!("../../../cards/ex1/EX1-068.yaml");

// ── Card-data factories ──────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn make_attacker(id: &str) -> CardData {
    let mut c = make_test_card(id, "Test Attacker");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (YAML parse + clause shape)
// ═══════════════════════════════════════════════════════════════════════════════

/// EX1-068 YAML must parse and compile without errors.
#[test]
fn ex1_068_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX1-068 YAML must parse and compile without errors");
}

/// EX1-068 must compile as an Option card with cost 1 and Blue color.
#[test]
fn ex1_068_is_option_blue_cost_1() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX1-068 compiled card must be registered");

    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "EX1-068 must be an Option card"
    );
    assert_eq!(card.cost, Some(1), "EX1-068 prints Cost 1");
    assert!(
        card.color
            .iter()
            .any(|c| matches!(c, digimon_dsl::compiled::CompiledColor::Blue)),
        "EX1-068 must be a Blue Option (printed card_colors=[1]); got {:?}",
        card.color
    );
}

/// One clause total — the [Main] grant clause is OMITTED pending
/// G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT, leaving only the [Security]
/// clause:
///   [0] on_security (triggered, FaceUp scope, gain_memory: 2)
///
/// This test ALSO defends the no-approximations decision: if a future
/// change adds a half-implementation of the [Main] clause (e.g. a
/// `lose_memory: 2` step routed through some other timing), this test
/// will fail and force the author to either close the gap properly or
/// revert.
///
/// NOTE: When the YAML is updated to use `grant_triggered_effect`, this
/// count should become 2 and this test should be updated accordingly.
#[test]
fn ex1_068_has_one_clause_pending_main_gap() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("EX1-068 compiled");
    assert_eq!(
        card.effects.len(),
        1,
        "expected 1 clause (only [Security] gain_memory: 2; [Main] grant clause \
         OMITTED pending G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT — note: DSL gap \
         is now closed at engine level as of 2026-05-11; YAML update pending); \
         got {} clauses",
        card.effects.len()
    );
}

/// The [Main] grant clause MUST NOT be present until the YAML is updated
/// to use `grant_triggered_effect`. If a future change adds a
/// `MainFromHand` triggered clause that does NOT faithfully implement the
/// granted-trigger semantics, this test fails.
///
/// NOTE: When the YAML is updated to use `grant_triggered_effect`, this
/// test should be replaced with a positive assertion confirming the
/// MainFromHand clause's structure (grant target, timing, expiry, body).
#[test]
fn ex1_068_no_main_from_hand_clause_pending_gap() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("EX1-068 compiled");

    let has_main_from_hand = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::MainFromHand),
        _ => false,
    });
    assert!(
        !has_main_from_hand,
        "EX1-068 must NOT carry a MainFromHand clause until the YAML is updated \
         to use grant_triggered_effect (DSL gap G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT \
         closed 2026-05-11; YAML update pending) — a half-implementation \
         (e.g. silent lose_memory on every opponent attack) would over-fire on \
         Digimon played AFTER this Option resolves, violating no-approximations."
    );
}

/// Clause 0: the only clause is the [Security] one — `OnSecurity` timing,
/// FaceUp scope (not inherited; this Option's printed text routes through
/// security as a normal Option-security effect, not as a digivolution-
/// source inherited effect), mandatory.
#[test]
fn ex1_068_security_clause_shape() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("EX1-068 compiled");

    match &card.effects[0] {
        CompiledClause::Triggered(t) => {
            assert!(
                t.when.contains(&CompiledTiming::OnSecurity),
                "clause 0 must fire at OnSecurity; got {:?}",
                t.when
            );
            assert_eq!(
                t.scope,
                CompiledScope::FaceUp,
                "Option [Security] effects use FaceUp scope (no separate Security \
                 variant); got {:?}",
                t.scope
            );
            assert!(
                !t.optional,
                "[Security] effects are mandatory per RULES_CONTEXT.md §16; \
                 DCGO has no canNoSelect on the SecuritySkill ActivateClass"
            );
            assert!(
                !t.once_per_turn,
                "no [Once Per Turn] in printed [Security] text"
            );
        }
        other => panic!("clause 0 must be Triggered(on_security); got {:?}", other),
    }
}

/// The security clause's process must contain `GainMemory(2)`.
/// NOTE: Per RUST_DSL_TEST_API.md §11 anti-pattern 7, asserting process
/// step contents is generally discouraged as vocabulary can change. However,
/// for a minimal single-step body like gain_memory: 2 this structural
/// assertion pairs with the behavioral test below and provides a cheap
/// compile-time guard. The behavioral test `ex1_068_security_reveal_gains_2_memory`
/// is the authoritative coverage.
#[test]
fn ex1_068_security_process_contains_gain_memory_2() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("EX1-068 compiled");

    let CompiledClause::Triggered(t) = &card.effects[0] else {
        panic!("clause 0 must be Triggered; got {:?}", card.effects[0]);
    };

    let has_gain_memory_2 = t
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::GainMemory(2)));
    assert!(
        has_gain_memory_2,
        "[Security] process must contain GainMemory(2) — printed text 'Gain 2 memory'; \
         got {:?}",
        t.process
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: [Security] gain 2 memory
// ═══════════════════════════════════════════════════════════════════════════════
//
// Pattern: place EX1-068 in P1's security stack, park a vanilla attacker on
// P0's field, and call `runner.attack_player(attacker, 1, false)`. The
// security-check pipeline reveals EX1-068, fires its OnSecurity body, and
// settles. Mirrors ST2-13's `st2_13_security_reveal_gains_2_memory`.
//
// Memory semantics: `CompiledStep::GainMemory(n)` lowers to
// `EffectContext::gain_memory(n)`, which gives memory to the resolving
// effect's controller. When P1's security EX1-068 triggers, ctx.player = 1,
// so `gain_memory(2)` moves the gauge toward P1's side (negative delta from
// P0's turn-player perspective).

/// P0 attacks P1's security where the only security card is EX1-068 →
/// EX1-068's on_security clause fires → P1 gains exactly 2 memory (gauge
/// shifts by -2 from P0's perspective).
#[test]
fn ex1_068_security_reveal_gains_2_memory() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX1-068 YAML parses")
        .add_card(make_attacker("ATK"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();

    let attacker = runner.place_on_field(0, "ATK", Some(0));
    let mem_before = runner.memory();
    let sec_before = runner.security_count(1);

    let _result = runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    // The attack consumed P1's security card.
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "P1's only security card must be revealed and consumed; \
         before={sec_before}, after={}",
        runner.security_count(1)
    );

    // P1 gains 2 memory — from P0's (turn-player) perspective the raw
    // gauge moves by -2.
    assert_eq!(
        runner.memory() - mem_before,
        -2,
        "[Security] gain_memory: 2 must give exactly 2 memory to the defender (P1); \
         before={mem_before}, after={}",
        runner.memory()
    );
}

/// Negative coverage: with a plain filler in P1's security (no EX1-068),
/// an identical attack does NOT move memory. Defends the assertion above
/// by ruling out the possibility that any security reveal triggers memory gain.
#[test]
fn ex1_068_security_other_card_does_not_gain_memory() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX1-068 YAML parses")
        .add_card(make_attacker("ATK"))
        .add_card(filler("FILL"))
        .add_card(filler("PLAIN-SEC"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(1, &["PLAIN-SEC"])
        .memory(0)
        .start();

    let attacker = runner.place_on_field(0, "ATK", Some(0));
    let mem_before = runner.memory();

    let _ = runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory() - mem_before,
        0,
        "attacking a plain filler security card must not change memory; \
         before={mem_before}, after={}",
        runner.memory()
    );
}

/// Idempotency: two separate games each fire EX1-068 security for +2.
/// Rules out double-fire from a leak in the effect queue or the
/// once-per-turn registry (security effects are not OPT, so this is
/// not an OPT test — it confirms no spurious second firing in a single
/// reveal sequence).
#[test]
fn ex1_068_security_memory_gain_is_exactly_2_not_4() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX1-068 YAML parses")
        .add_card(make_attacker("ATK"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();

    let attacker = runner.place_on_field(0, "ATK", Some(0));
    let mem_before = runner.memory();

    let _ = runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory() - mem_before,
        -2,
        "[Security] must gain exactly 2 (not 4 or more) — no double-fire; \
         before={mem_before}, after={}",
        runner.memory()
    );
}
