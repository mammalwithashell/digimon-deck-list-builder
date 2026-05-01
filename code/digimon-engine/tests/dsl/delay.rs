//! Tests for Task 3: Delay clause lowering.
//!
//! `CompiledDeclarativeClause::Delay` should emit exactly one `Effect` with
//! `timing == DelayEffect`. The `delay_trigger` field is mapped from
//! `CompiledTiming::EndOfYourTurn` → `DelayTrigger::EndOfThisTurn`; all
//! other timings default to `DelayTrigger::EndOfYourNextTurn`.
//! `CompiledScope::Inherited` sets `inherited == true`.

use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::enums::{DelayTrigger, EffectTiming};
use std::sync::Arc;

// ── Fixture ───────────────────────────────────────────────────────────────────

fn fixture_delay(scope: CompiledScope, trigger: CompiledTiming) -> CompiledCard {
    CompiledCard {
        card: "F-DELAY".into(),
        name: "Fixture Delay".into(),
        kind: CompiledCardKind::Option,
        level: None,
        color: vec![],
        cost: Some(4),
        dp: None,
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        dual: None,
        use_requirement: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::Delay {
                scope,
                active_when: None,
                trigger,
                process: vec![],
                summary: None,
                summary_key: None,
            },
        )],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// (a) `CompiledTiming::EndOfYourTurn` maps to `DelayTrigger::EndOfThisTurn`
/// and the emitted effect has `timing == DelayEffect`.
#[test]
fn delay_end_of_your_turn_maps_to_end_of_this_turn() {
    let dsl = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::EndOfYourTurn,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1, "expected exactly one effect for Delay");
    assert_eq!(
        effects[0].timing,
        EffectTiming::DelayEffect,
        "Delay effect must have DelayEffect timing"
    );
    assert_eq!(
        effects[0].delay_trigger,
        Some(DelayTrigger::EndOfThisTurn),
        "EndOfYourTurn must map to EndOfThisTurn"
    );
}

/// (b) A non-`EndOfYourTurn` timing (e.g. `OnPlay`) maps to
/// `DelayTrigger::EndOfYourNextTurn`.
#[test]
fn delay_other_timing_maps_to_end_of_your_next_turn() {
    let dsl = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::OnPlay,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(
        effects[0].timing,
        EffectTiming::DelayEffect,
        "timing must always be DelayEffect"
    );
    assert_eq!(
        effects[0].delay_trigger,
        Some(DelayTrigger::EndOfYourNextTurn),
        "non-EndOfYourTurn must map to EndOfYourNextTurn"
    );
}

/// (c) The emitted Effect has a `process` closure.
#[test]
fn delay_emits_effect_with_process() {
    let dsl = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::EndOfYourTurn,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert!(
        effects[0].process.is_some(),
        "Delay effect must have a process closure"
    );
}

/// (d) `CompiledScope::Inherited` sets `inherited == true`; `FaceUp` does not.
#[test]
fn delay_inherited_scope_sets_inherited_flag() {
    let inherited = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::Inherited,
        CompiledTiming::EndOfYourTurn,
    )));
    let face_up = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::EndOfYourTurn,
    )));

    let inh_effects = inherited.effects(CardHandle(0));
    let fu_effects = face_up.effects(CardHandle(0));

    assert_eq!(inh_effects.len(), 1);
    assert_eq!(fu_effects.len(), 1);

    assert!(
        inh_effects[0].inherited,
        "scope: Inherited should set the inherited flag"
    );
    assert!(
        !fu_effects[0].inherited,
        "scope: FaceUp must NOT set the inherited flag"
    );
}
