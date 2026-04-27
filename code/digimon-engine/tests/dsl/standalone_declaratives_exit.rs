//! Smoke exit test for the three standalone declarative lowerings:
//! Replacement, Partition, and Delay.
//!
//! Each test constructs a minimal synthetic `CompiledCard` with a single
//! clause of the given type and asserts that the expected Effect shape is
//! emitted. The combined test bundles all three clauses and asserts that
//! at least three Effects are produced — one per clause type — to catch
//! future dispatch-arm removal/reordering regressions.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::enums::{DelayTrigger, EffectTiming};

// ── Fixture helpers ───────────────────────────────────────────────────────────

fn base_card(id: &str, clauses: Vec<CompiledClause>) -> CompiledCard {
    CompiledCard {
        card: id.into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(6),
        color: vec![],
        cost: Some(10),
        dp: Some(10000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: clauses,
    }
}

fn replacement_clause() -> CompiledClause {
    CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
        scope: CompiledScope::FaceUp,
        active_when: None,
        trigger: "when_would_be_deleted".into(),
        process: vec![],
        summary: None,
        summary_key: None,
    })
}

fn partition_clause() -> CompiledClause {
    CompiledClause::Declarative(CompiledDeclarativeClause::Partition {
        scope: CompiledScope::FaceUp,
        active_when: None,
        sources: vec![],
        exclude_cause: vec![],
        summary: None,
        summary_key: None,
    })
}

fn delay_clause() -> CompiledClause {
    CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
        scope: CompiledScope::FaceUp,
        active_when: None,
        trigger: CompiledTiming::EndOfYourTurn,
        process: vec![],
        summary: None,
        summary_key: None,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Replacement clause with `trigger: "when_would_be_deleted"` must emit
/// exactly one Effect with `timing == WhenWouldBeDeleted` and a populated
/// `replacement_process`.
#[test]
fn replacement_clause_emits_would_be_deleted_effect() {
    let card = base_card("F-EXIT-REPL", vec![replacement_clause()]);
    let dsl = DslCardEffect::new(Arc::new(card));
    let effects = dsl.effects(CardHandle(0));

    assert_eq!(
        effects.len(),
        1,
        "Replacement clause must emit exactly one Effect, got {}",
        effects.len()
    );
    assert_eq!(
        effects[0].timing,
        EffectTiming::WhenWouldBeDeleted,
        "Replacement timing must be WhenWouldBeDeleted"
    );
    assert!(
        effects[0].replacement_process.is_some(),
        "replacement_process closure must be present"
    );
}

/// Partition clause must emit exactly one declarative Effect with a `process`
/// closure and a name that contains "Partition".
#[test]
fn partition_clause_emits_declarative_with_process() {
    let card = base_card("F-EXIT-PART", vec![partition_clause()]);
    let dsl = DslCardEffect::new(Arc::new(card));
    let effects = dsl.effects(CardHandle(0));

    assert_eq!(
        effects.len(),
        1,
        "Partition clause must emit exactly one Effect, got {}",
        effects.len()
    );
    assert!(
        effects[0].declarative,
        "Partition effect must have declarative == true"
    );
    assert!(
        effects[0].process.is_some(),
        "Partition effect must have a process closure"
    );
    assert!(
        effects[0].name.contains("Partition"),
        "Partition effect name must contain 'Partition', got '{}'",
        effects[0].name
    );
}

/// Delay clause with `trigger: CompiledTiming::EndOfYourTurn` must emit
/// exactly one Effect with `timing == DelayEffect`,
/// `delay_trigger == Some(DelayTrigger::EndOfThisTurn)`, and a `process`
/// closure.
#[test]
fn delay_clause_emits_delay_effect_with_end_of_this_turn_trigger() {
    let card = base_card("F-EXIT-DELAY", vec![delay_clause()]);
    let dsl = DslCardEffect::new(Arc::new(card));
    let effects = dsl.effects(CardHandle(0));

    assert_eq!(
        effects.len(),
        1,
        "Delay clause must emit exactly one Effect, got {}",
        effects.len()
    );
    assert_eq!(
        effects[0].timing,
        EffectTiming::DelayEffect,
        "Delay effect timing must be DelayEffect"
    );
    assert_eq!(
        effects[0].delay_trigger,
        Some(DelayTrigger::EndOfThisTurn),
        "EndOfYourTurn trigger must map to EndOfThisTurn"
    );
    assert!(
        effects[0].process.is_some(),
        "Delay effect must have a process closure"
    );
}

/// A single `CompiledCard` bundling all three clause types must emit at least
/// three Effects — one per clause — catching dispatch-arm removal/reordering.
#[test]
fn combined_card_with_all_three_clauses_emits_three_effects() {
    let card = base_card(
        "F-EXIT-ALL",
        vec![replacement_clause(), partition_clause(), delay_clause()],
    );
    let dsl = DslCardEffect::new(Arc::new(card));
    let effects = dsl.effects(CardHandle(0));

    assert!(
        effects.len() >= 3,
        "Combined card must emit at least 3 Effects (one per clause), got {}",
        effects.len()
    );

    // Assert each expected shape appears among the emitted effects.
    let has_replacement = effects
        .iter()
        .any(|e| e.timing == EffectTiming::WhenWouldBeDeleted && e.replacement_process.is_some());
    assert!(
        has_replacement,
        "No Replacement effect (WhenWouldBeDeleted + replacement_process) found among {} effects",
        effects.len()
    );

    let has_partition = effects
        .iter()
        .any(|e| e.declarative && e.process.is_some() && e.name.contains("Partition"));
    assert!(
        has_partition,
        "No Partition effect (declarative + process + 'Partition' name) found among {} effects",
        effects.len()
    );

    let has_delay = effects.iter().any(|e| {
        e.timing == EffectTiming::DelayEffect
            && e.delay_trigger == Some(DelayTrigger::EndOfThisTurn)
            && e.process.is_some()
    });
    assert!(
        has_delay,
        "No Delay effect (DelayEffect + EndOfThisTurn + process) found among {} effects",
        effects.len()
    );
}
