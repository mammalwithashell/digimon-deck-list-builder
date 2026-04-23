use digimon_engine::dsl_cards::modifier_map::{lookup_keyword, lookup_modifier_type};
use digimon_engine::enums::{Keyword, ModifierType};

#[test]
fn modifier_map_covers_flood_gate_names_used_by_examples() {
    assert_eq!(
        lookup_modifier_type("CannotActivateSecurityEffects"),
        Some(ModifierType::CannotActivateSecurityEffects)
    );
    assert_eq!(
        lookup_modifier_type("CannotBeDestroyed"),
        Some(ModifierType::CannotBeDestroyed)
    );
    assert_eq!(lookup_modifier_type("DoesNotExist"), None);
}

#[test]
fn keyword_map_covers_aura_grants_used_by_examples() {
    assert_eq!(lookup_keyword("Blocker", None), Some(Keyword::Blocker));
    assert_eq!(lookup_keyword("Raid", None), Some(Keyword::Raid));
    assert_eq!(
        lookup_keyword("SecurityAttackPlus", Some(1)),
        Some(Keyword::SecurityAttackPlus(1))
    );
    assert_eq!(lookup_keyword("MaterialSave", Some(1)), Some(Keyword::Save));
    assert_eq!(lookup_keyword("NotAKeyword", None), None);
}

use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use std::sync::Arc;

fn fixture_grant_keyword(keyword: &str, value: Option<i32>) -> CompiledCard {
    CompiledCard {
        card: "F-GK".into(),
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
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::GrantKeyword {
                keyword: keyword.into(),
                value,
                scope: CompiledScope::FaceUp,
                active_when: None,
                summary: None,
                summary_key: None,
            },
        )],
    }
}

#[test]
fn grant_keyword_emits_one_declarative_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_grant_keyword("Blocker", None)));
    let card = CardHandle(0);
    let effects = dsl.effects(card);
    assert_eq!(effects.len(), 1);
    assert!(effects[0].declarative);
    assert!(effects[0].name.contains("Blocker"));
}

#[test]
fn grant_keyword_unknown_name_skips_emission() {
    let dsl = DslCardEffect::new(Arc::new(fixture_grant_keyword("NotAKeyword", None)));
    let card = CardHandle(0);
    assert!(dsl.effects(card).is_empty());
}
