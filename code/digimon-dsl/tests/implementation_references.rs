#[path = "../src/bin/implementation_inventory/core.rs"]
mod core;
use core::{build_inventory, build_inventory_with_references, FileInput};
fn input(raw: &[u8]) -> FileInput {
    FileInput {
        path: "card.yaml".into(),
        bytes: raw.to_vec(),
    }
}
const SIMPLE:&[u8]=b"card: ST1-02\nname: Biyomon\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 2\ndp: 3000\neffects: []\n";
#[test]
fn opt_in_retains_exact_v1_and_original_bytes() {
    let old = build_inventory(vec!["ST1-02".into()], vec![input(SIMPLE)]).unwrap();
    let new = build_inventory_with_references(vec!["ST1-02".into()], vec![input(SIMPLE)]).unwrap();
    let n = serde_json::to_value(new).unwrap();
    assert_eq!(n["inventory"], serde_json::to_value(old).unwrap());
    assert_eq!(n["reference_graph"]["reference_closure_complete"], false);
    assert_eq!(n["reference_graph"]["files"][0]["traversal_complete"], true);
}
#[test]
fn actual_self_filters_are_matching_not_construction() {
    let raw = include_bytes!("../../digimon-engine/cards/p/P-180.yaml");
    let g = serde_json::to_value(
        build_inventory_with_references(vec!["P-180".into()], vec![input(raw)]).unwrap(),
    )
    .unwrap();
    let nodes = g["reference_graph"]["files"][0]["compiled"]["nodes"]
        .as_array()
        .unwrap();
    let refs: Vec<_> = nodes
        .iter()
        .filter(|n| n["role"] == "MATCH_CARD_ID" && n["value"] == "P-180")
        .collect();
    assert_eq!(refs.len(), 2);
    assert!(!nodes.iter().any(|n| n["role"] == "TOKEN_CONSTRUCTION"));
}
#[test]
fn both_dual_faces_are_visited_without_top_level_effects() {
    let raw = include_bytes!("../../digimon-engine/cards/bt25/BT25-085.yaml");
    let g = serde_json::to_value(
        build_inventory_with_references(vec!["BT25-085".into()], vec![input(raw)]).unwrap(),
    )
    .unwrap();
    let nodes = g["reference_graph"]["files"][0]["compiled"]["nodes"]
        .as_array()
        .unwrap();
    for face in ["dual.digimon.effects", "dual.option.effects"] {
        assert!(nodes
            .iter()
            .any(|n| n["path"].as_str().unwrap().contains(face)));
    }
}
#[test]
fn missing_is_not_traversed_or_vanilla() {
    let g = serde_json::to_value(
        build_inventory_with_references(vec!["MISSING".into()], vec![]).unwrap(),
    )
    .unwrap();
    assert_eq!(g["reference_graph"]["traversal_complete"], false);
    assert_eq!(g["inventory"]["cards"][0]["status"], "MISSING_DEFINITION");
}
#[test]
fn typed_recursive_branches_aliases_and_metadata_are_distinct() {
    let raw = br#"card: TEST-01
name: Test
kind: digimon
level: 3
color: [red]
cost: 2
dp: 3000
also_treated_as: [Alias]
digixros_aliases: [MaterialAlias]
identity:
  name_aliases:
    - treat_as: ConditionalAlias
      when:
        zone: [trash]
        has_inherited: { card_number_is: ST1-02 }
effects:
  - when: on_play
    summary: ST99-999 is not executable
    process:
      - if:
          condition: { all_of: [{name_is: A}, {not: {card_number_is: ST1-03}}] }
          then:
            - play_token: {controller: you, token_name: petrification}
          else:
            - optional:
                - raw_rust: {fn: unresolved_hook, consumes: [], binds: []}
"#;
    let g = serde_json::to_value(
        build_inventory_with_references(vec!["TEST-01".into()], vec![input(raw)]).unwrap(),
    )
    .unwrap();
    let f = &g["reference_graph"]["files"][0];
    assert!(f.is_object(), "{g}");
    let n = f["compiled"]["nodes"].as_array().unwrap();
    for (role, value) in [
        ("STATIC_NAME_DEFINITION", "Alias"),
        ("DIGIXROS_NAME_DEFINITION", "MaterialAlias"),
        ("CONDITIONAL_NAME_DEFINITION", "ConditionalAlias"),
        ("MATCH_CARD_ID", "ST1-02"),
        ("MATCH_CARD_ID", "ST1-03"),
        ("MATCH_NAME_EXACT", "A"),
        ("TOKEN_CONSTRUCTION", "petrification"),
        ("HOOK_STEP", "unresolved_hook"),
        (
            "METADATA_NO_EXECUTABLE_REFERENCE",
            "ST99-999 is not executable",
        ),
    ] {
        assert!(
            n.iter().any(|x| x["role"] == role && x["value"] == value),
            "missing {role}:{value}"
        );
    }
    assert_eq!(f["traversal_complete"], true);
    assert_eq!(g["reference_graph"]["reference_closure_complete"], false);
    assert!(f["compiled"]["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "UNRESOLVED_TOKEN_CONSTRUCTION"));
}
#[test]
fn repeated_formula_and_alternate_path_predicates_keep_occurrence_paths() {
    let mut s: digimon_dsl::CardSpec = serde_yml::from_slice(SIMPLE).unwrap();
    use digimon_dsl::{compile, compiled::*};
    let mut c = compile::compile(&s).unwrap();
    let p = CompiledPredicate {
        card_number_is: Some("ST1-02".into()),
        ..Default::default()
    };
    c.use_requirement = Some(p.clone());
    c.effects = vec![CompiledClause::Triggered(CompiledTriggeredClause {
        when: vec![CompiledTiming::OnPlay],
        scope: CompiledScope::FaceUp,
        active_when: Some(p.clone()),
        condition: Some(p.clone()),
        timing_conditions: vec![CompiledTimingCondition {
            when: CompiledTiming::OnPlay,
            condition: p.clone(),
        }],
        optional: false,
        outer_prompt: false,
        once_per_turn: false,
        max_per_turn: None,
        process: vec![CompiledStep::GainMemoryFn {
            formula: CompiledFormula::SourceStackCount {
                target: "source".into(),
                filter: Some(Box::new(p.clone())),
            },
        }],
        summary: None,
        summary_key: None,
    })];
    let (a, b) = core::references::visit(&s, &c);
    assert!(a.traversal_complete && b.traversal_complete);
    let refs: Vec<_> = b
        .nodes
        .iter()
        .filter(|x| x.role == "MATCH_CARD_ID")
        .collect();
    assert_eq!(refs.len(), 5);
    let paths: std::collections::BTreeSet<_> = refs.iter().map(|x| &x.path).collect();
    assert_eq!(paths.len(), 5);
    s.name = "ST1-02".into();
    let (a, _b) = core::references::visit(&s, &c);
    assert!(!a.nodes.iter().any(|x| x.role == "MATCH_CARD_ID"));
}
#[test]
fn opaque_authored_extension_is_blocked_even_if_compiler_drops_it() {
    let mut s: digimon_dsl::CardSpec = serde_yml::from_slice(SIMPLE).unwrap();
    let c = digimon_dsl::compile::compile(&s).unwrap();
    let mut p = digimon_dsl::predicate::PredicateSpec::default();
    p.extra
        .insert("future_hook".into(), serde_yml::Value::String("X".into()));
    s.use_requirement = Some(p);
    let (a, _) = core::references::visit(&s, &c);
    assert!(!a.traversal_complete);
    assert!(a.blockers.contains("OPAQUE_AUTHORED_VALUE"));
}
#[test]
fn binding_slot_and_name_slot_never_conflate() {
    let raw = String::from_utf8(SIMPLE.to_vec()).unwrap().replace(
        "effects: []",
        "use_requirement: {binding_card_name_is: {binding: chosen, name_is: Titamon}}\neffects: []",
    );
    let g = serde_json::to_value(
        build_inventory_with_references(vec!["ST1-02".into()], vec![input(raw.as_bytes())])
            .unwrap(),
    )
    .unwrap();
    for pass in ["authored", "compiled"] {
        let n = g["reference_graph"]["files"][0][pass]["nodes"]
            .as_array()
            .unwrap();
        assert!(n
            .iter()
            .any(|x| x["role"] == "EXISTING_INSTANCE_BINDING" && x["value"] == "chosen"));
        assert!(n
            .iter()
            .any(|x| x["role"] == "MATCH_BOUND_CARD_NAME_EXACT" && x["value"] == "Titamon"));
        assert!(!n
            .iter()
            .any(|x| x["role"].as_str().unwrap().starts_with("MATCH_") && x["value"] == "chosen"));
    }
}
#[test]
fn all_actual_seven_literal_filters_have_exact_typed_paths() {
    let cards: [(&str, &[u8], usize); 5] = [
        (
            "BT24-090",
            include_bytes!("../../digimon-engine/cards/bt24/BT24-090.yaml"),
            1,
        ),
        (
            "BT24-102",
            include_bytes!("../../digimon-engine/cards/bt24/BT24-102.yaml"),
            1,
        ),
        (
            "EX7-070",
            include_bytes!("../../digimon-engine/cards/ex7/EX7-070.yaml"),
            2,
        ),
        (
            "EX7-071",
            include_bytes!("../../digimon-engine/cards/ex7/EX7-071.yaml"),
            1,
        ),
        (
            "P-180",
            include_bytes!("../../digimon-engine/cards/p/P-180.yaml"),
            2,
        ),
    ];
    let mut total = 0;
    for (id, raw, expected) in cards {
        let g = serde_json::to_value(
            build_inventory_with_references(vec![id.into()], vec![input(raw)]).unwrap(),
        )
        .unwrap();
        let n = g["reference_graph"]["files"][0]["compiled"]["nodes"]
            .as_array()
            .unwrap();
        let refs: Vec<_> = n.iter().filter(|x| x["role"] == "MATCH_CARD_ID").collect();
        assert_eq!(refs.len(), expected, "{id}");
        for x in refs {
            assert_eq!(x["value"], id);
            assert!(
                x["path"].as_str().unwrap().contains("card_number_is")
                    || x["path"]
                        .as_str()
                        .unwrap()
                        .contains("trashed_source_card_id_is")
            );
        }
        total += expected;
    }
    assert_eq!(total, 7);
}
#[test]
fn duplicate_rows_remain_blocked_and_v1_bytes_stable() {
    let a = vec![
        FileInput {
            path: "b.yaml".into(),
            bytes: SIMPLE.to_vec(),
        },
        input(SIMPLE),
    ];
    let b = a.clone();
    let old = serde_json::to_vec(&build_inventory(vec!["ST1-02".into()], a).unwrap()).unwrap();
    let new = build_inventory_with_references(vec!["ST1-02".into()], b).unwrap();
    assert_eq!(old, serde_json::to_vec(&new.inventory).unwrap());
    assert!(!new.reference_graph.traversal_complete);
    assert!(new.reference_graph.files.is_empty());
    assert!(new
        .reference_graph
        .blockers
        .contains("DUPLICATE_DEFINITION"));
}
#[test]
fn visitor_patterns_have_no_rest_or_fallback_success() {
    for code in [
        include_str!("../src/bin/implementation_inventory/references.rs"),
        include_str!("../src/bin/implementation_inventory/reference_predicates.rs"),
        include_str!("../src/bin/implementation_inventory/reference_steps.rs"),
        include_str!("../src/bin/implementation_inventory/reference_formulas.rs"),
    ] {
        assert!(
            !code.contains(".."),
            "rest pattern bypasses added-field exhaustiveness"
        );
        assert!(
            !code.contains("_ =>"),
            "fallback arm bypasses added-variant exhaustiveness"
        );
    }
}
