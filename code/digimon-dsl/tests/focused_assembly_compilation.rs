//! Exact original Assembly CardSpec -> IR metadata, not engine behavior.
use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathDirection, CompiledAltPathKind, CompiledCard, CompiledCardKind,
    CompiledColor, CompiledCost, CompiledDpConstraint, CompiledMaterial, CompiledPredicate,
    CompiledZone,
};
use digimon_dsl::CardSpec;

const TITA: &[u8] =
    include_bytes!("../../tests/fixtures/card_data/assembly_recipe_binding/BT24-081.yaml");
const AEGIO: &[u8] =
    include_bytes!("../../tests/fixtures/card_data/assembly_recipe_binding/BT26-073.yaml");

fn compile(raw: &[u8]) -> CompiledCard {
    let spec: CardSpec = serde_yml::from_slice(raw).unwrap();
    let compiled = digimon_dsl::compile::compile(&spec).unwrap();
    assert_eq!(compiled.effects.len(), spec.effects.len());
    assert_eq!(compiled.alt_paths.len(), spec.alt_paths.len());
    compiled
}

fn path(kind: CompiledAltPathKind, cost: i32) -> CompiledAltPath {
    CompiledAltPath {
        kind,
        from: None,
        materials: vec![],
        cost: Some(CompiledCost::Literal(cost)),
        stacks_unsuspended: false,
        ignore_requirements: false,
        source_treated_as: None,
        extra_cost: vec![],
        on_burst_turn_end: vec![],
        marker: false,
        condition: None,
        direction: CompiledAltPathDirection::From,
        extra_material_zones: vec![],
    }
}

fn material(filter: CompiledPredicate) -> CompiledMaterial {
    CompiledMaterial {
        filter,
        repeat: None,
        distinct_by: None,
        zones: vec![CompiledZone::Trash],
        cost_delta: None,
        stack_under: true,
    }
}

fn circle(level: u8, color: CompiledColor) -> CompiledAltPath {
    let mut result = path(CompiledAltPathKind::Digivolve, 4);
    result.from = Some(Box::new(CompiledPredicate {
        level_eq: Some(level),
        color_is: Some(color),
        ..Default::default()
    }));
    result
}

fn expected_tita() -> Vec<CompiledAltPath> {
    let mut assembly = path(CompiledAltPathKind::Assembly, 6);
    assembly.materials = ["Titamon", "SkullBaluchimon"]
        .into_iter()
        .map(|name| {
            material(CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                name_is: Some(name.into()),
                ..Default::default()
            })
        })
        .collect();
    vec![
        circle(6, CompiledColor::Purple),
        circle(6, CompiledColor::Green),
        assembly,
    ]
}

fn expected_aegio() -> Vec<CompiledAltPath> {
    let mut named = path(CompiledAltPathKind::Digivolve, 3);
    named.from = Some(Box::new(CompiledPredicate {
        name_is: Some("Aegiomon".into()),
        ..Default::default()
    }));
    let mut assembly = path(CompiledAltPathKind::Assembly, 2);
    assembly.materials = vec![material(CompiledPredicate {
        all_of: vec![
            CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            },
            CompiledPredicate {
                level_lte: Some(CompiledDpConstraint::Literal(4)),
                ..Default::default()
            },
            CompiledPredicate {
                any_of: vec![
                    CompiledPredicate {
                        in_text_contains: Some("Chronomon".into()),
                        ..Default::default()
                    },
                    CompiledPredicate {
                        trait_has: Some("TS".into()),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        ],
        ..Default::default()
    })];
    vec![
        circle(4, CompiledColor::Purple),
        circle(4, CompiledColor::Red),
        named,
        assembly,
    ]
}

fn exact_paths(card: &CompiledCard, expected: &[CompiledAltPath]) -> bool {
    card.alt_paths == expected
}

#[test]
fn original_titamon_full_ordered_assembly_metadata() {
    assert_eq!(
        TITA,
        include_bytes!("../../digimon-engine/cards/bt24/BT24-081.yaml")
    );
    let c = compile(TITA);
    assert_eq!(c.card, "BT24-081");
    assert_eq!(c.kind, CompiledCardKind::Digimon);
    assert_eq!(c.effects.len(), 5);
    assert!(exact_paths(&c, &expected_tita()));
}

#[test]
fn original_aegiochusmon_nested_filter_and_separate_aegiomon_path() {
    assert_eq!(
        AEGIO,
        include_bytes!("../../digimon-engine/cards/bt26/BT26-073.yaml")
    );
    let c = compile(AEGIO);
    assert_eq!(c.card, "BT26-073");
    assert_eq!(c.kind, CompiledCardKind::Digimon);
    assert_eq!(c.effects.len(), 3);
    assert!(exact_paths(&c, &expected_aegio()));
}

fn mutate(raw: &[u8], old: &str, new: &str) -> Vec<u8> {
    let text = std::str::from_utf8(raw).unwrap();
    assert_eq!(
        text.matches(old).count(),
        1,
        "mutation must target one precise source segment"
    );
    text.replacen(old, new, 1).into_bytes()
}

fn rejects_changed_paths(raw: &[u8], expected: &[CompiledAltPath], changes: &[(&str, &str)]) {
    let original = compile(raw);
    for (old, new) in changes {
        let changed = compile(&mutate(raw, old, new));
        assert!(
            !exact_paths(&changed, expected),
            "changed metadata accepted: {old} -> {new}"
        );
        // Every unrelated effect and original top-level field must remain identical.
        let mut restored = changed;
        restored.alt_paths = original.alt_paths.clone();
        assert_eq!(restored, original, "test must not modify unrelated clauses");
    }
}

#[test]
fn named_recipe_cost_origin_stack_multiplicity_and_exact_name_refusals() {
    rejects_changed_paths(
        TITA,
        &expected_tita(),
        &[
            ("    cost: 6", "    cost: 7"),
            (
                "name_is: \"Titamon\", zones: [trash]",
                "name_is: \"Titamon\", zones: [hand]",
            ),
            (
                "name_is: \"Titamon\", zones: [trash], stack_under: true",
                "name_is: \"Titamon\", zones: [trash], stack_under: false",
            ),
            (
                "kind: digimon, name_is: \"Titamon\", zones",
                "kind: tamer, name_is: \"Titamon\", zones",
            ),
            (
                "name_is: \"Titamon\", zones",
                "name_contains: \"Titamon\", zones",
            ),
            (
                "name_is: \"SkullBaluchimon\", zones",
                "name_is: \"SkullBaluchimon X\", zones",
            ),
            (
                "name_is: \"Titamon\", zones",
                "name_is: \"Titamon\", repeat: { min: 2, max: 2 }, zones",
            ),
        ],
    );
    let mut reordered = compile(TITA);
    reordered.alt_paths[2].materials.swap(0, 1);
    assert!(!exact_paths(&reordered, &expected_tita()));
}

#[test]
fn restricted_recipe_both_or_branches_outer_level_kind_and_named_route_refusals() {
    rejects_changed_paths(AEGIO, &expected_aegio(), &[
        ("    cost: 2", "    cost: 3"),
        ("            - level_lte: 4", "            - level_lte: 5"),
        ("            - level_lte: 4\n            - any_of:\n                - in_text_contains: \"Chronomon\"\n                - trait_has: TS",
         "            - any_of:\n                - all_of:\n                    - level_lte: 4\n                    - in_text_contains: \"Chronomon\"\n                - trait_has: TS"),
        ("            - kind: digimon\n            - level_lte: 4", "            - kind: tamer\n            - level_lte: 4"),
        ("                - in_text_contains: \"Chronomon\"", "                - name_contains: \"Chronomon\""),
        ("                - in_text_contains: \"Chronomon\"\n                - trait_has: TS", "                - in_text_contains: \"Chronomon\"\n                - trait_has: Titan"),
        ("            - any_of:\n                - in_text_contains:", "            - all_of:\n                - in_text_contains:"),
        ("        zones: [trash]", "        zones: [hand]"),
        ("        stack_under: true", "        stack_under: false"),
        ("        zones: [trash]", "        repeat: { min: 2, max: 2 }\n        zones: [trash]"),
        ("    from: { name_is: \"Aegiomon\" }", "    from: { name_contains: \"Aegiomon\" }"),
        ("    from: { name_is: \"Aegiomon\" }\n    cost: 3", "    from: { name_is: \"Aegiomon\" }\n    cost: 4"),
    ]);
}
