//! Tests for Task 1: Replacement clause lowering.
//!
//! `CompiledDeclarativeClause::Replacement` with a known trigger string
//! should emit exactly one `Effect` whose `.timing` equals the corresponding
//! `EffectTiming::WhenWould*` variant and whose `.replacement_process` is
//! populated. Unknown trigger strings must produce no effects.

use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::enums::EffectTiming;
use std::sync::Arc;

// ── Fixture ──────────────────────────────────────────────────────────────────

fn fixture_replacement(trigger: &str, scope: CompiledScope) -> CompiledCard {
    CompiledCard {
        card: "F-REPL".into(),
        name: "Fixture Replacement".into(),
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
        digixros_aliases: Vec::new(),
        dual: None,
        use_requirement: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::Replacement {
                scope,
                active_when: None,
                trigger: trigger.into(),
                optional: false,
                once_per_turn: false,
                process: vec![],
                summary: None,
                summary_key: None,
            },
        )],
    }
}

// ── Happy-path tests — every known trigger string ────────────────────────────

#[test]
fn when_would_be_deleted_emits_replacement_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_be_deleted",
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(
        effects.len(),
        1,
        "expected one effect for when_would_be_deleted"
    );
    assert_eq!(effects[0].timing, EffectTiming::WhenWouldBeDeleted);
    assert!(
        effects[0].replacement_process.is_some(),
        "replacement_process closure must be present"
    );
}

#[test]
fn when_would_leave_battle_area_emits_replacement_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_leave_battle_area",
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, EffectTiming::WhenWouldLeaveBattleArea);
    assert!(effects[0].replacement_process.is_some());
}

#[test]
fn when_would_be_returned_to_hand_emits_replacement_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_be_returned_to_hand",
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, EffectTiming::WhenWouldBeReturnedToHand);
    assert!(effects[0].replacement_process.is_some());
}

#[test]
fn when_would_be_returned_to_deck_emits_replacement_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_be_returned_to_deck",
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, EffectTiming::WhenWouldBeReturnedToDeck);
    assert!(effects[0].replacement_process.is_some());
}

#[test]
fn when_would_be_trashed_emits_replacement_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_be_trashed",
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, EffectTiming::WhenWouldBeTrashed);
    assert!(effects[0].replacement_process.is_some());
}

#[test]
fn when_would_be_de_digivolved_emits_replacement_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_be_de_digivolved",
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, EffectTiming::WhenWouldBeDeDigivolved);
    assert!(effects[0].replacement_process.is_some());
}

#[test]
fn when_would_lose_security_emits_replacement_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_lose_security",
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, EffectTiming::WhenWouldLoseSecurity);
    assert!(effects[0].replacement_process.is_some());
}

#[test]
fn when_would_draw_emits_replacement_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_draw",
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, EffectTiming::WhenWouldDraw);
    assert!(effects[0].replacement_process.is_some());
}

#[test]
fn when_would_place_in_security_emits_replacement_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_place_in_security",
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, EffectTiming::WhenWouldPlaceInSecurity);
    assert!(effects[0].replacement_process.is_some());
}

// ── Scope: Inherited sets the inherited flag ──────────────────────────────────

#[test]
fn inherited_scope_sets_inherited_flag_on_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_be_deleted",
        CompiledScope::Inherited,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert!(
        effects[0].inherited,
        "scope: Inherited should set the inherited flag"
    );
}

#[test]
fn face_up_scope_does_not_set_inherited_flag_on_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_be_deleted",
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert!(
        !effects[0].inherited,
        "scope: FaceUp must NOT set the inherited flag"
    );
}

// ── Unknown trigger → empty emission ─────────────────────────────────────────

#[test]
fn unknown_trigger_string_produces_no_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_replacement(
        "when_would_explode_the_universe",
        CompiledScope::FaceUp,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert!(
        effects.is_empty(),
        "unknown trigger must produce no effects, got {} effect(s)",
        effects.len()
    );
}

// ── trigger_map lookup directly ──────────────────────────────────────────────

#[test]
fn trigger_map_covers_all_nine_known_triggers() {
    use digimon_engine::dsl_cards::trigger_map::lookup_replacement_trigger;
    let known = [
        ("when_would_be_deleted", EffectTiming::WhenWouldBeDeleted),
        (
            "when_would_leave_battle_area",
            EffectTiming::WhenWouldLeaveBattleArea,
        ),
        (
            "when_would_be_returned_to_hand",
            EffectTiming::WhenWouldBeReturnedToHand,
        ),
        (
            "when_would_be_returned_to_deck",
            EffectTiming::WhenWouldBeReturnedToDeck,
        ),
        ("when_would_be_trashed", EffectTiming::WhenWouldBeTrashed),
        (
            "when_would_be_de_digivolved",
            EffectTiming::WhenWouldBeDeDigivolved,
        ),
        (
            "when_would_lose_security",
            EffectTiming::WhenWouldLoseSecurity,
        ),
        ("when_would_draw", EffectTiming::WhenWouldDraw),
        (
            "when_would_place_in_security",
            EffectTiming::WhenWouldPlaceInSecurity,
        ),
    ];
    for (name, expected) in &known {
        assert_eq!(
            lookup_replacement_trigger(name),
            Some(*expected),
            "trigger_map missing entry for '{name}'"
        );
    }
    assert_eq!(lookup_replacement_trigger("not_a_trigger"), None);
}
