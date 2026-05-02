use digimon_engine::dsl_cards::modifier_map::{lookup_keyword, lookup_modifier_type};
use digimon_engine::enums::{Keyword, ModifierType};

#[test]
fn modifier_map_covers_flood_gate_names_used_by_examples() {
    assert_eq!(
        lookup_modifier_type("CannotActivateSecurityEffects"),
        Some(ModifierType::CannotActivateSecurityEffects)
    );
    assert_eq!(
        lookup_modifier_type("CannotActivateWhenAttackingEffects"),
        Some(ModifierType::CannotActivateWhenAttackingEffects)
    );
    assert_eq!(
        lookup_modifier_type("CannotPlayTamerByEffect"),
        Some(ModifierType::CannotPlayTamerByEffect)
    );
    assert_eq!(
        lookup_modifier_type("CannotReduceDigivolveCost"),
        Some(ModifierType::CannotReduceDigivolveCost)
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
    assert_eq!(
        lookup_keyword("MaterialSave", Some(1)),
        Some(Keyword::MaterialSave(1))
    );
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
        digixros_aliases: Vec::new(),
        dual: None,
        use_requirement: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::GrantKeyword {
                keyword: keyword.into(),
                value,
                scope: CompiledScope::FaceUp,
                overclock_cost_filter: None,
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

use digimon_dsl::compiled::{CompiledGrantKeywordValue, CompiledPlayerRef, CompiledPredicate};

fn fixture_aura_self_dp(amount: i32) -> CompiledCard {
    CompiledCard {
        card: "F-AURA-SELF".into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(4),
        color: vec![],
        cost: Some(5),
        dp: Some(4000),
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
            CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Inherited,
                active_when: None,
                target: CompiledPredicate::default(),
                target_player: None,
                dp_modifier: Some(amount),
                dp_modifier_fn: None,
                security_attack_fn: None,
                grant_keyword: None,
                modifier: None,
                summary: None,
                summary_key: None,
            },
        )],
    }
}

#[test]
fn self_aura_with_dp_modifier_sets_static_dp_field() {
    let dsl = DslCardEffect::new(Arc::new(fixture_aura_self_dp(2000)));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].dp_modifier, 2000);
    assert!(
        effects[0].inherited,
        "scope: inherited should set the inherited flag"
    );
}

fn fixture_aura_filtered(
    target: CompiledPredicate,
    grant: Option<CompiledGrantKeywordValue>,
    dp: Option<i32>,
) -> CompiledCard {
    CompiledCard {
        card: "F-AURA-FILT".into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Tamer,
        level: None,
        color: vec![],
        cost: Some(3),
        dp: None,
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
            CompiledDeclarativeClause::Aura {
                scope: CompiledScope::FaceUp,
                active_when: None,
                target,
                target_player: None,
                dp_modifier: dp,
                dp_modifier_fn: None,
                security_attack_fn: None,
                grant_keyword: grant,
                modifier: None,
                summary: None,
                summary_key: None,
            },
        )],
    }
}

#[test]
fn filtered_aura_emits_declarative_with_process_but_no_static_dp() {
    let target = CompiledPredicate {
        owner: Some(CompiledPlayerRef::You),
        name_contains: Some("Omnimon".into()),
        ..Default::default()
    };
    let grant = Some(CompiledGrantKeywordValue {
        keyword: "SecurityAttackPlus".into(),
        value: Some(1),
    });
    let dsl = DslCardEffect::new(Arc::new(fixture_aura_filtered(target, grant, None)));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert!(effects[0].declarative);
    assert_eq!(effects[0].dp_modifier, 0);
    assert!(effects[0].process.is_some());
}

fn fixture_cost_reduction(amount: i32, condition: Option<CompiledPredicate>) -> CompiledCard {
    CompiledCard {
        card: "F-CR".into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(6),
        color: vec![],
        cost: Some(11),
        dp: Some(11000),
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
            CompiledDeclarativeClause::CostReduction {
                scope: CompiledScope::FaceUp,
                active_when: None,
                reduction_timing: Some("before_pay_cost".into()),
                when_playing_this: true,
                when_any_ally_played: None,
                condition,
                optional: false,
                once_per_turn: false,
                amount: Some(amount),
                amount_fn: None,
                pay_cost: vec![],
                summary: None,
                summary_key: None,
            },
        )],
    }
}

#[test]
fn cost_reduction_when_playing_this_emits_before_pay_cost_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_cost_reduction(3, None)));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(
        effects[0].timing,
        digimon_engine::enums::EffectTiming::BeforePayCost
    );
    assert!(effects[0].cost_reduction_fn.is_some());
}

#[test]
fn cost_reduction_without_literal_amount_skips_emission() {
    // amount_fn path is Phase 2+ — drop for now.
    let mut c = fixture_cost_reduction(0, None);
    if let CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
        amount, ..
    }) = &mut c.effects[0]
    {
        *amount = None;
    }
    let dsl = DslCardEffect::new(Arc::new(c));
    assert!(dsl.effects(CardHandle(0)).is_empty());
}

fn fixture_flood_gate(modifier: &str, target: CompiledPredicate) -> CompiledCard {
    CompiledCard {
        card: "F-FG".into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(7),
        color: vec![],
        cost: Some(15),
        dp: Some(17000),
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
            CompiledDeclarativeClause::FloodGate {
                scope: CompiledScope::FaceUp,
                active_when: Some(CompiledPredicate {
                    your_turn: Some(true),
                    ..Default::default()
                }),
                modifier: modifier.into(),
                target: Some(target),
                target_player: None,
                expiry: None,
                summary: None,
                summary_key: None,
            },
        )],
    }
}

#[test]
fn flood_gate_emits_declarative_with_process_closure() {
    let target = CompiledPredicate {
        owner: Some(CompiledPlayerRef::Opponent),
        kind: Some(CompiledCardKind::Option),
        ..Default::default()
    };
    let dsl = DslCardEffect::new(Arc::new(fixture_flood_gate(
        "CannotActivateSecurityEffects",
        target,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert!(effects[0].declarative);
    assert!(effects[0].process.is_some());
}

fn fixture_player_flood_gate(modifier: &str, target_player: CompiledPlayerRef) -> CompiledCard {
    CompiledCard {
        card: "F-PFG".into(),
        name: "Player Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(3),
        color: vec![],
        cost: Some(3),
        dp: Some(3000),
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
            CompiledDeclarativeClause::FloodGate {
                scope: CompiledScope::FaceUp,
                active_when: None,
                modifier: modifier.into(),
                target: None,
                target_player: Some(target_player),
                expiry: None,
                summary: None,
                summary_key: None,
            },
        )],
    }
}

#[test]
fn player_scoped_flood_gate_emits_declarative_with_process_closure() {
    let dsl = DslCardEffect::new(Arc::new(fixture_player_flood_gate(
        "CannotPlayDigimonByEffect",
        CompiledPlayerRef::Any,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert!(effects[0].declarative);
    assert!(effects[0].process.is_some());
}

#[test]
fn flood_gate_unknown_modifier_skips_emission() {
    let dsl = DslCardEffect::new(Arc::new(fixture_flood_gate(
        "NoSuchModifier",
        CompiledPredicate::default(),
    )));
    assert!(dsl.effects(CardHandle(0)).is_empty());
}

// ── Task 9: ace_overflow accessor ────────────────────────────────────────────

#[test]
fn ace_overflow_reads_from_compiled_card() {
    let mut c = fixture_grant_keyword("Blocker", None);
    c.ace_overflow = Some(-5);
    let dsl = DslCardEffect::new(Arc::new(c));
    assert_eq!(dsl.ace_overflow(), Some(-5));
}

#[test]
fn ace_overflow_is_none_when_unset() {
    let dsl = DslCardEffect::new(Arc::new(fixture_grant_keyword("Blocker", None)));
    assert_eq!(dsl.ace_overflow(), None);
}

#[test]
fn dsl_ace_overflow_populates_runtime_card_data() {
    let yaml = r#"
card: ACE-RUNTIME
name: Ace Runtime
kind: digimon
level: 5
color: [red]
cost: 7
dp: 7000
ace_overflow: -4
"#;
    let spec: digimon_dsl::spec::CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    let card_data = digimon_engine::debug_runner::card_data_for_test_from_compiled(&compiled);
    assert_eq!(card_data.ace_overflow, Some(-4));
}

// ── Task 10: register_dsl_cards + build_registry ─────────────────────────────

#[test]
fn register_dsl_cards_inserts_every_pack_card_into_registry() {
    let pack = digimon_engine::dsl_registry::from_embedded().expect("embedded pack loads");
    let mut effects = digimon_engine::cards::CardEffectRegistry::new();
    digimon_engine::dsl_cards::register_dsl_cards(&mut effects, &pack);
    assert_eq!(effects.len(), pack.len());
    for (card_id, _) in pack.iter() {
        assert!(
            effects.get(card_id).is_some(),
            "missing DSL registration for {card_id}"
        );
    }
}

#[test]
fn build_registry_contains_both_dsl_and_hand_written_cards() {
    let registry = digimon_engine::cards::build_registry();
    assert!(
        registry.get("TEST-001").is_some(),
        "hand-written TEST-001 present"
    );
    assert!(
        registry.get("ST2-13").is_some(),
        "DSL-authored ST2-13 present"
    );
}
