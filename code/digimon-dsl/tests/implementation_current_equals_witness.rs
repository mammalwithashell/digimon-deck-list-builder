//! Saved-current correspondence fixtures only: no game, oracle, or admission.
//! Keep the existing witness helper and legacy compatibility tests unchanged.
#[path = "../src/bin/implementation_inventory/equals_witness.rs"]
mod witness;

use digimon_dsl::{compile::compile, compiled::CompiledCard, CardSpec};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use witness::{witness_equals_if, Branch, EqualsWitness, IfLocation};

const FIXTURE: &str = include_str!("fixtures/current_equals_witness/three_cards.json");
const INVENTORY_SHA: &str =
    "6c54ce1c9b8608d2f60fc9936659c7a4118284275062eb2d07ad21b15e901424";

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn cards(f: &Value) -> &[Value] {
    f["cards"].as_array().unwrap()
}
fn raw(c: &Value) -> &[u8] {
    c["source_utf8"].as_str().unwrap().as_bytes()
}
fn owner(c: &Value) -> &str {
    c["card_id"].as_str().unwrap()
}
fn saved(c: &Value) -> CompiledCard {
    serde_json::from_value(c["compiled_card"].clone()).unwrap()
}
fn location(x: &Value) -> IfLocation {
    IfLocation {
        effect: x["effect"].as_u64().unwrap() as usize,
        ancestors: x["ancestors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| {
                let branch = match a[1].as_str().unwrap() {
                    "Then" => Branch::Then,
                    "Else" => Branch::Else,
                    other => panic!("unexpected fixture branch {other}"),
                };
                (a[0].as_u64().unwrap() as usize, branch)
            })
            .collect(),
        step: x["step"].as_u64().unwrap() as usize,
    }
}
fn paths(x: &Value, key: &str) -> Vec<String> {
    x[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["path"].as_str().unwrap().to_owned())
        .collect()
}
fn prove(c: &Value, x: &Value) -> EqualsWitness {
    witness_equals_if(raw(c), owner(c), &location(x), &saved(c)).unwrap()
}
fn changed_first_condition(c: &Value, replacement: &str) -> Vec<u8> {
    let text = c["source_utf8"].as_str().unwrap();
    let needle = "{ equals: [branch, 0] }";
    assert_eq!(text.matches(needle).count(), 1);
    text.replacen(needle, replacement, 1).into_bytes()
}

#[test]
fn saved_inventory_identity_and_unresolved_gates_are_retained() {
    let f = fixture();
    assert_eq!(f["schema_version"], 1);
    assert_eq!(f["inventory"]["sha256"], INVENTORY_SHA);
    assert_eq!(f["inventory"]["bytes"], 100_769_635);
    let requested = f["inventory"]["requested_ids"].as_array().unwrap();
    assert_eq!(requested.len(), 99);
    assert_eq!(requested.iter().map(|v| v.as_str().unwrap()).collect::<BTreeSet<_>>().len(), 99);
    assert!(f["admission"].as_object().unwrap().values().all(|v| v == &Value::Bool(false)));
    assert_eq!(f["inventory"]["inventory_blockers"].as_array().unwrap().len(), 3);
    assert_eq!(f["inventory"]["reference_blockers"].as_array().unwrap().len(), 7);
    assert_eq!(cards(&f).iter().map(owner).collect::<Vec<_>>(), ["BT25-078", "BT26-073", "P-156"]);
    for c in cards(&f) {
        assert_eq!(c["original_graph"]["authored_traversal_complete"], false);
        assert_eq!(c["original_graph"]["compiled_traversal_complete"], true);
        assert!(c["original_graph"]["authored_blockers"].as_array().unwrap().contains(&json!("OPAQUE_AUTHORED_VALUE")));
    }
}

#[test]
fn current_three_full_cards_bind_all_eight_locations_and_sixteen_paths() {
    let f = fixture();
    let mut original_paths = BTreeSet::new();
    let mut compiled_paths = BTreeSet::new();
    let mut witnesses = Vec::new();
    for (c, expected_count) in cards(&f).iter().zip([2, 3, 3]) {
        let cases = c["cases"].as_array().unwrap();
        assert_eq!(cases.len(), expected_count);
        for x in cases {
            let w = prove(c, x);
            assert_eq!(w.version, 1);
            assert_eq!(w.card_id, owner(c));
            assert_eq!(w.location, location(x));
            assert_eq!(w.original_source_bytes, raw(c));
            assert_eq!(w.compiled_card, saved(c));
            assert_eq!(w.binding, x["binding"].as_str().unwrap());
            assert_eq!(w.integer, x["integer"].as_i64().unwrap());
            assert_eq!(serde_json::to_value(&w.original_condition).unwrap(), x["original_condition"]);
            assert_eq!(w.original_opaque_paths, paths(x, "authored_opaque_nodes"));
            assert_eq!(w.compiled_operand_paths, paths(x, "compiled_operand_nodes"));
            assert!(w.original_opaque_nodes_preserved);
            assert!(!w.source_authority && !w.runtime_admission && !w.reference_closure_complete);
            for path in &w.original_opaque_paths {
                assert!(original_paths.insert((owner(c).to_owned(), path.clone())));
            }
            for path in &w.compiled_operand_paths {
                assert!(compiled_paths.insert((owner(c).to_owned(), path.clone())));
            }
            witnesses.push(w);
        }
    }
    assert_eq!(witnesses.len(), 8);
    assert_eq!(original_paths.len(), 16);
    assert_eq!(compiled_paths.len(), 16);
    // Optional existing-test-style evidence export, exclusive and never a verdict.
    if let Ok(path) = std::env::var("CURRENT_EQUALS_WITNESS_OUTPUT") {
        use std::io::Write;
        let mut out = std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
        out.write_all(&serde_json::to_vec_pretty(&witnesses).unwrap()).unwrap();
        out.sync_all().unwrap();
    }
}

#[test]
fn whole_compiled_card_metadata_and_unrelated_effect_drift_refuse() {
    let f = fixture();
    for c in cards(&f) {
        let x = &c["cases"][0];
        let mut changed = saved(c);
        changed.name.push_str(" altered");
        assert!(witness_equals_if(raw(c), owner(c), &location(x), &changed).is_err());
        let mut changed = saved(c);
        let unrelated_effect = changed.effects[0].clone();
        changed.effects.push(unrelated_effect);
        assert!(witness_equals_if(raw(c), owner(c), &location(x), &changed).is_err());
    }
}

#[test]
fn source_cost_drift_cannot_reuse_the_saved_whole_compiled_card() {
    let f = fixture();
    for c in cards(&f) {
        let text = c["source_utf8"].as_str().unwrap();
        let cost = text.lines().find(|line| line.starts_with("cost: ")).unwrap();
        let changed = text.replacen(cost, "cost: 99", 1);
        assert_ne!(changed.as_bytes(), raw(c));
        assert!(witness_equals_if(changed.as_bytes(), owner(c), &location(&c["cases"][0]), &saved(c)).is_err());
    }
}

#[test]
fn comment_only_change_is_preserved_not_misrepresented_as_old_source() {
    let f = fixture();
    let c = &cards(&f)[0];
    let mut changed = raw(c).to_vec();
    changed.extend_from_slice(b"\n# distinct current-witness test source\n");
    let w = witness_equals_if(&changed, owner(c), &location(&c["cases"][0]), &saved(c)).unwrap();
    assert_eq!(w.original_source_bytes, changed);
    assert_ne!(w.original_source_bytes, raw(c));
    assert_eq!(w.compiled_card, saved(c));
    assert!(!w.source_authority && !w.runtime_admission && !w.reference_closure_complete);
    // External exact-source pins distinguish this valid new witness from the old one.
}

#[test]
fn foreign_owner_and_foreign_compiled_card_refuse() {
    let f = fixture();
    for (i, c) in cards(&f).iter().enumerate() {
        let other = &cards(&f)[(i + 1) % 3];
        let l = location(&c["cases"][0]);
        assert!(witness_equals_if(raw(c), owner(other), &l, &saved(c)).is_err());
        assert!(witness_equals_if(raw(c), owner(c), &l, &saved(other)).is_err());
    }
}

#[test]
fn missing_effect_step_and_ancestor_locations_refuse() {
    let f = fixture();
    for c in cards(&f) {
        let good = location(&c["cases"][0]);
        let invalid = [
            IfLocation { effect: usize::MAX, ..good.clone() },
            IfLocation { step: usize::MAX, ..good.clone() },
            IfLocation { ancestors: vec![(usize::MAX, Branch::Then)], ..good },
        ];
        for bad in invalid {
            assert!(witness_equals_if(raw(c), owner(c), &bad, &saved(c)).is_err());
        }
    }
}

#[test]
fn then_else_and_other_valid_locations_remain_distinct_occurrences() {
    let f = fixture();
    let c = &cards(&f)[1];
    let then_witness = prove(c, &c["cases"][0]);
    let else_witness = prove(c, &c["cases"][2]);
    assert_ne!(then_witness.location, else_witness.location);
    assert_ne!(then_witness.binding, else_witness.binding);
    assert_ne!(then_witness.original_opaque_paths, else_witness.original_opaque_paths);
    assert_ne!(then_witness.compiled_operand_paths, else_witness.compiled_operand_paths);
    // A different valid location succeeds as itself, never as the requested old occurrence.
    assert_eq!(then_witness.compiled_card, else_witness.compiled_card);
}

#[test]
fn changed_binding_cannot_use_the_saved_original_compiled_record() {
    let f = fixture();
    let c = &cards(&f)[0];
    let changed = changed_first_condition(c, "{ equals: [another_binding, 0] }");
    assert!(witness_equals_if(&changed, owner(c), &location(&c["cases"][0]), &saved(c)).is_err());
}

#[test]
fn compiler_fallback_on_the_actual_card_is_not_lossless_correspondence() {
    let f = fixture();
    let c = &cards(&f)[0];
    for value in ["true", "false", "null", "1.5", "18446744073709551615", "{}", "[]"] {
        let changed = changed_first_condition(c, &format!("{{ equals: [branch, {value}] }}"));
        let spec: CardSpec = serde_yml::from_slice(&changed).unwrap();
        let current = compile(&spec).unwrap();
        assert!(witness_equals_if(&changed, owner(c), &location(&c["cases"][0]), &current).is_err(), "{value}");
    }
}

#[test]
fn actual_target_reordering_coercion_and_adjacent_predicates_refuse() {
    let f = fixture();
    let c = &cards(&f)[0];
    for replacement in [
        "{ equals: [0, branch] }",
        "{ equals: [branch, '0'] }",
        "{ equals: ['', 0] }",
        "{ equals: [branch, 0], binding_present: branch }",
    ] {
        let changed = changed_first_condition(c, replacement);
        let spec: CardSpec = serde_yml::from_slice(&changed).unwrap();
        let current = compile(&spec).unwrap();
        assert!(witness_equals_if(&changed, owner(c), &location(&c["cases"][0]), &current).is_err(), "{replacement}");
    }
    // This adjacent key is invalid DSL, so assert the precise compiler refusal.
    let changed = changed_first_condition(c, "{ equals: [branch, 0], future_unknown: 1 }");
    let spec: CardSpec = serde_yml::from_slice(&changed).unwrap();
    let errors = compile(&spec).expect_err("unknown predicate must be rejected");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].card_id, owner(c));
    assert_eq!(errors[0].path, "effects[0].process[2].if.condition");
    assert_eq!(errors[0].message, "if condition parse failed: unknown predicate key(s): future_unknown (not a PredicateSpec field — check the spelling against code/digimon-dsl/src/predicate.rs)");
    assert!(witness_equals_if(&changed, owner(c), &location(&c["cases"][0]), &saved(c)).is_err());
}
