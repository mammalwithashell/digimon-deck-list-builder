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
