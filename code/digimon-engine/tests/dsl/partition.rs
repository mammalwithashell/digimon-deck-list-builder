//! Tests for Task 2: Partition clause lowering.
//!
//! `CompiledDeclarativeClause::Partition` with an empty source list should
//! emit exactly one declarative `Effect` with a `process` closure and a name
//! containing "Partition". `CompiledScope::Inherited` must set
//! `inherited == true`; `CompiledScope::FaceUp` must not.

use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledPredicate,
    CompiledScope,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use std::sync::Arc;

// ── Fixture ───────────────────────────────────────────────────────────────────

fn fixture_partition(scope: CompiledScope) -> CompiledCard {
    CompiledCard {
        card: "F-PART".into(),
        name: "Fixture Partition".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(5),
        color: vec![],
        cost: Some(7),
        dp: Some(6000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        dual: None,
        use_requirement: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::Partition {
                scope,
                active_when: None,
                sources: vec![],
                exclude_cause: vec![],
                process: vec![],
                summary: None,
                summary_key: None,
            },
        )],
    }
}

fn fixture_partition_with_sources(scope: CompiledScope) -> CompiledCard {
    CompiledCard {
        card: "F-PART-SRC".into(),
        name: "Fixture Partition Sources".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(5),
        color: vec![],
        cost: Some(7),
        dp: Some(6000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        dual: None,
        use_requirement: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::Partition {
                scope,
                active_when: None,
                sources: vec![CompiledPredicate::default(), CompiledPredicate::default()],
                exclude_cause: vec!["own_effect".into(), "battle".into()],
                process: vec![],
                summary: Some("Partition into WarGreymon/MetalGarurumon".into()),
                summary_key: None,
            },
        )],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// (a) Default empty Partition emits one declarative Effect with a process
/// closure and "Partition" in the name.
#[test]
fn partition_emits_one_declarative_effect_with_process() {
    let dsl = DslCardEffect::new(Arc::new(fixture_partition(CompiledScope::FaceUp)));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(
        effects.len(),
        1,
        "expected exactly one effect for Partition"
    );
    assert!(
        effects[0].declarative,
        "Partition effect must be declarative"
    );
    assert!(
        effects[0].process.is_some(),
        "Partition effect must have a process closure"
    );
    assert!(
        effects[0].name.contains("Partition"),
        "effect name must contain 'Partition', got '{}'",
        effects[0].name
    );
}

/// (b) `CompiledScope::Inherited` sets `inherited == true`.
#[test]
fn partition_inherited_scope_sets_inherited_flag() {
    let dsl = DslCardEffect::new(Arc::new(fixture_partition(CompiledScope::Inherited)));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert!(
        effects[0].inherited,
        "scope: Inherited should set the inherited flag"
    );
}

/// (c) `CompiledScope::FaceUp` does NOT set `inherited`.
#[test]
fn partition_face_up_scope_does_not_set_inherited_flag() {
    let dsl = DslCardEffect::new(Arc::new(fixture_partition(CompiledScope::FaceUp)));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert!(
        !effects[0].inherited,
        "scope: FaceUp must NOT set the inherited flag"
    );
}

/// Non-empty sources and exclude_cause fields are accepted without error;
/// the lowering still emits exactly one effect (source-list enforcement is
/// engine-side replacement dispatch, orthogonal to this lowering).
#[test]
fn partition_with_sources_and_excludes_still_emits_one_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_partition_with_sources(
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(
        effects.len(),
        1,
        "Partition with non-empty sources must still emit exactly one effect"
    );
    assert!(effects[0].declarative);
    assert!(effects[0].process.is_some());
}
