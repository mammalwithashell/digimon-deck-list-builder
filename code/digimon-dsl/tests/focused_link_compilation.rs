//! Native parse/IR compilation of exact reviewed Link YAML fixtures.
//! This is not engine Effect lowering, source authority, or gameplay validation.
use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause as D,
    CompiledPredicate, CompiledScope,
};
use digimon_dsl::CardSpec;

const SCOP: &[u8] =
    include_bytes!("../../tests/fixtures/card_data/link_yaml_binding/BT21-071.yaml");
const SAT: &[u8] = include_bytes!("../../tests/fixtures/card_data/link_yaml_binding/BT21-074.yaml");

fn compile(raw: &[u8]) -> Result<CompiledCard, String> {
    let spec: CardSpec = serde_yml::from_slice(raw).map_err(|e| e.to_string())?;
    let card = digimon_dsl::compile::compile(&spec).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        card.effects.len(),
        spec.effects.len(),
        "every original effect remains compiled"
    );
    Ok(card)
}

fn matches_link(card: &CompiledCard, id: &str, cost: u16, dp: i32) -> bool {
    if card.card != id || card.kind != CompiledCardKind::Digimon {
        return false;
    }
    let links: Vec<_> = card
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(d @ D::LinkCondition { .. }) => Some(d),
            _ => None,
        })
        .collect();
    let auras: Vec<_> = card
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(
                d @ D::Aura {
                    scope: CompiledScope::Linked,
                    ..
                },
            ) => Some(d),
            _ => None,
        })
        .collect();
    if links.len() != 1
        || auras.len() != 1
        || card
            .effects
            .iter()
            .any(|e| matches!(e, CompiledClause::Declarative(D::LinkRequirement { .. })))
    {
        return false;
    }
    let expected_filter = CompiledPredicate {
        trait_has: Some("Appmon".into()),
        ..Default::default()
    };
    let link_ok = matches!(links[0], D::LinkCondition {scope: CompiledScope::FaceUp, cost:c, filter, ..} if *c==cost && *filter==expected_filter);
    // Complete equality against a minimal aura prevents extra conditional,
    // target, formula, immunity or other modifier fields being silently ignored.
    let aura_ok = match auras[0] {
        D::Aura {
            summary,
            summary_key,
            ..
        } => {
            *auras[0]
                == D::Aura {
                    scope: CompiledScope::Linked,
                    active_when: None,
                    target: CompiledPredicate::default(),
                    target_player: None,
                    dp_modifier: Some(dp),
                    dp_modifier_fn: None,
                    security_attack: None,
                    security_attack_fn: None,
                    grant_keyword: None,
                    grant_traits: vec![],
                    modifier: None,
                    modifier_value: None,
                    modifier_name: None,
                    synth_identity: None,
                    while_condition: None,
                    applies_to_opponent_security_dp: false,
                    applies_to_own_security_dp: false,
                    effect_immunity: None,
                    modifier_from: None,
                    dna_material_identity: None,
                    summary: summary.clone(),
                    summary_key: summary_key.clone(),
                }
        }
        _ => false,
    };
    link_ok && aura_ok
}

#[test]
fn original_scopemon_compiles_exact_link_metadata() {
    assert_eq!(
        SCOP,
        include_bytes!("../../digimon-engine/cards/bt21/BT21-071.yaml")
    );
    let c = compile(SCOP).unwrap();
    assert_eq!(c.effects.len(), 5);
    assert!(matches_link(&c, "BT21-071", 2, 3000));
}

#[test]
fn original_satellamon_compiles_exact_link_metadata() {
    assert_eq!(
        SAT,
        include_bytes!("../../digimon-engine/cards/bt21/BT21-074.yaml")
    );
    let c = compile(SAT).unwrap();
    assert_eq!(c.effects.len(), 5);
    assert!(matches_link(&c, "BT21-074", 3, 4000));
}

fn changed(raw: &[u8], from: &str, to: &str) -> Vec<u8> {
    let text = std::str::from_utf8(raw).unwrap();
    assert_eq!(
        text.matches(from).count(),
        1,
        "mutation must identify exactly one source segment"
    );
    text.replacen(from, to, 1).into_bytes()
}

#[test]
fn metadata_mutations_cannot_satisfy_original_proof() {
    let mutations = [
        (
            "cost: 2\n    filter: { trait_has: Appmon }",
            "cost: 3\n    filter: { trait_has: Appmon }",
        ),
        (
            "\n    filter: { trait_has: Appmon }",
            "\n    filter: { trait_has: Tool }",
        ),
        (
            "\n    filter: { trait_has: Appmon }",
            "\n    filter: { trait_has: Appmon, level_eq: 4 }",
        ),
        (
            "\n  - kind: link_condition",
            "\n  - scope: inherited\n    kind: link_condition",
        ),
        ("dp_modifier: 3000", "dp_modifier: 4000"),
        ("target: {}", "target: { level_eq: 4 }"),
        (
            "scope: linked\n    kind: aura",
            "scope: face_up\n    kind: aura",
        ),
        (
            "dp_modifier: 3000",
            "dp_modifier: 3000\n    active_when: { your_turn: true }",
        ),
        (
            "dp_modifier: 3000",
            "dp_modifier: 3000\n    security_attack: 1",
        ),
        ("card: BT21-071", "card: BT21-074"),
        (
            "cost: 2\n    filter: { trait_has: Appmon }",
            "cost: 65536\n    filter: { trait_has: Appmon }",
        ),
    ];
    for (old, new) in mutations {
        let result = compile(&changed(SCOP, old, new));
        if !new.contains("65536") {
            assert!(
                result.is_ok(),
                "valid native mutation did not compile: {new}: {result:?}"
            );
        }
        assert!(
            !result
                .as_ref()
                .is_ok_and(|c| matches_link(c, "BT21-071", 2, 3000)),
            "mutation unexpectedly matched: {new}"
        );
    }
}

#[test]
fn duplicated_link_clause_or_aura_is_not_unique() {
    for suffix in [
        "\n  - kind: link_condition\n    cost: 2\n    filter: { trait_has: Appmon }\n",
        "\n  - scope: linked\n    kind: aura\n    target: {}\n    dp_modifier: 3000\n",
    ] {
        let mut raw = SCOP.to_vec();
        raw.extend_from_slice(suffix.as_bytes());
        let result = compile(&raw);
        assert!(!result
            .as_ref()
            .is_ok_and(|c| matches_link(c, "BT21-071", 2, 3000)));
    }
}
