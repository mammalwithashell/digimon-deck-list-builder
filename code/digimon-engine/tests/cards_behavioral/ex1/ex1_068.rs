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
//!   `run_inherited_security_effect`, mirroring the option_flow tests.
//!
//! # Faithfulness audit (per clause)
//!
//! 1. **[Main] grant clause** — BLOCKED. Test below
//!    `ex1_068_no_main_from_hand_clause_pending_gap` documents the
//!    omission and prevents accidental half-implementation.
//!
//! 2. **[Security] Gain 2 memory** — `when: on_security` + `process:
//!    [gain_memory: 2]` mirrors DCGO `card.Owner.AddMemory(2,
//!    activateClass)` with `SetIsSecurityEffect(true)`. Mandatory
//!    (`optional: false`); no [OPT].

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
        card.color.iter().any(|c| matches!(c,
            digimon_dsl::compiled::CompiledColor::Blue)),
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
         OMITTED pending G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT); got {} clauses",
        card.effects.len()
    );
}

/// The [Main] grant clause MUST NOT be present until the DSL gap closes.
/// If a future change adds a `MainFromHand` triggered clause that does NOT
/// faithfully implement the granted-trigger semantics, this test fails.
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
        "EX1-068 must NOT carry a MainFromHand clause until \
         G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT closes — a half-implementation \
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
