//! Phase 1c exit: every fixture YAML compiles into CompiledCard, registers
//! into CardEffectRegistry, and fixtures with declarative clauses produce
//! at least one Effect. Fixtures that have ONLY triggered clauses / alt-path
//! registration (not lowered in Phase 1c) are exempt.

use digimon_dsl::compiled::CompiledClause;
use digimon_engine::card_source::CardHandle;
use digimon_engine::cards::build_registry;

#[test]
fn all_fixture_cards_register() {
    let registry = build_registry();
    let pack = digimon_engine::dsl_registry::from_embedded()
        .expect("embedded pack loads");
    assert!(pack.len() >= 15, "at least 15 fixtures in the embedded pack");
    for (card_id, _) in pack.iter() {
        assert!(
            registry.get(card_id).is_some(),
            "pack card {card_id} not registered",
        );
    }
}

#[test]
fn declarative_fixtures_produce_at_least_one_effect() {
    // Cards whose declarative clauses Phase 1c lowers into at least one Effect.
    let must_have_effect = &[
        "BT17-015", // cost_reduction (when_playing_this + literal amount)
        "BT10-111", // grant_keyword + aura
        "BT5-093",  // aura
        "AD1-025",  // grant_keyword (Raid + Blocker)
        "BT12-112", // flood_gate (CannotActivateSecurityEffects)
    ];

    let registry = build_registry();
    let pack = digimon_engine::dsl_registry::from_embedded().unwrap();
    for card_id in must_have_effect {
        let compiled = pack
            .lookup(card_id)
            .unwrap_or_else(|| panic!("{card_id} missing from pack"));
        let has_declarative = compiled
            .effects
            .iter()
            .any(|c| matches!(c, CompiledClause::Declarative(_)));
        assert!(
            has_declarative,
            "{card_id} has no declarative clauses — fixture audit needed"
        );
        let effect = registry
            .get(card_id)
            .unwrap_or_else(|| panic!("{card_id} not registered"));
        let out = effect.effects(CardHandle(0));
        assert!(
            !out.is_empty(),
            "{card_id} declarative clauses lowered to zero Effects"
        );
    }
}
