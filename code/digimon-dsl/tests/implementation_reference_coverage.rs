//! Tool reference traversal only: these tests do not qualify card behavior.
#[path = "../src/bin/implementation_inventory/references.rs"]
mod references;
use references::{Collector, Node, Walk};
use serde_json::json;
use std::collections::BTreeSet;
fn nodes(value: &impl Walk) -> Vec<Node> {
    let mut collector = Collector { nodes: Vec::new(), blockers: BTreeSet::new(), complete: true };
    value.walk(&mut collector, "root", "UNCLASSIFIED_STRING");
    assert!(collector.complete);
    collector.nodes
}
fn has(nodes: &[Node], role: &str, value: &str) -> bool {
    nodes.iter().any(|node| node.role == role && node.value == value)
}
#[test]
fn new_predicate_fields_visit_nested_bindings_and_name_matches_in_both_forms() {
    let authored: digimon_dsl::predicate::PredicateSpec = serde_yml::from_str(r#"
can_play_from_effect_without_cost: true
can_play_to_breeding_from_hand: false
can_unsuspend: true
owner_can_add_security: true
controller_can_add_security: false
source_name_is: ST1-02
event_card_name_is: ST1-03
event_card_kind: tamer
event_battle_surviving_deleter: { binding_exists: ST1-04 }
"#).unwrap();
    let compiled = digimon_dsl::compiled::CompiledPredicate {
        can_play_from_effect_without_cost: Some(true), can_play_to_breeding_from_hand: Some(false),
        can_unsuspend: Some(true), owner_can_add_security: Some(true), controller_can_add_security: Some(false),
        source_name_is: Some("ST1-02".into()), event_card_name_is: Some("ST1-03".into()),
        event_card_kind: Some(digimon_dsl::compiled::CompiledCardKind::Tamer),
        event_battle_surviving_deleter: Some(Box::new(digimon_dsl::compiled::CompiledPredicate {
            binding_exists: Some("ST1-04".into()), ..Default::default()
        })), ..Default::default()
    };
    for rows in [nodes(&authored), nodes(&compiled)] {
        for field in ["can_play_from_effect_without_cost", "can_play_to_breeding_from_hand", "can_unsuspend",
            "owner_can_add_security", "controller_can_add_security", "event_card_kind"] {
            assert!(rows.iter().any(|node| node.path == format!("root.{field}")), "{field}");
        }
        assert!(has(&rows, "MATCH_NAME_EXACT", "ST1-02"));
        assert!(has(&rows, "MATCH_NAME_EXACT", "ST1-03"));
        assert!(has(&rows, "EXISTING_INSTANCE_BINDING", "ST1-04"));
        assert!(!has(&rows, "MATCH_CARD_ID", "ST1-02"));
    }
}
#[test]
fn top_card_references_and_runtime_finalizers_preserve_binding_roles() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate, CompiledStep};
    let authored: digimon_dsl::step::StructuredBindingRef = serde_yml::from_str("top_of: ST1-02").unwrap();
    for rows in [nodes(&authored), nodes(&CompiledBindingRef::TopOf("ST1-02".into()))] {
        assert!(has(&rows, "EXISTING_INSTANCE_BINDING", "ST1-02"));
    }
    let steps = vec![
        CompiledStep::FieldSourceFinalize { receipt: 7, bind_as: "CREATED".into(), source_anchor: Some((0, 42)),
            source_breeding: false, rebind: vec![("EXISTING".into(), (1, 99))] },
        CompiledStep::WithSimultaneousPlaySubject { binding: "SUBJECT".into(), body: vec![
            CompiledStep::TrashSelectedSourcesAsCost { source_refs: "PAYMENT".into(), min_count: Some(2) },
            CompiledStep::PaySourcePrefix { host_filter: CompiledPredicate {
                binding_present: Some("NESTED".into()), ..Default::default()
            }, minimum: 39, optional: false },
        ] },
        CompiledStep::DeleteReceiptFinalize { receipt: 8, cost_credit: Some((1, 11, 4)) },
    ];
    let rows = nodes(&steps);
    for binding in ["EXISTING", "SUBJECT", "PAYMENT", "NESTED"] {
        assert!(has(&rows, "EXISTING_INSTANCE_BINDING", binding), "{binding}");
        assert!(!has(&rows, "UNCLASSIFIED_STRING", binding));
    }
    assert!(has(&rows, "BINDING_DEFINITION", "CREATED"));
    assert!(rows.iter().any(|row| row.path.ends_with("cost_credit.2") && row.value == json!(4)));
}
#[test]
fn authored_new_steps_and_deck_evidence_are_traversed_without_blessing_references() {
    let steps: Vec<digimon_dsl::step::StepSpec> = serde_yml::from_str(r#"
- pay_source_prefix: { host_filter: { binding_exists: HOST }, minimum: 12 }
- trash_selected_sources_as_cost: { source_refs: COST, min_count: 2 }
- play_to_breeding_from_hand: { of: you, hand_index: HAND }
- return_all_to_deck: { over: { event_battle_surviving_deleter: { binding_exists: NESTED } }, position: bottom }
- suspend_and_cancel_all_matching_leaves: { target: TARGET, filter: { source_name_is: Name } }
"#).unwrap();
    let rows = nodes(&steps);
    for binding in ["HOST", "COST", "HAND", "NESTED", "TARGET"] {
        assert!(has(&rows, "EXISTING_INSTANCE_BINDING", binding), "{binding}");
    }
    assert!(has(&rows, "MATCH_NAME_EXACT", "Name"));
    let evidence = "a".repeat(64);
    let rows = nodes(&digimon_dsl::deck_rule::DeckCopyRule { max_copies: 5, evidence_observation_id: evidence.clone() });
    assert!(has(&rows, "METADATA_NO_EXECUTABLE_REFERENCE", &evidence));
    assert!(!has(&rows, "MATCH_CARD_ID", &evidence));
}

#[test]
fn runtime_security_play_receipts_walk_all_correlated_literal_fields() {
    use digimon_dsl::compiled::CompiledStep;
    let steps=vec![CompiledStep::SecurityPlayCaptured{receipt:7,player:1,card:23},CompiledStep::SecurityPlayFinalize{receipt:7,player:1,card:23}];
    let rows=nodes(&steps);
    for kind in ["SecurityPlayCaptured","SecurityPlayFinalize"] {
        for (field,value) in [("receipt",json!(7)),("player",json!(1)),("card",json!(23))] {
            assert!(rows.iter().any(|row|row.path.ends_with(&format!("@{kind}.{field}")) && row.value==value));
        }
    }
}
